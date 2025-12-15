use std::path::PathBuf;

use crate::ui::editor::ImageTransform;
use crate::ui::selection_canvas::CropSelection;

use super::SelectionState;

#[derive(Debug, Clone, Default, PartialEq)]
pub enum RenderStatus {
    #[default]
    Idle,
    Rendering,
    Completed(PathBuf),
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct PlayerViewState {
    pub frame_handle: Option<iced::advanced::image::Handle>,
    pub timestamp: f64,
    pub video_path: PathBuf,
    pub duration: f64,

    pub video_width: u32,
    pub video_height: u32,
    pub video_fps: f64,
    pub preview_width: f32,
    pub preview_height: f32,

    // Extraction state
    pub is_extracting: bool,
    pub last_extraction_request_id: u64,
    pub pending_timestamp: Option<f64>,

    // Crop selection state (in preview image space, not canvas space)
    pub crop_selection: Option<CropSelection>,
    pub selection_state: SelectionState,
    pub mouse_position: (f32, f32),
    pub canvas_bounds: (f32, f32), // (width, height) of the canvas container

    pub trim_start: f64,
    pub trim_end: f64,

    pub render_status: RenderStatus,

    pub is_command_hovered: bool,

    pub video_file_size_mb: f64,
    pub target_size_mb: String, // Target size for compression

    // Streaming playback (experimental)
    pub is_streaming: bool,
}

impl PlayerViewState {
    /// Get the current image transform based on canvas bounds and preview dimensions
    pub fn get_image_transform(&self) -> ImageTransform {
        ImageTransform::from_contain(
            self.canvas_bounds.0,
            self.canvas_bounds.1,
            self.preview_width,
            self.preview_height,
        )
    }
}
