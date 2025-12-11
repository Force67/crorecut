use std::path::{Path, PathBuf};

use crate::ui::selection_canvas::CropSelection;

/// Video dimensions (original and preview)
pub struct VideoDimensions {
    pub video_width: u32,
    pub video_height: u32,
    pub preview_width: f32,
    pub preview_height: f32,
}

/// Structured FFmpeg command for display and execution
pub struct FfmpegCommand {
    /// The full command string (for display/copy)
    pub full: String,
    /// Command with placeholders for input/output (for display)
    pub masked: String,
    /// Raw arguments for direct execution (without shell quoting)
    pub args: Vec<String>,
    /// Output path for the rendered video
    pub output_path: PathBuf,
}

/// Format seconds as HH:MM:SS.mmm for FFmpeg
fn format_timestamp(seconds: f64) -> String {
    let hours = (seconds / 3600.0).floor() as u32;
    let minutes = ((seconds % 3600.0) / 60.0).floor() as u32;
    let secs = seconds % 60.0;
    format!("{:02}:{:02}:{:06.3}", hours, minutes, secs)
}

/// Build an FFmpeg command from the current editing state.
///
/// Returns a structured command with both full and masked versions.
/// Crop coordinates are automatically scaled from preview space to original video dimensions.
pub fn build_ffmpeg_command(
    input_path: &Path,
    crop: Option<&CropSelection>,
    trim_start: f64,
    trim_end: f64,
    duration: f64,
    dimen: &VideoDimensions,
) -> FfmpegCommand {
    // display_args: for copy-paste (with quotes around paths)
    // masked_args: for display with placeholders
    // exec_args: raw args for direct execution (no quotes needed)
    let mut display_args = vec!["ffmpeg".to_string()];
    let mut masked_args = vec!["ffmpeg".to_string()];
    let mut exec_args: Vec<String> = Vec::new();

    // Check if trim is active (not at default 0..duration)
    let is_trimming = trim_start > 0.01 || (duration > 0.0 && trim_end < duration - 0.01);

    // Add seek position before input for fast seeking
    if is_trimming && trim_start > 0.01 {
        let ts = format_timestamp(trim_start);
        display_args.push("-ss".to_string());
        display_args.push(ts.clone());
        masked_args.push("-ss".to_string());
        masked_args.push(ts.clone());
        exec_args.push("-ss".to_string());
        exec_args.push(ts);
    }

    // Input file
    let input_str = input_path.to_string_lossy().to_string();
    display_args.push("-i".to_string());
    display_args.push(format!("\"{}\"", input_str));
    masked_args.push("-i".to_string());
    masked_args.push("{input}".to_string());
    exec_args.push("-i".to_string());
    exec_args.push(input_str);

    // Add duration/end time after input
    if is_trimming {
        // Use -t (duration) rather than -to since we're seeking with -ss
        let trim_duration = trim_end - trim_start;
        let ts = format_timestamp(trim_duration);
        display_args.push("-t".to_string());
        display_args.push(ts.clone());
        masked_args.push("-t".to_string());
        masked_args.push(ts.clone());
        exec_args.push("-t".to_string());
        exec_args.push(ts);
    }

    // Video filters
    let mut filters = Vec::new();

    if let Some(crop_sel) = crop {
        // Scale crop coordinates from preview space to original video dimensions
        let scale_x = if dimen.preview_width > 0.0 {
            dimen.video_width as f32 / dimen.preview_width
        } else {
            1.0
        };

        let scale_y = if dimen.preview_height > 0.0 {
            dimen.video_height as f32 / dimen.preview_height
        } else {
            1.0
        };

        let w = (crop_sel.width * scale_x).round() as i32;
        let h = (crop_sel.height * scale_y).round() as i32;
        let x = (crop_sel.x * scale_x).round() as i32;
        let y = (crop_sel.y * scale_y).round() as i32;

        if w > 0 && h > 0 {
            filters.push(format!("crop={}:{}:{}:{}", w, h, x, y));
        }
    }

    if !filters.is_empty() {
        let filter_str = filters.join(",");
        display_args.push("-vf".to_string());
        display_args.push(format!("\"{}\"", filter_str));
        masked_args.push("-vf".to_string());
        masked_args.push(format!("\"{}\"", filter_str));
        exec_args.push("-vf".to_string());
        exec_args.push(filter_str);
    }

    // Generate output filename
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let ext = input_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("mp4");

    let output_name = format!("{}_edited.{}", stem, ext);

    // Output path (same directory as input)
    let output_path = input_path
        .parent()
        .map(|p| p.join(&output_name))
        .unwrap_or_else(|| output_name.clone().into());

    let output_str = output_path.to_string_lossy().to_string();
    display_args.push(format!("\"{}\"", output_str));
    masked_args.push("{output}".to_string());
    exec_args.push(output_str);

    FfmpegCommand {
        full: display_args.join(" "),
        masked: masked_args.join(" "),
        args: exec_args,
        output_path,
    }
}
