mod command_builder;
mod dir_scanner;
pub mod error;
mod ffmpeg;
pub mod frame_streaming;
mod metadata;
mod resolution;

pub use command_builder::{
    CompressionParams, VideoDimensions, build_compression_command, build_ffmpeg_command,
};
pub use dir_scanner::scan_videos_directory;
pub use ffmpeg::{
    RenderEvent, RenderProgress, VideoStreamInfo, extract_frame, get_video_stream_info,
    render_two_pass_stream, run_ffmpeg_command,
};
pub use metadata::VideoMetadata;
pub use resolution::Resolution;
