use crate::{Vertex, SculptType, Aabb};

pub fn generate_sculpt_mesh(pixels: &[u8], width: u32, height: u32, sculpt_type: SculptType) -> (Vec<Vertex>, Vec<u32>, Aabb) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut min = [f32::MAX, f32::MAX, f32::MAX];
    let mut max = [f32::MIN, f32::MIN, f32::MIN];

    if pixels.is_empty() || width == 0 || height == 0 {
        return (vertices, indices, Aabb::new([0.0, 0.0, 0.0], [0.1, 0.1, 0.1]));
    }

    let channels = if pixels.len() as u32 >= width * height * 4 { 4 } else { 3 };
    
    // 1. Generate Vertices from XYZ map
    for y in 0..height {
        for x in 0..width {
            let p_idx = ((y * width + x) * channels) as usize;
            if p_idx + 2 >= pixels.len() { break; }
            
            let r = pixels[p_idx] as f32 / 255.0;
            let g = pixels[p_idx + 1] as f32 / 255.0;
            let b = pixels[p_idx + 2] as f32 / 255.0;
            
            // SL XYZ map: R->X, G->Y, B->Z (centered at 0.5,0.5,0.5)
            // We map to [-0.5, 0.5]
            let pos = [r - 0.5, g - 0.5, b - 0.5];
            
            for i in 0..3 {
                min[i] = min[i].min(pos[i]);
                max[i] = max[i].max(pos[i]);
            }

            vertices.push(Vertex {
                position: pos,
                normal: [0.0, 1.0, 0.0], // To be calculated
                tex_coord: [x as f32 / (width - 1).max(1) as f32, y as f32 / (height - 1).max(1) as f32],
            });
        }
    }

    if vertices.is_empty() {
        return (vertices, indices, Aabb::new([0.0, 0.0, 0.0], [0.1, 0.1, 0.1]));
    }

    // 2. Generate Grid Indices
    for y in 0..height - 1 {
        for x in 0..width - 1 {
            let curr = y * width + x;
            let next = (y + 1) * width + x;

            indices.push(curr);
            indices.push(curr + 1);
            indices.push(next);

            indices.push(next);
            indices.push(curr + 1);
            indices.push(next + 1);
        }
    }

    // 3. Topology Wrapping (Simplified: standard wrapping for Sphere/Cylinder)
    match sculpt_type {
        SculptType::Sphere | SculptType::Cylinder | SculptType::Torus => {
            // Horizontal wrap
            for y in 0..height - 1 {
                let curr = y * width + (width - 1);
                let next = (y + 1) * width + (width - 1);
                let wrap_curr = y * width;
                let wrap_next = (y + 1) * width;

                indices.push(curr);
                indices.push(wrap_curr);
                indices.push(next);

                indices.push(next);
                indices.push(wrap_curr);
                indices.push(wrap_next);
            }
        }
        _ => {}
    }

    if sculpt_type == SculptType::Torus {
        // Vertical wrap
        for x in 0..width - 1 {
            let curr = (height - 1) * width + x;
            let wrap_curr = x;
            let next = (height - 1) * width + (x + 1);
            let wrap_next = x + 1;

            indices.push(curr);
            indices.push(next);
            indices.push(wrap_curr);

            indices.push(wrap_curr);
            indices.push(next);
            indices.push(wrap_next);
        }
    }

    // 4. Calculate Normals
    for y in 0..height {
        for x in 0..width {
            let v_curr = vertices[(y * width + x) as usize].position;
            
            // Samples
            let x_next = (x + 1) % width;
            let y_next = (y + 1) % height;
            
            let v_x = vertices[(y * width + x_next) as usize].position;
            let v_y = vertices[(y_next * width + x) as usize].position;
            
            let tx = [v_x[0] - v_curr[0], v_x[1] - v_curr[1], v_x[2] - v_curr[2]];
            let ty = [v_y[0] - v_curr[0], v_y[1] - v_curr[1], v_y[2] - v_curr[2]];
            
            // Cross product
            let mut nx = tx[1] * ty[2] - tx[2] * ty[1];
            let mut ny = tx[2] * ty[0] - tx[0] * ty[2];
            let mut nz = tx[0] * ty[1] - tx[1] * ty[0];
            
            let len = (nx*nx + ny*ny + nz*nz).sqrt();
            if len > 0.0 {
                nx /= len; ny /= len; nz /= len;
            } else {
                ny = 1.0;
            }
            
            vertices[(y * width + x) as usize].normal = [nx, ny, nz];
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

    (vertices, indices, Aabb::new(center, size))
}
