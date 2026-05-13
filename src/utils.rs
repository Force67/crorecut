/// Format duration in seconds to HH:MM:SS.mmm format (for FFmpeg)
pub fn format_timestamp_precise(seconds: f64) -> String {
    let hours = (seconds / 3600.0).floor() as u32;
    let minutes = ((seconds % 3600.0) / 60.0).floor() as u32;
    let secs = seconds % 60.0;
    format!("{hours:02}:{minutes:02}:{secs:06.3}")
}

/// Format duration in seconds to mm:ss format.
pub fn format_duration_mmss(seconds: f64) -> String {
    let total_minutes = (seconds / 60.0).floor() as u32;
    let secs = (seconds % 60.0).floor() as u32;
    format!("{total_minutes:02}:{secs:02}")
}

pub fn format_duration_human_readable(seconds: f64) -> String {
    let total_seconds = seconds.max(0.0).round() as u64;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{hours}h {minutes:02}m")
    } else if minutes > 0 {
        format!("{minutes}m {seconds:02}s")
    } else {
        format!("{seconds}s")
    }
}

/// Represents the transform from canvas space to image space
#[derive(Debug, Clone, Copy)]
pub struct ImageTransform {
    pub scale: f32,
    pub offset_x: f32,
    pub offset_y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CropSelection {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl ImageTransform {
    pub fn from_contain(
        container_width: f32,
        container_height: f32,
        image_width: f32,
        image_height: f32,
    ) -> Self {
        if image_width <= 0.0 || image_height <= 0.0 {
            return Self {
                scale: 1.0,
                offset_x: 0.0,
                offset_y: 0.0,
            };
        }

        // Calculate scale to fit within container while maintaining aspect ratio
        let scale = (container_width / image_width).min(container_height / image_height);

        let displayed_width = image_width * scale;
        let displayed_height = image_height * scale;

        // Center the image
        let offset_x = (container_width - displayed_width) / 2.0;
        let offset_y = (container_height - displayed_height) / 2.0;

        Self {
            scale,
            offset_x,
            offset_y,
        }
    }

    /// Transform from canvas space to image space
    pub fn canvas_to_image(&self, canvas_x: f32, canvas_y: f32) -> (f32, f32) {
        let image_x = (canvas_x - self.offset_x) / self.scale;
        let image_y = (canvas_y - self.offset_y) / self.scale;
        (image_x, image_y)
    }

    /// Transform from image space to canvas space
    pub fn image_to_canvas(&self, image_x: f32, image_y: f32) -> (f32, f32) {
        let canvas_x = image_x * self.scale + self.offset_x;
        let canvas_y = image_y * self.scale + self.offset_y;
        (canvas_x, canvas_y)
    }
}
