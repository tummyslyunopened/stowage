mod config;
pub mod handlers;
pub mod file_utils;
pub mod multipart_utils;
pub mod db_utils;
use std::path::PathBuf;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
pub use config::Config;
pub use handlers::{
    serve_file, upload_file, download_file, get_job_status,
    FileUploadResponse, DownloadResponse, about
};

#[derive(Clone)]
pub struct AppState {
    pub media_path: PathBuf,
    pub db_pool: Pool<SqliteConnectionManager>,
}

pub fn config(cfg: &mut actix_web::web::ServiceConfig) {
    
    let cors = actix_cors::Cors::default()
        .allow_any_origin()
        .allow_any_method()
        .allow_any_header()
        .max_age(3600);
    
    cfg.service(
        actix_web::web::scope("")
            .wrap(cors)
            .service(handlers::upload_file)
            .service(handlers::download_file)
            .service(handlers::get_job_status)
            .service(handlers::serve_file)
            .service(handlers::about)
    );
}

pub fn routes(cfg: &mut actix_web::web::ServiceConfig) {
    config(cfg);
}