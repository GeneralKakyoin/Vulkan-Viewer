use anyhow::Result;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use viewer_core::{AssetContinuityMetrics, AssetID, AssetPriority};

use crate::AssetStatus;

pub const A10_CONTINUITY_WINDOW_MS: u64 = 45_000;
pub const A10_MAX_NEIGHBOR_SCOPES: usize = 8;
pub const A10_REQUESTS_PER_TICK_CAP: usize = 64;
pub const A10_PROMOTIONS_PER_TICK_CAP: usize = 24;

pub const A10_QUOTA_ACTIVE: usize = 96;
pub const A10_QUOTA_PREVIOUS: usize = 48;
pub const A10_QUOTA_NEIGHBOR: usize = 64;
pub const A10_QUOTA_TOTAL_CONTINUITY: usize = 160;

#[derive(Debug, Clone)]
pub struct CacheEntryMetadata {
    pub priority: AssetPriority,
    pub last_touched_tick: u64,
    pub added_at_ms: u64,
    pub size_bytes: usize,
}

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
    metadata: HashMap<AssetID, CacheEntryMetadata>,
    current_tick: u64,

    pending: VecDeque<(AssetID, AssetPriority)>,
    pending_set: HashSet<AssetID>,
    negative: VecDeque<AssetID>,
    negative_set: HashSet<AssetID>,
    negative_cap: usize,
    oversized_ready: HashMap<AssetID, Arc<DecodedRgbaImage>>,
    oversized_lru: VecDeque<AssetID>,
    oversized_cap: usize,

    // Metrics
    pub metrics: AssetContinuityMetrics,
    promotions_this_tick: usize,
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
            metadata: HashMap::new(),
            current_tick: 0,
            pending: VecDeque::new(),
            pending_set: HashSet::new(),
            negative: VecDeque::new(),
            negative_set: HashSet::new(),
            negative_cap: 2048,
            oversized_ready: HashMap::new(),
            oversized_lru: VecDeque::new(),
            oversized_cap: 16,
            metrics: AssetContinuityMetrics::default(),
            promotions_this_tick: 0,
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
        self.request_png_rgba8_with_priority(id, AssetPriority::Normal)
    }

    /// Explicitly request with priority (A10)
    pub fn request_png_rgba8_with_priority(
        &mut self,
        id: &AssetID,
        priority: AssetPriority,
    ) -> Result<FixtureTextureStatus> {
        if id.is_empty() {
            return Ok(AssetStatus::Missing);
        }

        if let Some(entry) = self.entries.get(id).cloned() {
            self.touch(id, priority);
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
            self.promote_pending_priority(id, priority);
            return Ok(AssetStatus::Loading);
        }

        if self.pending.len() >= A10_REQUESTS_PER_TICK_CAP {
            self.metrics.continuity_requests_dropped_cap += 1;
            return Ok(AssetStatus::Loading); // Treat as loading so it retries later
        }

        let priority = self.clamp_priority_to_quota(priority);
        self.pending.push_back((id.clone(), priority));
        self.pending_set.insert(id.clone());
        self.metrics.continuity_requests_enqueued += 1;
        Ok(AssetStatus::Loading)
    }

    /// Process queued fixture requests.
    ///
    /// Returns the number of requests that were attempted (success or failure).
    pub fn poll_png_rgba8(&mut self, max_to_process: usize) -> Result<usize> {
        let mut attempted = 0usize;
        self.current_tick += 1;
        self.promotions_this_tick = 0;
        let now = now_unix_ms();

        for _ in 0..max_to_process {
            let Some((id, priority)) = self.pending.pop_front() else {
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
            self.metadata.insert(
                id.clone(),
                CacheEntryMetadata {
                    priority: self.clamp_priority_to_quota(priority),
                    last_touched_tick: self.current_tick,
                    added_at_ms: now,
                    size_bytes: size,
                },
            );

            match priority {
                AssetPriority::Active => self.metrics.continuity_retained_active += 1,
                AssetPriority::Previous => self.metrics.continuity_retained_previous += 1,
                AssetPriority::Neighbor => self.metrics.continuity_retained_neighbor += 1,
                AssetPriority::Normal => {}
            }
        }

        Ok(attempted)
    }

    fn touch(&mut self, id: &AssetID, requested_priority: AssetPriority) {
        let priority = self.clamp_priority_to_quota(requested_priority);
        if let Some(meta) = self.metadata.get_mut(id) {
            meta.last_touched_tick = self.current_tick;
            // Promote priority if the new request has higher priority
            if priority < meta.priority {
                if self.promotions_this_tick >= A10_PROMOTIONS_PER_TICK_CAP {
                    return;
                }
                meta.priority = priority;
                self.promotions_this_tick += 1;
                if priority == AssetPriority::Neighbor {
                    self.metrics.continuity_promoted_neighbor += 1;
                }
            }
        }
    }

    fn touch_oversized_lru(&mut self, id: &AssetID) {
        self.oversized_lru.retain(|k| k != id);
        self.oversized_lru.push_back(id.clone());
    }

    fn ensure_capacity(&mut self, incoming: usize) {
        let now = now_unix_ms();
        // 1. Evict expired previous/neighbor entries
        let mut to_evict = Vec::new();
        for (id, meta) in &self.metadata {
            if (meta.priority == AssetPriority::Previous
                || meta.priority == AssetPriority::Neighbor)
                && now.saturating_sub(meta.added_at_ms) > A10_CONTINUITY_WINDOW_MS
            {
                to_evict.push(id.clone());
            }
        }
        for id in to_evict {
            self.evict(&id);
            self.metrics.continuity_evicted_expired += 1;
        }

        if self.current_bytes + incoming > self.budget_bytes {
            self.metrics.continuity_budget_pressure_events += 1;
        }

        // 2. Budget eviction
        while self.current_bytes + incoming > self.budget_bytes && !self.entries.is_empty() {
            if let Some(id) = self.find_eviction_candidate() {
                self.evict(&id);
                self.metrics.continuity_evicted_budget += 1;
            } else {
                break;
            }
        }
    }

    fn find_eviction_candidate(&self) -> Option<AssetID> {
        // Priority rank (highest to lowest): Active -> Previous -> Neighbor -> Normal
        // We want to evict the lowest priority first.
        let mut candidates: Vec<_> = self.metadata.iter().collect();
        // Sort by (priority [desc], last_touched_tick [asc], id [asc])
        candidates.sort_by(|(id_a, meta_a), (id_b, meta_b)| {
            meta_b
                .priority
                .cmp(&meta_a.priority)
                .then_with(|| meta_a.last_touched_tick.cmp(&meta_b.last_touched_tick))
                .then_with(|| id_a.as_str().cmp(id_b.as_str()))
        });

        candidates.first().map(|(id, _)| (*id).clone())
    }

    fn evict(&mut self, id: &AssetID) {
        if let Some(entry) = self.entries.remove(id) {
            self.current_bytes = self.current_bytes.saturating_sub(entry.byte_len());
        }
        self.metadata.remove(id);
    }

    fn continuity_counts(&self) -> (usize, usize, usize) {
        let mut active = 0usize;
        let mut previous = 0usize;
        let mut neighbor = 0usize;
        for meta in self.metadata.values() {
            match meta.priority {
                AssetPriority::Active => active += 1,
                AssetPriority::Previous => previous += 1,
                AssetPriority::Neighbor => neighbor += 1,
                AssetPriority::Normal => {}
            }
        }
        (active, previous, neighbor)
    }

    fn clamp_priority_to_quota(&self, requested: AssetPriority) -> AssetPriority {
        if matches!(requested, AssetPriority::Normal) {
            return AssetPriority::Normal;
        }
        let (active, previous, neighbor) = self.continuity_counts();
        let total = active + previous + neighbor;
        if total >= A10_QUOTA_TOTAL_CONTINUITY {
            return AssetPriority::Normal;
        }

        match requested {
            AssetPriority::Active => {
                if active < A10_QUOTA_ACTIVE {
                    AssetPriority::Active
                } else {
                    AssetPriority::Normal
                }
            }
            AssetPriority::Previous => {
                if previous < A10_QUOTA_PREVIOUS {
                    AssetPriority::Previous
                } else {
                    AssetPriority::Normal
                }
            }
            AssetPriority::Neighbor => {
                if neighbor < A10_QUOTA_NEIGHBOR {
                    AssetPriority::Neighbor
                } else {
                    AssetPriority::Normal
                }
            }
            AssetPriority::Normal => AssetPriority::Normal,
        }
    }

    fn promote_pending_priority(&mut self, id: &AssetID, requested_priority: AssetPriority) {
        let priority = self.clamp_priority_to_quota(requested_priority);
        for (queued_id, queued_priority) in &mut self.pending {
            if queued_id != id {
                continue;
            }
            if priority < *queued_priority
                && self.promotions_this_tick < A10_PROMOTIONS_PER_TICK_CAP
            {
                *queued_priority = priority;
                self.promotions_this_tick += 1;
                if priority == AssetPriority::Neighbor {
                    self.metrics.continuity_promoted_neighbor += 1;
                }
            }
            break;
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
        self.current_tick += 1;
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
        self.metadata.insert(
            id.clone(),
            CacheEntryMetadata {
                priority: AssetPriority::Normal,
                last_touched_tick: self.current_tick,
                added_at_ms: now_unix_ms(),
                size_bytes: size,
            },
        );
    }
}

fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
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
    fn priority_eviction_is_deterministic() {
        let mut cache = FixtureTextureCache::with_base_dir(PathBuf::from("."), 10);
        // "a" is Normal, "b" is Active
        cache.insert_dummy_for_test(AssetID::new("a"), 4);
        cache.insert_dummy_for_test(AssetID::new("b"), 4);

        // Manually set b to Active
        cache.touch(&AssetID::new("b"), AssetPriority::Active);

        assert_eq!(cache.current_bytes(), 8);

        // Insert "c" (Normal). Total 12 > 10.
        // "a" (Normal) should be evicted before "b" (Active) even if "a" was added first,
        // because we sort by priority descending (Normal is lower than Active).
        // Wait, my find_eviction_candidate sorts by meta_b.priority.cmp(&meta_a.priority).
        // Active (0) < Previous (1) < Neighbor (2) < Normal (3)
        // cmp will return Less for (Active, Normal).
        // meta_b (Normal) .cmp (meta_a Active) -> Greater.
        // So Normal comes first in candidates (highest rank for eviction).
        cache.insert_dummy_for_test(AssetID::new("c"), 4);

        assert!(cache.entries.contains_key(&AssetID::new("b")));
        assert!(cache.entries.contains_key(&AssetID::new("c")));
        assert!(!cache.entries.contains_key(&AssetID::new("a")));
    }

    #[test]
    fn pending_request_priority_can_be_promoted() {
        let mut cache = FixtureTextureCache::new();
        let id = AssetID::new("water_diffuse");

        let initial = cache.request_png_rgba8(&id).unwrap();
        assert!(matches!(initial, AssetStatus::Loading));

        let promoted = cache
            .request_png_rgba8_with_priority(&id, AssetPriority::Active)
            .unwrap();
        assert!(matches!(promoted, AssetStatus::Loading));

        cache.poll_png_rgba8(1).unwrap();
        let meta = cache.metadata.get(&id).expect("metadata should exist");
        assert_eq!(meta.priority, AssetPriority::Active);
    }

    #[test]
    fn active_priority_is_clamped_when_quota_is_full() {
        let mut cache = FixtureTextureCache::with_base_dir(PathBuf::from("."), 1024 * 1024);
        for i in 0..A10_QUOTA_ACTIVE {
            let id = AssetID::new(format!("active_{i}"));
            cache.insert_dummy_for_test(id.clone(), 1);
            if let Some(meta) = cache.metadata.get_mut(&id) {
                meta.priority = AssetPriority::Active;
            }
        }
        assert_eq!(
            cache.clamp_priority_to_quota(AssetPriority::Active),
            AssetPriority::Normal
        );
    }
}
