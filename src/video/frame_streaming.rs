use iced::advanced::subscription::{self, Recipe};
use iced::futures::stream::BoxStream;
use std::hash::Hash;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};

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
    pub async fn start(
        video_path: &PathBuf,
        width: u32,
        height: u32,
        start_timestamp: f64,
    ) -> Result<Self, String> {
        let child = Self::spawn_ffmpeg(video_path, width, height, start_timestamp)?;
        let frame_size = (width * height * 4) as usize; // RGBA

        Ok(Self { child, frame_size })
    }

    fn spawn_ffmpeg(
        video_path: &PathBuf,
        width: u32,
        height: u32,
        start_timestamp: f64,
    ) -> Result<Child, String> {
        let mut command = Command::new("ffmpeg");
        command
            .arg("-ss")
            .arg(crate::utils::format_timestamp_precise(start_timestamp))
            .arg("-i")
            .arg(video_path)
            .arg("-vf")
            .arg(format!("scale={}:{}", width, height))
            .arg("-f")
            .arg("rawvideo")
            .arg("-pix_fmt")
            .arg("rgba")
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
///
/// This holds all parameters needed to create a frame stream, and iced
/// will use the `hash` method to determine subscription identity.
struct FrameStreamRecipe {
    video_path: PathBuf,
    start_timestamp: f64,
    width: u32,
    height: u32,
    fps: f64,
}

impl Recipe for FrameStreamRecipe {
    type Output = crate::app::Message;

    fn hash(&self, state: &mut subscription::Hasher) {
        // Use video_path for identity - same video = same subscription
        // This means the subscription won't restart if only timestamp changes
        self.video_path.hash(state);
    }

    fn stream(
        self: Box<Self>,
        _input: subscription::EventStream,
    ) -> BoxStream<'static, Self::Output> {
        use iced::futures::SinkExt;

        let video_path = self.video_path;
        let start_timestamp = self.start_timestamp;
        let width = self.width;
        let height = self.height;
        let fps = self.fps;

        log::info!(
            "FrameStreamRecipe::stream: path={:?}, ts={}, {}x{}, fps={}",
            video_path,
            start_timestamp,
            width,
            height,
            fps
        );

        Box::pin(iced::stream::channel(
            1,
            move |mut output: iced::futures::channel::mpsc::Sender<crate::app::Message>| async move {
                log::info!("Frame stream channel started, spawning ffmpeg...");

                // Create the streamer
                let mut streaming = match FrameStreaming::start(
                    &video_path,
                    width,
                    height,
                    start_timestamp,
                )
                .await
                {
                    Ok(s) => {
                        log::info!("FrameStreaming started successfully");
                        s
                    }
                    Err(e) => {
                        log::error!("Failed to start frame streaming: {}", e);
                        return;
                    }
                };

                let mut current_timestamp = start_timestamp;
                let frame_duration = 1.0 / fps;
                let frame_delay_ms = (1000.0 / fps) as u64;
                let mut frame_count = 0u64;

                loop {
                    match streaming.next_frame().await {
                        Ok(Some(rgba_data)) => {
                            current_timestamp += frame_duration;
                            frame_count += 1;

                            // Log once per second (approximately)
                            if frame_count % fps.round() as u64 == 1 {
                                log::info!(
                                    "Streaming frame #{} at timestamp {:.2}s ({}fps)",
                                    frame_count,
                                    current_timestamp,
                                    fps.round()
                                );
                            }

                            let msg = crate::app::Message::StreamedFrame {
                                frame_bytes: rgba_data,
                                timestamp: current_timestamp,
                            };

                            if output.send(msg).await.is_err() {
                                log::info!("Channel closed, stopping stream");
                                break;
                            }

                            tokio::time::sleep(std::time::Duration::from_millis(frame_delay_ms))
                                .await;
                        }
                        Ok(None) => {
                            log::info!("Frame streaming reached end of video");
                            break;
                        }
                        Err(e) => {
                            log::error!("Frame streaming error: {}", e);
                            break;
                        }
                    }
                }
            },
        ))
    }
}

/// Creates an iced subscription that streams frames from a video.
///
/// The subscription yields `Message::StreamedFrame` messages continuously
/// until it's cancelled (by returning `Subscription::none()` when is_streaming becomes false).
pub fn frame_stream_subscription(
    video_path: PathBuf,
    start_timestamp: f64,
    width: u32,
    height: u32,
    fps: f64,
) -> iced::Subscription<crate::app::Message> {
    log::info!(
        "frame_stream_subscription called: path={:?}, ts={}, {}x{}, fps={}",
        video_path,
        start_timestamp,
        width,
        height,
        fps
    );

    subscription::from_recipe(FrameStreamRecipe {
        video_path,
        start_timestamp,
        width,
        height,
        fps,
    })
}
