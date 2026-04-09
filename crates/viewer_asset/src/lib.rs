use std::collections::HashMap;
use std::sync::Arc;
use viewer_core::geometry::llvolume::{SubMesh, generate_volume_mesh};
use viewer_core::geometry::sculpt::generate_sculpt_mesh;
use viewer_core::{SculptType, Vertex, VolumeParams};

pub mod mesh_loader;
mod sl_mesh_loader;
pub mod texture_fixture;

pub use sl_mesh_loader::{
    MeshSourceFormat, debug_triangle_second_life_mesh_bytes, detect_mesh_source_format,
};
pub use texture_fixture::{DecodedRgbaImage, FixtureTextureCache};

pub mod mesh_decode_utils;
pub mod texture_decode_utils;

pub(crate) use mesh_decode_utils::*;
pub use texture_decode_utils::*;

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

#[derive(Debug)]
pub struct ProcessedMesh {
    pub vertices: Vec<Vertex>,
    pub submeshes: Vec<SubMesh>,
    pub aabb: viewer_core::Aabb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshCacheLookupState {
    EmptyData,
    CachedReady,
    Decoded,
    DecodeFailed,
}

#[derive(Debug, Clone)]
pub struct MeshCacheLookup {
    pub mesh: Arc<ProcessedMesh>,
    pub state: MeshCacheLookupState,
    pub format: MeshSourceFormat,
    pub failure: Option<AssetFetchFailureReason>,
    pub detail: Option<String>,
}

#[derive(Default)]
pub struct GeometryCache {
    pub procedural: HashMap<VolumeParams, Arc<ProcessedMesh>>,
    pub sculpts: HashMap<(String, SculptType), Arc<ProcessedMesh>>,
    pub meshes: HashMap<(String, u32), Arc<ProcessedMesh>>, // UUID, LOD
    pub mesh_attempted_hash: HashMap<(String, u32), u64>, // prevents per-frame reload of same bytes
    pub mesh_source_formats: HashMap<(String, u32), MeshSourceFormat>,
    pub mesh_failure_details: HashMap<(String, u32), String>,
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

    pub fn get_mesh_with_status(&mut self, uuid: &str, lod: u32, data: &[u8]) -> MeshCacheLookup {
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
            return MeshCacheLookup {
                mesh: Arc::clone(cached),
                state: MeshCacheLookupState::CachedReady,
                format: self
                    .mesh_source_formats
                    .get(&key)
                    .copied()
                    .unwrap_or(MeshSourceFormat::Unknown),
                failure: None,
                detail: None,
            };
        }

        if data.is_empty() {
            self.mesh_attempted_hash.insert(key, 0);
            return MeshCacheLookup {
                mesh: Arc::clone(cached),
                state: MeshCacheLookupState::EmptyData,
                format: MeshSourceFormat::Unknown,
                failure: None,
                detail: None,
            };
        }

        let new_hash = hash_bytes(data);
        let detected_format = detect_mesh_source_format(data);
        if self.mesh_attempted_hash.get(&key).copied() == Some(new_hash) {
            return MeshCacheLookup {
                mesh: Arc::clone(cached),
                state: MeshCacheLookupState::DecodeFailed,
                format: self
                    .mesh_source_formats
                    .get(&key)
                    .copied()
                    .unwrap_or(detected_format),
                failure: Some(if detected_format == MeshSourceFormat::Unknown {
                    AssetFetchFailureReason::Unsupported
                } else {
                    AssetFetchFailureReason::Decode
                }),
                detail: self.mesh_failure_details.get(&key).cloned(),
            };
        }

        match load_mesh_bytes(data, lod) {
            Ok(mesh) => {
                self.mesh_source_formats
                    .insert(key.clone(), detected_format);
                self.mesh_failure_details.remove(&key);
                let mesh_arc = Arc::new(mesh);
                self.meshes.insert(key.clone(), Arc::clone(&mesh_arc));
                self.mesh_attempted_hash.remove(&key);
                MeshCacheLookup {
                    mesh: mesh_arc,
                    state: MeshCacheLookupState::Decoded,
                    format: detected_format,
                    failure: None,
                    detail: None,
                }
            }
            Err(err) => {
                self.mesh_attempted_hash.insert(key, new_hash);
                let failure = if detected_format == MeshSourceFormat::Unknown {
                    AssetFetchFailureReason::Unsupported
                } else {
                    AssetFetchFailureReason::Decode
                };
                self.mesh_source_formats
                    .insert((uuid.to_string(), lod), detected_format);
                self.mesh_failure_details
                    .insert((uuid.to_string(), lod), err.to_string());
                MeshCacheLookup {
                    mesh: Arc::clone(cached),
                    state: MeshCacheLookupState::DecodeFailed,
                    format: detected_format,
                    failure: Some(failure),
                    detail: Some(err.to_string()),
                }
            }
        }
    }

    pub fn get_mesh(&mut self, uuid: &str, lod: u32, data: &[u8]) -> Arc<ProcessedMesh> {
        self.get_mesh_with_status(uuid, lod, data).mesh
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_mesh_caches_missing_entry_on_empty_data() {
        let mut cache = GeometryCache::new();
        let a = cache.get_mesh_with_status("dummy", 0, &[]);
        let b = cache.get_mesh_with_status("dummy", 0, &[]);
        assert_eq!(a.state, MeshCacheLookupState::EmptyData);
        assert_eq!(b.state, MeshCacheLookupState::EmptyData);
        assert!(Arc::ptr_eq(&a.mesh, &b.mesh));
        assert!(a.mesh.vertices.is_empty());
        assert!(a.mesh.submeshes.is_empty());
    }

    #[test]
    fn get_mesh_does_not_reparse_same_invalid_bytes_every_call() {
        let mut cache = GeometryCache::new();
        let a = cache.get_mesh_with_status("dummy", 0, b"not-a-gltf");
        let b = cache.get_mesh_with_status("dummy", 0, b"not-a-gltf");
        assert_eq!(a.state, MeshCacheLookupState::DecodeFailed);
        assert_eq!(b.state, MeshCacheLookupState::DecodeFailed);
        assert_eq!(a.failure, Some(AssetFetchFailureReason::Unsupported));
        assert!(Arc::ptr_eq(&a.mesh, &b.mesh));
        assert!(a.mesh.vertices.is_empty());
        assert!(a.mesh.submeshes.is_empty());
    }

    #[test]
    fn get_mesh_decodes_second_life_mesh_bytes() {
        let bytes = crate::debug_triangle_second_life_mesh_bytes();
        let mut cache = GeometryCache::new();
        let lookup = cache.get_mesh_with_status("mesh", 0, &bytes);
        assert_eq!(lookup.state, MeshCacheLookupState::Decoded);
        assert_eq!(lookup.format, MeshSourceFormat::SecondLifeMesh);
        assert!(lookup.mesh.vertices.len() >= 3);
        assert!(!lookup.mesh.submeshes.is_empty());
    }

    #[test]
    fn debug_second_life_mesh_fixture_is_detectable() {
        let bytes = crate::debug_triangle_second_life_mesh_bytes();
        assert!(!bytes.is_empty());
        assert_eq!(
            detect_mesh_source_format(&bytes),
            MeshSourceFormat::SecondLifeMesh
        );
    }
}
