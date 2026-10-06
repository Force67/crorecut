mod command_builder;
mod dir_scanner;
pub mod error;
mod ffmpeg;
mod ffmpeg_detection;
pub mod frame_streaming;
mod metadata;
mod resolution;

pub use command_builder::{
    CompressionParams, VideoDimensions, build_compression_command, build_ffmpeg_command,
};
pub use dir_scanner::scan_videos_directory;
pub use ffmpeg::{
    RenderEvent, RenderProgress, THUMBNAIL_HEIGHT, THUMBNAIL_WIDTH, VideoStreamInfo,
    extract_first_frames_batch, extract_frame, get_video_stream_info, run_ffmpeg_cmd,
    run_two_pass_ffmpeg_cmd,
};
pub use ffmpeg_detection::{FfmpegAvailability, detect_ffmpeg};
pub use metadata::VideoMetadata;
pub use resolution::Resolution;
