//! Audio playback using cpal.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Stream, StreamConfig};

/// Thread-safe ring buffer for audio samples.
///
/// The async FFmpeg reader pushes samples, and cpal's callback pulls them.
pub struct AudioRingBuffer {
    buffer: Mutex<VecDeque<i16>>,
    sample_rate: u32,
    channels: u16,
}

impl AudioRingBuffer {
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        Self {
            // ~1 sec buffer
            buffer: Mutex::new(VecDeque::with_capacity(
                sample_rate as usize * channels as usize,
            )),
            sample_rate,
            channels,
        }
    }

    /// Pull samples from the buffer (called from cpal callback).
    /// Returns the number of samples actually pulled.
    pub fn pull_samples(&self, out: &mut [i16]) -> usize {
        let mut buf = self.buffer.lock().unwrap();
        let available = buf.len().min(out.len());

        for (dest, src) in out.iter_mut().zip(buf.drain(..available)) {
            *dest = src;
        }
        out[available..].fill(0);

        available
    }

    /// Current buffer level in samples.
    pub fn len(&self) -> usize {
        self.buffer.lock().unwrap().len()
    }

    /// Buffer level in seconds.
    pub fn buffered_duration(&self) -> f64 {
        let samples = self.len();
        samples as f64 / (self.sample_rate as f64 * self.channels as f64)
    }

    /// Push raw s16le bytes directly into the buffer.
    pub fn push_s16le_bytes(&self, bytes: &[u8]) {
        let mut buf = self.buffer.lock().unwrap();
        buf.extend(
            bytes
                .chunks_exact(2)
                .map(|b| i16::from_le_bytes([b[0], b[1]])),
        );
    }
}

/// Audio player that wraps a cpal output stream.
///
/// Tracks the number of samples played for position reporting (A/V sync).
pub struct AudioPlayer {
    stream: Stream,
    samples_played: Arc<AtomicU64>,
    sample_rate: u32,
    channels: u16,
}

impl AudioPlayer {
    /// Create a new audio player.
    ///
    /// The ring buffer is shared with the caller - push samples into it
    /// and this player will pull and play them.
    pub fn new(
        ring_buffer: Arc<AudioRingBuffer>,
        sample_rate: u32,
        channels: u16,
    ) -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| "No audio output device available".to_string())?;

        log::info!(
            "Using audio device: {}",
            device
                .description()
                .map(|desc| desc.name().to_string())
                .unwrap_or_else(|_| "unknown".to_string())
        );

        let config = StreamConfig {
            channels,
            sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        let samples_played = Arc::new(AtomicU64::new(0));
        let samples_played_clone = samples_played.clone();

        let stream = device
            .build_output_stream(
                config,
                move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    let pulled = ring_buffer.pull_samples(data);
                    samples_played_clone.fetch_add(pulled as u64, Ordering::Release);
                },
                |err| {
                    log::error!("Audio stream error: {}", err);
                },
                None,
            )
            .map_err(|e| format!("Failed to build audio stream: {}", e))?;

        Ok(Self {
            stream,
            samples_played,
            sample_rate,
            channels,
        })
    }

    /// Start audio playback.
    pub fn play(&self) -> Result<(), String> {
        self.stream
            .play()
            .map_err(|e| format!("Failed to play audio: {}", e))
    }

    /// Get the current playback position in seconds.
    ///
    /// This is the master clock for A/V synchronization.
    pub fn position_seconds(&self) -> f64 {
        let samples = self.samples_played.load(Ordering::Acquire);
        // Divide by channels because samples_played counts individual channel samples
        samples as f64 / (self.sample_rate as f64 * self.channels as f64)
    }
}
