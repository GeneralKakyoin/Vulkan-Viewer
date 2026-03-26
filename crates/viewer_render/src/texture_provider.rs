use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use viewer_core::AssetID;

/// Represents the state of a texture asset in the rendering pipeline.
#[derive(Clone)]
pub enum PendingTexture {
    /// Asset is being fetched or prepared.
    Loading,
    /// Asset is ready for GPU use.
    Ready(Arc<wgpu::TextureView>),
    /// Asset failed to load or does not exist.
    Missing,
}

/// A trait for providing GPU texture views for given AssetIDs.
pub trait TextureProvider {
    /// Request a texture view for the given ID.
    fn get_texture(&self, id: &AssetID) -> PendingTexture;
    /// Update visibility score for the given ID.
    fn touch(&self, id: &AssetID, score: f32);
    /// Decay all scores.
    fn clear_scores(&self);
}

/// A fallback texture provider that returns Loading for all non-empty lookups.
pub struct DefaultTextureProvider;

impl DefaultTextureProvider {
    pub fn new() -> Self {
        Self
    }
}

impl TextureProvider for DefaultTextureProvider {
    fn get_texture(&self, _id: &AssetID) -> PendingTexture {
        if _id.is_empty() {
            PendingTexture::Missing
        } else {
            PendingTexture::Loading // Simulate loading for any non-empty ID
        }
    }
    fn touch(&self, _id: &AssetID, _score: f32) {}
    fn clear_scores(&self) {}
}

/// A provider that holds a map of ready texture views and returns Loading for others.
/// Includes VRAM management and LRU eviction.
pub struct StreamingTextureProvider {
    views: Mutex<std::collections::HashMap<AssetID, (Arc<wgpu::TextureView>, u64)>>, // (View, SizeInBytes)
    lru_scores: Mutex<std::collections::HashMap<AssetID, f32>>, // ID -> Visibility Score
    current_vram: AtomicU64,
    max_vram: u64,
}

impl StreamingTextureProvider {
    pub fn new(max_vram_mb: u64) -> Self {
        Self {
            views: Mutex::new(std::collections::HashMap::new()),
            lru_scores: Mutex::new(std::collections::HashMap::new()),
            current_vram: AtomicU64::new(0),
            max_vram: max_vram_mb * 1024 * 1024,
        }
    }

    pub fn insert(&self, id: AssetID, view: Arc<wgpu::TextureView>, size: u64) -> Vec<AssetID> {
        let mut views = self.views.lock().unwrap();
        let mut lru_scores = self.lru_scores.lock().unwrap();
        let mut evicted = Vec::new();

        // Evict if over limit
        while self.current_vram.load(Ordering::Relaxed) + size > self.max_vram && !views.is_empty()
        {
            if let Some(to_evict) = self.find_eviction_candidate_locked(&lru_scores) {
                if let Some((_, evicted_size)) = views.remove(&to_evict) {
                    self.current_vram.fetch_sub(evicted_size, Ordering::Relaxed);
                }
                lru_scores.remove(&to_evict);
                evicted.push(to_evict);
            } else {
                break; // Nothing more to evict
            }
        }

        if !views.contains_key(&id) {
            self.current_vram.fetch_add(size, Ordering::Relaxed);
        }
        views.insert(id.clone(), (view, size));
        lru_scores.insert(id, 0.0); // Reset score
        evicted
    }

    pub fn touch_mut(&self, id: &AssetID, score: f32) {
        let mut scores = self.lru_scores.lock().unwrap();
        if let Some(s) = scores.get_mut(id) {
            *s = s.max(score);
        }
    }

    pub fn clear_scores_mut(&self) {
        let mut scores = self.lru_scores.lock().unwrap();
        for s in scores.values_mut() {
            *s *= 0.5; // Decay scores over time
        }
    }

    fn find_eviction_candidate_locked(
        &self,
        scores: &std::collections::HashMap<AssetID, f32>,
    ) -> Option<AssetID> {
        scores
            .iter()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(id, _)| id.clone())
    }

    // The original `evict` method is no longer needed as its logic is integrated into `insert`.
    pub fn contains(&self, id: &AssetID) -> bool {
        self.views.lock().unwrap().contains_key(id)
    }

    pub fn vram_usage_mb(&self) -> f32 {
        self.current_vram.load(Ordering::Relaxed) as f32 / (1024.0 * 1024.0)
    }
}

impl TextureProvider for StreamingTextureProvider {
    fn get_texture(&self, id: &AssetID) -> PendingTexture {
        let views = self.views.lock().unwrap();
        if let Some((view, _)) = views.get(id) {
            PendingTexture::Ready(Arc::clone(view))
        } else if id.is_empty() {
            PendingTexture::Missing
        } else {
            PendingTexture::Loading
        }
    }

    fn touch(&self, id: &AssetID, score: f32) {
        self.touch_mut(id, score);
    }

    fn clear_scores(&self) {
        self.clear_scores_mut();
    }
}
