//! Shared audio clock for A/V synchronization. Audio playback is the master clock

use std::sync::atomic::{AtomicU64, Ordering};

/// Thread-safe audio clock that can be shared between audio and video subscriptions.
#[derive(Debug, Default)]
pub struct AudioClock {
    /// Audio position encoded as f64 bits (using to_bits/from_bits).
    position_bits: AtomicU64,
    active: std::sync::atomic::AtomicBool,
}

impl AudioClock {
    pub fn new() -> Self {
        Self {
            position_bits: AtomicU64::new(0f64.to_bits()),
            active: std::sync::atomic::AtomicBool::new(false),
        }
    }

    pub fn set_position(&self, position: f64) {
        self.position_bits
            .store(position.to_bits(), Ordering::Release);
    }

    pub fn position(&self) -> f64 {
        f64::from_bits(self.position_bits.load(Ordering::Acquire))
    }

    pub fn set_active(&self, active: bool) {
        self.active
            .store(active, std::sync::atomic::Ordering::Release);
    }
}
