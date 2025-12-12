/// Format duration in seconds to HH:MM:SS format (for UI display)
pub fn format_timestamp(seconds: f64) -> String {
    let hours = (seconds / 3600.0).floor() as u32;
    let minutes = ((seconds % 3600.0) / 60.0).floor() as u32;
    let secs = (seconds % 60.0).floor() as u32;
    format!("{hours:02}:{minutes:02}:{secs:02}")
}

/// Format duration in seconds to HH:MM:SS.mmm format (for FFmpeg)
pub fn format_timestamp_precise(seconds: f64) -> String {
    let hours = (seconds / 3600.0).floor() as u32;
    let minutes = ((seconds % 3600.0) / 60.0).floor() as u32;
    let secs = seconds % 60.0;
    format!("{hours:02}:{minutes:02}:{secs:06.3}")
}

/// Format duration in seconds to MM:SS format (for short display)
pub fn format_duration_short(seconds: f64) -> String {
    let total_minutes = (seconds / 60.0).floor() as u32;
    let secs = (seconds % 60.0).floor() as u32;
    format!("{total_minutes}:{secs:02}")
}
