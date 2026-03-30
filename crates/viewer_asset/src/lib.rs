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

#[derive(Debug, thiserror::Error)]
pub enum TextureDecodeError {
    #[error("unsupported image format")]
    Unsupported,
}

pub fn decode_texture_rgba8(bytes: &[u8]) -> Result<DecodedRgbaImage, TextureDecodeError> {
    if let Ok(img) = image::load_from_memory(bytes) {
        let (width, height) = image::GenericImageView::dimensions(&img);
        let rgba = img.to_rgba8().into_raw();
        return Ok(DecodedRgbaImage {
            width,
            height,
            rgba,
        });
    }

    if let Ok(j2k) = jpeg2k::Image::from_bytes(bytes)
        && let Ok(decoded) = image::DynamicImage::try_from(&j2k)
    {
        let rgba = decoded.to_rgba8();
        return Ok(DecodedRgbaImage {
            width: rgba.width(),
            height: rgba.height(),
            rgba: rgba.into_raw(),
        });
    }

    let jp2 = justjp2::decode(bytes).map_err(|_| TextureDecodeError::Unsupported)?;
    if jp2.components.is_empty() || jp2.width == 0 || jp2.height == 0 {
        return Err(TextureDecodeError::Unsupported);
    }

    let width = jp2.width as usize;
    let height = jp2.height as usize;
    let mut rgba = vec![0u8; width * height * 4];
    for y in 0..height {
        for x in 0..width {
            let r = sample_jp2_component_u8(&jp2.components, 0, x, y, width, height);
            let g = sample_jp2_component_u8(&jp2.components, 1, x, y, width, height);
            let b = sample_jp2_component_u8(&jp2.components, 2, x, y, width, height);
            let alpha = sample_jp2_component_u8(&jp2.components, 3, x, y, width, height);
            let idx = (y * width + x) * 4;
            rgba[idx] = r;
            rgba[idx + 1] = g;
            rgba[idx + 2] = b;
            rgba[idx + 3] = if jp2.components.len() >= 4 {
                alpha
            } else {
                255
            };
        }
    }

    Ok(DecodedRgbaImage {
        width: jp2.width,
        height: jp2.height,
        rgba,
    })
}

fn sample_jp2_component_u8(
    components: &[justjp2::Component],
    component_idx: usize,
    x: usize,
    y: usize,
    out_width: usize,
    out_height: usize,
) -> u8 {
    let component = components
        .get(component_idx)
        .or_else(|| components.first())
        .expect("jp2 components non-empty");
    let comp_width = component.width.max(1) as usize;
    let comp_height = component.height.max(1) as usize;
    let sx = (x * comp_width) / out_width.max(1);
    let sy = (y * comp_height) / out_height.max(1);
    let idx = sy.saturating_mul(comp_width).saturating_add(sx);
    let sample = *component.data.get(idx).unwrap_or(&0);
    let precision = component.precision.clamp(1, 31);
    let max = ((1i64 << precision) - 1).max(1);
    let normalized = if component.signed {
        let bias = 1i64 << (precision - 1);
        (i64::from(sample) + bias).clamp(0, max)
    } else {
        i64::from(sample).clamp(0, max)
    };
    ((normalized * 255) / max) as u8
}

pub fn decode_png_rgba8(bytes: &[u8]) -> anyhow::Result<DecodedRgbaImage> {
    decode_texture_rgba8(bytes).map_err(|e| anyhow::anyhow!(e.to_string()))
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

    #[test]
    fn decode_texture_rgba8_decodes_png() {
        let image = image::RgbaImage::from_raw(1, 1, vec![1, 2, 3, 255]).expect("valid image");
        let mut bytes = Vec::new();
        image
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .expect("encode png");

        let decoded = decode_texture_rgba8(&bytes).expect("png should decode");
        assert_eq!(decoded.width, 1);
        assert_eq!(decoded.height, 1);
        assert_eq!(decoded.rgba, vec![1, 2, 3, 255]);
    }

    #[test]
    fn decode_texture_rgba8_decodes_jpeg2000() {
        let bytes = include_bytes!(
            "..\\..\\..\\reference\\firestorm\\indra\\newview\\skins\\starlight\\themes\\mono_teal\\textures\\default_profile_picture.j2c"
        );
        let decoded = decode_texture_rgba8(bytes).expect("jpeg2000 should decode");
        assert!(decoded.width > 0);
        assert!(decoded.height > 0);
        assert_eq!(
            decoded.rgba.len(),
            (decoded.width as usize) * (decoded.height as usize) * 4
        );
    }

    #[test]
    fn decode_texture_rgba8_rejects_unsupported_bytes() {
        let err = decode_texture_rgba8(b"not-an-image").expect_err("must reject invalid bytes");
        assert!(matches!(err, TextureDecodeError::Unsupported));
    }
}
