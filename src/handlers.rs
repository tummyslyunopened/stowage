use super::AppState;
use actix_multipart::Multipart;
use actix_web::{
    error, get, post, web, Error, HttpRequest, HttpResponse, Result,
};
use uuid::Uuid;
use actix_files::NamedFile;
use crate::file_utils::*;
use crate::multipart_utils::*;
use crate::db_utils;
use sha2::{Sha256, Digest};
use std::fs::File;
use futures_util::stream::StreamExt;

#[derive(serde::Serialize)]
pub struct JobStatusResponse {
    pub job_id: String,
    pub status: String,
    pub file_id: Option<i64>,
    pub download_url: Option<String>,
    pub error: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(serde::Serialize)]
pub struct FileUploadResponse {
    pub file_id: String,
    pub download_url: String,
    pub message: String,
}

#[derive(serde::Deserialize)]
pub struct DownloadRequest {
    pub download_url: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct DownloadResponse {
    pub job_id: String,
    pub status: String,
    pub status_url: String,
}

#[post("/download")]
pub async fn download_file(
    data: web::Data<AppState>,
    req: web::Json<DownloadRequest>,
) -> Result<HttpResponse, Error> {
    // Generate a new job ID
    let job_id = Uuid::new_v4().to_string();
    
    // Get a database connection
    let conn = data.db_pool.get()
        .map_err(|e| error::ErrorInternalServerError(e))?;
    
    // Insert the new job with NotStarted status and null file_id
    db_utils::insert_job(
        &conn,
        &job_id,
        &db_utils::JobStatus::NotStarted,
        None,
        &req.download_url
    ).map_err(|e| error::ErrorInternalServerError(e))?;
    
    // Return the job ID and status URL
    let status_url = format!("/jobs/{}", job_id);
    Ok(HttpResponse::Accepted().json(DownloadResponse {
        job_id: job_id.clone(),
        status: "NotStarted".to_string(),
        status_url: status_url.clone(),
    }))
}

#[get("/jobs/{job_id}")]
pub async fn get_job_status(
    path: web::Path<String>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let job_id = path.into_inner();
    let conn = data.db_pool.get()
        .map_err(|e| error::ErrorInternalServerError(e))?;
    
    match db_utils::get_job_by_id(&conn, &job_id) {
        Ok(Some(job)) => {
            let (status, _error): (&str, Option<String>) = match job.status {
                db_utils::JobStatus::NotStarted => ("NotStarted", None),
                db_utils::JobStatus::Running => ("Running", None),
                db_utils::JobStatus::Completed => ("Completed", None),
            };
            
            // Get file info if file_id exists
            let (_file_id, download_url) = if let Some(f_id) = job.file_id {
                match db_utils::get_file_by_id(&conn, f_id) {
                    Ok(file) => (Some(f_id), Some(file.url)),
                    Err(_) => (Some(f_id), None),
                }
            } else {
                (None, None)
            };
            
            // Get timestamps
            let created_at = conn.query_row::<String, _, _>(
                "SELECT created_at FROM Job WHERE id = ?1",
                [&job.id],
                |row| row.get(0)
            ).ok();
            
            let updated_at = conn.query_row::<String, _, _>(
                "SELECT updated_at FROM Job WHERE id = ?1",
                [&job.id],
                |row| row.get(0)
            ).ok();
            
            Ok(HttpResponse::Ok().json(JobStatusResponse {
                job_id: job.id,
                status: status.to_string(),
                file_id: job.file_id,
                download_url,
                error: job.error,
                created_at,
                updated_at,
            }))
        },
        Ok(None) => {
            Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": "Job not found"
            })))
        },
        Err(e) => {
            Err(error::ErrorInternalServerError(e))
        }
    }
}

#[post("/upload")]
pub async fn upload_file(
    mut payload: Multipart,
    data: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    
    let file_id = Uuid::new_v4().to_string();
    
    let file_path = data.media_path.join(&file_id);
    
    let temp_path = file_path.with_extension("tmp");
    
    if let Some(item) = payload.next().await {
        let field = item.map_err(|e| error::ErrorBadRequest(format!("Multipart error: {}", e)))?;
        let _filename = get_filename_from_field(&field);
        eprintln!("DEBUG: filename={:?}", _filename);
        write_temp_file(field, &temp_path).await?;
        eprintln!("DEBUG: Finished writing file: {:?}", temp_path);
        let final_path = validate_and_get_final_path(&temp_path, &file_path, &_filename)?;
        rename_temp_file(&temp_path, &final_path)?;
        eprintln!("DEBUG: Renamed file to {:?}", final_path);
        let download_url = format!("/files/{}", file_id);

        // Calculate hash
        let mut file = File::open(&final_path).map_err(|e| error::ErrorInternalServerError(e))?;
        let mut hasher = Sha256::new();
        std::io::copy(&mut file, &mut hasher).map_err(|e| error::ErrorInternalServerError(e))?;
        let hash = format!("{:x}", hasher.finalize());

        // Insert into DB or deduplicate
        let conn = data.db_pool.get().map_err(|e| error::ErrorInternalServerError(e))?;
        
        // Check for existing file with same hash
        match db_utils::get_filepath_by_hash(&conn, &hash).map_err(|e| error::ErrorInternalServerError(e))? {
            Some(existing_path) => {
                // Duplicate: delete new file, use original path in DB
                let _ = std::fs::remove_file(&final_path);
                
                // For duplicates, return 200 OK with the original file's URL
                let original_file_id = std::path::Path::new(&existing_path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                    
                let download_url = format!("/files/{}", original_file_id);
                Ok(HttpResponse::Ok().json(FileUploadResponse {
                    file_id: original_file_id,
                    download_url,
                    message: "File already exists".to_string(),
                }))
            },
            None => {
                // New file: insert as normal and return 201 Created
                db_utils::insert_file(&conn, final_path.to_string_lossy().as_ref(), &download_url, &hash)
                    .map_err(|e| error::ErrorInternalServerError(e))?;
                    
                Ok(HttpResponse::Created().json(FileUploadResponse {
                    file_id: file_id.clone(),
                    download_url: format!("/files/{}", file_id),
                    message: "File uploaded successfully".to_string(),
                }))
            }
        }
    } else {
        eprintln!("DEBUG: No file provided in multipart");
        Err(error::ErrorBadRequest("No file provided"))
    }
}

#[get("/files/{file_id}")]
pub async fn serve_file(
    path: web::Path<String>,
    data: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let file_id = path.into_inner();
    let dir_entries = std::fs::read_dir(&data.media_path)?;
    
    for entry in dir_entries {
        let entry = entry?;
        let path = entry.path();
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            if stem == file_id {
                
                let named_file = NamedFile::open_async(path).await?;
                return Ok(named_file.into_response(&req));
            }
        }
    }
    
    Err(error::ErrorNotFound("File not found"))
}

#[get("/about")]
pub async fn about() -> HttpResponse {
    HttpResponse::Ok().content_type("text/plain").body(
        "Stowage - File Server\n\
A high-performance file server for audio, video, images, RSS, and JSON files, built with Rust and Actix-web.\n\
\nFeatures:\n\
- RESTful API for file uploads\n\
- Unique, non-sequential file IDs\n\
- Optimized for serving various file types: audio, video, images, RSS/XML, JSON\n\
- Built with Docker for easy deployment\n\
- CORS support\n\
- File type validation\n\
- Configurable file size limits\n"
    )
}