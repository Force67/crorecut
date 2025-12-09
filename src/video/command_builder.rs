use std::path::Path;

use crate::ui::selection_canvas::CropSelection;

/// Structured FFmpeg command for display purposes
pub struct FfmpegCommand {
    /// The full command string
    pub full: String,
    /// Command with placeholders for input/output
    pub masked: String,
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
/// Note: Crop coordinates are in canvas/preview space. The actual video
/// dimensions may differ, so the crop values shown are relative to the preview.
pub fn build_ffmpeg_command(
    input_path: &Path,
    crop: Option<&CropSelection>,
    trim_start: f64,
    trim_end: f64,
    duration: f64,
) -> FfmpegCommand {
    let mut args = vec!["ffmpeg".to_string()];
    let mut masked_args = vec!["ffmpeg".to_string()];

    // Check if trim is active (not at default 0..duration)
    let is_trimming = trim_start > 0.01 || (duration > 0.0 && trim_end < duration - 0.01);

    // Add seek position before input for fast seeking
    if is_trimming && trim_start > 0.01 {
        args.push("-ss".to_string());
        args.push(format_timestamp(trim_start));
        masked_args.push("-ss".to_string());
        masked_args.push(format_timestamp(trim_start));
    }

    // Input file
    args.push("-i".to_string());
    args.push(format!("\"{}\"", input_path.display()));
    masked_args.push("-i".to_string());
    masked_args.push("{input}".to_string());

    // Add duration/end time after input
    if is_trimming {
        // Use -t (duration) rather than -to since we're seeking with -ss
        let trim_duration = trim_end - trim_start;
        args.push("-t".to_string());
        args.push(format_timestamp(trim_duration));
        masked_args.push("-t".to_string());
        masked_args.push(format_timestamp(trim_duration));
    }

    // Video filters
    let mut filters = Vec::new();

    if let Some(crop_sel) = crop {
        // Round to integers for FFmpeg
        let w = crop_sel.width.round() as i32;
        let h = crop_sel.height.round() as i32;
        let x = crop_sel.x.round() as i32;
        let y = crop_sel.y.round() as i32;

        if w > 0 && h > 0 {
            filters.push(format!("crop={}:{}:{}:{}", w, h, x, y));
        }
    }

    if !filters.is_empty() {
        let filter_str = format!("\"{}\"", filters.join(","));
        args.push("-vf".to_string());
        args.push(filter_str.clone());
        masked_args.push("-vf".to_string());
        masked_args.push(filter_str);
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
    let output_path_buf = input_path
        .parent()
        .map(|p| p.join(&output_name))
        .unwrap_or_else(|| output_name.clone().into());

    args.push(format!("\"{}\"", output_path_buf.display()));
    masked_args.push("{output}".to_string());

    FfmpegCommand {
        full: args.join(" "),
        masked: masked_args.join(" "),
    }
}
