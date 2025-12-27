use iced::advanced::image;

use crate::video::VideoMetadata;

#[derive(Debug, Clone)]
pub enum ThumbnailState {
    Pending,
    Generating,
    Ready(image::Handle),
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct VideoItem {
    pub metadata: VideoMetadata,
    pub thumbnail_state: ThumbnailState,
    pub duration: Option<f64>,
}
