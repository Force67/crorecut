use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Cache key combining video path and rounded timestamp
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    video_path: PathBuf,
    timestamp_ms: u64, // Rounded to nearest 100ms for better hit rate
}

impl CacheKey {
    fn new(video_path: PathBuf, timestamp: f64) -> Self {
        Self {
            video_path,
            timestamp_ms: round_timestamp(timestamp),
        }
    }
}

/// Cache entry containing frame data and access time
#[derive(Debug, Clone)]
struct CacheEntry {
    frame_bytes: Vec<u8>, // Raw PNG bytes
    last_accessed: Instant,
}

/// LRU cache for video frames to reduce FFmpeg process spawning
#[derive(Debug, Clone)]
pub struct FrameCache {
    entries: HashMap<CacheKey, CacheEntry>,
    max_entries: usize,
    access_order: Vec<CacheKey>, // Track LRU order
}

impl FrameCache {
    /// Create a new frame cache with the specified capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::new(),
            max_entries: capacity,
            access_order: Vec::new(),
        }
    }

    /// Get a cached frame if available
    ///
    /// Returns a cloned copy of the frame bytes if found in cache.
    /// Updates LRU tracking on cache hit.
    pub fn get(&mut self, video_path: &Path, timestamp: f64) -> Option<Vec<u8>> {
        let key = CacheKey::new(video_path.to_path_buf(), timestamp);

        if let Some(entry) = self.entries.get_mut(&key) {
            // Cache hit! Update access time
            entry.last_accessed = Instant::now();

            // Update LRU order: move to end (most recently used)
            if let Some(pos) = self.access_order.iter().position(|k| k == &key) {
                self.access_order.remove(pos);
            }
            self.access_order.push(key);

            Some(entry.frame_bytes.clone())
        } else {
            // Cache miss
            None
        }
    }

    /// Insert a frame into the cache
    ///
    /// If the cache is at capacity, evicts the least recently used entry.
    pub fn insert(&mut self, video_path: PathBuf, timestamp: f64, frame_bytes: Vec<u8>) {
        let key = CacheKey::new(video_path, timestamp);

        // Check if we need to evict before inserting
        if self.entries.len() >= self.max_entries && !self.entries.contains_key(&key) {
            self.evict_lru();
        }

        // Insert or update entry
        self.entries.insert(
            key.clone(),
            CacheEntry {
                frame_bytes,
                last_accessed: Instant::now(),
            },
        );

        // Update LRU order: remove if exists, then add to end
        if let Some(pos) = self.access_order.iter().position(|k| k == &key) {
            self.access_order.remove(pos);
        }
        self.access_order.push(key);
    }

    /// Clear all cached frames
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.entries.clear();
        self.access_order.clear();
    }

    /// Remove all cached frames for a specific video
    #[allow(dead_code)]
    pub fn invalidate_video(&mut self, video_path: &Path) {
        // Remove entries matching the video path
        self.entries.retain(|key, _| key.video_path != video_path);
        self.access_order.retain(|key| key.video_path != video_path);
    }

    /// Evict the least recently used entry
    fn evict_lru(&mut self) {
        if let Some(lru_key) = self.access_order.first().cloned() {
            self.entries.remove(&lru_key);
            self.access_order.remove(0);
        }
    }
}

/// Round timestamp to nearest 100ms to maximize cache hits
///
/// This aligns with the existing 100ms throttle window.
/// Examples:
/// - 1.234s → 1200ms
/// - 1.289s → 1300ms
/// - 1.250s → 1300ms
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn round_timestamp(timestamp: f64) -> u64 {
    (timestamp * 10.0).round() as u64 * 100
}
