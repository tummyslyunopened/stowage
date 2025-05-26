use actix_web::{web, App, HttpServer};
use log;
use r2d2;
use stowage::{self, config, db_utils};
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
    let db_path = env::var("DB_PATH").unwrap_or_else(|_| "stowage.db".to_string());
    let manager = r2d2_sqlite::SqliteConnectionManager::file(&db_path);
    let db_pool = r2d2::Pool::new(manager).expect("Failed to create DB pool");
    {
        let conn = db_pool.get().expect("Failed to get DB connection");
        db_utils::init_db(&conn).expect("Failed to initialize DB");
    }
    // Get max concurrent downloads from env or use a default of 5
    let max_concurrent_downloads = env::var("MAX_CONCURRENT_DOWNLOADS")
        .unwrap_or_else(|_| "5".to_string())
        .parse::<usize>()
        .expect("Invalid MAX_CONCURRENT_DOWNLOADS value");

    log::info!("Starting server on {}:{}", host, port);
    log::info!("Serving files from: {}", media_path);
    log::info!("Max concurrent downloads: {}", max_concurrent_downloads);

    // Create app state with worker
    let app_state = stowage::create_app_state(
        PathBuf::from(&media_path),
        db_pool.clone(),
        max_concurrent_downloads,
    ).await;

    // Start the HTTP server
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .configure(config)
    })
    .bind((host, port))?
    .run();

    // Wait for server to finish
    server.await
}