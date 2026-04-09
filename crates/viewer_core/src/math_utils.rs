use super::*;

pub fn perspective_rh_zo(fovy_radians: f32, aspect: f32, znear: f32, zfar: f32) -> [[f32; 4]; 4] {
    let f = 1.0 / (0.5 * fovy_radians).tan();
    [
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, zfar / (znear - zfar), -1.0],
        [0.0, 0.0, (zfar * znear) / (znear - zfar), 0.0],
    ]
}

pub fn look_to_rh(eye: [f32; 3], direction: [f32; 3], up: [f32; 3]) -> [[f32; 4]; 4] {
    let forward = normalize(direction);
    let side = normalize(cross(up, forward));
    let camera_up = cross(forward, side);

    [
        [side[0], camera_up[0], -forward[0], 0.0],
        [side[1], camera_up[1], -forward[1], 0.0],
        [side[2], camera_up[2], -forward[2], 0.0],
        [
            -dot(side, eye),
            -dot(camera_up, eye),
            dot(forward, eye),
            1.0,
        ],
    ]
}

pub fn mat4_mul(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0; 4]; 4];
    for c in 0..4 {
        for r in 0..4 {
            out[c][r] =
                a[0][r] * b[c][0] + a[1][r] * b[c][1] + a[2][r] * b[c][2] + a[3][r] * b[c][3];
        }
    }
    out
}

pub fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (dot(v, v)).sqrt();
    if len > 0.0 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        [0.0, 0.0, -1.0]
    }
}

pub fn mat4_mul_vec4(m: [[f32; 4]; 4], v: [f32; 4]) -> [f32; 4] {
    [
        m[0][0] * v[0] + m[1][0] * v[1] + m[2][0] * v[2] + m[3][0] * v[3],
        m[0][1] * v[0] + m[1][1] * v[1] + m[2][1] * v[2] + m[3][1] * v[3],
        m[0][2] * v[0] + m[1][2] * v[1] + m[2][2] * v[2] + m[3][2] * v[3],
        m[0][3] * v[0] + m[1][3] * v[1] + m[2][3] * v[2] + m[3][3] * v[3],
    ]
}

pub fn quat_mul(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
        a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
        a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
        a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
    ]
}

pub fn quat_to_mat4(q: [f32; 4]) -> [[f32; 4]; 4] {
    let q_len = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    let normalized = if q_len > 0.0 {
        [q[0] / q_len, q[1] / q_len, q[2] / q_len, q[3] / q_len]
    } else {
        [0.0, 0.0, 0.0, 1.0]
    };
    let x2 = normalized[0] + normalized[0];
    let y2 = normalized[1] + normalized[1];
    let z2 = normalized[2] + normalized[2];
    let xx = normalized[0] * x2;
    let xy = normalized[0] * y2;
    let xz = normalized[0] * z2;
    let yy = normalized[1] * y2;
    let yz = normalized[1] * z2;
    let zz = normalized[2] * z2;
    let wx = normalized[3] * x2;
    let wy = normalized[3] * y2;
    let wz = normalized[3] * z2;

    [
        [1.0 - (yy + zz), xy + wz, xz - wy, 0.0],
        [xy - wz, 1.0 - (xx + zz), yz + wx, 0.0],
        [xz + wy, yz - wx, 1.0 - (xx + yy), 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

pub fn transform_to_mat4(t: Transform) -> [[f32; 4]; 4] {
    let s = [
        [t.scale[0], 0.0, 0.0, 0.0],
        [0.0, t.scale[1], 0.0, 0.0],
        [0.0, 0.0, t.scale[2], 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let r = quat_to_mat4(t.rotation);
    let tr = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [t.position[0], t.position[1], t.position[2], 1.0],
    ];
    // T * R * S
    mat4_mul(tr, mat4_mul(r, s))
}

pub fn flatten_mat4(m: [[f32; 4]; 4]) -> [f32; 16] {
    [
        m[0][0], m[0][1], m[0][2], m[0][3], m[1][0], m[1][1], m[1][2], m[1][3], m[2][0], m[2][1],
        m[2][2], m[2][3], m[3][0], m[3][1], m[3][2], m[3][3],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_to_mat4_default_transform_is_identity() {
        let matrix = transform_to_mat4(Transform::default());
        assert_eq!(
            matrix,
            [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ]
        );
    }

    #[test]
    fn transform_to_mat4_preserves_identity_rotation_and_applies_scale_translation() {
        let transform = Transform {
            position: [5.0, 6.0, 7.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [2.0, 3.0, 4.0],
        };
        let matrix = transform_to_mat4(transform);
        assert_eq!(matrix[0][0], 2.0);
        assert_eq!(matrix[1][1], 3.0);
        assert_eq!(matrix[2][2], 4.0);
        assert_eq!(matrix[3][0], 5.0);
        assert_eq!(matrix[3][1], 6.0);
        assert_eq!(matrix[3][2], 7.0);
        assert_eq!(matrix[3][3], 1.0);
    }

    #[test]
    fn quat_to_mat4_normalizes_non_unit_identity_quaternion() {
        let matrix = quat_to_mat4([0.0, 0.0, 0.0, 2.0]);
        assert_eq!(
            matrix,
            [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ]
        );
    }
}
