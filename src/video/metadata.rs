use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct VideoMetadata {
    pub path: PathBuf,
    pub file_name: String,
    pub parent_dir_name: String,
    pub modified_time: std::time::SystemTime,
}
