use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub position: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshKind {
    AxisMarker,
    GroundPlane,
    Cube,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceRole {
    SceneStatic,
    LivePlaceholder,
}

#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub position: [f32; 3],
    pub scale: [f32; 3],
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RenderableInstance {
    pub mesh: MeshKind,
    pub role: InstanceRole,
    pub transform: Transform,
    pub color: [f32; 3],
}

#[derive(Debug, Clone, Default)]
pub struct Scene {
    pub instances: Vec<RenderableInstance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveVisualSnapshot {
    pub source: String,
    pub logged_in: bool,
    pub first_sim_endpoint: Option<String>,
    pub first_sim_region_x: Option<u32>,
    pub first_sim_region_y: Option<u32>,
    pub handshake_agent_movement_complete: bool,
    pub traffic_summary_available: bool,
    pub post_boundary_observations: u32,
    pub region_transition_control_observations: u32,
    pub crossed_region: u32,
    pub confirm_enable_simulator: u32,
    pub likely_broader_traffic: u32,
    pub unknown: u32,
    pub observed_at_unix_ms: u64,
}

impl Scene {
    pub fn prototype() -> Self {
        Self {
            instances: vec![
                RenderableInstance {
                    mesh: MeshKind::AxisMarker,
                    role: InstanceRole::SceneStatic,
                    transform: Transform::default(),
                    color: [1.0, 1.0, 1.0],
                },
                RenderableInstance {
                    mesh: MeshKind::GroundPlane,
                    role: InstanceRole::SceneStatic,
                    transform: Transform {
                        position: [3.0, 0.0, 0.0],
                        scale: [8.0, 1.0, 8.0],
                    },
                    color: [0.22, 0.24, 0.28],
                },
                RenderableInstance {
                    mesh: MeshKind::Cube,
                    role: InstanceRole::SceneStatic,
                    transform: Transform {
                        position: [3.0, 0.5, 0.0],
                        scale: [1.0, 1.0, 1.0],
                    },
                    color: [0.85, 0.35, 0.25],
                },
            ],
        }
    }

    pub fn apply_live_visual_snapshot(&mut self, snapshot: Option<&LiveVisualSnapshot>) {
        let Some(cube) = self
            .instances
            .iter_mut()
            .find(|instance| {
                instance.mesh == MeshKind::Cube && instance.role == InstanceRole::SceneStatic
            })
        else {
            return;
        };

        cube.color = match snapshot {
            Some(state) if state.logged_in && state.handshake_agent_movement_complete => {
                [0.20, 0.82, 0.34]
            }
            Some(state) if state.logged_in => [0.94, 0.74, 0.20],
            _ => [0.85, 0.35, 0.25],
        };

        cube.transform.scale = match snapshot {
            Some(state) if state.logged_in && state.handshake_agent_movement_complete => {
                [1.25, 1.25, 1.25]
            }
            Some(state) if state.logged_in => [1.10, 1.10, 1.10],
            _ => [1.0, 1.0, 1.0],
        };

        let live_transform = live_placeholder_transform(snapshot);
        let live_color = live_placeholder_color(snapshot);
        let live_placeholder = self
            .instances
            .iter_mut()
            .find(|instance| instance.role == InstanceRole::LivePlaceholder);
        if let Some(instance) = live_placeholder {
            instance.transform = live_transform;
            instance.color = live_color;
        } else {
            self.instances.push(RenderableInstance {
                mesh: MeshKind::AxisMarker,
                role: InstanceRole::LivePlaceholder,
                transform: live_transform,
                color: live_color,
            });
        }
    }
}

fn live_placeholder_transform(snapshot: Option<&LiveVisualSnapshot>) -> Transform {
    match snapshot {
        Some(state) if state.logged_in => {
            let rx = state.first_sim_region_x.unwrap_or(0) % 256;
            let ry = state.first_sim_region_y.unwrap_or(0) % 256;
            let offset_x = ((rx as f32 / 255.0) - 0.5) * 4.0;
            let offset_z = ((ry as f32 / 255.0) - 0.5) * 4.0;
            let y = if state.handshake_agent_movement_complete {
                1.5
            } else {
                1.0
            };
            let scale = if state.handshake_agent_movement_complete {
                [0.65, 0.65, 0.65]
            } else {
                [0.45, 0.45, 0.45]
            };
            Transform {
                position: [3.0 + offset_x, y, offset_z],
                scale,
            }
        }
        _ => Transform {
            position: [3.0, 0.9, 0.0],
            scale: [0.35, 0.35, 0.35],
        },
    }
}

fn live_placeholder_color(snapshot: Option<&LiveVisualSnapshot>) -> [f32; 3] {
    match snapshot {
        Some(state) if state.logged_in && state.handshake_agent_movement_complete => {
            [0.12, 0.86, 0.98]
        }
        Some(state) if state.logged_in => [0.98, 0.83, 0.24],
        _ => [0.70, 0.32, 0.24],
    }
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
        let right_vec = [sin_yaw, 0.0, -cos_yaw];

        self.position[0] += forward_vec[0] * forward + right_vec[0] * right;
        self.position[1] += up;
        self.position[2] += forward_vec[2] * forward + right_vec[2] * right;
    }

    pub fn add_look_delta(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.yaw += delta_yaw;
        self.pitch = (self.pitch + delta_pitch).clamp(-1.553343, 1.553343);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_snapshot(logged_in: bool, amc: bool) -> LiveVisualSnapshot {
        LiveVisualSnapshot {
            source: String::from("test"),
            logged_in,
            first_sim_endpoint: None,
            first_sim_region_x: None,
            first_sim_region_y: None,
            handshake_agent_movement_complete: amc,
            traffic_summary_available: false,
            post_boundary_observations: 0,
            region_transition_control_observations: 0,
            crossed_region: 0,
            confirm_enable_simulator: 0,
            likely_broader_traffic: 0,
            unknown: 0,
            observed_at_unix_ms: 0,
        }
    }

    #[test]
    fn scene_applies_offline_live_visual_defaults() {
        let mut scene = Scene::prototype();
        scene.apply_live_visual_snapshot(None);
        let cube = scene
            .instances
            .iter()
            .find(|instance| {
                instance.mesh == MeshKind::Cube && instance.role == InstanceRole::SceneStatic
            })
            .expect("cube should exist");
        assert_eq!(cube.color, [0.85, 0.35, 0.25]);
        assert_eq!(cube.transform.scale, [1.0, 1.0, 1.0]);
        let live_anchor = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::LivePlaceholder)
            .expect("live placeholder should exist");
        assert_eq!(live_anchor.mesh, MeshKind::AxisMarker);
        assert_eq!(live_anchor.transform.position, [3.0, 0.9, 0.0]);
    }

    #[test]
    fn scene_applies_logged_in_pre_amc_live_visual_state() {
        let mut scene = Scene::prototype();
        let snapshot = sample_snapshot(true, false);
        scene.apply_live_visual_snapshot(Some(&snapshot));
        let cube = scene
            .instances
            .iter()
            .find(|instance| {
                instance.mesh == MeshKind::Cube && instance.role == InstanceRole::SceneStatic
            })
            .expect("cube should exist");
        assert_eq!(cube.color, [0.94, 0.74, 0.20]);
        assert_eq!(cube.transform.scale, [1.10, 1.10, 1.10]);
        let live_anchor = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::LivePlaceholder)
            .expect("live placeholder should exist");
        assert_eq!(live_anchor.transform.position[1], 1.0);
        assert_eq!(live_anchor.transform.scale, [0.45, 0.45, 0.45]);
    }

    #[test]
    fn scene_applies_amc_reached_live_visual_state() {
        let mut scene = Scene::prototype();
        let snapshot = sample_snapshot(true, true);
        scene.apply_live_visual_snapshot(Some(&snapshot));
        let cube = scene
            .instances
            .iter()
            .find(|instance| {
                instance.mesh == MeshKind::Cube && instance.role == InstanceRole::SceneStatic
            })
            .expect("cube should exist");
        assert_eq!(cube.color, [0.20, 0.82, 0.34]);
        assert_eq!(cube.transform.scale, [1.25, 1.25, 1.25]);
        let live_anchor = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::LivePlaceholder)
            .expect("live placeholder should exist");
        assert_eq!(live_anchor.transform.position[1], 1.5);
        assert_eq!(live_anchor.transform.scale, [0.65, 0.65, 0.65]);
    }

    #[test]
    fn scene_live_placeholder_position_reflects_region_coordinates() {
        let mut scene = Scene::prototype();
        let snapshot = LiveVisualSnapshot {
            source: String::from("test"),
            logged_in: true,
            first_sim_endpoint: Some(String::from("127.0.0.1:13009")),
            first_sim_region_x: Some(461824),
            first_sim_region_y: Some(307200),
            handshake_agent_movement_complete: true,
            traffic_summary_available: true,
            post_boundary_observations: 10,
            region_transition_control_observations: 0,
            crossed_region: 0,
            confirm_enable_simulator: 0,
            likely_broader_traffic: 7,
            unknown: 0,
            observed_at_unix_ms: 1,
        };
        scene.apply_live_visual_snapshot(Some(&snapshot));
        let live_anchor = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::LivePlaceholder)
            .expect("live placeholder should exist");
        assert_ne!(live_anchor.transform.position, [3.0, 0.9, 0.0]);
        assert_eq!(live_anchor.color, [0.12, 0.86, 0.98]);
    }
}
