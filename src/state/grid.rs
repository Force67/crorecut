use std::path::PathBuf;

use crate::ui::grid::VideoItem;

pub const MAX_CONCURRENT_THUMBNAILS: usize = 3;

#[derive(Debug, Clone)]
pub enum ScanStatus {
    NotStarted,
    Scanning,
    Complete,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct GridViewState {
    pub videos: Vec<VideoItem>,
    pub scan_status: ScanStatus,
    pub thumbnail_generation_queue: Vec<PathBuf>,
    pub thumbnails_in_progress: usize,
}

impl Default for GridViewState {
    fn default() -> Self {
        Self {
            videos: Vec::new(),
            scan_status: ScanStatus::NotStarted,
            thumbnail_generation_queue: Vec::new(),
            thumbnails_in_progress: 0,
        }
    }
}
