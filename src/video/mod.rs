pub mod dir_scanner;
pub mod error;
pub mod ffmpeg;
pub mod metadata;

pub use dir_scanner::scan_videos_directory;
pub use ffmpeg::{extract_frame, get_video_duration_sec};
pub use metadata::VideoMetadata;
