use tokio::process::Command;

#[cfg(windows)]
#[allow(unused_imports)] // Trait must be in scope for .creation_flags() method
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

const MAX_VERSION_CHARS: usize = 25;

/// Whether `ffmpeg` and `ffprobe` can be launched from PATH
#[derive(Debug, Clone)]
pub enum FfmpegAvailability {
    Detected { version: String },
    Missing { ffmpeg: bool, ffprobe: bool },
}

pub async fn detect_ffmpeg() -> FfmpegAvailability {
    let ffmpeg = binary_version("ffmpeg").await;
    let ffprobe = binary_version("ffprobe").await;

    match (ffmpeg, ffprobe) {
        (Some(version), Some(_)) => FfmpegAvailability::Detected {
            version: shorten_version(&version),
        },
        (ffmpeg, ffprobe) => FfmpegAvailability::Missing {
            ffmpeg: ffmpeg.is_none(),
            ffprobe: ffprobe.is_none(),
        },
    }
}

/// Runs `<binary> -version` and extracts the version token from
/// the first line, e.g. "ffmpeg version 7.1.1-essentials_build-www.gyan.dev Copyright ..."
async fn binary_version(binary: &str) -> Option<String> {
    let mut command = Command::new(binary);
    command.arg("-version");

    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    let output = command.output().await.ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let version = stdout
        .lines()
        .next()?
        .strip_prefix(&format!("{binary} version "))?
        .split_whitespace()
        .next()?;

    Some(version.to_string())
}

/// Strips the `-<flavor>_build-...` suffix that gyan.dev builds append,
/// then caps the length with an ellipsis
fn shorten_version(version: &str) -> String {
    let version = version
        .find("_build")
        .and_then(|index| version[..index].rfind('-'))
        .map_or(version, |dash| &version[..dash]);

    if version.chars().count() > MAX_VERSION_CHARS {
        let truncated: String = version.chars().take(MAX_VERSION_CHARS - 1).collect();
        format!("{truncated}…")
    } else {
        version.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::shorten_version;

    #[test]
    fn strips_gyan_build_suffix() {
        assert_eq!(
            shorten_version("2023-10-16-git-5ddab49d48-essentials_build-www.gyan.dev"),
            "2023-10-16-git-5ddab49d48"
        );
        assert_eq!(shorten_version("7.1.1-full_build-www.gyan.dev"), "7.1.1");
    }

    #[test]
    fn keeps_short_versions() {
        assert_eq!(shorten_version("7.1"), "7.1");
    }

    #[test]
    fn truncates_long_versions() {
        assert_eq!(
            shorten_version("N-117780-g1fc2e2b8a4-20241108"),
            "N-117780-g1fc2e2b8a4-202…"
        );
    }
}
