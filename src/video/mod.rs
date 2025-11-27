use std::path::PathBuf;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

#[derive(Debug, Clone)]
pub enum FrameExtractionError {
    FFmpegNotFound,
    VideoNotFound(PathBuf),
    ExtractionFailed(String),
    InvalidOutput,
}

impl std::fmt::Display for FrameExtractionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FFmpegNotFound => write!(f, "FFmpeg not found in PATH"),
            Self::VideoNotFound(path) => write!(f, "Video file not found: {}", path.display()),
            Self::ExtractionFailed(msg) => write!(f, "Frame extraction failed: {}", msg),
            Self::InvalidOutput => write!(f, "FFmpeg produced invalid output"),
        }
    }
}

/// Extract a single frame from a video at the specified timestamp. Returns PNG-encoded frame bytes
pub async fn extract_frame(
    video_path: PathBuf,
    timestamp_seconds: f64,
) -> Result<Vec<u8>, FrameExtractionError> {
    if !video_path.exists() {
        return Err(FrameExtractionError::VideoNotFound(video_path));
    }

    // Format timestamp as HH:MM:SS.mmm
    let hours = (timestamp_seconds / 3600.0).floor() as u32;
    let minutes = ((timestamp_seconds % 3600.0) / 60.0).floor() as u32;
    let seconds = timestamp_seconds % 60.0;
    let timestamp_str = format!("{:02}:{:02}:{:06.3}", hours, minutes, seconds);

    // -ss before -i enables faster keyframe seeking
    let mut child = Command::new("ffmpeg")
        .arg("-ss")
        .arg(&timestamp_str)
        .arg("-i")
        .arg(&video_path)
        .arg("-frames:v")
        .arg("1")
        .arg("-f")
        .arg("image2pipe")
        .arg("-vcodec")
        .arg("png")
        .arg("-")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|_| FrameExtractionError::FFmpegNotFound)?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or(FrameExtractionError::InvalidOutput)?;
    let mut frame_bytes = Vec::new();
    stdout
        .read_to_end(&mut frame_bytes)
        .await
        .map_err(|e| FrameExtractionError::ExtractionFailed(e.to_string()))?;

    let status = child
        .wait()
        .await
        .map_err(|e| FrameExtractionError::ExtractionFailed(e.to_string()))?;

    if !status.success() {
        if let Some(mut stderr) = child.stderr.take() {
            let mut error_msg = String::new();
            let _ = stderr.read_to_string(&mut error_msg).await;
            return Err(FrameExtractionError::ExtractionFailed(error_msg));
        }
        return Err(FrameExtractionError::ExtractionFailed(
            "Unknown error".to_string(),
        ));
    }

    if frame_bytes.is_empty() {
        return Err(FrameExtractionError::InvalidOutput);
    }

    Ok(frame_bytes)
}

pub async fn get_video_duration_sec(video_path: PathBuf) -> Result<f64, FrameExtractionError> {
    if !video_path.exists() {
        return Err(FrameExtractionError::VideoNotFound(video_path));
    }

    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-show_entries")
        .arg("format=duration")
        .arg("-of")
        .arg("default=noprint_wrappers=1:nokey=1")
        .arg(&video_path)
        .output()
        .await
        .map_err(|_| FrameExtractionError::FFmpegNotFound)?;

    if !output.status.success() {
        return Err(FrameExtractionError::ExtractionFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    let duration_str = String::from_utf8_lossy(&output.stdout);
    duration_str.trim().parse::<f64>().map_err(|e| {
        FrameExtractionError::ExtractionFailed(format!("Failed to parse duration: {}", e))
    })
}
