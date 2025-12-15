pub mod command_builder;
pub mod dir_scanner;
pub mod error;
pub mod ffmpeg;
pub mod frame_streaming;
pub mod metadata;

pub use command_builder::{
    CompressionParams, VideoDimensions, build_compression_command, build_ffmpeg_command,
};
pub use dir_scanner::scan_videos_directory;
pub use ffmpeg::{
    extract_frame, get_audio_bitrate, get_video_dimensions, get_video_duration_sec,
    run_ffmpeg_command, run_two_pass_compression,
};
pub use metadata::VideoMetadata;
