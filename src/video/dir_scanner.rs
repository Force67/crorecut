use std::fs;
use std::path::PathBuf;

use super::metadata::VideoMetadata;

pub const SUPPORTED_EXTENSIONS: &[&str] = &["mp4", "avi", "mkv", "mov"];

pub async fn scan_videos_directory(dir: PathBuf) -> Result<Vec<VideoMetadata>, String> {
    tokio::task::spawn_blocking(move || scan_videos_directory_blocking(dir))
        .await
        .map_err(|e| format!("Task join error: {}", e))?
}

// Scan video files in ~/Videos with max depth of 2 (main directory and one subdirectory deep)
fn scan_videos_directory_blocking(dir: PathBuf) -> Result<Vec<VideoMetadata>, String> {
    let mut videos = Vec::new();

    let entries = fs::read_dir(&dir).map_err(|e| format!("Failed to read directory: {}", e))?;

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();

        if path.is_file() {
            if let Some(meta) = extract_video_metadata(path)? {
                videos.push(meta);
            }
        } else if path.is_dir() {
            let subentries =
                fs::read_dir(&path).map_err(|e| format!("Failed to read subdirectory: {}", e))?;

            for subentry in subentries.filter_map(|e| e.ok()) {
                let subpath = subentry.path();
                if subpath.is_file()
                    && let Some(meta) = extract_video_metadata(subpath)?
                {
                    videos.push(meta);
                }
            }
        }
    }

    videos.sort_by_key(|v| std::cmp::Reverse(v.modified_time));
    Ok(videos)
}

fn extract_video_metadata(path: PathBuf) -> Result<Option<VideoMetadata>, String> {
    let Some(extension) = path.extension() else {
        return Ok(None);
    };

    let ext = extension.to_string_lossy().to_lowercase();
    if !SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
        return Ok(None);
    }

    let metadata = fs::metadata(&path).map_err(|e| format!("Failed to read metadata: {}", e))?;

    let modified_time = metadata
        .modified()
        .map_err(|e| format!("Failed to get modified time: {}", e))?;

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let parent_dir_name = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    Ok(Some(VideoMetadata {
        path,
        file_name,
        parent_dir_name,
        modified_time,
    }))
}
