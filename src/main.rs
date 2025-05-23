use actix_web::{web, App, HttpServer};
use stowage::{AppState, config};
use std::env;
use std::path::PathBuf;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("Invalid PORT value");
    let media_path = env::var("MEDIA_PATH").unwrap_or_else(|_| "./media".to_string());
    std::fs::create_dir_all(&media_path)?;
    log::info!("Starting server on {}:{}", host, port);
    log::info!("Serving files from: {}", media_path);
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                media_path: PathBuf::from(&media_path),
            }))
            .configure(config)
    })
    .bind((host, port))?
    .run()
    .await
}