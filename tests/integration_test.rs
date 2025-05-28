use sha2::{Sha256, Digest};

#[actix_web::test]
async fn test_db_contents_after_upload() {
    init_test_logger();
    let media_path = tempfile::tempdir().unwrap();
    let db_file = tempfile::NamedTempFile::new().unwrap();
    let manager = r2d2_sqlite::SqliteConnectionManager::file(db_file.path());
    let db_pool = r2d2::Pool::new(manager).unwrap();
    {
        let conn = db_pool.get().unwrap();
        stowage::db_utils::init_db(&conn).unwrap();
    }
    let app = test::init_service(
        App::new()
            .app_data(actix_web::web::Data::new(stowage::AppState {
                media_path: media_path.path().to_path_buf(),
                db_pool: db_pool.clone(),
                worker: None
            }))
            .configure(stowage::routes),
    )
    .await;

    let mut data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    data_dir.push(".data");
    let file_path = data_dir.join("example.json");
    let file_bytes = fs::read(&file_path).unwrap();
    // Precompute hash
    let mut hasher = Sha256::new();
    hasher.update(&file_bytes);
    let expected_hash = format!("{:x}", hasher.finalize());

    let boundary = "XBOUNDARY";
    let body = build_multipart_body("file", "example.json", &file_bytes, boundary);

    let req = test::TestRequest::post()
        .uri("/upload")
        .insert_header(("content-type", format!("multipart/form-data; boundary={}", boundary)))
        .set_payload(body)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201, "File should upload");
    let body = test::read_body(resp).await;
    let resp_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let file_id = resp_json["file_id"].as_str().unwrap();
    let download_url = format!("/files/{}", file_id);

    // Check DB contents
    let conn = db_pool.get().unwrap();
    let mut stmt = conn.prepare("SELECT filepath, url, hash FROM File WHERE url = ?1").unwrap();
    let mut rows = stmt.query([&download_url]).unwrap();
    let row = rows.next().unwrap().expect("No row found in DB");
    let db_filepath: String = row.get(0).unwrap();
    let db_url: String = row.get(1).unwrap();
    let db_hash: String = row.get(2).unwrap();
    assert_eq!(db_url, download_url);
    assert_eq!(db_hash, expected_hash);
    // The file should exist on disk
    assert!(std::path::Path::new(&db_filepath).exists());
}
use actix_web::{test, App, http::StatusCode}; 
use std::fs; 
use std::path::PathBuf;
use serde_json::json; 

fn build_multipart_body(field_name: &str, file_name: &str, file_bytes: &[u8], boundary: &str) -> Vec<u8> {
    let mut body = Vec::new(); 
    use std::io::Write; 
    write!(
        body,
        "--{}\r\nContent-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\nContent-Type: application/octet-stream\r\n\r\n",
        boundary, field_name, file_name
    ).unwrap(); 
    body.extend_from_slice(file_bytes); 
    write!(body, "\r\n--{}--\r\n", boundary).unwrap(); 
    body 
}


async fn upload_and_download(file_name: &str, should_succeed: bool) {
    let media_path = tempfile::tempdir().unwrap(); 
    let db_file = tempfile::NamedTempFile::new().unwrap();
    let manager = r2d2_sqlite::SqliteConnectionManager::file(db_file.path());
    let db_pool = r2d2::Pool::new(manager).unwrap();
    {
        let conn = db_pool.get().unwrap();
        stowage::db_utils::init_db(&conn).unwrap();
    }
    let app = test::init_service(
        App::new()
            .app_data(actix_web::web::Data::new(stowage::AppState {
                media_path: media_path.path().to_path_buf(),
                db_pool: db_pool.clone(),
                worker: None
            }))
            .configure(stowage::routes),
    )
    .await;

    let mut data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")); 
    data_dir.push(".data"); 
    let file_path = data_dir.join(file_name); 
    let file_bytes = fs::read(&file_path).unwrap(); 
    let boundary = "XBOUNDARY"; 
    let body = build_multipart_body("file", file_name, &file_bytes, boundary); 

    let req = test::TestRequest::post()
        .uri("/upload")
        .insert_header(("content-type", format!("multipart/form-data; boundary={}", boundary)))
        .set_payload(body)
        .to_request(); 

    let resp = test::call_service(&app, req).await; 

    if should_succeed {
        assert_eq!(resp.status(), 201, "File {} should upload", file_name); 
        let body = test::read_body(resp).await; 
        let resp_json: serde_json::Value = serde_json::from_slice(&body).unwrap(); 
        let file_id = resp_json["file_id"].as_str().unwrap(); 
        let download_uri = format!("/files/{}", file_id); 
        let req = test::TestRequest::get().uri(&download_uri).to_request(); 
        let resp = test::call_service(&app, req).await; 
        assert_eq!(resp.status(), 200, "Download should succeed for {}", file_name); 
    } else {
        assert_eq!(resp.status(), 400, "File {} should be rejected", file_name); 
    }
}

