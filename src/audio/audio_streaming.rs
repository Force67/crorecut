//! FFmpeg audio streaming - decodes audio to raw PCM samples.
//!
//! Similar to frame_streaming.rs but for audio. Spawns a persistent FFmpeg
//! process that outputs raw s16le PCM samples.

use std::path::Path;
use std::process::Stdio;

use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};

#[cfg(windows)]
#[allow(unused_imports)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Streams decoded audio samples from a persistent FFmpeg process.
pub struct AudioStreaming {
    child: Child,
}

impl AudioStreaming {
    /// Start streaming audio from a video at the given timestamp.
    pub fn start(
        video_path: &Path,
        start_timestamp: f64,
        sample_rate: u32,
        channels: u16,
    ) -> Result<Self, String> {
        let child = Self::spawn_ffmpeg(video_path, start_timestamp, sample_rate, channels)?;
        Ok(Self { child })
    }

    fn spawn_ffmpeg(
        video_path: &Path,
        start_timestamp: f64,
        sample_rate: u32,
        channels: u16,
    ) -> Result<Child, String> {
        let timestamp_str = crate::utils::format_timestamp_precise(start_timestamp);
        log::info!("FFmpeg audio seek timestamp: {}", timestamp_str);

        let mut command = Command::new("ffmpeg");
        command
            // Seek before input for fast seeking
            .args(["-ss", &timestamp_str])
            .arg("-i")
            .arg(video_path)
            // Audio only, no video
            .arg("-vn")
            // Output format: raw PCM s16le
            .args(["-f", "s16le"])
            .args(["-acodec", "pcm_s16le"])
            // Sample rate
            .args(["-ar", &sample_rate.to_string()])
            // Channel count
            .args(["-ac", &channels.to_string()])
            // Output to stdout
            .arg("-")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        command.creation_flags(CREATE_NO_WINDOW);

        command
            .spawn()
            .map_err(|e| format!("Failed to spawn ffmpeg for audio: {}", e))
    }

    /// Read audio samples into the provided buffer.
    ///
    /// Returns the number of bytes read, or 0 for EOF.
    pub async fn read_samples(&mut self, buffer: &mut [u8]) -> Result<usize, String> {
        let stdout = self
            .child
            .stdout
            .as_mut()
            .ok_or_else(|| "No stdout available".to_string())?;

        match stdout.read(buffer).await {
            Ok(n) => Ok(n),
            Err(e) => Err(format!("Audio read error: {}", e)),
        }
    }
}

impl Drop for AudioStreaming {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}
