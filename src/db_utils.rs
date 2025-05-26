pub fn get_filepath_by_hash(conn: &Connection, hash: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT filepath FROM File WHERE hash = ?1 LIMIT 1")?;
    let mut rows = stmt.query([hash])?;
    if let Some(row) = rows.next()? {
        let filepath: String = row.get(0)?;
        Ok(Some(filepath))
    } else {
        Ok(None)
    }
}
// src/db_utils.rs
use rusqlite::{params, Connection, Result};

// Job status enum for Job table
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum JobStatus {
    NotStarted,
    Running,
    Completed,
}

impl ToString for JobStatus {
    fn to_string(&self) -> String {
        match self {
            JobStatus::NotStarted => "Not Started".to_string(),
            JobStatus::Running => "Running".to_string(),
            JobStatus::Completed => "Completed".to_string(),
        }
    }
}

impl std::str::FromStr for JobStatus {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Not Started" => Ok(JobStatus::NotStarted),
            "Running" => Ok(JobStatus::Running),
            "Completed" => Ok(JobStatus::Completed),
            _ => Err(()),
        }
    }
}

pub struct JobRecord {
    pub id: String, // UUID
    pub status: JobStatus,
    pub file_id: Option<i64>,
    pub download_url: String,
}

pub struct FileRecord {
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
            status TEXT NOT NULL, -- 'Not Started', 'Running', 'Completed'
            file_id INTEGER,
            download_url TEXT NOT NULL,
            FOREIGN KEY(file_id) REFERENCES File(id)
        )",
        [],
    )?;
    Ok(())
}

pub fn insert_file(conn: &Connection, filepath: &str, url: &str, hash: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO File (filepath, url, hash) VALUES (?1, ?2, ?3)",
        params![filepath, url, hash],
    )?;
    Ok(())
}

/// Insert a new Job into the Job table
pub fn insert_job(conn: &Connection, id: &str, status: &JobStatus, file_id: Option<i64>, download_url: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO Job (id, status, file_id, download_url) VALUES (?1, ?2, ?3, ?4)",
        params![id, &status.to_string(), file_id, download_url],
    )?;
    Ok(())
}

/// Get a Job by UUID
pub fn get_job_by_id(conn: &Connection, id: &str) -> Result<Option<JobRecord>> {
    let mut stmt = conn.prepare("SELECT id, status, file_id, download_url FROM Job WHERE id = ?1 LIMIT 1")?;
    let mut rows = stmt.query([id])?;
    if let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let status_str: String = row.get(1)?;
        let status = status_str.parse().unwrap_or(JobStatus::NotStarted);
        let file_id: Option<i64> = row.get(2)?;
        let download_url: String = row.get(3)?;
        Ok(Some(JobRecord { id, status, file_id, download_url }))
    } else {
        Ok(None)
    }
}
