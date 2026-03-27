// =============================================================================
// UV ANIMATION ENGINE: TextureAnim
// =============================================================================
// Implements SL-compatible 2D texture coordinate transformations.
// Logic:
// 1. Supports Translation (scroll), Rotation, and Scaling (smooth).
// 2. Supports Discrete Cell-based (flipbook) animation.
// 3. Implements LOOP, PING_PONG, and REVERSE playback modes.
// 4. Output: A 3x3 matrix representing the UV transform.
// =============================================================================

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TextureAnim {
    pub mode: u8,
    pub face: i8,
    pub size_x: u8,
    pub size_y: u8,
    pub start: f32,
    pub length: f32,
    pub rate: f32,
}

impl Default for TextureAnim {
    fn default() -> Self {
        Self {
            mode: 0,
            face: -1, // -1 means all faces
            size_x: 1,
            size_y: 1,
            start: 0.0,
            length: 0.0,
            rate: 0.0,
        }
    }
}

impl TextureAnim {
    /// Enforces safe limits on texture animation parameters.
    pub fn clamp_to_safe_limits(&mut self) {
        // Mode is a bitfield, keep as is for now but could be masked.
        // size_x/y should be at least 1 if used for tiling.
        if self.size_x == 0 {
            self.size_x = 1;
        }
        if self.size_y == 0 {
            self.size_y = 1;
        }

        // Rate should be bounded to avoid epilepsy or rendering issues.
        self.rate = self.rate.clamp(-100.0, 100.0);

        // Start and length should be finite.
        if !self.start.is_finite() {
            self.start = 0.0;
        }
        if !self.length.is_finite() {
            self.length = 0.0;
        }
    }
}

pub const ANIM_ON: u8 = 0x01;
pub const LOOP: u8 = 0x02;
pub const REVERSE: u8 = 0x04;
pub const PING_PONG: u8 = 0x08;
pub const SMOOTH: u8 = 0x10;
pub const ROTATE: u8 = 0x20;
pub const SCALE: u8 = 0x40;

use crate::{TextureEntry, mat3_mul, mat3_rotate, mat3_scale, mat3_translate};

