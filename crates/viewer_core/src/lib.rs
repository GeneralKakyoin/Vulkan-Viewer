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
    WorldRegionAnchor,
    WorldEntryBeacon,
    WorldSimTargetMarker,
    WorldTrafficBroaderPillar,
    WorldTrafficUnknownPillar,
    WorldTrafficRegionControlPillar,
    WorldIngestionProxy,
    WorldIngestionTrafficPayload,
    WorldIngestionDecodedEndpointPayload,
    WorldIngestionDecodedCoarseLocationPayload,
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
    #[serde(default)]
    pub decoded_coarse_updates: u32,
    #[serde(default)]
    pub decoded_coarse_location_count: Option<u8>,
    #[serde(default)]
    pub decoded_coarse_first_x: Option<u8>,
    #[serde(default)]
    pub decoded_coarse_first_y: Option<u8>,
    #[serde(default)]
    pub decoded_coarse_first_z: Option<u8>,
    pub observed_at_unix_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldEntryStage {
    Offline,
    Connected,
    EnteredFirstRegion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirstRegionPresence {
    pub stage: WorldEntryStage,
    pub has_sim_endpoint: bool,
    pub region_coords: Option<[u32; 2]>,
}

impl FirstRegionPresence {
    pub fn from_live_snapshot(snapshot: Option<&LiveVisualSnapshot>) -> Self {
        match snapshot {
            Some(state) => Self {
                stage: if state.logged_in && state.handshake_agent_movement_complete {
                    WorldEntryStage::EnteredFirstRegion
                } else if state.logged_in {
                    WorldEntryStage::Connected
                } else {
                    WorldEntryStage::Offline
                },
                has_sim_endpoint: state.first_sim_endpoint.is_some(),
                region_coords: match (state.first_sim_region_x, state.first_sim_region_y) {
                    (Some(x), Some(y)) => Some([x, y]),
                    _ => None,
                },
            },
            None => Self {
                stage: WorldEntryStage::Offline,
                has_sim_endpoint: false,
                region_coords: None,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldTrafficSummary {
    pub available: bool,
    pub likely_broader: u32,
    pub unknown: u32,
    pub region_control: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldDiagnosticSlice {
    pub presence: FirstRegionPresence,
    pub traffic: WorldTrafficSummary,
}

impl WorldDiagnosticSlice {
    pub fn from_live_snapshot(snapshot: Option<&LiveVisualSnapshot>) -> Self {
        let presence = FirstRegionPresence::from_live_snapshot(snapshot);
        let traffic = match snapshot {
            Some(state) => WorldTrafficSummary {
                available: state.traffic_summary_available,
                likely_broader: state.likely_broader_traffic,
                unknown: state.unknown,
                region_control: state.region_transition_control_observations,
            },
            None => WorldTrafficSummary {
                available: false,
                likely_broader: 0,
                unknown: 0,
                region_control: 0,
            },
        };
        Self { presence, traffic }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldObjectIngestionLane {
    FirstRegionPresenceProxy,
    TrafficSignalPayload,
    DecodedSimulatorEndpointPayload,
    DecodedCoarseLocationPayload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldObjectIngestionItem {
    pub lane: WorldObjectIngestionLane,
    pub stage: WorldEntryStage,
    pub region_coords: Option<[u32; 2]>,
    pub simulator_target_present: bool,
    pub traffic_broader_count: u32,
    pub traffic_unknown_count: u32,
    pub traffic_region_control_count: u32,
    pub decoded_endpoint_port: Option<u16>,
    pub decoded_endpoint_host_tail: Option<u8>,
    pub decoded_coarse_location_count: Option<u8>,
    pub decoded_coarse_first_xyz: Option<[u8; 3]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorldObjectIngestionSeam {
    pub items: Vec<WorldObjectIngestionItem>,
}

impl WorldObjectIngestionSeam {
    pub fn from_diagnostic_slice(slice: WorldDiagnosticSlice) -> Self {
        if slice.presence.stage == WorldEntryStage::Offline {
            return Self::default();
        }
        let mut items = vec![WorldObjectIngestionItem {
            lane: WorldObjectIngestionLane::FirstRegionPresenceProxy,
            stage: slice.presence.stage,
            region_coords: slice.presence.region_coords,
            simulator_target_present: slice.presence.has_sim_endpoint,
            traffic_broader_count: 0,
            traffic_unknown_count: 0,
            traffic_region_control_count: 0,
            decoded_endpoint_port: None,
            decoded_endpoint_host_tail: None,
            decoded_coarse_location_count: None,
            decoded_coarse_first_xyz: None,
        }];
        if slice.traffic.available {
            items.push(WorldObjectIngestionItem {
                lane: WorldObjectIngestionLane::TrafficSignalPayload,
                stage: slice.presence.stage,
                region_coords: slice.presence.region_coords,
                simulator_target_present: false,
                traffic_broader_count: slice.traffic.likely_broader,
                traffic_unknown_count: slice.traffic.unknown,
                traffic_region_control_count: slice.traffic.region_control,
                decoded_endpoint_port: None,
                decoded_endpoint_host_tail: None,
                decoded_coarse_location_count: None,
                decoded_coarse_first_xyz: None,
            });
        }
        Self {
            items,
        }
    }

    pub fn from_live_snapshot(snapshot: Option<&LiveVisualSnapshot>) -> Self {
        let mut seam = Self::from_diagnostic_slice(WorldDiagnosticSlice::from_live_snapshot(snapshot));
        if let Some((endpoint_port, endpoint_host_tail)) =
            decode_simulator_endpoint(snapshot.and_then(|s| s.first_sim_endpoint.as_deref()))
        {
            let (stage, region_coords) = snapshot
                .map(|s| {
                    (
                        if s.logged_in && s.handshake_agent_movement_complete {
                            WorldEntryStage::EnteredFirstRegion
                        } else if s.logged_in {
                            WorldEntryStage::Connected
                        } else {
                            WorldEntryStage::Offline
                        },
                        match (s.first_sim_region_x, s.first_sim_region_y) {
                            (Some(x), Some(y)) => Some([x, y]),
                            _ => None,
                        },
                    )
                })
                .unwrap_or((WorldEntryStage::Offline, None));
            seam.items.push(WorldObjectIngestionItem {
                lane: WorldObjectIngestionLane::DecodedSimulatorEndpointPayload,
                stage,
                region_coords,
                simulator_target_present: true,
                traffic_broader_count: 0,
                traffic_unknown_count: 0,
                traffic_region_control_count: 0,
                decoded_endpoint_port: Some(endpoint_port),
                decoded_endpoint_host_tail: Some(endpoint_host_tail),
                decoded_coarse_location_count: None,
                decoded_coarse_first_xyz: None,
            });
        }
        if let Some(location_count) = snapshot.and_then(|s| s.decoded_coarse_location_count) {
            let first_xyz = snapshot.and_then(|s| {
                Some([
                    s.decoded_coarse_first_x?,
                    s.decoded_coarse_first_y?,
                    s.decoded_coarse_first_z?,
                ])
            });
            let (stage, region_coords) = snapshot
                .map(|s| {
                    (
                        if s.logged_in && s.handshake_agent_movement_complete {
                            WorldEntryStage::EnteredFirstRegion
                        } else if s.logged_in {
                            WorldEntryStage::Connected
                        } else {
                            WorldEntryStage::Offline
                        },
                        match (s.first_sim_region_x, s.first_sim_region_y) {
                            (Some(x), Some(y)) => Some([x, y]),
                            _ => None,
                        },
                    )
                })
                .unwrap_or((WorldEntryStage::Offline, None));
            seam.items.push(WorldObjectIngestionItem {
                lane: WorldObjectIngestionLane::DecodedCoarseLocationPayload,
                stage,
                region_coords,
                simulator_target_present: false,
                traffic_broader_count: 0,
                traffic_unknown_count: 0,
                traffic_region_control_count: 0,
                decoded_endpoint_port: None,
                decoded_endpoint_host_tail: None,
                decoded_coarse_location_count: Some(location_count),
                decoded_coarse_first_xyz: first_xyz,
            });
        }
        seam
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct WorldObjectIngestionAdapter;

impl WorldObjectIngestionAdapter {
    pub fn adapt(snapshot: Option<&LiveVisualSnapshot>) -> WorldObjectIngestionSeam {
        WorldObjectIngestionSeam::from_live_snapshot(snapshot)
    }
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
        let world_presence = FirstRegionPresence::from_live_snapshot(snapshot);
        let world_slice = WorldDiagnosticSlice::from_live_snapshot(snapshot);
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
        upsert_instance(
            &mut self.instances,
            InstanceRole::LivePlaceholder,
            MeshKind::AxisMarker,
            live_transform,
            live_color,
        );

        upsert_instance(
            &mut self.instances,
            InstanceRole::WorldRegionAnchor,
            MeshKind::Cube,
            world_region_anchor_transform(world_presence),
            world_region_anchor_color(world_presence),
        );

        upsert_instance(
            &mut self.instances,
            InstanceRole::WorldEntryBeacon,
            MeshKind::AxisMarker,
            world_entry_beacon_transform(world_presence),
            world_entry_beacon_color(world_presence),
        );

        upsert_instance(
            &mut self.instances,
            InstanceRole::WorldSimTargetMarker,
            MeshKind::AxisMarker,
            world_sim_target_transform(snapshot, world_presence),
            world_sim_target_color(world_presence),
        );

        upsert_instance(
            &mut self.instances,
            InstanceRole::WorldTrafficBroaderPillar,
            MeshKind::Cube,
            world_traffic_pillar_transform(world_slice, TrafficPillarKind::Broader),
            world_traffic_pillar_color(TrafficPillarKind::Broader),
        );
        upsert_instance(
            &mut self.instances,
            InstanceRole::WorldTrafficUnknownPillar,
            MeshKind::Cube,
            world_traffic_pillar_transform(world_slice, TrafficPillarKind::Unknown),
            world_traffic_pillar_color(TrafficPillarKind::Unknown),
        );
        upsert_instance(
            &mut self.instances,
            InstanceRole::WorldTrafficRegionControlPillar,
            MeshKind::Cube,
            world_traffic_pillar_transform(world_slice, TrafficPillarKind::RegionControl),
            world_traffic_pillar_color(TrafficPillarKind::RegionControl),
        );
    }

    pub fn apply_world_object_ingestion_seam(&mut self, seam: &WorldObjectIngestionSeam) {
        if let Some(item) = seam
            .items
            .iter()
            .copied()
            .find(|item| item.lane == WorldObjectIngestionLane::FirstRegionPresenceProxy)
        {
            upsert_instance(
                &mut self.instances,
                InstanceRole::WorldIngestionProxy,
                MeshKind::Cube,
                world_ingestion_proxy_transform(item),
                world_ingestion_proxy_color(item),
            );
        } else {
            remove_instance(&mut self.instances, InstanceRole::WorldIngestionProxy);
        }

        if let Some(item) = seam
            .items
            .iter()
            .copied()
            .find(|item| item.lane == WorldObjectIngestionLane::TrafficSignalPayload)
        {
            upsert_instance(
                &mut self.instances,
                InstanceRole::WorldIngestionTrafficPayload,
                MeshKind::AxisMarker,
                world_ingestion_traffic_payload_transform(item),
                world_ingestion_traffic_payload_color(item),
            );
        } else {
            remove_instance(&mut self.instances, InstanceRole::WorldIngestionTrafficPayload);
        }

        if let Some(item) = seam
            .items
            .iter()
            .copied()
            .find(|item| item.lane == WorldObjectIngestionLane::DecodedSimulatorEndpointPayload)
        {
            upsert_instance(
                &mut self.instances,
                InstanceRole::WorldIngestionDecodedEndpointPayload,
                MeshKind::AxisMarker,
                world_ingestion_decoded_endpoint_transform(item),
                world_ingestion_decoded_endpoint_color(item),
            );
        } else {
            remove_instance(
                &mut self.instances,
                InstanceRole::WorldIngestionDecodedEndpointPayload,
            );
        }

        if let Some(item) = seam
            .items
            .iter()
            .copied()
            .find(|item| item.lane == WorldObjectIngestionLane::DecodedCoarseLocationPayload)
        {
            upsert_instance(
                &mut self.instances,
                InstanceRole::WorldIngestionDecodedCoarseLocationPayload,
                MeshKind::AxisMarker,
                world_ingestion_decoded_coarse_location_transform(item),
                world_ingestion_decoded_coarse_location_color(item),
            );
        } else {
            remove_instance(
                &mut self.instances,
                InstanceRole::WorldIngestionDecodedCoarseLocationPayload,
            );
        }
    }
}

fn upsert_instance(
    instances: &mut Vec<RenderableInstance>,
    role: InstanceRole,
    mesh: MeshKind,
    transform: Transform,
    color: [f32; 3],
) {
    if let Some(instance) = instances.iter_mut().find(|instance| instance.role == role) {
        instance.transform = transform;
        instance.color = color;
    } else {
        instances.push(RenderableInstance {
            mesh,
            role,
            transform,
            color,
        });
    }
}

fn remove_instance(instances: &mut Vec<RenderableInstance>, role: InstanceRole) {
    if let Some(index) = instances.iter().position(|instance| instance.role == role) {
        instances.remove(index);
    }
}

fn world_presence_offset(region_coords: Option<[u32; 2]>) -> [f32; 2] {
    match region_coords {
        Some([x, y]) => {
            let rx = x % 256;
            let ry = y % 256;
            let offset_x = ((rx as f32 / 255.0) - 0.5) * 4.0;
            let offset_z = ((ry as f32 / 255.0) - 0.5) * 4.0;
            [offset_x, offset_z]
        }
        None => [0.0, 0.0],
    }
}

fn world_region_anchor_transform(presence: FirstRegionPresence) -> Transform {
    let [offset_x, offset_z] = world_presence_offset(presence.region_coords);
    Transform {
        position: [3.0 + offset_x, 0.25, offset_z],
        scale: [0.35, 0.35, 0.35],
    }
}

fn world_region_anchor_color(presence: FirstRegionPresence) -> [f32; 3] {
    match presence.stage {
        WorldEntryStage::EnteredFirstRegion => [0.26, 0.90, 0.64],
        WorldEntryStage::Connected => [0.98, 0.80, 0.32],
        WorldEntryStage::Offline => [0.45, 0.37, 0.33],
    }
}

fn world_entry_beacon_transform(presence: FirstRegionPresence) -> Transform {
    let [offset_x, offset_z] = world_presence_offset(presence.region_coords);
    let (y, scale) = match presence.stage {
        WorldEntryStage::EnteredFirstRegion => (2.0, [0.78, 0.78, 0.78]),
        WorldEntryStage::Connected => (1.4, [0.58, 0.58, 0.58]),
        WorldEntryStage::Offline => (1.0, [0.40, 0.40, 0.40]),
    };
    Transform {
        position: [3.0 + offset_x, y, offset_z],
        scale,
    }
}

fn world_entry_beacon_color(presence: FirstRegionPresence) -> [f32; 3] {
    match presence.stage {
        WorldEntryStage::EnteredFirstRegion => [0.12, 0.86, 0.98],
        WorldEntryStage::Connected => [0.98, 0.83, 0.24],
        WorldEntryStage::Offline => [0.62, 0.36, 0.30],
    }
}

fn world_sim_target_transform(
    snapshot: Option<&LiveVisualSnapshot>,
    presence: FirstRegionPresence,
) -> Transform {
    let [offset_x, offset_z] = world_presence_offset(presence.region_coords);
    let base_x = 3.0 + offset_x;
    let base_z = offset_z;
    let heading = endpoint_heading(snapshot.and_then(|state| state.first_sim_endpoint.as_deref()));
    let radius = if presence.has_sim_endpoint { 1.2 } else { 0.8 };
    let px = base_x + heading.cos() * radius;
    let pz = base_z + heading.sin() * radius;
    let y = match presence.stage {
        WorldEntryStage::EnteredFirstRegion => 1.2,
        WorldEntryStage::Connected => 1.0,
        WorldEntryStage::Offline => 0.85,
    };
    let scale = if presence.has_sim_endpoint {
        [0.33, 0.33, 0.33]
    } else {
        [0.24, 0.24, 0.24]
    };
    Transform {
        position: [px, y, pz],
        scale,
    }
}

fn world_sim_target_color(presence: FirstRegionPresence) -> [f32; 3] {
    if !presence.has_sim_endpoint {
        return [0.48, 0.44, 0.38];
    }
    match presence.stage {
        WorldEntryStage::EnteredFirstRegion => [0.32, 0.86, 0.98],
        WorldEntryStage::Connected => [0.98, 0.82, 0.30],
        WorldEntryStage::Offline => [0.58, 0.40, 0.34],
    }
}

fn endpoint_heading(endpoint: Option<&str>) -> f32 {
    let Some(text) = endpoint else {
        return 0.0;
    };
    let mut hash: u32 = 2_166_136_261;
    for byte in text.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16_777_619);
    }
    let normalized = (hash as f32) / (u32::MAX as f32);
    normalized * std::f32::consts::TAU
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrafficPillarKind {
    Broader,
    Unknown,
    RegionControl,
}

fn world_traffic_pillar_transform(slice: WorldDiagnosticSlice, kind: TrafficPillarKind) -> Transform {
    let [offset_x, offset_z] = world_presence_offset(slice.presence.region_coords);
    let base_x = 3.0 + offset_x;
    let base_z = offset_z;
    let (x_shift, z_shift) = match kind {
        TrafficPillarKind::Broader => (-0.9, 0.9),
        TrafficPillarKind::Unknown => (0.0, 1.0),
        TrafficPillarKind::RegionControl => (0.9, 0.9),
    };
    let raw_count = match kind {
        TrafficPillarKind::Broader => slice.traffic.likely_broader,
        TrafficPillarKind::Unknown => slice.traffic.unknown,
        TrafficPillarKind::RegionControl => slice.traffic.region_control,
    };
    let height = traffic_pillar_height(slice.traffic.available, raw_count);
    Transform {
        position: [base_x + x_shift, height * 0.5, base_z + z_shift],
        scale: [0.20, height, 0.20],
    }
}

fn traffic_pillar_height(available: bool, count: u32) -> f32 {
    if !available {
        return 0.18;
    }
    let clamped = count.min(20) as f32;
    0.18 + clamped * 0.04
}

fn world_traffic_pillar_color(kind: TrafficPillarKind) -> [f32; 3] {
    match kind {
        TrafficPillarKind::Broader => [0.30, 0.72, 0.94],
        TrafficPillarKind::Unknown => [0.94, 0.46, 0.30],
        TrafficPillarKind::RegionControl => [0.48, 0.88, 0.56],
    }
}

fn world_ingestion_proxy_transform(item: WorldObjectIngestionItem) -> Transform {
    let [offset_x, offset_z] = world_presence_offset(item.region_coords);
    let (y, scale) = match item.stage {
        WorldEntryStage::EnteredFirstRegion => (0.32, [0.18, 0.18, 0.18]),
        WorldEntryStage::Connected => (0.24, [0.14, 0.14, 0.14]),
        WorldEntryStage::Offline => (0.20, [0.10, 0.10, 0.10]),
    };
    Transform {
        position: [3.0 + offset_x, y, offset_z - 0.85],
        scale,
    }
}

fn world_ingestion_proxy_color(item: WorldObjectIngestionItem) -> [f32; 3] {
    if !item.simulator_target_present {
        return [0.42, 0.40, 0.36];
    }
    match item.stage {
        WorldEntryStage::EnteredFirstRegion => [0.20, 0.78, 0.96],
        WorldEntryStage::Connected => [0.96, 0.74, 0.24],
        WorldEntryStage::Offline => [0.42, 0.40, 0.36],
    }
}

fn world_ingestion_traffic_payload_transform(item: WorldObjectIngestionItem) -> Transform {
    let [offset_x, offset_z] = world_presence_offset(item.region_coords);
    let weighted = item
        .traffic_broader_count
        .saturating_add(item.traffic_unknown_count.saturating_mul(2))
        .saturating_add(item.traffic_region_control_count.saturating_mul(3))
        .min(30) as f32;
    let y = 0.42 + weighted * 0.015;
    let scale = 0.22 + weighted * 0.006;
    Transform {
        position: [3.0 + offset_x + 0.85, y, offset_z - 0.85],
        scale: [scale, scale, scale],
    }
}

fn world_ingestion_traffic_payload_color(item: WorldObjectIngestionItem) -> [f32; 3] {
    if item.traffic_region_control_count > 0 {
        [0.48, 0.88, 0.56]
    } else if item.traffic_unknown_count > 0 {
        [0.94, 0.46, 0.30]
    } else {
        [0.30, 0.72, 0.94]
    }
}

fn decode_simulator_endpoint(endpoint: Option<&str>) -> Option<(u16, u8)> {
    let endpoint = endpoint?;
    let (host, port_text) = endpoint.rsplit_once(':')?;
    let port = port_text.parse::<u16>().ok()?;
    let host_tail = host
        .split('.')
        .next_back()
        .and_then(|octet| octet.parse::<u8>().ok())?;
    Some((port, host_tail))
}

fn world_ingestion_decoded_endpoint_transform(item: WorldObjectIngestionItem) -> Transform {
    let [offset_x, offset_z] = world_presence_offset(item.region_coords);
    let host_tail = f32::from(item.decoded_endpoint_host_tail.unwrap_or(0));
    let port = f32::from(item.decoded_endpoint_port.unwrap_or(0));
    let x = 3.0 + offset_x - 0.85 + ((host_tail / 255.0) - 0.5) * 0.9;
    let z = offset_z - 1.2;
    let y = 0.48 + ((port % 1000.0) / 1000.0) * 0.6;
    let scale = 0.18 + ((host_tail % 32.0) / 32.0) * 0.14;
    Transform {
        position: [x, y, z],
        scale: [scale, scale, scale],
    }
}

fn world_ingestion_decoded_endpoint_color(item: WorldObjectIngestionItem) -> [f32; 3] {
    let port = item.decoded_endpoint_port.unwrap_or(0);
    if port >= 13000 {
        [0.30, 0.84, 0.96]
    } else if port >= 9000 {
        [0.34, 0.90, 0.62]
    } else {
        [0.92, 0.66, 0.30]
    }
}

fn world_ingestion_decoded_coarse_location_transform(item: WorldObjectIngestionItem) -> Transform {
    let [offset_x, offset_z] = world_presence_offset(item.region_coords);
    let [coarse_x, coarse_y, coarse_z] = item.decoded_coarse_first_xyz.unwrap_or([128, 128, 0]);
    let x = 3.0 + offset_x - 0.45 + ((f32::from(coarse_x) / 255.0) - 0.5) * 1.6;
    let z = offset_z - 1.85 + ((f32::from(coarse_y) / 255.0) - 0.5) * 1.6;
    let y = 0.28 + (f32::from(coarse_z) / 255.0) * 1.0;
    let count = f32::from(item.decoded_coarse_location_count.unwrap_or(0));
    let scale = (0.16 + count * 0.01).clamp(0.16, 0.42);
    Transform {
        position: [x, y, z],
        scale: [scale, scale, scale],
    }
}

fn world_ingestion_decoded_coarse_location_color(item: WorldObjectIngestionItem) -> [f32; 3] {
    let count = item.decoded_coarse_location_count.unwrap_or(0);
    if count >= 8 {
        [0.96, 0.38, 0.30]
    } else if count >= 3 {
        [0.98, 0.76, 0.28]
    } else {
        [0.38, 0.88, 0.96]
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

    fn apply_scene_from_snapshot(scene: &mut Scene, snapshot: Option<&LiveVisualSnapshot>) {
        scene.apply_live_visual_snapshot(snapshot);
        let seam = WorldObjectIngestionAdapter::adapt(snapshot);
        scene.apply_world_object_ingestion_seam(&seam);
    }

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
            decoded_coarse_updates: 0,
            decoded_coarse_location_count: None,
            decoded_coarse_first_x: None,
            decoded_coarse_first_y: None,
            decoded_coarse_first_z: None,
            observed_at_unix_ms: 0,
        }
    }

    #[test]
    fn scene_applies_offline_live_visual_defaults() {
        let mut scene = Scene::prototype();
        apply_scene_from_snapshot(&mut scene, None);
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
        let region_anchor = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldRegionAnchor)
            .expect("region anchor should exist");
        assert_eq!(region_anchor.mesh, MeshKind::Cube);
        assert_eq!(region_anchor.color, [0.45, 0.37, 0.33]);
        let entry_beacon = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldEntryBeacon)
            .expect("entry beacon should exist");
        assert_eq!(entry_beacon.mesh, MeshKind::AxisMarker);
        assert_eq!(entry_beacon.transform.position[1], 1.0);
        let target = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldSimTargetMarker)
            .expect("sim target marker should exist");
        assert_eq!(target.color, [0.48, 0.44, 0.38]);
        let broader = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldTrafficBroaderPillar)
            .expect("broader traffic pillar should exist");
        assert_eq!(broader.transform.scale, [0.20, 0.18, 0.20]);
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldIngestionProxy)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldIngestionTrafficPayload)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldIngestionDecodedEndpointPayload)
        );
    }

    #[test]
    fn scene_applies_logged_in_pre_amc_live_visual_state() {
        let mut scene = Scene::prototype();
        let snapshot = sample_snapshot(true, false);
        apply_scene_from_snapshot(&mut scene, Some(&snapshot));
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
        let region_anchor = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldRegionAnchor)
            .expect("region anchor should exist");
        assert_eq!(region_anchor.color, [0.98, 0.80, 0.32]);
        let entry_beacon = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldEntryBeacon)
            .expect("entry beacon should exist");
        assert_eq!(entry_beacon.color, [0.98, 0.83, 0.24]);
        assert_eq!(entry_beacon.transform.position[1], 1.4);
        let target = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldSimTargetMarker)
            .expect("sim target marker should exist");
        assert_eq!(target.color, [0.48, 0.44, 0.38]);
        let seam_proxy = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldIngestionProxy)
            .expect("ingestion seam proxy should exist");
        assert_eq!(seam_proxy.mesh, MeshKind::Cube);
        assert_eq!(seam_proxy.transform.position[1], 0.24);
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldIngestionTrafficPayload)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldIngestionDecodedEndpointPayload)
        );
    }

    #[test]
    fn scene_applies_amc_reached_live_visual_state() {
        let mut scene = Scene::prototype();
        let snapshot = sample_snapshot(true, true);
        apply_scene_from_snapshot(&mut scene, Some(&snapshot));
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
        let region_anchor = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldRegionAnchor)
            .expect("region anchor should exist");
        assert_eq!(region_anchor.color, [0.26, 0.90, 0.64]);
        let entry_beacon = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldEntryBeacon)
            .expect("entry beacon should exist");
        assert_eq!(entry_beacon.color, [0.12, 0.86, 0.98]);
        assert_eq!(entry_beacon.transform.position[1], 2.0);
        let target = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldSimTargetMarker)
            .expect("sim target marker should exist");
        assert_eq!(target.color, [0.48, 0.44, 0.38]);
        let seam_proxy = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldIngestionProxy)
            .expect("ingestion seam proxy should exist");
        assert_eq!(seam_proxy.transform.position[1], 0.32);
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldIngestionTrafficPayload)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldIngestionDecodedEndpointPayload)
        );
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
            decoded_coarse_updates: 0,
            decoded_coarse_location_count: None,
            decoded_coarse_first_x: None,
            decoded_coarse_first_y: None,
            decoded_coarse_first_z: None,
            observed_at_unix_ms: 1,
        };
        apply_scene_from_snapshot(&mut scene, Some(&snapshot));
        let live_anchor = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::LivePlaceholder)
            .expect("live placeholder should exist");
        assert_ne!(live_anchor.transform.position, [3.0, 0.9, 0.0]);
        assert_eq!(live_anchor.color, [0.12, 0.86, 0.98]);
        let region_anchor = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldRegionAnchor)
            .expect("region anchor should exist");
        assert_ne!(region_anchor.transform.position, [3.0, 0.25, 0.0]);
        let entry_beacon = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldEntryBeacon)
            .expect("entry beacon should exist");
        assert_ne!(entry_beacon.transform.position, [3.0, 2.0, 0.0]);
        let target = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldSimTargetMarker)
            .expect("sim target marker should exist");
        assert_ne!(target.transform.position, [3.0, 1.2, 0.0]);
        assert_eq!(target.color, [0.32, 0.86, 0.98]);
        let broader = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldTrafficBroaderPillar)
            .expect("broader traffic pillar should exist");
        let unknown = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldTrafficUnknownPillar)
            .expect("unknown traffic pillar should exist");
        let region_control = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldTrafficRegionControlPillar)
            .expect("region control traffic pillar should exist");
        assert!(broader.transform.scale[1] > unknown.transform.scale[1]);
        assert!(unknown.transform.scale[1] >= region_control.transform.scale[1]);
        let seam_proxy = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldIngestionProxy)
            .expect("ingestion seam proxy should exist");
        assert_ne!(seam_proxy.transform.position, [3.0, 0.32, -0.85]);
        assert_eq!(seam_proxy.color, [0.20, 0.78, 0.96]);
        let traffic_payload = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldIngestionTrafficPayload)
            .expect("ingestion traffic payload marker should exist");
        assert_eq!(traffic_payload.mesh, MeshKind::AxisMarker);
        assert!(traffic_payload.transform.scale[0] > 0.22);
        assert_eq!(traffic_payload.color, [0.30, 0.72, 0.94]);
        let decoded_endpoint_payload = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldIngestionDecodedEndpointPayload)
            .expect("decoded endpoint payload marker should exist");
        assert_eq!(decoded_endpoint_payload.mesh, MeshKind::AxisMarker);
        assert!(decoded_endpoint_payload.transform.position[1] > 0.48);
        assert_eq!(decoded_endpoint_payload.color, [0.30, 0.84, 0.96]);
    }

    #[test]
    fn first_region_presence_maps_snapshot_stages() {
        let offline = FirstRegionPresence::from_live_snapshot(None);
        assert_eq!(offline.stage, WorldEntryStage::Offline);
        assert!(!offline.has_sim_endpoint);
        assert_eq!(offline.region_coords, None);

        let connected = FirstRegionPresence::from_live_snapshot(Some(&sample_snapshot(true, false)));
        assert_eq!(connected.stage, WorldEntryStage::Connected);
        assert!(!connected.has_sim_endpoint);

        let entered = FirstRegionPresence::from_live_snapshot(Some(&sample_snapshot(true, true)));
        assert_eq!(entered.stage, WorldEntryStage::EnteredFirstRegion);
    }

    #[test]
    fn first_region_presence_maps_endpoint_and_region_coords() {
        let snapshot = LiveVisualSnapshot {
            source: String::from("test"),
            logged_in: true,
            first_sim_endpoint: Some(String::from("198.51.100.10:13009")),
            first_sim_region_x: Some(1000),
            first_sim_region_y: Some(2000),
            handshake_agent_movement_complete: false,
            traffic_summary_available: false,
            post_boundary_observations: 0,
            region_transition_control_observations: 0,
            crossed_region: 0,
            confirm_enable_simulator: 0,
            likely_broader_traffic: 0,
            unknown: 0,
            decoded_coarse_updates: 0,
            decoded_coarse_location_count: None,
            decoded_coarse_first_x: None,
            decoded_coarse_first_y: None,
            decoded_coarse_first_z: None,
            observed_at_unix_ms: 0,
        };
        let presence = FirstRegionPresence::from_live_snapshot(Some(&snapshot));
        assert_eq!(presence.stage, WorldEntryStage::Connected);
        assert!(presence.has_sim_endpoint);
        assert_eq!(presence.region_coords, Some([1000, 2000]));
    }

    #[test]
    fn world_diagnostic_slice_maps_traffic_summary() {
        let snapshot = LiveVisualSnapshot {
            source: String::from("test"),
            logged_in: true,
            first_sim_endpoint: Some(String::from("198.51.100.10:13009")),
            first_sim_region_x: Some(1000),
            first_sim_region_y: Some(2000),
            handshake_agent_movement_complete: true,
            traffic_summary_available: true,
            post_boundary_observations: 14,
            region_transition_control_observations: 2,
            crossed_region: 0,
            confirm_enable_simulator: 0,
            likely_broader_traffic: 11,
            unknown: 3,
            decoded_coarse_updates: 0,
            decoded_coarse_location_count: None,
            decoded_coarse_first_x: None,
            decoded_coarse_first_y: None,
            decoded_coarse_first_z: None,
            observed_at_unix_ms: 7,
        };
        let slice = WorldDiagnosticSlice::from_live_snapshot(Some(&snapshot));
        assert_eq!(slice.presence.stage, WorldEntryStage::EnteredFirstRegion);
        assert!(slice.presence.has_sim_endpoint);
        assert!(slice.traffic.available);
        assert_eq!(slice.traffic.likely_broader, 11);
        assert_eq!(slice.traffic.unknown, 3);
        assert_eq!(slice.traffic.region_control, 2);
    }

    #[test]
    fn traffic_pillar_height_respects_summary_availability_and_caps() {
        assert_eq!(traffic_pillar_height(false, 10), 0.18);
        assert_eq!(traffic_pillar_height(true, 0), 0.18);
        assert_eq!(traffic_pillar_height(true, 5), 0.38);
        let capped = traffic_pillar_height(true, 200);
        assert!((capped - 0.98).abs() < 0.0001);
    }

    #[test]
    fn world_object_ingestion_seam_maps_from_live_snapshot() {
        let offline = WorldObjectIngestionSeam::from_live_snapshot(None);
        assert!(offline.items.is_empty());

        let connected = WorldObjectIngestionSeam::from_live_snapshot(Some(&sample_snapshot(true, false)));
        assert_eq!(connected.items.len(), 1);
        let first = connected.items[0];
        assert_eq!(first.lane, WorldObjectIngestionLane::FirstRegionPresenceProxy);
        assert_eq!(first.stage, WorldEntryStage::Connected);
        assert!(!first.simulator_target_present);
    }

    #[test]
    fn world_object_ingestion_seam_includes_traffic_signal_payload_when_available() {
        let snapshot = LiveVisualSnapshot {
            source: String::from("test"),
            logged_in: true,
            first_sim_endpoint: Some(String::from("198.51.100.10:13009")),
            first_sim_region_x: Some(1024),
            first_sim_region_y: Some(2048),
            handshake_agent_movement_complete: true,
            traffic_summary_available: true,
            post_boundary_observations: 14,
            region_transition_control_observations: 2,
            crossed_region: 0,
            confirm_enable_simulator: 0,
            likely_broader_traffic: 11,
            unknown: 3,
            decoded_coarse_updates: 0,
            decoded_coarse_location_count: None,
            decoded_coarse_first_x: None,
            decoded_coarse_first_y: None,
            decoded_coarse_first_z: None,
            observed_at_unix_ms: 9,
        };
        let seam = WorldObjectIngestionSeam::from_live_snapshot(Some(&snapshot));
        assert!(seam
            .items
            .iter()
            .any(|item| item.lane == WorldObjectIngestionLane::FirstRegionPresenceProxy));
        let traffic = seam
            .items
            .iter()
            .find(|item| item.lane == WorldObjectIngestionLane::TrafficSignalPayload)
            .expect("traffic payload should exist");
        assert_eq!(traffic.traffic_broader_count, 11);
        assert_eq!(traffic.traffic_unknown_count, 3);
        assert_eq!(traffic.traffic_region_control_count, 2);
    }

    #[test]
    fn world_object_ingestion_seam_includes_decoded_endpoint_payload_when_parseable() {
        let snapshot = LiveVisualSnapshot {
            source: String::from("test"),
            logged_in: true,
            first_sim_endpoint: Some(String::from("198.51.100.42:13009")),
            first_sim_region_x: Some(1024),
            first_sim_region_y: Some(2048),
            handshake_agent_movement_complete: true,
            traffic_summary_available: false,
            post_boundary_observations: 0,
            region_transition_control_observations: 0,
            crossed_region: 0,
            confirm_enable_simulator: 0,
            likely_broader_traffic: 0,
            unknown: 0,
            decoded_coarse_updates: 0,
            decoded_coarse_location_count: None,
            decoded_coarse_first_x: None,
            decoded_coarse_first_y: None,
            decoded_coarse_first_z: None,
            observed_at_unix_ms: 9,
        };
        let seam = WorldObjectIngestionSeam::from_live_snapshot(Some(&snapshot));
        let decoded = seam
            .items
            .iter()
            .find(|item| item.lane == WorldObjectIngestionLane::DecodedSimulatorEndpointPayload)
            .expect("decoded endpoint payload should exist");
        assert_eq!(decoded.decoded_endpoint_host_tail, Some(42));
        assert_eq!(decoded.decoded_endpoint_port, Some(13009));
    }

    #[test]
    fn world_object_ingestion_seam_omits_decoded_endpoint_payload_when_unparseable() {
        let snapshot = LiveVisualSnapshot {
            source: String::from("test"),
            logged_in: true,
            first_sim_endpoint: Some(String::from("bad-endpoint")),
            first_sim_region_x: Some(1024),
            first_sim_region_y: Some(2048),
            handshake_agent_movement_complete: true,
            traffic_summary_available: false,
            post_boundary_observations: 0,
            region_transition_control_observations: 0,
            crossed_region: 0,
            confirm_enable_simulator: 0,
            likely_broader_traffic: 0,
            unknown: 0,
            decoded_coarse_updates: 0,
            decoded_coarse_location_count: None,
            decoded_coarse_first_x: None,
            decoded_coarse_first_y: None,
            decoded_coarse_first_z: None,
            observed_at_unix_ms: 9,
        };
        let seam = WorldObjectIngestionSeam::from_live_snapshot(Some(&snapshot));
        assert!(seam
            .items
            .iter()
            .all(|item| item.lane != WorldObjectIngestionLane::DecodedSimulatorEndpointPayload));
    }

    #[test]
    fn world_object_ingestion_seam_includes_decoded_coarse_payload_when_present() {
        let snapshot = LiveVisualSnapshot {
            source: String::from("test"),
            logged_in: true,
            first_sim_endpoint: Some(String::from("198.51.100.42:13009")),
            first_sim_region_x: Some(1024),
            first_sim_region_y: Some(2048),
            handshake_agent_movement_complete: true,
            traffic_summary_available: true,
            post_boundary_observations: 4,
            region_transition_control_observations: 0,
            crossed_region: 0,
            confirm_enable_simulator: 0,
            likely_broader_traffic: 3,
            unknown: 1,
            decoded_coarse_updates: 1,
            decoded_coarse_location_count: Some(2),
            decoded_coarse_first_x: Some(64),
            decoded_coarse_first_y: Some(32),
            decoded_coarse_first_z: Some(12),
            observed_at_unix_ms: 9,
        };
        let seam = WorldObjectIngestionSeam::from_live_snapshot(Some(&snapshot));
        let decoded = seam
            .items
            .iter()
            .find(|item| item.lane == WorldObjectIngestionLane::DecodedCoarseLocationPayload)
            .expect("decoded coarse payload should exist");
        assert_eq!(decoded.decoded_coarse_location_count, Some(2));
        assert_eq!(decoded.decoded_coarse_first_xyz, Some([64, 32, 12]));
    }

    #[test]
    fn world_object_ingestion_seam_carries_region_and_target_hints() {
        let snapshot = LiveVisualSnapshot {
            source: String::from("test"),
            logged_in: true,
            first_sim_endpoint: Some(String::from("198.51.100.10:13009")),
            first_sim_region_x: Some(1024),
            first_sim_region_y: Some(2048),
            handshake_agent_movement_complete: true,
            traffic_summary_available: true,
            post_boundary_observations: 14,
            region_transition_control_observations: 2,
            crossed_region: 0,
            confirm_enable_simulator: 0,
            likely_broader_traffic: 11,
            unknown: 3,
            decoded_coarse_updates: 0,
            decoded_coarse_location_count: None,
            decoded_coarse_first_x: None,
            decoded_coarse_first_y: None,
            decoded_coarse_first_z: None,
            observed_at_unix_ms: 9,
        };
        let seam = WorldObjectIngestionSeam::from_live_snapshot(Some(&snapshot));
        let item = seam
            .items
            .iter()
            .find(|item| item.lane == WorldObjectIngestionLane::FirstRegionPresenceProxy)
            .expect("first-region presence payload should exist");
        assert_eq!(item.stage, WorldEntryStage::EnteredFirstRegion);
        assert!(item.simulator_target_present);
        assert_eq!(item.region_coords, Some([1024, 2048]));
    }

    #[test]
    fn world_object_ingestion_adapter_matches_seam_from_live_snapshot() {
        let snapshot = sample_snapshot(true, true);
        let seam_from_adapter = WorldObjectIngestionAdapter::adapt(Some(&snapshot));
        let seam_direct = WorldObjectIngestionSeam::from_live_snapshot(Some(&snapshot));
        assert_eq!(seam_from_adapter, seam_direct);
    }

    #[test]
    fn scene_removes_ingestion_proxy_when_seam_is_empty() {
        let mut scene = Scene::prototype();
        let snapshot = sample_snapshot(true, true);
        apply_scene_from_snapshot(&mut scene, Some(&snapshot));
        assert!(
            scene
                .instances
                .iter()
                .any(|instance| instance.role == InstanceRole::WorldIngestionProxy)
        );

        scene.apply_world_object_ingestion_seam(&WorldObjectIngestionSeam::default());
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldIngestionProxy)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldIngestionTrafficPayload)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldIngestionDecodedEndpointPayload)
        );
    }
}


