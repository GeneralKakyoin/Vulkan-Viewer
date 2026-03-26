use crate::ProcessedMesh;
use anyhow::{Context, Result};
use viewer_core::Vertex;
use viewer_core::geometry::llvolume::SubMesh;

pub fn load_gltf_mesh(data: &[u8]) -> Result<ProcessedMesh> {
    if data.is_empty() {
        return Ok(ProcessedMesh {
            vertices: vec![],
            submeshes: vec![],
            aabb: viewer_core::Aabb::new([0.0, 0.0, 0.0], [0.1, 0.1, 0.1]),
        });
    }

    let (document, buffers, _images): (
        gltf::Document,
        Vec<gltf::buffer::Data>,
        Vec<gltf::image::Data>,
    ) = gltf::import_slice(data).map_err(|e| anyhow::anyhow!("glTF import failed: {}", e))?;

    let mut vertices: Vec<Vertex> = Vec::new();
    let mut submeshes: Vec<SubMesh> = Vec::new();

    for mesh in document.meshes() {
        for (prim_idx, primitive) in mesh.primitives().enumerate() {
            let reader = primitive.reader(|buffer: gltf::Buffer| {
                let index = buffer.index();
                let data_ref: &gltf::buffer::Data = &buffers[index];
                Some(&data_ref.0[..])
            });

            let positions_iter = reader.read_positions().context("Mesh missing positions")?;
            let positions: Vec<[f32; 3]> = positions_iter.collect();

            let mut normals_vec: Vec<[f32; 3]> = Vec::new();
            if let Some(normals_read) = reader.read_normals() {
                for n in normals_read {
                    normals_vec.push(n);
                }
            }

            let mut tex_coords_vec: Vec<[f32; 2]> = Vec::new();
            if let Some(tex_read) = reader.read_tex_coords(0) {
                match tex_read {
                    gltf::mesh::util::ReadTexCoords::U8(iter) => {
                        for [u, v] in iter {
                            tex_coords_vec.push([u as f32 / 255.0, v as f32 / 255.0]);
                        }
                    }
                    gltf::mesh::util::ReadTexCoords::U16(iter) => {
                        for [u, v] in iter {
                            tex_coords_vec.push([u as f32 / 65535.0, v as f32 / 65535.0]);
                        }
                    }
                    gltf::mesh::util::ReadTexCoords::F32(iter) => {
                        for uv in iter {
                            tex_coords_vec.push(uv);
                        }
                    }
                }
            }

            let base_idx = vertices.len() as u32;

            for (i, pos) in positions.into_iter().enumerate() {
                let normal = normals_vec.get(i).copied().unwrap_or([0.0, 1.0, 0.0]);
                let uv = tex_coords_vec.get(i).copied().unwrap_or([0.0, 0.0]);

                vertices.push(Vertex {
                    position: pos,
                    normal,
                    tex_coord: uv,
                });
            }

            let mut indices: Vec<u32> = Vec::new();
            if let Some(indices_read) = reader.read_indices() {
                match indices_read {
                    gltf::mesh::util::ReadIndices::U8(iter) => {
                        for i in iter {
                            indices.push(i as u32);
                        }
                    }
                    gltf::mesh::util::ReadIndices::U16(iter) => {
                        for i in iter {
                            indices.push(i as u32);
                        }
                    }
                    gltf::mesh::util::ReadIndices::U32(iter) => {
                        for i in iter {
                            indices.push(i);
                        }
                    }
                }
            } else {
                for i in 0..(vertices.len() as u32 - base_idx) {
                    indices.push(i);
                }
            }

            let shifted_indices: Vec<u32> = indices.into_iter().map(|i| i + base_idx).collect();

            submeshes.push(SubMesh {
                face_id: prim_idx as u16,
                indices: shifted_indices,
            });
        }
    }

    // Calculate overall AABB
    let mut min = [f32::MAX, f32::MAX, f32::MAX];
    let mut max = [f32::MIN, f32::MIN, f32::MIN];

    if vertices.is_empty() {
        min = [-0.1, -0.1, -0.1];
        max = [0.1, 0.1, 0.1];
    } else {
        for v in &vertices {
            for i in 0..3 {
                min[i] = min[i].min(v.position[i]);
                max[i] = max[i].max(v.position[i]);
            }
        }
    }

    let center = [
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    ];
    let size = [
        (max[0] - min[0]) * 0.5,
        (max[1] - min[1]) * 0.5,
        (max[2] - min[2]) * 0.5,
    ];

    Ok(ProcessedMesh {
        vertices,
        submeshes,
        aabb: viewer_core::Aabb::new(center, size),
    })
}