/// Computes a 3x3 texture transformation matrix based on material params and animation state.
///
/// This matrix transforms standard (0,0)-(1,1) UV coordinates into the animated
/// coordinates consumed by the shader.
pub fn compute_texture_matrix(
    params: &TextureEntry,
    anim: &TextureAnim,
    time: f32,
) -> [[f32; 3]; 3] {
    let mut offset_s = params.offset_s;
    let mut offset_t = params.offset_t;
    let mut scale_s = params.scale_s;
    let mut scale_t = params.scale_t;
    let mut rotation = params.rotation;

    if (anim.mode & ANIM_ON) != 0 {
        let mut t = time * anim.rate;

        // Handle LOOP, PING_PONG logic
        if (anim.mode & PING_PONG) != 0 {
            let len = if anim.length > 0.0 { anim.length } else { 1.0 };
            let normalized_t = (t - anim.start) % (2.0 * len);
            let phase = if normalized_t < 0.0 {
                normalized_t + 2.0 * len
            } else {
                normalized_t
            };

            t = if phase < len {
                anim.start + phase
            } else {
                anim.start + (2.0 * len - phase)
            };
        } else if (anim.mode & LOOP) != 0 {
            let len = if anim.length > 0.0 { anim.length } else { 1.0 };
            let normalized_t = (t - anim.start) % len;
            t = if normalized_t < 0.0 {
                normalized_t + len
            } else {
                normalized_t
            };
            t += anim.start;
        } else {
            // One-shot
            let end = anim.start + anim.length;
            t = t.clamp(anim.start.min(end), anim.start.max(end));
        }

        if (anim.mode & SMOOTH) != 0 {
            if (anim.mode & ROTATE) != 0 {
                rotation += t;
            } else if (anim.mode & SCALE) != 0 {
                scale_s *= t;
                scale_t *= t;
            } else {
                // Smooth scroll
                offset_s += t;
            }
        } else {
            // Discrete steps (CELLS)
            let size_x = anim.size_x.max(1) as f32;
            let size_y = anim.size_y.max(1) as f32;
            let frames = size_x * size_y;
            let frame = t.floor() % frames;
            let frame = if frame < 0.0 { frame + frames } else { frame };

            let col = frame % size_x;
            let row = (frame / size_x).floor();

            scale_s = 1.0 / size_x;
            scale_t = 1.0 / size_y;

            offset_s = col * scale_s;
            // SL rows are top-to-bottom for flipbooks, but V-coord is bottom-to-top.
            // Row 0 is at offset_t = (size_y - 1) * scale_t
            offset_t = (size_y - 1.0 - row) * scale_t;
        }
    }

    // SL texture transform order (simplified):
    // 1. Center UVs at (0,0) (offset by 0.5)
    // 2. Rotate
    // 3. Un-center
    // 4. Scale (repeat/tiles)
    // 5. Offset

    let m_center = mat3_translate(-0.5, -0.5);
    let m_rotate = mat3_rotate(rotation);
    let m_uncenter = mat3_translate(0.5, 0.5);
    let m_scale = mat3_scale(scale_s, scale_t);
    let m_offset = mat3_translate(offset_s, offset_t);

    // Matrix concat (Column-major logic): M = Offset * Scale * Uncenter * Rotate * Center
    let res = mat3_mul(m_rotate, m_center);
    let res = mat3_mul(m_uncenter, res);
    let res = mat3_mul(m_scale, res);
    mat3_mul(m_offset, res)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mat3_transform_vec2;
    use std::f32::consts::PI;

    #[test]
    fn test_static_uv_transform() {
        let params = TextureEntry {
            offset_s: 0.1,
            offset_t: 0.2,
            scale_s: 2.0,
            scale_t: 3.0,
            ..TextureEntry::default()
        };
        let anim = TextureAnim::default();
        let m = compute_texture_matrix(&params, &anim, 0.0);

        let uv_in = [0.5, 0.5];
        let uv_out = mat3_transform_vec2(m, uv_in);

        // Scale 2,3 Offset 0.1, 0.2 around center 0.5, 0.5
        // U' = (0.5-0.5)*1.0 + 0.5 = 0.5; U'' = 0.5 * 2.0 + 0.1 = 1.1
        // V' = (0.5-0.5)*1.0 + 0.5 = 0.5; V'' = 0.5 * 3.0 + 0.2 = 1.7
        assert!((uv_out[0] - 1.1).abs() < 1e-5);
        assert!((uv_out[1] - 1.7).abs() < 1e-5);
    }

    #[test]
    fn test_rotation_around_center() {
        let params = TextureEntry {
            rotation: PI * 0.5, // 90 degrees
            ..TextureEntry::default()
        };
        let anim = TextureAnim::default();
        let m = compute_texture_matrix(&params, &anim, 0.0);

        // Center should stay center
        let uv_center = mat3_transform_vec2(m, [0.5, 0.5]);
        assert!((uv_center[0] - 0.5).abs() < 1e-5);
        assert!((uv_center[1] - 0.5).abs() < 1e-5);

        // Right middle (1.0, 0.5) rotates to top middle (0.5, 1.0)
        let uv_right = mat3_transform_vec2(m, [1.0, 0.5]);
        assert!((uv_right[0] - 0.5).abs() < 1e-5);
        assert!((uv_right[1] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_smooth_scroll_loop() {
        let params = TextureEntry::default();
        let anim = TextureAnim {
            mode: ANIM_ON | SMOOTH | LOOP,
            rate: 1.0,
            length: 1.0,
            ..TextureAnim::default()
        };

        // At t=0.5, offset should be 0.5
        let m = compute_texture_matrix(&params, &anim, 0.5);
        let uv_out = mat3_transform_vec2(m, [0.0, 0.0]);
        // M = T(0.5, 0) * S(1,1) * T(0.5, 0.5) * R(0) * T(-0.5, -0.5)
        // (0,0) -> (-0.5, -0.5) -> (-0.5, -0.5) -> (0, 0) -> (0, 0) -> (0.5, 0.0)
        assert!((uv_out[0] - 0.5).abs() < 1e-5);

        // At t=1.5, loop should wrap back to 0.5
        let m2 = compute_texture_matrix(&params, &anim, 1.5);
        let uv_out2 = mat3_transform_vec2(m2, [0.0, 0.0]);
        assert!((uv_out2[0] - 0.5).abs() < 1e-5);
    }

    #[test]
    fn test_cells_flipbook() {
        let params = TextureEntry::default();
        let anim = TextureAnim {
            mode: ANIM_ON | LOOP, // Not SMOOTH, so it's cells
            size_x: 2,
            size_y: 2,
            rate: 1.0,
            length: 4.0, // 4 frames to loop over
            ..TextureAnim::default()
        };

        // Frame 0 (t=0.1)
        let m0 = compute_texture_matrix(&params, &anim, 0.1);
        let uv_out0 = mat3_transform_vec2(m0, [0.0, 0.0]);
        // scale=0.5, offset=(0, 0.5) [row 0 is top, V counts up from bottom, so offset_t=0.5]
        // (0,0) -> (-0.5, -0.5) -> (-0.5, -0.5) -> (0, 0) -> (0, 0) -> (0, 0.5)
        assert!((uv_out0[0] - 0.0).abs() < 1e-5);
        assert!((uv_out0[1] - 0.5).abs() < 1e-5);

        // Frame 1 (t=1.1)
        let m1 = compute_texture_matrix(&params, &anim, 1.1);
        let uv_out1 = mat3_transform_vec2(m1, [0.0, 0.0]);
        println!("uv_out1: {:?}", uv_out1);
        // col=1, row=0, offset=(0.5, 0.5)
        assert!((uv_out1[0] - 0.5).abs() < 1e-5);
        assert!((uv_out1[1] - 0.5).abs() < 1e-5);
    }

    #[test]
    fn test_ping_pong() {
        let params = TextureEntry::default();
        let anim = TextureAnim {
            mode: ANIM_ON | SMOOTH | PING_PONG,
            rate: 1.0,
            length: 1.0,
            ..TextureAnim::default()
        };

        // t=0.5 -> 0.5
        let m1 = compute_texture_matrix(&params, &anim, 0.5);
        assert!((mat3_transform_vec2(m1, [0.0, 0.0])[0] - 0.5).abs() < 1e-5);

        // t=1.5 -> phase 1.5 is > 1.0, so 2.0 - 1.5 = 0.5
        let m2 = compute_texture_matrix(&params, &anim, 1.5);
        assert!((mat3_transform_vec2(m2, [0.0, 0.0])[0] - 0.5).abs() < 1e-5);

        // t=2.1 -> wrap to 0.1
        let m3 = compute_texture_matrix(&params, &anim, 2.1);
        assert!((mat3_transform_vec2(m3, [0.0, 0.0])[0] - 0.1).abs() < 1e-5);
    }
}
