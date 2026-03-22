use crate::{Vertex, VolumeParams, ProfileType, HoleType, PathType, Aabb};
use std::f32::consts::PI;

pub const FACE_PATH_BEGIN: u16 = 0x1;
pub const FACE_PATH_END: u16 = 0x2;
pub const FACE_INNER_SIDE: u16 = 0x4;
pub const FACE_PROFILE_BEGIN: u16 = 0x8;
pub const FACE_PROFILE_END: u16 = 0x10;
pub const FACE_OUTER_SIDE_0: u16 = 0x20;
pub const FACE_OUTER_SIDE_1: u16 = 0x40;
pub const FACE_OUTER_SIDE_2: u16 = 0x80;
pub const FACE_OUTER_SIDE_3: u16 = 0x100;

#[derive(Debug, Clone)]
pub struct SubMesh {
    pub face_id: u16,
    pub indices: Vec<u32>,
}

pub fn generate_volume_mesh(params: &VolumeParams, detail: f32) -> (Vec<Vertex>, Vec<SubMesh>, Aabb) {
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut submeshes: Vec<SubMesh> = Vec::new();
    let mut min = [f32::MAX, f32::MAX, f32::MAX];
    let mut max = [f32::MIN, f32::MIN, f32::MIN];

    // 1. Profile & Path Generation
    let (outer_profile, inner_profile) = generate_profiles(params, detail);
    let path = generate_path(params, detail);

    // 2. Extrusion & Mesh Building
    let profile_len = outer_profile.len();
    let path_len = path.len();

    // Side faces (Outer)
    let mut side_indices = Vec::new();
    for i in 0..path_len {
        let pt = &path[i];
        let t_path = i as f32 / (path_len - 1) as f32;

        for (j, p) in outer_profile.iter().enumerate() {
            let u = j as f32 / (profile_len - 1) as f32;
            
            let scaled_p = [p[0] * pt.scale[0], p[1] * pt.scale[1], 0.0];
            let rotated_p = quat_rotate(pt.rot, scaled_p);
            let world_p = [
                rotated_p[0] + pt.pos[0],
                rotated_p[1] + pt.pos[1],
                rotated_p[2] + pt.pos[2],
            ];

            let normal = normalize(rotated_p);

            let vertex_pos = world_p;
            for k in 0..3 {
                min[k] = min[k].min(vertex_pos[k]);
                max[k] = max[k].max(vertex_pos[k]);
            }

            vertices.push(Vertex {
                position: vertex_pos,
                normal,
                tex_coord: [u, t_path],
            });

            if i < path_len - 1 && j < profile_len - 1 {
                let curr_row = i * profile_len + j;
                let next_row = (i + 1) * profile_len + j;
                
                side_indices.push(curr_row as u32);
                side_indices.push((curr_row + 1) as u32);
                side_indices.push(next_row as u32);

                side_indices.push(next_row as u32);
                side_indices.push((curr_row + 1) as u32);
                side_indices.push((next_row + 1) as u32);
            }
        }
    }

    submeshes.push(SubMesh {
        face_id: FACE_OUTER_SIDE_0,
        indices: side_indices,
    });

    // 3. Begin Cap
    let begin_pt = &path[0];
    let mut begin_indices = Vec::new();
    let begin_offset = vertices.len();
    
    // Begin Cap Vertices (Need distinct normals for caps)
    for p in &outer_profile {
        let scaled_p = [p[0] * begin_pt.scale[0], p[1] * begin_pt.scale[1], 0.0];
        let rotated_p = quat_rotate(begin_pt.rot, scaled_p);
        let vertex_pos = [rotated_p[0] + begin_pt.pos[0], rotated_p[1] + begin_pt.pos[1], rotated_p[2] + begin_pt.pos[2]];
        for k in 0..3 {
            min[k] = min[k].min(vertex_pos[k]);
            max[k] = max[k].max(vertex_pos[k]);
        }
        vertices.push(Vertex {
            position: vertex_pos,
            normal: quat_rotate(begin_pt.rot, [0.0, 0.0, -1.0]), // Pointing "back" along path
            tex_coord: [p[0] + 0.5, p[1] + 0.5],
        });
    }

    if let Some(inner) = &inner_profile {
        let inner_offset = vertices.len();
        for p in inner {
            let scaled_p = [p[0] * begin_pt.scale[0], p[1] * begin_pt.scale[1], 0.0];
            let rotated_p = quat_rotate(begin_pt.rot, scaled_p);
            let vertex_pos = [rotated_p[0] + begin_pt.pos[0], rotated_p[1] + begin_pt.pos[1], rotated_p[2] + begin_pt.pos[2]];
            for k in 0..3 {
                min[k] = min[k].min(vertex_pos[k]);
                max[k] = max[k].max(vertex_pos[k]);
            }
            vertices.push(Vertex {
                position: vertex_pos,
                normal: quat_rotate(begin_pt.rot, [0.0, 0.0, -1.0]),
                tex_coord: [p[0] + 0.5, p[1] + 0.5],
            });
        }

        // Bridge outer to inner for cap
        for j in 0..profile_len - 1 {
            begin_indices.push((begin_offset + j) as u32);
            begin_indices.push((inner_offset + j) as u32);
            begin_indices.push((begin_offset + j + 1) as u32);

            begin_indices.push((begin_offset + j + 1) as u32);
            begin_indices.push((inner_offset + j) as u32);
            begin_indices.push((inner_offset + j + 1) as u32);
        }
    } else {
        // Triangle fan to center
        let center_idx = vertices.len() as u32;
        let vertex_pos = begin_pt.pos;
        for k in 0..3 {
            min[k] = min[k].min(vertex_pos[k]);
            max[k] = max[k].max(vertex_pos[k]);
        }
        vertices.push(Vertex {
            position: vertex_pos,
            normal: quat_rotate(begin_pt.rot, [0.0, 0.0, -1.0]),
            tex_coord: [0.5, 0.5],
        });
        for j in 0..profile_len - 1 {
            begin_indices.push(center_idx);
            begin_indices.push((begin_offset + j) as u32);
            begin_indices.push((begin_offset + j + 1) as u32);
        }
    }

    submeshes.push(SubMesh {
        face_id: FACE_PATH_BEGIN,
        indices: begin_indices,
    });

    // 4. End Cap
    let end_pt = &path[path_len - 1];
    let mut end_indices = Vec::new();
    let end_offset = vertices.len();
    
    for p in &outer_profile {
        let scaled_p = [p[0] * end_pt.scale[0], p[1] * end_pt.scale[1], 0.0];
        let rotated_p = quat_rotate(end_pt.rot, scaled_p);
        let vertex_pos = [rotated_p[0] + end_pt.pos[0], rotated_p[1] + end_pt.pos[1], rotated_p[2] + end_pt.pos[2]];
        for k in 0..3 {
            min[k] = min[k].min(vertex_pos[k]);
            max[k] = max[k].max(vertex_pos[k]);
        }
        vertices.push(Vertex {
            position: vertex_pos,
            normal: quat_rotate(end_pt.rot, [0.0, 0.0, 1.0]), // Pointing "forward" along path
            tex_coord: [p[0] + 0.5, p[1] + 0.5],
        });
    }

    if let Some(inner) = &inner_profile {
        let inner_offset_end = vertices.len();
        for p in inner {
            let scaled_p = [p[0] * end_pt.scale[0], p[1] * end_pt.scale[1], 0.0];
            let rotated_p = quat_rotate(end_pt.rot, scaled_p);
            let vertex_pos = [rotated_p[0] + end_pt.pos[0], rotated_p[1] + end_pt.pos[1], rotated_p[2] + end_pt.pos[2]];
            for k in 0..3 {
                min[k] = min[k].min(vertex_pos[k]);
                max[k] = max[k].max(vertex_pos[k]);
            }
            vertices.push(Vertex {
                position: vertex_pos,
                normal: quat_rotate(end_pt.rot, [0.0, 0.0, 1.0]),
                tex_coord: [p[0] + 0.5, p[1] + 0.5],
            });
        }

        for j in 0..profile_len - 1 {
            // Flipped winding for end cap
            end_indices.push((end_offset + j) as u32);
            end_indices.push((end_offset + j + 1) as u32);
            end_indices.push((inner_offset_end + j) as u32);

            end_indices.push((end_offset + j + 1) as u32);
            end_indices.push((inner_offset_end + j + 1) as u32);
            end_indices.push((inner_offset_end + j) as u32);
        }
    } else {
        let center_idx = vertices.len() as u32;
        let vertex_pos = end_pt.pos;
        for k in 0..3 {
            min[k] = min[k].min(vertex_pos[k]);
            max[k] = max[k].max(vertex_pos[k]);
        }
        vertices.push(Vertex {
            position: vertex_pos,
            normal: quat_rotate(end_pt.rot, [0.0, 0.0, 1.0]),
            tex_coord: [0.5, 0.5],
        });
        for j in 0..profile_len - 1 {
            end_indices.push(center_idx);
            end_indices.push((end_offset + j + 1) as u32);
            end_indices.push((end_offset + j) as u32);
        }
    }

    submeshes.push(SubMesh {
        face_id: FACE_PATH_END,
        indices: end_indices,
    });

    // 5. Inner Side (if hollow)
    if let Some(inner) = &inner_profile {
        let mut inner_indices = Vec::new();
        let base_inner_offset = vertices.len();
        
        for i in 0..path_len {
            let pt = &path[i];
            let t_path = i as f32 / (path_len - 1) as f32;

            for (j, p) in inner.iter().enumerate() {
                let u = j as f32 / (profile_len - 1) as f32;
                let scaled_p = [p[0] * pt.scale[0], p[1] * pt.scale[1], 0.0];
                let rotated_p = quat_rotate(pt.rot, scaled_p);
                let vertex_pos = [rotated_p[0] + pt.pos[0], rotated_p[1] + pt.pos[1], rotated_p[2] + pt.pos[2]];
                for k in 0..3 {
                    min[k] = min[k].min(vertex_pos[k]);
                    max[k] = max[k].max(vertex_pos[k]);
                }
                vertices.push(Vertex {
                    position: vertex_pos,
                    normal: quat_rotate(pt.rot, [-p[0], -p[1], 0.0]), // Inward facing normal 
                    tex_coord: [u, t_path],
                });

                if i < path_len - 1 && j < profile_len - 1 {
                    let curr_row = base_inner_offset + i * profile_len + j;
                    let next_row = base_inner_offset + (i + 1) * profile_len + j;
                    
                    // Flipped winding for inner faces
                    inner_indices.push(curr_row as u32);
                    inner_indices.push(next_row as u32);
                    inner_indices.push((curr_row + 1) as u32);

                    inner_indices.push(next_row as u32);
                    inner_indices.push((next_row + 1) as u32);
                    inner_indices.push((curr_row + 1) as u32);
                }
            }
        }
        submeshes.push(SubMesh {
            face_id: FACE_INNER_SIDE,
            indices: inner_indices,
        });
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

    (vertices, submeshes, Aabb::new(center, size))
}

// Helper for rotating a vector by a quaternion
fn quat_rotate(q: [f32; 4], v: [f32; 3]) -> [f32; 3] {
    let u = [q[0], q[1], q[2]];
    let s = q[3];
    
    // let _dot_uv = u[0]*v[0] + u[1]*v[1] + u[2]*v[2];
    let cross_uv = [
        u[1]*v[2] - u[2]*v[1],
        u[2]*v[0] - u[0]*v[2],
        u[0]*v[1] - u[1]*v[0]
    ];
    let cross_u_cross_uv = [
        u[1]*cross_uv[2] - u[2]*cross_uv[1],
        u[2]*cross_uv[0] - u[0]*cross_uv[2],
        u[0]*cross_uv[1] - u[1]*cross_uv[0]
    ];

    [
        v[0] + 2.0 * (s * cross_uv[0] + cross_u_cross_uv[0]),
        v[1] + 2.0 * (s * cross_uv[1] + cross_u_cross_uv[1]),
        v[2] + 2.0 * (s * cross_uv[2] + cross_u_cross_uv[2]),
    ]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len_sq = v[0]*v[0] + v[1]*v[1] + v[2]*v[2];
    if len_sq > 0.0 {
        let len = len_sq.sqrt();
        [v[0]/len, v[1]/len, v[2]/len]
    } else {
        [0.0, 0.0, 1.0]
    }
}

fn generate_profiles(params: &VolumeParams, detail: f32) -> (Vec<[f32; 2]>, Option<Vec<[f32; 2]>>) {
    let outer = match params.profile_type {
        ProfileType::Square => gen_ngon(4, -0.375, 1.0, params.begin_cut, params.end_cut),
        ProfileType::Circle => gen_ngon((24.0 * detail) as u32, 0.0, 1.0, params.begin_cut, params.end_cut),
        ProfileType::TriangleIso | ProfileType::TriangleEqual | ProfileType::TriangleRight => gen_ngon(3, 0.0, 1.0, params.begin_cut, params.end_cut),
        ProfileType::HalfCircle => gen_ngon((12.0 * detail) as u32, 0.5, 0.5, params.begin_cut, params.end_cut),
    };

    let inner = if params.hollow > 0.0 {
        Some(match params.hole_type {
            HoleType::Same => outer.iter().map(|p| [p[0] * params.hollow, p[1] * params.hollow]).collect(),
            HoleType::Square => gen_ngon(4, -0.375, params.hollow, params.begin_cut, params.end_cut),
            HoleType::Circle => gen_ngon((24.0 * detail) as u32, 0.0, params.hollow, params.begin_cut, params.end_cut),
            HoleType::Triangle => gen_ngon(3, 0.0, params.hollow, params.begin_cut, params.end_cut),
        })
    } else {
        None
    };

    (outer, inner)
}

fn gen_ngon(sides: u32, offset: f32, scale: f32, begin: f32, end: f32) -> Vec<[f32; 2]> {
    let mut points = Vec::new();
    let step = 1.0 / sides as f32;
    
    // SL logic for fractional facets
    let mut t = (begin * sides as f32).floor() * step;
    while t < end {
        let actual_t = t.max(begin).min(end);
        let ang = 2.0 * PI * (actual_t + offset);
        points.push([ang.cos() * scale, ang.sin() * scale]);
        
        if t + step > end && end > actual_t {
             let final_ang = 2.0 * PI * (end + offset);
             points.push([final_ang.cos() * scale, final_ang.sin() * scale]);
        }
        t += step;
    }
    
    points
}

fn generate_path(params: &VolumeParams, detail: f32) -> Vec<PathPt> {
    let mut path = Vec::new();
    let steps = match params.path_type {
        PathType::Line => 2,
        PathType::Circle => (24.0 * detail * params.revolutions.abs().max(1.0)) as u32,
    };

    for i in 0..steps {
        let t = i as f32 / (steps - 1) as f32;
        
        let mut pt = PathPt::default();
        
        // 1. Position along path
        match params.path_type {
            PathType::Line => {
                pt.pos = [0.0, 0.0, t - 0.5]; // Centered at 0, spans -0.5 to 0.5
            }
            PathType::Circle => {
                let ang = 2.0 * PI * t * params.revolutions;
                let radius = 0.5 + params.radius_offset;
                pt.pos = [ang.cos() * radius, ang.sin() * radius, 0.0];
                pt.rot = [0.0, 0.0, (ang + PI/2.0).sin(), (ang + PI/2.0).cos()]; // Look along circle
            }
        }

        // 2. Twist
        let twist = params.twist_begin + t * (params.twist_end - params.twist_begin);
        let twist_q = [0.0, 0.0, (twist * PI).sin(), (twist * PI).cos()];
        pt.rot = quat_mul(pt.rot, twist_q);

        // 3. Taper & Scale
        pt.scale = [
            1.0 - t * params.taper_x,
            1.0 - t * params.taper_y
        ];
        
        // 4. Shear
        pt.pos[0] += t * params.shear_x;
        pt.pos[1] += t * params.shear_y;

        path.push(pt);
    }
    path
}

#[derive(Debug, Clone, Default)]
struct PathPt {
    pub pos: [f32; 3],
    pub rot: [f32; 4],
    pub scale: [f32; 2],
}

// Minimal quat_mul for local use
fn quat_mul(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
        a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
        a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
        a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
    ]
}
