mod config;
pub mod handlers;
use std::path::PathBuf;
pub use config::Config;
pub use handlers::{serve_file, upload_file, FileUploadResponse, about};

#[derive(Clone)]
pub struct AppState {
    
    pub media_path: PathBuf,
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
            .service(handlers::serve_file)
            .service(handlers::about)
    );
}

pub fn routes(cfg: &mut actix_web::web::ServiceConfig) {
    config(cfg);
}