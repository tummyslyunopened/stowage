use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::sleep;
use log::{info, error, debug};
use sha2::{Sha256, Digest};
use std::fs::File;
use std::io::Write;

use crate::db_utils;
use crate::AppState;

#[derive(Debug, Clone)]
pub struct DownloadWorker {
    state: Arc<AppState>,
    max_concurrent_downloads: usize,
    running: Arc<AtomicBool>,
}

impl DownloadWorker {
    pub fn new(state: Arc<AppState>, max_concurrent_downloads: usize) -> Self {
        Self {
            state,
            max_concurrent_downloads,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn start(&self) {
        if self.running.swap(true, Ordering::SeqCst) {
            info!("Worker is already running");
            return;
        }

        let running = self.running.clone();
        let worker = self.clone();

        tokio::spawn(async move {
            info!("Starting download worker with {} max concurrent downloads", worker.max_concurrent_downloads);
            let semaphore = Arc::new(Semaphore::new(worker.max_concurrent_downloads));
            
            while running.load(Ordering::SeqCst) {
                match worker.process_next_job(semaphore.clone()).await {
                    Ok(processed) => {
                        if !processed {
                            // No jobs to process, sleep for a bit
                            sleep(Duration::from_secs(1)).await;
                        }
                    }
                    Err(e) => {
                        error!("Error processing job: {}", e);
                        sleep(Duration::from_secs(5)).await; // Back off on error
                    }
                }
            }
            info!("Download worker stopped");
        });
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    async fn process_next_job(&self, semaphore: Arc<Semaphore>) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let conn = self.state.db_pool.get()?;
        
        // Find a job that's not started and mark it as running
        match db_utils::get_and_start_job(&conn)? {
            Some(job) => {
                let worker = self.clone();
                let permit = semaphore.acquire_owned().await?;
                
                tokio::spawn(async move {
                    debug!("Processing job: {}", job.id);
                    
                    // Download the file
                    let result = worker.download_file(&job.id, &job.download_url).await;
                    
                    // Update job status
                    let conn = match worker.state.db_pool.get() {
                        Ok(conn) => conn,
                        Err(e) => {
                            error!("Failed to get DB connection: {}", e);
                            return;
                        }
                    };
                    
                    match result {
                        Ok(file_id) => {
                            if let Err(e) = db_utils::complete_job(&conn, &job.id, file_id) {
                                error!("Failed to mark job as completed: {}", e);
                            } else {
                                info!("Successfully processed job: {}", job.id);
                            }
                        }
                        Err(e) => {
                            error!("Job {} failed: {}", job.id, e);
                            if let Err(e) = db_utils::fail_job(&conn, &job.id, &e.to_string()) {
                                error!("Failed to mark job as failed: {}", e);
                            }
                        }
                    }
                    
                    drop(permit); // Release the semaphore permit
                });
                
                Ok(true) // Processed a job
            }
            None => Ok(false), // No jobs to process
        }
    }

    async fn download_file(&self, job_id: &str, url: &str) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
        // Create a temporary file path
        let temp_path = self.state.media_path.join(format!("{}.tmp", job_id));
        let final_path = self.state.media_path.join(job_id);
        
        // Download the file
        let response = reqwest::get(url).await?;
        if !response.status().is_success() {
            return Err(format!("Failed to download file: {}", response.status()).into());
        }
        
        // Get content type before consuming the response
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|ct| ct.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();
            
        let content = response.bytes().await?;
        
        // Calculate hash
        let hash = format!("{:x}", Sha256::digest(&content));
        
        // Check for duplicates
        let conn = self.state.db_pool.get()?;
        if let Some(existing_path) = db_utils::get_filepath_by_hash(&conn, &hash)? {
            // File already exists, return the existing file ID
            let file_id = db_utils::get_file_id_by_path(&conn, &existing_path)?;
            return Ok(file_id);
        }
        
        // Write to temporary file first
        let mut file = File::create(&temp_path)?;
        file.write_all(&content)?;
        
        // Determine file extension from content type
        let extension = match content_type.split('/').nth(1) {
            Some(ext) => format!(".{}", ext.split(';').next().unwrap_or("")),
            None => String::new(),
        };
        
        // Rename to final path with extension
        let final_path = final_path.with_extension(extension);
        std::fs::rename(&temp_path, &final_path)?;
        
        // Insert file record
        let download_url = format!("/files/{}", job_id);
        let file_id = db_utils::insert_file(&conn, final_path.to_str().unwrap(), &download_url, &hash)?;
        
        Ok(file_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::sync::atomic::Ordering;
    
    #[tokio::test]
    async fn test_worker_stop() {
        let temp_dir = tempdir().unwrap();
        let db_file = tempfile::NamedTempFile::new().unwrap();
        let manager = r2d2_sqlite::SqliteConnectionManager::file(db_file.path());
        let db_pool = r2d2::Pool::new(manager).unwrap();
        
        // Initialize database
        {
            let conn = db_pool.get().unwrap();
            db_utils::init_db(&conn).unwrap();
        }
        
        let state = Arc::new(AppState {
            media_path: temp_dir.path().to_path_buf(),
            db_pool: db_pool.clone(),
            worker: None,
        });
        
        let worker = DownloadWorker::new(Arc::clone(&state), 5);
        
        // Start the worker
        worker.start().await;
        
        // Give it a moment to start
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Stop the worker
        worker.stop();
        
        // Give it a moment to stop
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        assert!(!worker.running.load(Ordering::SeqCst));
    }
}
