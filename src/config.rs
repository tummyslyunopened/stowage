use std::path::PathBuf; 

#[derive(Debug, Clone)] 
pub struct Config { 
    pub media_path: PathBuf, 
    pub max_file_size: u64, 
    pub allowed_mime_types: Vec<String>, 
}

impl Default for Config { 
    fn default() -> Self { 
        Self { 
            media_path: std::env::current_dir().unwrap().join("media"), 
            max_file_size: 100 * 1024 * 1024, 
            allowed_mime_types: vec![ 
                "audio/*".into(), 
                "video/*".into(),
                "image/*".into(),
                "application/octet-stream".into(),
                "application/json".into(),
                "text/xml".into(),
                "application/rss+xml".into(),
                "application/xml".into(),
            ],
        }
    }
}
