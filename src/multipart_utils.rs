use actix_multipart::Field;
use actix_web::{web, Error};
use futures_util::stream::StreamExt;
use std::io::Write;

pub fn get_filename_from_field(field: &Field) -> String {
    let content_disposition = field.content_disposition();
    content_disposition.get_filename().unwrap_or("file").to_string()
}

pub async fn write_temp_file(mut field: Field, temp_path: &std::path::Path) -> Result<(), Error> {
    let temp_path_clone = temp_path.to_path_buf();
    let mut file = web::block(move || std::fs::File::create(&temp_path_clone)).await
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("File create error: {:?}", e)))??;
    while let Some(chunk) = field.next().await {
        let chunk = chunk.map_err(|e| actix_web::error::ErrorBadRequest(format!("Chunk error: {}", e)))?;
        file = web::block(move || file.write_all(&chunk).map(|_| file)).await
            .map_err(|e| actix_web::error::ErrorBadRequest(format!("Write error: {:?}", e)))??;
    }
    Ok(())
}
