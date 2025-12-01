use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum FrameExtractionError {
    FFmpegNotFound,
    VideoNotFound(PathBuf),
    ExtractionFailed(String),
    InvalidOutput,
}

impl std::fmt::Display for FrameExtractionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FFmpegNotFound => write!(f, "FFmpeg not found in PATH"),
            Self::VideoNotFound(path) => write!(f, "Video file not found: {}", path.display()),
            Self::ExtractionFailed(msg) => write!(f, "Frame extraction failed: {}", msg),
            Self::InvalidOutput => write!(f, "FFmpeg produced invalid output"),
        }
    }
}
