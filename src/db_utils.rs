// src/db_utils.rs
use rusqlite::{params, Connection, Result};

// Job status enum for Job table
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum JobStatus {
    NotStarted,
    Running,
    Completed,
}

impl ToString for JobStatus {
    fn to_string(&self) -> String {
        match self {
            JobStatus::NotStarted => "NotStarted".to_string(),
            JobStatus::Running => "Running".to_string(),
            JobStatus::Completed => "Completed".to_string(),
        }
    }
}

impl std::str::FromStr for JobStatus {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "NotStarted" => Ok(JobStatus::NotStarted),
            "Running" => Ok(JobStatus::Running),
            "Completed" => Ok(JobStatus::Completed),
            _ => Err(()),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct JobRecord {
    pub id: String, // UUID
    pub status: JobStatus,
    pub file_id: Option<i64>,
    pub download_url: String,
    pub error: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct FileRecord {
    pub id: i64,
    pub filepath: String,
    pub url: String,
    pub hash: String,
}

pub fn init_db(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS File (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            filepath TEXT NOT NULL,
            url TEXT NOT NULL,
            hash TEXT NOT NULL
        )",
        [],
    )?;

    // Create Job table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS Job (
            id TEXT PRIMARY KEY, -- UUID as string
            status TEXT NOT NULL, -- 'NotStarted', 'Running', 'Completed', 'Failed'
            file_id INTEGER,
            download_url TEXT NOT NULL,
            error TEXT, -- Error message if the job failed
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(file_id) REFERENCES File(id)
        )",
        [],
    )?;
    
    // Add error column if it doesn't exist (for existing databases)
    let _ = conn.execute(
        "ALTER TABLE Job ADD COLUMN error TEXT",
        [],
    );
    
    // Add created_at and updated_at columns if they don't exist
    let _ = conn.execute(
        "ALTER TABLE Job ADD COLUMN created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE Job ADD COLUMN updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP",
        [],
    );
    
    // Create index on status for faster lookups
    let _ = conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_job_status ON Job(status)",
        [],
    );
    Ok(())
}


/// Insert a new Job into the Job table
pub fn insert_job(conn: &Connection, id: &str, status: &JobStatus, file_id: Option<i64>, download_url: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO Job (id, status, file_id, download_url, error) VALUES (?1, ?2, ?3, ?4, NULL)",
        params![id, &status.to_string(), file_id, download_url],
    )?;
    Ok(())
}

/// Get a Job by UUID
pub fn get_job_by_id(conn: &Connection, id: &str) -> Result<Option<JobRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, status, file_id, download_url, error FROM Job WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map([id], |row| {
        let status_str: String = row.get(1)?;
        let status = status_str.parse().unwrap_or(JobStatus::NotStarted);
        
        Ok(JobRecord {
            id: row.get(0)?,
            status,
            file_id: row.get(2)?,
            download_url: row.get(3)?,
            error: row.get(4)?,
        })
    })?;
    
    rows.next().transpose()
}

/// Get and start a job that's not started yet
pub fn get_and_start_job(conn: &Connection) -> Result<Option<JobRecord>> {
    // First, find a job that's not started
    let job_id: Option<String> = {
        let tx = conn.unchecked_transaction()?;
        let mut stmt = tx.prepare(
            "SELECT id FROM Job WHERE status = 'NotStarted' ORDER BY created_at ASC LIMIT 1"
        )?;
        
        let mut rows = stmt.query_map([], |row| row.get(0))?;
        let job_id = rows.next().transpose()?;
        drop(rows);
        drop(stmt);
        
        job_id
    };
    
    // If we found a job, update its status to Running
    if let Some(job_id) = job_id {
        // Start a new transaction for the update
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "UPDATE Job SET status = ?1 WHERE id = ?2 AND status = 'Not Started'",
            params!["Running", job_id],
        )?;
        
        // If the update affected 1 row, get the full job details
        if tx.changes() == 1 {
            tx.commit()?;
            
            // Get the full job details
            if let Some(mut job) = get_job_by_id(conn, &job_id)? {
                job.status = JobStatus::Running;
                return Ok(Some(job));
            }
        } else {
            // If no rows were updated, the job was taken by another process
            tx.rollback()?;
        }
    }
    
    Ok(None)
}

/// Mark a job as completed with the resulting file ID
pub fn complete_job(conn: &Connection, job_id: &str, file_id: i64) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    
    // Update the job status and file ID
    tx.execute(
        "UPDATE Job SET status = ?1, file_id = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = ?3",
        params!["Completed", file_id, job_id],
    )?;
    
    tx.commit()?;
    Ok(())
}

/// Mark a job as failed with an error message
pub fn fail_job(conn: &Connection, job_id: &str, error: &str) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    
    // Update the job status and error message
    tx.execute(
        "UPDATE Job SET status = 'Failed', error = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
        params![error, job_id],
    )?;
    
    tx.commit()?;
    Ok(())
}

/// Get file path by hash
pub fn get_filepath_by_hash(conn: &Connection, hash: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT filepath FROM File WHERE hash = ?1 LIMIT 1")?;
    let mut rows = stmt.query_map([hash], |row| row.get(0))?;    
    rows.next().transpose().map_err(Into::into)
}

/// Get file ID by path
pub fn get_file_by_id(conn: &Connection, id: i64) -> Result<FileRecord> {
    conn.query_row(
        "SELECT id, filepath, url, hash FROM File WHERE id = ?1",
        [id],
        |row| Ok(FileRecord {
            id: row.get(0)?,
            filepath: row.get(1)?,
            url: row.get(2)?,
            hash: row.get(3)?,
        }),
    )
}

pub fn get_file_id_by_path(conn: &Connection, path: &str) -> Result<i64> {
    conn.query_row(
        "SELECT id FROM File WHERE filepath = ?1",
        [path],
        |row| row.get(0),
    )
}

/// Insert a file record and return its ID
pub fn insert_file(conn: &Connection, filepath: &str, url: &str, hash: &str) -> Result<i64> {
    conn.execute(
        "INSERT INTO File (filepath, url, hash) VALUES (?1, ?2, ?3)",
        params![filepath, url, hash],
    )?;
    Ok(conn.last_insert_rowid())
}
