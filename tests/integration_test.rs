use actix_web::{test, App}; 
use std::fs; 
use std::path::PathBuf; 
use stowage::AppState; 

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
    let app = test::init_service(
        App::new()
            .app_data(actix_web::web::Data::new(AppState {
                media_path: media_path.path().to_path_buf(), 
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