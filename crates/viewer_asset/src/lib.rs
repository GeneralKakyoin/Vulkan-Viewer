use std::collections::HashMap;
use std::sync::Arc;
use viewer_core::geometry::llvolume::{SubMesh, generate_volume_mesh};
use viewer_core::geometry::sculpt::generate_sculpt_mesh;
use viewer_core::{SculptType, Vertex, VolumeParams};

pub mod mesh_loader;
pub mod texture_fixture;
use mesh_loader::load_gltf_mesh;
pub use texture_fixture::{DecodedRgbaImage, FixtureTextureCache};

#[derive(Debug, Clone)]
pub enum AssetStatus<T> {
    Loading,
    Ready(T),
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AssetSourceKind {
    Fixture,
    Live,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AssetFetchFailureReason {
    Transport,
    Decode,
    Unsupported,
    MissingCapability,
    Timeout,
    Other,
}

#[derive(Debug, Clone)]
pub struct AssetFetchRequest {
    pub id: viewer_core::AssetID,
    pub priority: viewer_core::AssetPriority,
}

#[derive(Debug, Clone)]
pub struct AssetFetchOutcome<T> {
    pub status: AssetStatus<T>,
    pub source: AssetSourceKind,
    pub failure: Option<AssetFetchFailureReason>,
}

pub trait LiveTextureProvider: Send + Sync {
    fn request_texture(&mut self, request: &AssetFetchRequest) -> anyhow::Result<()>;
    fn poll_texture(
        &mut self,
        id: &viewer_core::AssetID,
    ) -> anyhow::Result<Option<AssetFetchOutcome<DecodedRgbaImage>>>;
}

pub struct ProcessedMesh {
    pub vertices: Vec<Vertex>,
    pub submeshes: Vec<SubMesh>,
    pub aabb: viewer_core::Aabb,
}

#[derive(Default)]
pub struct GeometryCache {
    pub procedural: HashMap<VolumeParams, Arc<ProcessedMesh>>,
    pub sculpts: HashMap<(String, SculptType), Arc<ProcessedMesh>>,
    pub meshes: HashMap<(String, u32), Arc<ProcessedMesh>>, // UUID, LOD
    pub mesh_attempted_hash: HashMap<(String, u32), u64>, // prevents per-frame reload of same bytes
}

impl GeometryCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_procedural(&mut self, params: &VolumeParams, detail: f32) -> Arc<ProcessedMesh> {
        if let Some(mesh) = self.procedural.get(params) {
            return Arc::clone(mesh);
        }
        let (vertices, submeshes, aabb) = generate_volume_mesh(params, detail);
        let mesh = Arc::new(ProcessedMesh {
            vertices,
            submeshes,
            aabb,
        });
        self.procedural.insert(params.clone(), Arc::clone(&mesh));
        mesh
    }

    pub fn get_sculpt(
        &mut self,
        uuid: &str,
        sculpt_type: SculptType,
        pixels: &[u8],
        width: u32,
        height: u32,
    ) -> Arc<ProcessedMesh> {
        let key = (uuid.to_string(), sculpt_type);
        if let Some(mesh) = self.sculpts.get(&key) {
            return Arc::clone(mesh);
        }
        let (vertices, indices, aabb) = generate_sculpt_mesh(pixels, width, height, sculpt_type);
        let mesh = Arc::new(ProcessedMesh {
            vertices,
            submeshes: vec![SubMesh {
                face_id: 0, // Sculpted prims have one face (default to 0)
                indices,
            }],
            aabb,
        });
        self.sculpts.insert(key, Arc::clone(&mesh));
        mesh
    }

    pub fn get_mesh(&mut self, uuid: &str, lod: u32, data: &[u8]) -> Arc<ProcessedMesh> {
        let key = (uuid.to_string(), lod);

        // Ensure we always have a stable cached entry for determinism (even when missing).
        // This avoids per-frame work and also prevents "poisoning" the cache permanently:
        // if real bytes arrive later, we can retry when the input hash changes.
        let cached = self.meshes.entry(key.clone()).or_insert_with(|| {
            Arc::new(ProcessedMesh {
                vertices: vec![],
                submeshes: vec![],
                aabb: viewer_core::Aabb::new([0.0, 0.0, 0.0], [0.1, 0.1, 0.1]),
            })
        });

        let is_ready = !cached.vertices.is_empty() && !cached.submeshes.is_empty();
        if is_ready {
            return Arc::clone(cached);
        }

        if data.is_empty() {
            self.mesh_attempted_hash.insert(key, 0);
            return Arc::clone(cached);
        }

        let new_hash = hash_bytes(data);
        if self.mesh_attempted_hash.get(&key).copied() == Some(new_hash) {
            return Arc::clone(cached);
        }

        match load_gltf_mesh(data) {
            Ok(mesh) => {
                let mesh_arc = Arc::new(mesh);
                self.meshes.insert(key.clone(), Arc::clone(&mesh_arc));
                self.mesh_attempted_hash.remove(&key);
                mesh_arc
            }
            Err(_) => {
                self.mesh_attempted_hash.insert(key, new_hash);
                Arc::clone(cached)
            }
        }
    }
}

pub fn get_lod_level(distance: f32) -> u32 {
    if distance < 15.0 {
        0
    }
    // High
    else if distance < 40.0 {
        1
    }
    // Mid
    else if distance < 80.0 {
        2
    }
    // Low
    else {
        3
    } // Tiny
}

fn hash_bytes(data: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}

pub fn decode_png_rgba8(bytes: &[u8]) -> anyhow::Result<DecodedRgbaImage> {
    let img = image::load_from_memory(bytes)?;
    let (width, height) = image::GenericImageView::dimensions(&img);
    let rgba = img.to_rgba8().into_raw();
    Ok(DecodedRgbaImage {
        width,
        height,
        rgba,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_mesh_caches_missing_entry_on_empty_data() {
        let mut cache = GeometryCache::new();
        let a = cache.get_mesh("dummy", 0, &[]);
        let b = cache.get_mesh("dummy", 0, &[]);
        assert!(Arc::ptr_eq(&a, &b));
        assert!(a.vertices.is_empty());
        assert!(a.submeshes.is_empty());
    }

    #[test]
    fn get_mesh_does_not_reparse_same_invalid_bytes_every_call() {
        let mut cache = GeometryCache::new();
        let a = cache.get_mesh("dummy", 0, b"not-a-gltf");
        let b = cache.get_mesh("dummy", 0, b"not-a-gltf");
        assert!(Arc::ptr_eq(&a, &b));
        assert!(a.vertices.is_empty());
        assert!(a.submeshes.is_empty());
    }
}
