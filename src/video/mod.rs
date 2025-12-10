pub mod command_builder;
pub mod dir_scanner;
pub mod error;
pub mod ffmpeg;
pub mod metadata;

pub use command_builder::build_ffmpeg_command;
pub use dir_scanner::scan_videos_directory;
pub use ffmpeg::{extract_frame, get_video_dimensions, get_video_duration_sec, run_ffmpeg_command};
pub use metadata::VideoMetadata;
