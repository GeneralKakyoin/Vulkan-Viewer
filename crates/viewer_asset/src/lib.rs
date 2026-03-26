use std::collections::HashMap;
use std::sync::Arc;
use viewer_core::geometry::llvolume::{SubMesh, generate_volume_mesh};
use viewer_core::geometry::sculpt::generate_sculpt_mesh;
use viewer_core::{SculptType, Vertex, VolumeParams};

pub mod mesh_loader;
use mesh_loader::load_gltf_mesh;

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
        if let Some(mesh) = self.meshes.get(&key) {
            return Arc::clone(mesh);
        }

        // If not in cache, load it.
        // In M2, we expect the data to be provided or empty if not yet fetched.
        if let Ok(mesh) = load_gltf_mesh(data) {
            let mesh_arc = Arc::new(mesh);
            self.meshes.insert(key, Arc::clone(&mesh_arc));
            mesh_arc
        } else {
            // Fallback to a tiny cube if loading fails
            Arc::new(ProcessedMesh {
                vertices: vec![],
                submeshes: vec![],
                aabb: viewer_core::Aabb::new([0.0, 0.0, 0.0], [0.1, 0.1, 0.1]),
            })
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
