use anyhow::Result;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use viewer_core::AssetID;

use crate::AssetStatus;

#[derive(Debug, Clone)]
pub struct DecodedRgbaImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl DecodedRgbaImage {
    pub fn byte_len(&self) -> usize {
        self.rgba.len()
    }
}

pub type FixtureTextureStatus = AssetStatus<Arc<DecodedRgbaImage>>;

pub struct FixtureTextureCache {
    base_dir: PathBuf,
    budget_bytes: usize,
    current_bytes: usize,
    entries: HashMap<AssetID, Arc<DecodedRgbaImage>>,
    lru: VecDeque<AssetID>,
    pending: VecDeque<AssetID>,
    pending_set: HashSet<AssetID>,
    negative: VecDeque<AssetID>,
    negative_set: HashSet<AssetID>,
    negative_cap: usize,
    oversized_ready: HashMap<AssetID, Arc<DecodedRgbaImage>>,
    oversized_lru: VecDeque<AssetID>,
    oversized_cap: usize,
}

impl FixtureTextureCache {
    pub fn new() -> Self {
        Self::with_base_dir(default_fixture_dir(), budget_bytes_from_env())
    }

    pub fn with_base_dir(base_dir: PathBuf, budget_bytes: usize) -> Self {
        Self {
            base_dir,
            budget_bytes: budget_bytes.max(1),
            current_bytes: 0,
            entries: HashMap::new(),
            lru: VecDeque::new(),
            pending: VecDeque::new(),
            pending_set: HashSet::new(),
            negative: VecDeque::new(),
            negative_set: HashSet::new(),
            negative_cap: 2048,
            oversized_ready: HashMap::new(),
            oversized_lru: VecDeque::new(),
            oversized_cap: 16,
        }
    }

    pub fn budget_bytes(&self) -> usize {
        self.budget_bytes
    }

    pub fn current_bytes(&self) -> usize {
        self.current_bytes
    }

    /// Request a fixture PNG for the given `AssetID`.
    ///
    /// Contract:
    /// - `Missing` for empty IDs, invalid filenames, missing files, or decode failures.
    /// - `Loading` on the first request of a valid on-disk fixture until `poll_png_rgba8(...)`
    ///   processes the queued request.
    /// - `Ready` when the decoded image is cached (or returned uncached if it exceeds the budget).
    pub fn request_png_rgba8(&mut self, id: &AssetID) -> Result<FixtureTextureStatus> {
        if id.is_empty() {
            return Ok(AssetStatus::Missing);
        }

        if let Some(entry) = self.entries.get(id).cloned() {
            self.touch_lru(id);
            return Ok(AssetStatus::Ready(entry));
        }

        if let Some(entry) = self.oversized_ready.get(id).cloned() {
            self.touch_oversized_lru(id);
            return Ok(AssetStatus::Ready(entry));
        }

        if self.negative_set.contains(id) {
            return Ok(AssetStatus::Missing);
        }

        let file = match sanitize_fixture_filename(id.as_str()) {
            Some(file) => file,
            None => {
                self.record_negative(id.clone());
                return Ok(AssetStatus::Missing);
            }
        };

        let path = self.base_dir.join(file);
        if !path.exists() {
            self.record_negative(id.clone());
            return Ok(AssetStatus::Missing);
        }

        if self.pending_set.contains(id) {
            return Ok(AssetStatus::Loading);
        }

        self.pending.push_back(id.clone());
        self.pending_set.insert(id.clone());
        Ok(AssetStatus::Loading)
    }

    /// Process queued fixture requests.
    ///
    /// Returns the number of requests that were attempted (success or failure).
    pub fn poll_png_rgba8(&mut self, max_to_process: usize) -> Result<usize> {
        let mut attempted = 0usize;
        for _ in 0..max_to_process {
            let Some(id) = self.pending.pop_front() else {
                break;
            };
            self.pending_set.remove(&id);
            attempted += 1;

            if id.is_empty() || self.entries.contains_key(&id) || self.negative_set.contains(&id) {
                continue;
            }

            let file = match sanitize_fixture_filename(id.as_str()) {
                Some(file) => file,
                None => {
                    self.record_negative(id);
                    continue;
                }
            };
            let path = self.base_dir.join(file);
            if !path.exists() {
                self.record_negative(id);
                continue;
            }

            let bytes = match std::fs::read(&path) {
                Ok(bytes) => bytes,
                Err(_) => {
                    self.record_negative(id);
                    continue;
                }
            };

            let decoded = match decode_png_rgba8(&bytes) {
                Ok(decoded) => decoded,
                Err(_) => {
                    self.record_negative(id);
                    continue;
                }
            };

            let size = decoded.byte_len();
            let decoded = Arc::new(decoded);

            // If a single asset exceeds the cache budget, do not cache it, but still consider it ready.
            if size > self.budget_bytes {
                self.record_oversized_ready(id, decoded);
                continue;
            }

            self.ensure_capacity(size);
            self.current_bytes += size;
            self.entries.insert(id.clone(), Arc::clone(&decoded));
            self.touch_lru(&id);
        }

        Ok(attempted)
    }

    fn touch_lru(&mut self, id: &AssetID) {
        self.lru.retain(|k| k != id);
        self.lru.push_back(id.clone());
    }