#[cfg(test)] 
fn init_test_logger() {
    let _ = env_logger::builder().is_test(true).try_init(); 
}

#[actix_web::test]
async fn test_upload_and_download_json() {
    init_test_logger();
    upload_and_download("example.json", true).await;
}

#[actix_web::test]
async fn test_upload_and_download_mp3() {
    init_test_logger();
    upload_and_download("example.mp3", true).await;
}

#[actix_web::test]
async fn test_upload_and_download_png() {
    init_test_logger();
    upload_and_download("example.png", true).await;
}

#[actix_web::test]
async fn test_upload_and_download_xml() {
    init_test_logger();
    upload_and_download("example.xml", true).await;
}

#[actix_web::test]
async fn test_upload_and_download_exe() {
    init_test_logger();
    upload_and_download("example.exe", false).await;
}

#[actix_web::test]
async fn test_upload_and_download_disguised_mp3() {
    init_test_logger();
    upload_and_download("disguised.mp3", false).await;
}

#[actix_web::test]
async fn test_upload_and_download_disguised_png() {
    init_test_logger();
    upload_and_download("disguised.png", false).await;
}

#[actix_web::test]
async fn test_upload_and_download_disguised_xml() {
    init_test_logger();
    upload_and_download("disguised.xml", false).await;
}

#[actix_web::test]
async fn test_upload_same_filename_different_content() {
    init_test_logger();
    
    // Setup test environment
    let media_path = tempfile::tempdir().unwrap();
    let db_file = tempfile::NamedTempFile::new().unwrap();
    let manager = r2d2_sqlite::SqliteConnectionManager::file(db_file.path());
    let db_pool = r2d2::Pool::new(manager).unwrap();
    {
        let conn = db_pool.get().unwrap();
        stowage::db_utils::init_db(&conn).unwrap();
    }
    let app = test::init_service(
        App::new()
            .app_data(actix_web::web::Data::new(stowage::AppState {
                media_path: media_path.path().to_path_buf(),
                db_pool: db_pool.clone(),
                worker: None
            }))
            .configure(stowage::routes),
    )
    .await;

    // Get paths to both files
    let mut data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    data_dir.push(".data");
    let file1_path = data_dir.join("example.json");
    let file2_path = data_dir.join(".dup/example.json");

    // Read both files
    let file1_bytes = fs::read(&file1_path).unwrap();
    let file2_bytes = fs::read(&file2_path).unwrap();

    // Upload first file
    let boundary1 = "BOUNDARY1";
    let body1 = build_multipart_body("file", "example.json", &file1_bytes, boundary1);
    let req1 = test::TestRequest::post()
        .uri("/upload")
        .insert_header(("content-type", format!("multipart/form-data; boundary={}", boundary1)))
        .set_payload(body1)
        .to_request();
    let resp1 = test::call_service(&app, req1).await;
    assert_eq!(resp1.status(), 201, "First file should upload successfully");
    let body1 = test::read_body(resp1).await;
    let resp_json1: serde_json::Value = serde_json::from_slice(&body1).unwrap();
    let download_url1 = resp_json1["download_url"].as_str().unwrap().to_string();
    
    // Upload second file with same name but different content
    let boundary2 = "BOUNDARY2";
    let body2 = build_multipart_body("file", "example.json", &file2_bytes, boundary2);
    let req2 = test::TestRequest::post()
        .uri("/upload")
        .insert_header(("content-type", format!("multipart/form-data; boundary={}", boundary2)))
        .set_payload(body2)
        .to_request();
    let resp2 = test::call_service(&app, req2).await;
    assert_eq!(resp2.status(), 201, "Second file should upload successfully");
    let body2 = test::read_body(resp2).await;
    let resp_json2: serde_json::Value = serde_json::from_slice(&body2).unwrap();
    let download_url2 = resp_json2["download_url"].as_str().unwrap().to_string();
    
    // Verify different URLs were generated
    assert_ne!(
        download_url1, download_url2,
        "Different files with same name should have different download URLs"
    );
}

