use std::path::PathBuf;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

#[cfg(windows)]
#[allow(unused_imports)] // Trait must be in scope for .creation_flags() method
use std::os::windows::process::CommandExt;

use super::error::FrameExtractionError;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Extract a single frame from a video at the specified timestamp. Returns PNG-encoded frame bytes
pub async fn extract_frame(
    video_path: PathBuf,
    timestamp_seconds: f64,
) -> Result<Vec<u8>, FrameExtractionError> {
    if !tokio::fs::try_exists(&video_path).await.unwrap_or(false) {
        return Err(FrameExtractionError::VideoNotFound(video_path));
    }

    // Format timestamp as HH:MM:SS.mmm
    let hours = (timestamp_seconds / 3600.0).floor() as u32;
    let minutes = ((timestamp_seconds % 3600.0) / 60.0).floor() as u32;
    let seconds = timestamp_seconds % 60.0;
    let timestamp_str = format!("{:02}:{:02}:{:06.3}", hours, minutes, seconds);

    // -ss before -i enables faster keyframe seeking
    let mut command = Command::new("ffmpeg");
    command
        .arg("-ss")
        .arg(&timestamp_str)
        .arg("-i")
        .arg(&video_path)
        .arg("-vf")
        .arg("scale=-1:360") // 360p previews
        .arg("-frames:v")
        .arg("1")
        .arg("-f")
        .arg("image2pipe")
        .arg("-vcodec")
        .arg("png")
        .arg("-")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    let mut child = command
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
    if !tokio::fs::try_exists(&video_path).await.unwrap_or(false) {
        return Err(FrameExtractionError::VideoNotFound(video_path));
    }

    let mut command = Command::new("ffprobe");
    command
        .arg("-v")
        .arg("error")
        .arg("-show_entries")
        .arg("format=duration")
        .arg("-of")
        .arg("default=noprint_wrappers=1:nokey=1")
        .arg(&video_path);

    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    let output = command
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

/// Get the dimensions (width x height) of a video file
pub async fn get_video_dimensions(video_path: PathBuf) -> Result<(u32, u32), FrameExtractionError> {
    if !tokio::fs::try_exists(&video_path).await.unwrap_or(false) {
        return Err(FrameExtractionError::VideoNotFound(video_path));
    }

    let mut command = Command::new("ffprobe");
    command
        .arg("-v")
        .arg("error")
        .arg("-select_streams")
        .arg("v:0")
        .arg("-show_entries")
        .arg("stream=width,height")
        .arg("-of")
        .arg("csv=p=0")
        .arg(&video_path);

    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    let output = command
        .output()
        .await
        .map_err(|_| FrameExtractionError::FFmpegNotFound)?;

    if !output.status.success() {
        return Err(FrameExtractionError::ExtractionFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    let dimensions_str = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = dimensions_str.trim().split(',').collect();

    if parts.len() != 2 {
        return Err(FrameExtractionError::ExtractionFailed(
            "Failed to parse video dimensions".to_string(),
        ));
    }

    let width = parts[0].parse::<u32>().map_err(|e| {
        FrameExtractionError::ExtractionFailed(format!("Failed to parse width: {}", e))
    })?;

    let height = parts[1].parse::<u32>().map_err(|e| {
        FrameExtractionError::ExtractionFailed(format!("Failed to parse height: {}", e))
    })?;

    Ok((width, height))
}