    fn touch_oversized_lru(&mut self, id: &AssetID) {
        self.oversized_lru.retain(|k| k != id);
        self.oversized_lru.push_back(id.clone());
    }

    fn ensure_capacity(&mut self, incoming: usize) {
        while self.current_bytes + incoming > self.budget_bytes && !self.lru.is_empty() {
            let Some(evict) = self.lru.pop_front() else {
                break;
            };
            if let Some(entry) = self.entries.remove(&evict) {
                self.current_bytes = self.current_bytes.saturating_sub(entry.byte_len());
            }
        }
    }

    fn record_negative(&mut self, id: AssetID) {
        if self.negative_set.contains(&id) {
            return;
        }
        if self.negative_set.len() >= self.negative_cap {
            if let Some(old) = self.negative.pop_front() {
                self.negative_set.remove(&old);
            }
        }
        self.negative.push_back(id.clone());
        self.negative_set.insert(id);
    }

    fn record_oversized_ready(&mut self, id: AssetID, decoded: Arc<DecodedRgbaImage>) {
        if !self.oversized_ready.contains_key(&id)
            && self.oversized_ready.len() >= self.oversized_cap
        {
            if let Some(old) = self.oversized_lru.pop_front() {
                self.oversized_ready.remove(&old);
            } else {
                self.oversized_ready.clear();
            }
        }
        self.oversized_ready.insert(id.clone(), decoded);
        self.touch_oversized_lru(&id);
    }

    #[cfg(test)]
    fn insert_dummy_for_test(&mut self, id: AssetID, bytes: usize) {
        let decoded = Arc::new(DecodedRgbaImage {
            width: 1,
            height: 1,
            rgba: vec![0u8; bytes],
        });
        let size = decoded.byte_len();
        if size > self.budget_bytes {
            return;
        }
        self.ensure_capacity(size);
        self.current_bytes += size;
        self.entries.insert(id.clone(), decoded);
        self.touch_lru(&id);
    }
}

fn decode_png_rgba8(bytes: &[u8]) -> Result<DecodedRgbaImage> {
    let image = image::load_from_memory(bytes)?;
    let rgba8 = image.to_rgba8();
    Ok(DecodedRgbaImage {
        width: rgba8.width(),
        height: rgba8.height(),
        rgba: rgba8.into_raw(),
    })
}

fn sanitize_fixture_filename(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return None;
    }
    if trimmed.contains(':') {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.ends_with(".png") {
        Some(trimmed.to_string())
    } else if lower.contains('.') {
        // Unknown extension: treat as filename, but still restrict to base-dir only.
        Some(trimmed.to_string())
    } else {
        Some(format!("{trimmed}.png"))
    }
}

fn default_fixture_dir() -> PathBuf {
    // crates/viewer_asset -> ../../test_assets
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("test_assets")
}

fn budget_bytes_from_env() -> usize {
    let mb = std::env::var("VIEWER_ASSET_CACHE_BUDGET_MB")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(512);
    let mb = mb.clamp(16, 4096);
    mb as usize * 1024 * 1024
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_on_empty_id() {
        let mut cache = FixtureTextureCache::with_base_dir(PathBuf::from("."), 1024);
        let status = cache.request_png_rgba8(&AssetID::default()).unwrap();
        assert!(matches!(status, AssetStatus::Missing));
    }

    #[test]
    fn request_then_poll_yields_ready_for_known_fixture() {
        let mut cache = FixtureTextureCache::new();
        let id = AssetID::new("water_diffuse");

        let first = cache.request_png_rgba8(&id).unwrap();
        assert!(matches!(first, AssetStatus::Loading));

        cache.poll_png_rgba8(8).unwrap();
        let second = cache.request_png_rgba8(&id).unwrap();
        match second {
            AssetStatus::Ready(img) => {
                assert!(img.width > 0);
                assert!(img.height > 0);
                assert_eq!(img.rgba.len(), img.width as usize * img.height as usize * 4);
            }
            other => panic!("expected Ready, got {other:?}"),
        }
    }

    #[test]
    fn missing_is_negative_cached() {
        let mut cache = FixtureTextureCache::new();
        let id = AssetID::new("does_not_exist");
        let a = cache.request_png_rgba8(&id).unwrap();
        assert!(matches!(a, AssetStatus::Missing));
        assert!(cache.pending_set.is_empty());
        let b = cache.request_png_rgba8(&id).unwrap();
        assert!(matches!(b, AssetStatus::Missing));
    }

    #[test]
    fn lru_eviction_is_deterministic() {
        let mut cache = FixtureTextureCache::with_base_dir(PathBuf::from("."), 10);
        cache.insert_dummy_for_test(AssetID::new("a"), 4);
        cache.insert_dummy_for_test(AssetID::new("b"), 4);
        assert_eq!(cache.current_bytes(), 8);
        cache.insert_dummy_for_test(AssetID::new("c"), 4);
        // Budget is 10, so one of the old entries must be evicted. Deterministic LRU: "a" goes first.
        assert!(cache.entries.contains_key(&AssetID::new("b")));
        assert!(cache.entries.contains_key(&AssetID::new("c")));
        assert!(!cache.entries.contains_key(&AssetID::new("a")));
    }
}
