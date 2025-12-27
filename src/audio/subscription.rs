//! Audio streaming subscription for iced.
//!
//! This module provides an iced subscription that streams audio from a video
//! file using FFmpeg and plays it via cpal. It writes position to a shared
//! AudioClock for A/V sync (audio is the master clock).

use std::hash::Hash;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait};
use iced::advanced::subscription::{self, Recipe};
use iced::futures::SinkExt;
use iced::futures::stream::BoxStream;

use super::audio_streaming::AudioStreaming;
use super::clock::AudioClock;
use super::player::{AudioPlayer, AudioRingBuffer};

/// How often to emit position updates
const POSITION_UPDATE_INTERVAL: Duration = Duration::from_millis(33);

/// Events emitted by the audio stream subscription
#[derive(Debug, Clone)]
pub enum AudioStreamEvent {
    PositionUpdate(f64),
    StreamEnded,
}

/// Recipe for streaming audio via iced's subscription system.
struct AudioStreamRecipe {
    video_path: PathBuf,
    start_timestamp: f64,
    sample_rate: u32,
    channels: u16,
    audio_clock: Arc<AudioClock>,
    seek_generation: u64, // Changes on seek to force subscription restart
}

impl Recipe for AudioStreamRecipe {
    type Output = AudioStreamEvent;

    fn hash(&self, state: &mut subscription::Hasher) {
        self.video_path.hash(state);
        "audio".hash(state);
        self.seek_generation.hash(state);
    }

    fn stream(
        self: Box<Self>,
        _input: subscription::EventStream,
    ) -> BoxStream<'static, Self::Output> {
        let video_path = self.video_path;
        let start_timestamp = self.start_timestamp;
        let sample_rate = self.sample_rate;
        let channels = self.channels;
        let audio_clock = self.audio_clock;

        Box::pin(iced::stream::channel(
            1,
            move |mut output: iced::futures::channel::mpsc::Sender<AudioStreamEvent>| async move {
                log::info!("Audio stream channel started, spawning ffmpeg...");

                audio_clock.set_position(start_timestamp);

                let ring_buffer = Arc::new(AudioRingBuffer::new(sample_rate, channels));

                let player = match AudioPlayer::new(ring_buffer.clone(), sample_rate, channels) {
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("Failed to create audio player: {}", e);
                        return;
                    }
                };

                // Start FFmpeg audio streaming
                let mut streaming = match AudioStreaming::start(
                    &video_path,
                    start_timestamp,
                    sample_rate,
                    channels,
                ) {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("Failed to start audio streaming: {}", e);
                        return;
                    }
                };

                let mut read_buffer = vec![0u8; 4096]; // 4KB chunks

                // Pre-buffer before starting playback (~100ms)
                let pre_buffer_samples = (sample_rate as usize * channels as usize) / 10;
                log::info!("Pre-buffering audio...");
                while ring_buffer.len() < pre_buffer_samples {
                    match streaming.read_samples(&mut read_buffer).await {
                        Ok(0) => break,
                        Ok(n) => push_s16le_bytes(&ring_buffer, &read_buffer[..n]),
                        Err(e) => {
                            log::error!("Error during pre-buffer: {}", e);
                            return;
                        }
                    }
                }
                log::info!(
                    "Pre-buffer complete: {:.2}s buffered",
                    ring_buffer.buffered_duration()
                );

                // Start playback
                if let Err(e) = player.play() {
                    log::error!("Failed to start audio playback: {}", e);
                    return;
                }

                // Mark clock as active - video can now sync to it
                audio_clock.set_active(true);

                let mut last_position_update = Instant::now();

                // Main loop: read from FFmpeg, push to buffer, emit position updates
                let mut ffmpeg_finished = false;

                loop {
                    // Read samples from FFmpeg (unless it's already finished)

                    if !ffmpeg_finished {
                        match streaming.read_samples(&mut read_buffer).await {
                            Ok(0) => ffmpeg_finished = true,
                            Ok(n) => push_s16le_bytes(&ring_buffer, &read_buffer[..n]),
                            Err(e) => {
                                log::error!("Audio stream error: {}", e);
                                break;
                            }
                        }
                    }

                    // If FFmpeg is done and buffer is nearly empty, we're done
                    if ffmpeg_finished && ring_buffer.buffered_duration() < 0.05 {
                        log::info!("Audio buffer drained, ending playback");
                        let _ = output.send(AudioStreamEvent::StreamEnded).await;
                        break;
                    }

                    let position = start_timestamp + player.position_seconds();
                    audio_clock.set_position(position);

                    // Emit UI position updates at a slower interval
                    if last_position_update.elapsed() >= POSITION_UPDATE_INTERVAL {
                        if output
                            .send(AudioStreamEvent::PositionUpdate(position))
                            .await
                            .is_err()
                        {
                            log::info!("Audio channel closed, stopping stream");
                            break;
                        }
                        last_position_update = Instant::now();
                    }

                    if ffmpeg_finished || ring_buffer.buffered_duration() > 0.5 {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                }

                audio_clock.set_active(false);
            },
        ))
    }
}

fn push_s16le_bytes(ring_buffer: &AudioRingBuffer, bytes: &[u8]) {
    let samples: Vec<i16> = bytes
        .chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]))
        .collect();
    ring_buffer.push_samples(&samples);
}

/// Creates an iced subscription that streams audio from a video.
///
/// The `audio_clock` is a shared clock that this subscription writes to.
/// Video subscription should read from this clock to stay in sync.
///
/// Emits `AudioStreamEvent::PositionUpdate` every ~50ms with the current audio position.
/// Emits `AudioStreamEvent::StreamEnded` when the audio track ends.
pub fn audio_stream_subscription(
    video_path: PathBuf,
    start_timestamp: f64,
    audio_clock: Arc<AudioClock>,
    seek_generation: u64,
) -> iced::Subscription<AudioStreamEvent> {
    // Query cpal's default output config
    let (sample_rate, channels) = cpal::default_host()
        .default_output_device()
        .and_then(|device| device.default_output_config().ok())
        .map(|config| (config.sample_rate().0, config.channels()))
        .unwrap_or((48000, 2)); // Fallback to 48kHz stereo if query fails

    subscription::from_recipe(AudioStreamRecipe {
        video_path,
        start_timestamp,
        sample_rate,
        channels,
        audio_clock,
        seek_generation,
    })
}
