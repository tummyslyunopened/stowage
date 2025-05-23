use super::AppState; 
use actix_multipart::Multipart; 
use actix_web::{
    error, get, post, web, Error, HttpRequest, HttpResponse, Result,
}; 
use futures_util::stream::StreamExt; 
use std::io::Write; 
use uuid::Uuid; 
use actix_files::NamedFile; 
use std::io::Read; 

#[derive(serde::Serialize)]
pub struct FileUploadResponse {
    pub file_id: String,      
    pub download_url: String, 
}

fn get_filename_from_field(field: &actix_multipart::Field) -> String {
    let content_disposition = field.content_disposition();
    content_disposition.get_filename().unwrap_or("file").to_string()
}

async fn write_temp_file(mut field: actix_multipart::Field, temp_path: &std::path::Path) -> Result<(), Error> {
    let temp_path_clone = temp_path.to_path_buf(); 
    let mut file = web::block(move || std::fs::File::create(&temp_path_clone)).await
        .map_err(|e| error::ErrorBadRequest(format!("File create error: {:?}", e)))??;
    while let Some(chunk) = field.next().await {
        let chunk = chunk.map_err(|e| error::ErrorBadRequest(format!("Chunk error: {}", e)))?;
        file = web::block(move || file.write_all(&chunk).map(|_| file)).await
            .map_err(|e| error::ErrorBadRequest(format!("Write error: {:?}", e)))??;
    }
    Ok(())
}

fn is_mime_allowed(mime_type: &mime::Mime) -> bool {
    let mime_str = mime_type.to_string();
    is_mime_category_allowed(&mime_str) || is_mime_specific_allowed(&mime_str)
}

fn is_mime_category_allowed(mime_str: &str) -> bool {
    ["audio/", "video/", "image/", "application/octet-stream"]
        .iter().any(|&cat| mime_str.starts_with(cat))
}

fn is_mime_specific_allowed(mime_str: &str) -> bool {
    ["application/json", "text/xml", "application/rss+xml", "application/xml", "text/xml; charset=utf-8"]
        .contains(&mime_str)
}

fn is_content_type_allowed(mime: &str) -> bool {
    is_content_type_prefix_allowed(mime) || is_content_type_specific_allowed(mime)
}

fn is_content_type_prefix_allowed(mime: &str) -> bool {
    ["image/", "audio/", "video/"]
        .iter().any(|p| mime.starts_with(p))
}

fn is_content_type_specific_allowed(mime: &str) -> bool {
    ["application/json", "application/xml", "application/rss+xml", "text/xml", "text/xml; charset=utf-8"]
        .contains(&mime)
}

fn get_extension_fallback(_filename: &str) -> Option<String> {
    let ext = extract_extension(_filename);
    if is_allowed_text_ext(&ext) { Some(ext) } else { None }
}

fn extract_extension(filename: &str) -> String {
    filename.rsplit('.').next().unwrap_or("").to_ascii_lowercase()
}

fn is_allowed_text_ext(ext: &str) -> bool {
    ["json", "xml", "rss"].contains(&ext)
}

fn cleanup_and_error<P: AsRef<std::path::Path>>(temp_path: P, msg: String) -> Result<std::path::PathBuf, Error> {
    let _ = std::fs::remove_file(temp_path); 
    Err(error::ErrorBadRequest(msg))
}

fn detect_content_type(temp_path: &std::path::Path) -> std::io::Result<Vec<u8>> {
    let mut file_head = [0u8; 8192];
    let mut file = std::fs::File::open(temp_path)?;
    let n = file.read(&mut file_head)?;
    Ok(file_head[..n].to_vec())
}

fn validate_and_get_final_path(temp_path: &std::path::Path, file_path: &std::path::Path, _filename: &str) -> Result<std::path::PathBuf, Error> {
    
    let mime_type = mime_guess::from_path(&temp_path).first_or_octet_stream();
    if !is_mime_allowed(&mime_type) {
        
        return cleanup_and_error(&temp_path, format!("Invalid file type: {}/{}", mime_type.type_(), mime_type.subtype()));
    }
    
    let file_head = detect_content_type(temp_path).map_err(|e| error::ErrorBadRequest(format!("File read error: {:?}", e)))?;
    
    if let Some(kind) = infer::get(&file_head) {
        
        if !is_content_type_allowed(kind.mime_type()) {
            return cleanup_and_error(&temp_path, "File type not allowed".to_string());
        }
        
        Ok(file_path.with_extension(kind.extension()))
    } else if let Some(ext) = get_extension_fallback(_filename) {
        
        Ok(file_path.with_extension(ext))
    } else {
        
        cleanup_and_error(&temp_path, "Unknown or unsupported file type".to_string())
    }
}

fn rename_temp_file(temp_path: &std::path::Path, final_path: &std::path::Path) -> Result<(), Error> {
    std::fs::rename(&temp_path, &final_path).map_err(|e| error::ErrorBadRequest(format!("Rename error: {:?}", e)))
}

#[post("/upload")]
pub async fn upload_file(
    mut payload: Multipart,
    data: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    
    let file_id = Uuid::new_v4().to_string();
    
    let file_path = data.media_path.join(&file_id);
    
    let temp_path = file_path.with_extension("tmp");
    
    while let Some(item) = payload.next().await {
        let field = item.map_err(|e| error::ErrorBadRequest(format!("Multipart error: {}", e)))?;
        let _filename = get_filename_from_field(&field);
        eprintln!("DEBUG: filename={:?}", _filename); 
        write_temp_file(field, &temp_path).await?;
        eprintln!("DEBUG: Finished writing file: {:?}", temp_path);
        let final_path = validate_and_get_final_path(&temp_path, &file_path, &_filename)?;
        rename_temp_file(&temp_path, &final_path)?;
        eprintln!("DEBUG: Renamed file to {:?}", final_path);
        let download_url = format!("/files/{}", file_id);
        
        return Ok(HttpResponse::Created().json(FileUploadResponse {
            file_id,
            download_url,
        }));
    }
    
    eprintln!("DEBUG: No file provided in multipart");
    Err(error::ErrorBadRequest("No file provided"))
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