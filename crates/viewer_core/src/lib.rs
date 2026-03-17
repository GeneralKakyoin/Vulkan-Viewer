#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub position: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            yaw: 0.0,
            pitch: 0.0,
        }
    }
}

impl Camera {
    pub fn move_local(&mut self, forward: f32, right: f32, up: f32) {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let forward_vec = [cos_yaw, 0.0, sin_yaw];
        let right_vec = [-sin_yaw, 0.0, cos_yaw];

        self.position[0] += forward_vec[0] * forward + right_vec[0] * right;
        self.position[1] += up;
        self.position[2] += forward_vec[2] * forward + right_vec[2] * right;
    }

    pub fn add_look_delta(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.yaw += delta_yaw;
        self.pitch = (self.pitch + delta_pitch).clamp(-1.553343, 1.553343);
    }
}