#[actix_web::test]
async fn test_upload_same_content_different_filename() {
    init_test_logger();
    
    // Setup test environment
    let media_path = tempfile::tempdir().unwrap();
    let db_file = tempfile::NamedTempFile::new().unwrap();
    let manager = r2d2_sqlite::SqliteConnectionManager::file(db_file.path());
    let db_pool = r2d2::Pool::new(manager).unwrap();
    {
        let conn = db_pool.get().unwrap();
        stowage::db_utils::init_db(&conn).unwrap();
    }
    let app = test::init_service(
        App::new()
            .app_data(actix_web::web::Data::new(stowage::AppState {
                media_path: media_path.path().to_path_buf(),
                db_pool: db_pool.clone(),
                worker: None
            }))
            .configure(stowage::routes),
    )
    .await;

    // Get path to the test file
    let mut data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    data_dir.push(".data");
    let file_path = data_dir.join("example.json");
    let file_bytes = fs::read(&file_path).unwrap();

    // Upload first file
    let boundary1 = "BOUNDARY1";
    let body1 = build_multipart_body("file", "original.json", &file_bytes, boundary1);
    let req1 = test::TestRequest::post()
        .uri("/upload")
        .insert_header(("content-type", format!("multipart/form-data; boundary={}", boundary1)))
        .set_payload(body1)
        .to_request();
    let resp1 = test::call_service(&app, req1).await;
    assert_eq!(resp1.status(), 201, "First file should upload successfully");
    let body1 = test::read_body(resp1).await;
    let resp_json1: serde_json::Value = serde_json::from_slice(&body1).unwrap();
    let download_url1 = resp_json1["download_url"].as_str().unwrap().to_string();
    let file_id1 = resp_json1["file_id"].as_str().unwrap().to_string();
    
    // Get the count of records before second upload
    let conn = db_pool.get().unwrap();
    let count_before: i64 = conn.query_row(
        "SELECT COUNT(*) FROM File",
        [],
        |row| row.get(0)
    ).unwrap();

    // Upload same content with different filename
    let boundary2 = "BOUNDARY2";
    let body2 = build_multipart_body("file", "duplicate.json", &file_bytes, boundary2);
    let req2 = test::TestRequest::post()
        .uri("/upload")
        .insert_header(("content-type", format!("multipart/form-data; boundary={}", boundary2)))
        .set_payload(body2)
        .to_request();
    let resp2 = test::call_service(&app, req2).await;
    assert_eq!(resp2.status(), 200, "Duplicate content should return 200 with original URL");
    let body2 = test::read_body(resp2).await;
    let resp_json2: serde_json::Value = serde_json::from_slice(&body2).unwrap();
    let download_url2 = resp_json2["download_url"].as_str().unwrap().to_string();
    
    // Verify the same URL is returned
    assert_eq!(
        download_url1, download_url2,
        "Same content should return the same download URL regardless of filename"
    );
    
    // Verify no new record was added to the database
    let count_after: i64 = conn.query_row(
        "SELECT COUNT(*) FROM File",
        [],
        |row| row.get(0)
    ).unwrap();
    assert_eq!(
        count_before, count_after,
        "No new database record should be created for duplicate content"
    );
    
    // Verify the file_id in the response matches the first upload
    let file_id2 = resp_json2["file_id"].as_str().unwrap().to_string();
    assert_eq!(
        file_id1, file_id2,
        "Same file_id should be returned for duplicate content"
    );
}

