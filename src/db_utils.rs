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
    Ok(())
}

pub fn insert_file(conn: &Connection, filepath: &str, url: &str, hash: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO File (filepath, url, hash) VALUES (?1, ?2, ?3)",
        params![filepath, url, hash],
    )?;
    Ok(())
}
