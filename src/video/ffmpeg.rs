use std::path::PathBuf;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

#[cfg(windows)]
#[allow(unused_imports)] // Trait must be in scope for .creation_flags() method
use std::os::windows::process::CommandExt;

use super::error::{FfmpegCommandError, FrameExtractionError};
use crate::utils::format_timestamp_precise;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Video stream properties from ffprobe
#[derive(Debug, Clone)]
pub struct VideoStreamInfo {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub duration: f64,
}

/// Get video stream info (dimensions, fps, duration) in a single ffprobe call.
pub async fn get_video_stream_info(
    video_path: PathBuf,
) -> Result<VideoStreamInfo, FfmpegCommandError> {
    if !tokio::fs::try_exists(&video_path).await.unwrap_or(false) {
        return Err(FfmpegCommandError::CommandFailed(format!(
            "Video not found: {}",
            video_path.display()
        )));
    }

    let mut command = Command::new("ffprobe");
    command
        .arg("-v")
        .arg("error")
        .arg("-select_streams")
        .arg("v:0")
        .arg("-show_entries")
        .arg("stream=width,height,r_frame_rate:format=duration")
        .arg("-of")
        .arg("default=noprint_wrappers=1")
        .arg(&video_path);

    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    let output = command
        .output()
        .await
        .map_err(|_| FfmpegCommandError::FFmpegNotFound)?;

    if !output.status.success() {
        return Err(FfmpegCommandError::CommandFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    // Parse output like:
    // width=1920
    // height=1080
    // r_frame_rate=30/1
    // duration=10.000000
    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut width: Option<u32> = None;
    let mut height: Option<u32> = None;
    let mut fps: Option<f64> = None;
    let mut duration: Option<f64> = None;

    for line in output_str.lines() {
        if let Some((key, value)) = line.split_once('=') {
            match key {
                "width" => width = value.parse().ok(),
                "height" => height = value.parse().ok(),
                "r_frame_rate" => {
                    // Parse fraction like "30/1" or "30000/1001"
                    if let Some((num, den)) = value.split_once('/')
                        && let (Ok(n), Ok(d)) = (num.parse::<f64>(), den.parse::<f64>())
                        && d != 0.0
                    {
                        fps = Some(n / d);
                    }
                }
                "duration" => duration = value.parse().ok(),
                _ => {}
            }
        }
    }

    Ok(VideoStreamInfo {
        width: width.ok_or_else(|| FfmpegCommandError::CommandFailed("Missing width".into()))?,
        height: height.ok_or_else(|| FfmpegCommandError::CommandFailed("Missing height".into()))?,
        fps: fps.unwrap_or(30.0), // Default to 30fps if not found
        duration: duration
            .ok_or_else(|| FfmpegCommandError::CommandFailed("Missing duration".into()))?,
    })
}

/// Extract a single frame from a video at the specified timestamp. Returns RGBA frame bytes
pub async fn extract_frame(
    video_path: PathBuf,
    timestamp_seconds: f64,
) -> Result<Vec<u8>, FrameExtractionError> {
    if !tokio::fs::try_exists(&video_path).await.unwrap_or(false) {
        return Err(FrameExtractionError::VideoNotFound(video_path));
    }

    let timestamp_str = format_timestamp_precise(timestamp_seconds);

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
        .arg("rawvideo")
        .arg("-pix_fmt")
        .arg("rgba")
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

/// Get the audio bitrate of a video file in bits per second.
/// Returns None if the video has no audio stream.
pub async fn get_audio_bitrate(video_path: PathBuf) -> Result<Option<u64>, FrameExtractionError> {
    if !tokio::fs::try_exists(&video_path).await.unwrap_or(false) {
        return Err(FrameExtractionError::VideoNotFound(video_path));
    }

    let mut command = Command::new("ffprobe");
    command
        .arg("-v")
        .arg("error")
        .arg("-select_streams")
        .arg("a:0")
        .arg("-show_entries")
        .arg("stream=bit_rate")
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
        // No audio stream or other error - return None
        return Ok(None);
    }

    let bitrate_str = String::from_utf8_lossy(&output.stdout);
    let trimmed = bitrate_str.trim();

    if trimmed.is_empty() || trimmed == "N/A" {
        return Ok(None);
    }

    match trimmed.parse::<u64>() {
        Ok(bitrate) => Ok(Some(bitrate)),
        Err(_) => Ok(None), // Couldn't parse - treat as no audio
    }
}

/// Execute an FFmpeg command with the given arguments.
pub async fn run_ffmpeg_command(args: &[String]) -> Result<(), FfmpegCommandError> {
    let mut cmd = Command::new("ffmpeg");
    cmd.args(args);

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let output = cmd
        .output()
        .await
        .map_err(|_| FfmpegCommandError::FFmpegNotFound)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(FfmpegCommandError::CommandFailed(stderr.to_string()));
    }

    Ok(())
}

/// Execute a two-pass FFmpeg compression.
/// Runs pass 1, then pass 2, and cleans up temporary log files.
pub async fn run_two_pass_compression(
    pass1_args: Vec<String>,
    pass2_args: Vec<String>,
    working_dir: PathBuf,
) -> Result<(), FfmpegCommandError> {
    // Pass 1: Analysis pass
    let mut cmd1 = Command::new("ffmpeg");
    cmd1.args(&pass1_args);
    cmd1.current_dir(&working_dir);

    #[cfg(windows)]
    cmd1.creation_flags(CREATE_NO_WINDOW);

    cmd1.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let output1 = cmd1
        .output()
        .await
        .map_err(|_| FfmpegCommandError::FFmpegNotFound)?;

    if !output1.status.success() {
        let stderr = String::from_utf8_lossy(&output1.stderr);
        return Err(FfmpegCommandError::CommandFailed(format!(
            "Pass 1 failed: {}",
            stderr
        )));
    }

    // Pass 2: Actual encoding
    let mut cmd2 = Command::new("ffmpeg");
    cmd2.args(&pass2_args);
    cmd2.current_dir(&working_dir);

    #[cfg(windows)]
    cmd2.creation_flags(CREATE_NO_WINDOW);

    cmd2.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let output2 = cmd2
        .output()
        .await
        .map_err(|_| FfmpegCommandError::FFmpegNotFound)?;

    // Clean up pass log files (ffmpeg2pass-0.log, ffmpeg2pass-0.log.mbtree)
    let log_files = ["ffmpeg2pass-0.log", "ffmpeg2pass-0.log.mbtree"];
    for log_file in &log_files {
        let log_path = working_dir.join(log_file);
        let _ = tokio::fs::remove_file(log_path).await; // Ignore errors
    }

    if !output2.status.success() {
        let stderr = String::from_utf8_lossy(&output2.stderr);
        return Err(FfmpegCommandError::CommandFailed(format!(
            "Pass 2 failed: {}",
            stderr
        )));
    }

    Ok(())
}