#[actix_web::test]
async fn test_download_job_creation() {
    init_test_logger();
    let media_path = tempfile::tempdir().unwrap();
    let db_file = tempfile::NamedTempFile::new().unwrap();
    let manager = r2d2_sqlite::SqliteConnectionManager::file(db_file.path());
    let db_pool = r2d2::Pool::new(manager).unwrap();
    
    // Initialize the database
    {
        let conn = db_pool.get().unwrap();
        stowage::db_utils::init_db(&conn).unwrap();
    }
    
    // Create test app
    let app = test::init_service(
        App::new()
            .app_data(actix_web::web::Data::new(stowage::AppState {
                media_path: media_path.path().to_path_buf(),
                db_pool: db_pool.clone(),
                worker: None
            }))
            .configure(stowage::routes),
    )
    .await;

    // Test data
    let test_url = "https://example.com/this/does/not/exist.txt";
    
    // 1. Make a request to create a download job
    let req = test::TestRequest::post()
        .uri("/download")
        .set_json(&json!({"download_url": test_url}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    
    // Verify the response
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: stowage::handlers::DownloadResponse = test::read_body_json(resp).await;
    assert!(!body.job_id.is_empty());
    assert!(body.status_url.ends_with(&format!("/jobs/{}", body.job_id)));
    
    // 2. Verify the job was created in the database
    let conn = db_pool.get().unwrap();
    let job = stowage::db_utils::get_job_by_id(&conn, &body.job_id)
        .expect("Failed to get job from database")
        .expect("Job not found in database");
    
    assert_eq!(job.id, body.job_id);
    assert_eq!(job.status, stowage::db_utils::JobStatus::NotStarted);
    assert_eq!(job.download_url, test_url);
    assert!(job.file_id.is_none(), "File ID should be None for new job");
    
    // 3. Verify the job status endpoint
    let status_req = test::TestRequest::get()
        .uri(&body.status_url)
        .to_request();
    let status_resp = test::call_service(&app, status_req).await;
    
    assert_eq!(status_resp.status(), StatusCode::OK);
    let status_body: serde_json::Value = test::read_body_json(status_resp).await;
    assert_eq!(status_body["job_id"], body.job_id);
    assert_eq!(status_body["status"], "NotStarted");
    assert_eq!(status_body["download_url"], test_url);
    assert_eq!(status_body["file_id"], serde_json::Value::Null);
}

// #[actix_web::test]
// async fn test_download_and_poll_until_complete() {
//     init_test_logger();
//     let media_path = tempfile::tempdir().unwrap();
//     let db_file = tempfile::NamedTempFile::new().unwrap();
//     let manager = r2d2_sqlite::SqliteConnectionManager::file(db_file.path());
//     let db_pool = r2d2::Pool::new(manager).unwrap();
    
//     // Initialize the database
//     {
//         let conn = db_pool.get().unwrap();
//         stowage::db_utils::init_db(&conn).unwrap();
//     }
    
// let state = stowage::create_app_state(
//     media_path.path().to_path_buf(),
//     db_pool.clone(),
//     5, // max_concurrent_downloads
// ).await;

// let app = test::init_service(
//     App::new()
//         .app_data(actix_web::web::Data::new(state))
//         .configure(stowage::routes),
// )
// .await;
//     // Test data - using the specified URL
//     let test_url = "http://kneecap.2wu.me/media/transcripts/052ef596-51d5-4133-bc27-8f1a84bd179b.m4a.json";
    
//     // 1. Make a request to create a download job
//     let req = test::TestRequest::post()
//         .uri("/download")
//         .set_json(&json!({ "download_url": test_url }))
//         .to_request();
//     let resp = test::call_service(&app, req).await;
    
//     // Verify the response
//     assert_eq!(resp.status(), StatusCode::ACCEPTED);
//     let body: serde_json::Value = test::read_body_json(resp).await;
//     let job_id = body["job_id"].as_str().expect("Job ID should be a string");
//     let status_url = body["status_url"].as_str().expect("Status URL should be a string");
    
//     assert!(!job_id.is_empty());
//     assert!(status_url.ends_with(&format!("/jobs/{}", job_id)));
    
//     // 2. Poll the status until completion or timeout
//     let max_attempts = 60; // 60 seconds max
//     let mut attempts = 0;
//     let mut completed = false;
    
//     while attempts < max_attempts && !completed {
//         attempts += 1;
        
//         // Get job status
//         let status_req = test::TestRequest::get()
//             .uri(status_url)
//             .to_request();
//         let status_resp = test::call_service(&app, status_req).await;
//         assert_eq!(status_resp.status(), StatusCode::OK);
        
//         let status_body: serde_json::Value = test::read_body_json(status_resp).await;
//         let status = status_body["status"].as_str().expect("Status should be a string");
        
//         match status {
//             "Completed" => {
//                 completed = true;
//                 // Verify the file_id is present
//                 assert_ne!(status_body["file_id"], serde_json::Value::Null, "File ID should be set for completed job");
                
//                 // Optional: Download the file and verify it's not empty
//                 let file_id = status_body["file_id"].as_str().expect("File ID should be a string");
//                 let download_req = test::TestRequest::get()
//                     .uri(&format!("/files/{}", file_id))
//                     .to_request();
//                 let download_resp = test::call_service(&app, download_req).await;
//                 assert_eq!(download_resp.status(), StatusCode::OK);
                
//                 let file_content = test::read_body(download_resp).await;
//                 assert!(!file_content.is_empty(), "Downloaded file should not be empty");
//             },
//             "Failed" => {
//                 panic!("Download job failed: {:?}", status_body["error"]);
//             },
//             _ => {
//                 // Still in progress, wait a bit before polling again
//                 std::thread::sleep(std::time::Duration::from_secs(1));
//             }
//         }
//     }
    
//     assert!(completed, "Download did not complete within {} seconds", max_attempts);
// }

#[actix_web::test]
async fn test_about_route() {
    let app = test::init_service(
        App::new().configure(stowage::routes)
    ).await;

    let req = test::TestRequest::get().uri("/about").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body = test::read_body(resp).await;
    let expected = "Stowage - File Server\n\
A high-performance file server for audio, video, images, RSS, and JSON files, built with Rust and Actix-web.\n\
\nFeatures:\n\
- RESTful API for file uploads\n\
- Unique, non-sequential file IDs\n\
- Optimized for serving various file types: audio, video, images, RSS/XML, JSON\n\
- Built with Docker for easy deployment\n\
- CORS support\n\
- File type validation\n\
- Configurable file size limits\n";
    assert_eq!(std::str::from_utf8(&body).unwrap(), expected);
}