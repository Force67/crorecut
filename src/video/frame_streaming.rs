use iced::advanced::subscription::{self, Recipe};
use iced::futures::stream::BoxStream;
use std::hash::Hash;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};

use crate::audio::AudioClock;

/// Events emitted by the frame stream subscription
#[derive(Debug, Clone)]
pub enum FrameStreamEvent {
    FrameReady {
        frame_bytes: Vec<u8>,
        timestamp: f64,
    },
}

#[cfg(windows)]
#[allow(unused_imports)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Streams decoded video frames from a persistent ffmpeg process.
///
/// Instead of spawning a new ffmpeg process for each frame, this keeps
/// a single process alive and reads frames sequentially from its stdout.
pub struct FrameStreaming {
    child: Child,
    frame_size: usize, // width * height * 4 (RGBA)
}

impl FrameStreaming {
    /// Start streaming frames from a video at the given timestamp. The stream
    /// will output frames sequentially from the start position.
    pub fn start(
        video_path: &PathBuf,
        width: u32,
        height: u32,
        start_ts: f64,
    ) -> Result<Self, String> {
        let child = Self::spawn_ffmpeg(video_path, width, height, start_ts)?;
        let frame_size = (width * height * 4) as usize; // RGBA

        Ok(Self { child, frame_size })
    }

    fn spawn_ffmpeg(
        video_path: &PathBuf,
        width: u32,
        height: u32,
        start_ts: f64,
    ) -> Result<Child, String> {
        let mut command = Command::new("ffmpeg");
        command
            .args(["-ss", &crate::utils::format_timestamp_precise(start_ts)])
            .arg("-i")
            .arg(video_path)
            .args(["-vf", &format!("scale={}:{}", width, height)])
            .args(["-f", "rawvideo"])
            .args(["-pix_fmt", "rgba"])
            .arg("-")
            .stdout(Stdio::piped())
            .stderr(Stdio::null()); // Suppress ffmpeg logs

        #[cfg(windows)]
        command.creation_flags(CREATE_NO_WINDOW);

        command
            .spawn()
            .map_err(|e| format!("Failed to spawn ffmpeg: {}", e))
    }

    /// Read the next frame from the stream.
    ///
    /// Returns raw RGBA pixel data (width * height * 4 bytes), or None if end of video.
    pub async fn next_frame(&mut self) -> Result<Option<Vec<u8>>, String> {
        let stdout = self
            .child
            .stdout
            .as_mut()
            .ok_or_else(|| "No stdout available".to_string())?;

        let mut buffer = vec![0u8; self.frame_size];

        match stdout.read_exact(&mut buffer).await {
            Ok(_) => Ok(Some(buffer)),
            Err(e) => {
                use tokio::io::ErrorKind;
                match e.kind() {
                    ErrorKind::UnexpectedEof => {
                        // If EOF encountered before any bytes read, we treat as end-of-video.
                        Err(format!("Unexpected EOF while reading frame: {}", e))
                    }
                    _ => Err(format!("Read error: {}", e)),
                }
            }
        }
    }
}

impl Drop for FrameStreaming {
    fn drop(&mut self) {
        // Try to kill the ffmpeg process when the streamer is dropped
        // We can't await here, so just start the kill
        let _ = self.child.start_kill();
    }
}

/// Recipe for streaming video frames via iced's subscription system.
struct FrameStreamRecipe {
    video_path: PathBuf,
    start_timestamp: f64,
    dimensions: (u32, u32),
    fps: f64,
    audio_clock: Arc<AudioClock>,
    has_audio: bool,
    seek_generation: u64,
}

impl Recipe for FrameStreamRecipe {
    type Output = FrameStreamEvent;

    fn hash(&self, state: &mut subscription::Hasher) {
        // Include seek_generation so subscription restarts on seek
        self.video_path.hash(state);
        self.seek_generation.hash(state);
    }

    fn stream(
        self: Box<Self>,
        _input: subscription::EventStream,
    ) -> BoxStream<'static, Self::Output> {
        use iced::futures::SinkExt;

        let video_path = self.video_path;
        let start_ts = self.start_timestamp;
        let (width, height) = self.dimensions;
        let fps = self.fps;
        let audio_clock = self.audio_clock;
        let has_audio = self.has_audio;

        Box::pin(iced::stream::channel(
            1,
            move |mut output: iced::futures::channel::mpsc::Sender<FrameStreamEvent>| async move {
                log::info!("Frame stream channel started, spawning ffmpeg...");

                // Create the streamer
                let mut streaming =
                    match FrameStreaming::start(&video_path, width, height, start_ts) {
                        Ok(s) => {
                            log::info!("FrameStreaming started successfully");
                            s
                        }
                        Err(e) => {
                            log::error!("Failed to start frame streaming: {}", e);
                            return;
                        }
                    };

                let frame_duration = 1.0 / fps;
                let frame_delay = Duration::from_secs_f64(frame_duration);

                // Next frame's target timestamp
                let mut next_frame_ts = start_ts;

                loop {
                    // Get frame ready
                    let frame = match streaming.next_frame().await {
                        Ok(Some(rgba_data)) => rgba_data,
                        Ok(None) => break,
                        Err(e) => {
                            log::error!("Frame streaming error: {}", e);
                            break;
                        }
                    };

                    if has_audio {
                        let audio_pos = audio_clock.position();

                        // Drop frames if we're too far behind
                        if next_frame_ts < audio_pos - 0.1 {
                            next_frame_ts += frame_duration;
                            continue;
                        }

                        let wait_secs = next_frame_ts - audio_pos;
                        if wait_secs > 0.0 {
                            tokio::time::sleep(Duration::from_secs_f64(wait_secs)).await;
                        }
                    } else {
                        tokio::time::sleep(frame_delay).await;
                    }

                    let msg = FrameStreamEvent::FrameReady {
                        frame_bytes: frame,
                        timestamp: next_frame_ts,
                    };

                    if output.send(msg).await.is_err() {
                        break;
                    }

                    next_frame_ts += frame_duration;
                }
            },
        ))
    }
}

/// Creates an iced subscription that streams frames from a video.
///
/// The subscription yields `FrameStreamEvent::FrameReady` messages continuously
/// until it's cancelled (by returning `Subscription::none()` when is_streaming becomes false).
pub fn frame_stream_subscription(
    video_path: PathBuf,
    start_timestamp: f64,
    dimensions: (u32, u32),
    fps: f64,
    audio_clock: Arc<AudioClock>,
    has_audio: bool,
    seek_generation: u64,
) -> iced::Subscription<FrameStreamEvent> {
    subscription::from_recipe(FrameStreamRecipe {
        video_path,
        start_timestamp,
        dimensions,
        fps,
        audio_clock,
        has_audio,
        seek_generation,
    })
}
