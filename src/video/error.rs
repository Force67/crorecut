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

#[derive(Debug, Clone)]
pub enum FfmpegCommandError {
    FFmpegNotFound,
    CommandFailed(String),
}

impl std::fmt::Display for FfmpegCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FFmpegNotFound => write!(f, "FFmpeg not found in PATH"),
            Self::CommandFailed(msg) => write!(f, "FFmpeg command failed: {}", msg),
        }
    }
}

impl std::error::Error for FfmpegCommandError {}

#[derive(Debug, Clone)]
pub enum CompressionError {
    TargetSizeTooSmall { min_size_mb: f64 },
    InvalidTargetSize,
}

impl std::fmt::Display for CompressionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TargetSizeTooSmall { min_size_mb } => {
                write!(
                    f,
                    "Target size is too small. Minimum size: {:.2} MB",
                    min_size_mb
                )
            }
            Self::InvalidTargetSize => write!(f, "Invalid target size"),
        }
    }
}

impl std::error::Error for CompressionError {}
