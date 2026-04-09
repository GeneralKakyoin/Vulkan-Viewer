use crate::{
    MeshSourceFormat, ProcessedMesh,
    mesh_loader::load_gltf_mesh,
    sl_mesh_loader::{detect_mesh_source_format, load_second_life_mesh},
};

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

pub(crate) fn hash_bytes(data: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}

pub(crate) fn load_mesh_bytes(data: &[u8], lod: u32) -> anyhow::Result<ProcessedMesh> {
    match detect_mesh_source_format(data) {
        MeshSourceFormat::Gltf => load_gltf_mesh(data),
        MeshSourceFormat::SecondLifeMesh => load_second_life_mesh(data, lod),
        MeshSourceFormat::Unknown => anyhow::bail!("unsupported mesh byte format"),
    }
}
