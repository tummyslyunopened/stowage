use actix_web::Error;
use std::io::Read;
pub fn validate_and_get_final_path(temp_path: &std::path::Path, file_path: &std::path::Path, _filename: &str) -> Result<std::path::PathBuf, Error> {
    let mime_type = mime_guess::from_path(&temp_path).first_or_octet_stream();
    if !is_mime_allowed(&mime_type) {
        return cleanup_and_error(&temp_path, format!("Invalid file type: {}/{}", mime_type.type_(), mime_type.subtype()));
    }
    let file_head = detect_content_type(temp_path).map_err(|e| actix_web::error::ErrorBadRequest(format!("File read error: {:?}", e)))?;
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

pub fn rename_temp_file(temp_path: &std::path::Path, final_path: &std::path::Path) -> Result<(), Error> {
    std::fs::rename(&temp_path, &final_path).map_err(|e| actix_web::error::ErrorBadRequest(format!("Rename error: {:?}", e)))
}

pub fn is_mime_allowed(mime_type: &mime::Mime) -> bool {
    let mime_str = mime_type.to_string();
    is_mime_category_allowed(&mime_str) || is_mime_specific_allowed(&mime_str)
}

pub fn is_mime_category_allowed(mime_str: &str) -> bool {
    ["audio/", "video/", "image/", "application/octet-stream"]
        .iter().any(|&cat| mime_str.starts_with(cat))
}

pub fn is_mime_specific_allowed(mime_str: &str) -> bool {
    ["application/json", "text/xml", "application/rss+xml", "application/xml", "text/xml; charset=utf-8"]
        .contains(&mime_str)
}

pub fn is_content_type_allowed(mime: &str) -> bool {
    is_content_type_prefix_allowed(mime) || is_content_type_specific_allowed(mime)
}

pub fn is_content_type_prefix_allowed(mime: &str) -> bool {
    ["image/", "audio/", "video/"]
        .iter().any(|p| mime.starts_with(p))
}

pub fn is_content_type_specific_allowed(mime: &str) -> bool {
    ["application/json", "application/xml", "application/rss+xml", "text/xml", "text/xml; charset=utf-8"]
        .contains(&mime)
}

pub fn get_extension_fallback(_filename: &str) -> Option<String> {
    let ext = extract_extension(_filename);
    if is_allowed_text_ext(&ext) { Some(ext) } else { None }
}

pub fn extract_extension(filename: &str) -> String {
    filename.rsplit('.').next().unwrap_or("").to_ascii_lowercase()
}

pub fn is_allowed_text_ext(ext: &str) -> bool {
    ["json", "xml", "rss"].contains(&ext)
}

pub fn cleanup_and_error<P: AsRef<std::path::Path>>(temp_path: P, msg: String) -> Result<std::path::PathBuf, Error> {
    let _ = std::fs::remove_file(temp_path);
    Err(actix_web::error::ErrorBadRequest(msg))
}

pub fn detect_content_type(temp_path: &std::path::Path) -> std::io::Result<Vec<u8>> {
    let mut file_head = [0u8; 8192];
    let mut file = std::fs::File::open(temp_path)?;
    let n = file.read(&mut file_head)?;
    Ok(file_head[..n].to_vec())
}
