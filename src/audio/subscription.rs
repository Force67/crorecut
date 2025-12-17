//! Audio streaming subscription for iced.
//!
//! This module provides an iced subscription that streams audio from a video
//! file using FFmpeg and plays it via cpal. It emits position updates for A/V sync.

use std::hash::Hash;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use iced::advanced::subscription::{self, Recipe};
use iced::futures::SinkExt;
use iced::futures::stream::BoxStream;

use crate::audio::audio_streaming::AudioStreaming;

use super::player::{AudioPlayer, AudioRingBuffer};

pub const SAMPLE_RATE: u32 = 48000;
pub const CHANNELS: u16 = 2;

/// How often to emit position updates
const POSITION_UPDATE_INTERVAL: Duration = Duration::from_millis(50);

/// How much audio to pre-buffer before starting playback (~100ms).
const PRE_BUFFER_SAMPLES: usize = 48000 / 10 * 2; // 100ms of stereo audio

/// Recipe for streaming audio via iced's subscription system.
struct AudioStreamRecipe {
    video_path: PathBuf,
    start_timestamp: f64,
    sample_rate: u32,
    channels: u16,
}

impl Recipe for AudioStreamRecipe {
    type Output = crate::app::Message;

    fn hash(&self, state: &mut subscription::Hasher) {
        self.video_path.hash(state);
        "audio".hash(state);
    }

    fn stream(
        self: Box<Self>,
        _input: subscription::EventStream,
    ) -> BoxStream<'static, Self::Output> {
        let video_path = self.video_path;
        let start_timestamp = self.start_timestamp;
        let sample_rate = self.sample_rate;
        let channels = self.channels;

        Box::pin(iced::stream::channel(
            1,
            move |mut output: iced::futures::channel::mpsc::Sender<crate::app::Message>| async move {
                log::info!("Audio stream channel started, spawning ffmpeg...");

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

                // Pre-buffer before starting playback
                log::info!("Pre-buffering audio...");
                while ring_buffer.len() < PRE_BUFFER_SAMPLES {
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
                        let _ = output.send(crate::app::Message::AudioStreamEnded).await;
                        break;
                    }

                    // Emit position update at regular intervals
                    if last_position_update.elapsed() >= POSITION_UPDATE_INTERVAL {
                        let position = start_timestamp + player.position_seconds();
                        if output
                            .send(crate::app::Message::AudioPositionUpdate(position))
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

                log::info!("Audio stream ended");
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
/// Emits `Message::AudioPositionUpdate` every ~50ms with the current audio position.
/// Emits `Message::AudioStreamEnded` when the audio track ends.
pub fn audio_stream_subscription(
    video_path: PathBuf,
    start_timestamp: f64,
) -> iced::Subscription<crate::app::Message> {
    subscription::from_recipe(AudioStreamRecipe {
        video_path,
        start_timestamp,
        sample_rate: SAMPLE_RATE,
        channels: CHANNELS,
    })
}
