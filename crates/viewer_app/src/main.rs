use anyhow::{Context, Result};
use dotenvy::dotenv;
mod capability_diagnostics_utils;
mod first_sim_diagnostics_utils;
mod object_feed_diagnostics_utils;
mod runtime_relay_utils;
mod social_cache;
mod start_location_utils;
use capability_diagnostics_utils::*;
use first_sim_diagnostics_utils::*;
use object_feed_diagnostics_utils::*;
use runtime_relay_utils::*;
use social_cache::{SocialCache, SocialCacheConfig};
use start_location_utils::*;
use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tracing_subscriber::FmtSubscriber;
use viewer_core::{
    AlphaMode, AssetID, AssetPriorityHint, AvatarAppearanceSummary, AvatarProfileState,
    AvatarProfileTab, AvatarRenderMode, Camera, ChatConnectionState, ChatMessage, ChatSendStatus,
    ChatState, DirectImMessage, FirstLifeProfile, FriendEntry, GeometrySource, LiveVisualSnapshot,
    MeshKind, NetworkDebugState, ProfileClassifiedDetails, ProfileClassifiedSummary,
    ProfileLoadStatus, ProfileNotes, ProfilePickDetails, ProfilePickSummary,
    RegionContinuitySummary, RuntimeRelayEvent, RuntimeRelayLevel, Scene, SecondLifeProfile,
    SocialState, WorldAvatarPlaceholder, WorldObjectIngestionAdapter, WorldObjectIngestionSeam,
    compute_p2p_session_id,
};
use viewer_grid::{
    AssetCapabilityPolicy, CapabilityProbeMethod, CapabilityProbeRequestShape, GridLoginResult,
    LoginIntent, SecondLifeAdapter, StartLocation, StartLocationIntent, ViewerAssetQueryKey,
};
use viewer_net::{
    AgentProfileData, CapabilityProbeRequest, CapabilityUrlFamily, Connection, ConnectionConfig,
    ConnectionError, FirstSimulatorInboundTrafficScope, LoginFallbackClassifiedReason,
    LoginFallbackOutcome, LoginTrace, LoginWireFormat, NearbyChatMessage, RegionObjectsInspection,
    SeedCapabilityInventoryEntry, SocialCircuit, SocialEvent, classify_capability_url,
    poll_event_queue_url_once,
};
use viewer_render::RenderBackend;
use viewer_ui::{RenderInput, UiSystem};
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes},
};

fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder().finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    let event_loop = EventLoop::new()?;
    let mut app = ViewerApp::default();
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[derive(Default)]
struct ViewerApp {
    state: Option<AppState>,
}

struct AppState {
    window: Arc<Window>,
    renderer: RenderBackend,
    geometry_cache: viewer_asset::GeometryCache,
    fixture_texture_cache: viewer_asset::FixtureTextureCache,
    fixture_texture_ids: Vec<viewer_core::AssetID>,
    fixture_mesh_ids: Vec<String>,
    fixture_texture_missing_logged: HashSet<viewer_core::AssetID>,
    live_mesh_assets: BTreeMap<(String, u32), LiveMeshAssetState>,
    ui: UiSystem,
    camera: Camera,
    scene: Scene,
    world_ingestion_seam: WorldObjectIngestionSeam,
    last_applied_live_visual_snapshot: Option<LiveVisualSnapshot>,
    last_applied_world_ingestion_seam: Option<WorldObjectIngestionSeam>,
    input: InputState,
    live_visual_state: LiveVisualState,
    chat_state: ChatState,
    social_state: SocialState,
    world_avatars: Vec<WorldAvatarPlaceholder>,
    avatar_name_cache: BTreeMap<String, String>,
    world_sim_name: Option<String>,
    startup_sim_name_fallback: Option<String>,
    world_self_location: Option<[f32; 3]>,
    profile_state: Option<AvatarProfileState>,
    profile_image_bytes: BTreeMap<String, Vec<u8>>,
    social_cache: Option<SocialCache>,
    environment: viewer_core::EnvironmentState,
    last_frame_time: Instant,
    app_start_time: Instant,
    smoothed_fps: f32,
    smoothed_frame_ms: f32,
    avg_scene_update_ms: f32,
    stress_test_mode: StressTestMode,
    auto_camera_config: AutoCameraConfig,
    screenshot_config: Option<ScreenshotConfig>,
    frame_counter: u64,
    captured_screenshots: u32,
    live_texture_results: Arc<
        std::sync::Mutex<
            BTreeMap<AssetID, viewer_asset::AssetFetchOutcome<viewer_asset::DecodedRgbaImage>>,
        >,
    >,
    last_probe_retry_ms: Option<u64>,
    last_asset_refresh_ms: Option<u64>,
    probe_in_flight: bool,
    last_recovery_result: Option<viewer_core::RecoveryActionResult>,
    transition_visual_state: viewer_core::TransitionVisualState,
    network_debug: NetworkDebugState,
    mesh_verification: MeshVerificationState,
}

const RECOVERY_PROBE_COOLDOWN_MS: u64 = 15_000;
const RECOVERY_ASSET_REFRESH_COOLDOWN_MS: u64 = 5_000;
const NETWORK_DEBUG_SECTION_MAX_LINES: usize = 8;
const LIVE_TEXTURE_FETCH_MAX_INFLIGHT: usize = 4;
const LIVE_TEXTURE_FETCH_MAX_ATTEMPTS: u8 = 3;
const LIVE_TEXTURE_FETCH_RETRY_BASE_TICKS: u64 = 4;

#[derive(Debug, Clone)]
struct ScheduledTextureFetch {
    priority: viewer_core::AssetPriority,
    first_enqueued_tick: u64,
    ready_at_tick: u64,
    completed_attempts: u8,
}

#[derive(Debug, Clone)]
struct InFlightTextureFetch {
    priority: viewer_core::AssetPriority,
    first_enqueued_tick: u64,
    attempt: u8,
}

#[derive(Debug)]
struct TextureFetchTaskResult {
    id: AssetID,
    priority: viewer_core::AssetPriority,
    first_enqueued_tick: u64,
    attempt: u8,
    outcome: TextureFetchTaskOutcome,
}

#[derive(Debug)]
enum TextureFetchTaskOutcome {
    Ready(Vec<u8>),
    Failed {
        reason: viewer_asset::AssetFetchFailureReason,
        detail: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StressTestMode {
    None,
    SceneStress,
    GeometryTorture,
    AutoCamera,
    Screenshot,
    SingleLiveTextureCenter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AssetSourceMode {
    Fixture,
    Auto,
    Live,
}

impl StressTestMode {
    fn from_value(value: Option<&str>) -> Self {
        let Some(value) = value else {
            return Self::None;
        };
        match value.trim().to_ascii_lowercase().as_str() {
            "1" => Self::SceneStress,
            "2" => Self::GeometryTorture,
            "camera" | "auto_camera" | "auto-camera" | "5" => Self::AutoCamera,
            "screenshot" | "screenshots" | "capture" | "6" => Self::Screenshot,
            "live_texture" | "live-texture" | "single_live_texture" | "7" => {
                Self::SingleLiveTextureCenter
            }
            _ => Self::None,
        }
    }

    fn from_env() -> Self {
        Self::from_value(std::env::var("STRESS_TEST").ok().as_deref())
    }

    fn uses_auto_camera(self) -> bool {
        matches!(self, Self::AutoCamera | Self::Screenshot)
    }
}

#[derive(Debug, Clone)]
struct AutoCameraConfig {
    center: [f32; 3],
    radius: f32,
    orbit_height: f32,
    look_height: f32,
    angular_speed_radians: f32,
    phase_radians: f32,
    path_script: Option<CameraPathScript>,
}

#[derive(Debug, Clone, Copy)]
struct CameraPathWaypoint {
    time_sec: f32,
    position: [f32; 3],
    look_at: [f32; 3],
}

#[derive(Debug, Clone)]
struct CameraPathScript {
    waypoints: Vec<CameraPathWaypoint>,
    duration_sec: f32,
}

impl Default for AutoCameraConfig {
    fn default() -> Self {
        Self {
            center: [0.0, 0.0, 0.0],
            radius: 22.0,
            orbit_height: 10.0,
            look_height: 2.0,
            angular_speed_radians: 0.5,
            phase_radians: 0.0,
            path_script: None,
        }
    }
}

impl AutoCameraConfig {
    fn from_env() -> Self {
        auto_camera_config_from_lookup(|key| std::env::var(key).ok())
    }

    fn apply_to_camera(&self, camera: &mut Camera, elapsed_seconds: f32) {
        if let Some(script) = &self.path_script
            && let Some((position, target)) = script.sample(elapsed_seconds)
        {
            apply_camera_look_at(camera, position, target);
            return;
        }

        let angle = elapsed_seconds * self.angular_speed_radians + self.phase_radians;
        let target = [
            self.center[0],
            self.center[1] + self.look_height,
            self.center[2],
        ];
        let position = [
            self.center[0] + self.radius * angle.cos(),
            self.center[1] + self.orbit_height,
            self.center[2] + self.radius * angle.sin(),
        ];

        apply_camera_look_at(camera, position, target);
    }
}

impl CameraPathScript {
    fn sample(&self, elapsed_seconds: f32) -> Option<([f32; 3], [f32; 3])> {
        if self.waypoints.is_empty() {
            return None;
        }
        if self.waypoints.len() == 1 {
            let only = self.waypoints[0];
            return Some((only.position, only.look_at));
        }

        let duration = self.duration_sec.max(0.001);
        let mut t = elapsed_seconds.rem_euclid(duration);
        if !t.is_finite() {
            t = 0.0;
        }

        for pair in self.waypoints.windows(2) {
            let start = pair[0];
            let end = pair[1];
            if t < start.time_sec || t > end.time_sec {
                continue;
            }
            let span = (end.time_sec - start.time_sec).max(0.0001);
            let alpha = ((t - start.time_sec) / span).clamp(0.0, 1.0);
            let position = lerp_vec3(start.position, end.position, alpha);
            let look_at = lerp_vec3(start.look_at, end.look_at, alpha);
            return Some((position, look_at));
        }

        let fallback = *self.waypoints.last()?;
        Some((fallback.position, fallback.look_at))
    }
}

#[derive(Debug, Clone)]
struct ScreenshotConfig {
    output_dir: PathBuf,
    every_n_frames: u64,
    max_frames: u32,
}

#[derive(Debug, Clone)]
struct LiveMeshAssetBytes {
    bytes: Vec<u8>,
    byte_len: usize,
    byte_signature: String,
    format_hint: viewer_asset::MeshSourceFormat,
}

#[derive(Debug, Clone)]
enum LiveMeshAssetState {
    Requested,
    Fetched(LiveMeshAssetBytes),
    Decoded {
        asset: LiveMeshAssetBytes,
        format: viewer_asset::MeshSourceFormat,
        vertices: usize,
        submeshes: usize,
    },
    Failed {
        reason: viewer_asset::AssetFetchFailureReason,
        detail: String,
        byte_len: Option<usize>,
        byte_signature: Option<String>,
        format: viewer_asset::MeshSourceFormat,
    },
}

#[derive(Debug, Clone)]
struct MeshVerificationConfig {
    target_mesh_id: Option<(String, u32)>,
    log_path: PathBuf,
    screenshot_dir: Option<PathBuf>,
}

#[derive(Debug, Default)]
struct MeshVerificationState {
    config: Option<MeshVerificationConfig>,
    selected_target: Option<(String, u32)>,
    emitted_events: BTreeSet<String>,
    pending_screenshot: Option<PathBuf>,
    captured_screenshot: Option<PathBuf>,
}

impl LiveMeshAssetState {
    fn bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Fetched(asset) => Some(asset.bytes.as_slice()),
            Self::Decoded { asset, .. } => Some(asset.bytes.as_slice()),
            Self::Requested | Self::Failed { .. } => None,
        }
    }
}

#[derive(Default)]
struct InputState {
    move_forward: bool,
    move_backward: bool,
    move_left: bool,
    move_right: bool,
    move_up: bool,
    move_down: bool,
    mouse_look_active: bool,
    pending_look_delta: [f32; 2],
    last_cursor_pos: Option<PhysicalPosition<f64>>,
}

struct LiveVisualState {
    path: PathBuf,
    last_modified: Option<std::time::SystemTime>,
    snapshot: Option<LiveVisualSnapshot>,
    in_process_rx: Option<Receiver<LiveFeedUpdate>>,
    in_process_tx: Option<Sender<LiveFeedCommand>>,
    in_process_enabled: bool,
    startup_status: LiveStartupStatus,
    chat_connection: ChatConnectionState,
    profile_cache_ttl_secs: u64,
}

#[derive(Debug, Clone)]
struct InProcessLiveFeedConfig {
    endpoint: String,
    username: String,
    password: String,
    connect_timeout_secs: u64,
    wire_format: LoginWireFormat,
    start_location: StartLocationIntent,
    agree_to_tos: bool,
    read_critical: bool,
    mfa_token: Option<String>,
    receive_bind: String,
    receive_timeout_secs: u64,
    receive_max_packets: usize,
    post_movement_tail_packets: usize,
    post_movement_timeout_secs: Option<u64>,
    stop_on_region_control: bool,
    auto_teleport_slurl: Option<String>,
    auto_teleport_delay_ticks: u32,
    region_objects_reprobe_delay_ticks: u32,
    lludp_startup_parity_bundle: bool,
    require_region_handshake_reply: bool,
    agent_update_far: f32,
    agent_update_keepalive_ticks_override: Option<u64>,
    run_probe: bool,
    worker_tick_ms: u64,
    event_queue_poll_timeout_ms: u64,
    event_queue_poll_every_ticks: u32,
    event_queue_failures_before_reconnect: u32,
    event_queue_cap_not_found_before_reconnect: u32,
    capability_probes_require_event_queue_ok: bool,
    lane_probe_asset_ids: Vec<String>,
    object_uuid_focus: Option<String>,
    social_poll_timeout_ms: u64,
    social_poll_max_packets: usize,
    nearby_poll_timeout_ms: u64,
    nearby_poll_max_packets: usize,
    nearby_send_receive_timeout_ms: u64,
    nearby_send_receive_packets: usize,
    profile_cache_ttl_secs: u64,
    asset_live_timeout_ms: u64,
}

const AGENT_UPDATE_KEEPALIVE_PERIOD_MS: u64 = 1_000;
const AGENT_UPDATE_CONTROL_FLAGS: u32 = 0;
const DEFAULT_AGENT_UPDATE_FAR: f32 = 96.0;
const REGION_HANDSHAKE_REPRIME_INTERVAL_TICKS: u64 = 40;
const REGION_HANDSHAKE_REPRIME_MAX_ATTEMPTS: u32 = 3;
const FIRST_SIM_TRANSCRIPT_TAIL_LEN: usize = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LiveStartupMode {
    Auto,
    On,
    Off,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LiveStartupStatus {
    DisabledByConfig,
    DisabledMissingConfig,
    Starting,
    Connected,
    Failed(LiveStartupFailure),
}

impl LiveStartupStatus {
    fn to_ux_status(
        &self,
        chat_connection: &viewer_core::ChatConnectionState,
    ) -> viewer_core::SessionUxStatus {
        use viewer_core::{SessionUxReason, SessionUxStatus};

        if matches!(
            chat_connection,
            viewer_core::ChatConnectionState::Reconnecting
        ) {
            return SessionUxStatus::Reconnecting { reason: None };
        }

        match self {
            LiveStartupStatus::DisabledByConfig => SessionUxStatus::Disabled {
                reason: Some(SessionUxReason::DisabledByConfig),
            },
            LiveStartupStatus::DisabledMissingConfig => SessionUxStatus::Disabled {
                reason: Some(SessionUxReason::MissingConfig),
            },
            LiveStartupStatus::Starting => SessionUxStatus::Starting,
            LiveStartupStatus::Connected => SessionUxStatus::Connected,
            LiveStartupStatus::Failed(failure) => {
                let reason = match failure.class {
                    LiveStartupFailureClass::MissingConfig => SessionUxReason::MissingConfig,
                    LiveStartupFailureClass::ConnectTransport => SessionUxReason::ConnectTransport,
                    LiveStartupFailureClass::LoginRequestTransport => {
                        SessionUxReason::LoginTransport
                    }
                    LiveStartupFailureClass::LoginRequestShape => SessionUxReason::LoginTransport,
                    LiveStartupFailureClass::LoginAuth => SessionUxReason::LoginAuth,
                    LiveStartupFailureClass::LoginRequiresTos => SessionUxReason::LoginRequiresTos,
                    LiveStartupFailureClass::LoginRequiresMfa => SessionUxReason::LoginRequiresMfa,
                    LiveStartupFailureClass::LoginUpdateRequired => {
                        SessionUxReason::LoginUpdateRequired
                    }
                    LiveStartupFailureClass::LoginOther => SessionUxReason::Other,
                    LiveStartupFailureClass::ConnectionLostReconnecting => {
                        SessionUxReason::ConnectionLost
                    }
                };

                if failure.class == LiveStartupFailureClass::ConnectionLostReconnecting {
                    SessionUxStatus::Reconnecting {
                        reason: Some(reason),
                    }
                } else {
                    SessionUxStatus::Failed { reason }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LiveStartupFailureClass {
    MissingConfig,
    ConnectTransport,
    LoginRequestTransport,
    LoginRequestShape,
    LoginAuth,
    LoginRequiresTos,
    LoginRequiresMfa,
    LoginUpdateRequired,
    LoginOther,
    ConnectionLostReconnecting,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LiveStartupFailure {
    class: LiveStartupFailureClass,
    message: String,
}

#[derive(Debug, Clone)]
enum LiveFeedUpdate {
    Snapshot(LiveVisualSnapshot),
    Status(LiveStartupStatus),
    ChatConnection(ChatConnectionState),
    ChatMessage(ChatMessage),
    ChatSendStatus(ChatSendStatus),
    FriendsBootstrap(Vec<FriendEntry>),
    FriendPresence {
        id: String,
        online: bool,
    },
    FriendRights {
        id: String,
        rights_has: i32,
        rights_given: i32,
    },
    FriendResolvedName {
        id: String,
        display_name: String,
        source: String,
    },
    AvatarResolvedName {
        id: String,
        display_name: String,
        source: String,
    },
    WorldAvatars {
        avatars: Vec<WorkerWorldAvatarSample>,
        decoded_sim_name: Option<String>,
        startup_sim_name: Option<String>,
        self_location: Option<[f32; 3]>,
        observed_at_unix_ms: u64,
    },
    DirectIm(DirectImMessage),
    ProfileOpenRequested {
        avatar_id: String,
    },
    ProfileTabLoadStarted {
        avatar_id: String,
        tab: AvatarProfileTab,
    },
    ProfileData {
        avatar_id: String,
        profile: AgentProfileData,
        requested_tab: AvatarProfileTab,
    },
    ProfileTabLoadFailed {
        avatar_id: String,
        tab: AvatarProfileTab,
        reason: String,
    },
    ProfileImageLoaded {
        asset_id: String,
        bytes: Vec<u8>,
    },
    ProfileImageFailed,
    Relay(RuntimeRelayEvent),
    TextureAsset {
        id: String,
        bytes: Vec<u8>,
    },
    TextureAssetFailed {
        id: String,
        reason: viewer_asset::AssetFetchFailureReason,
    },
    MeshAsset {
        id: String,
        lod: u32,
        bytes: Vec<u8>,
    },
    MeshAssetFailed {
        id: String,
        lod: u32,
        reason: viewer_asset::AssetFetchFailureReason,
    },
    ContinuityProbeResult(viewer_core::ProbeResultCode),
}

#[derive(Debug, Clone)]
struct WorkerWorldAvatarSample {
    agent_id: Option<String>,
    xyz: [u8; 3],
    is_self: bool,
    sim_name: Option<String>,
}

#[derive(Debug, Clone)]
enum LiveFeedCommand {
    SendChat {
        text: String,
        queued_at_unix_ms: u64,
    },
    SendDirectIm {
        to_agent_id: String,
        text: String,
        queued_at_unix_ms: u64,
    },
    OpenAvatarProfile {
        avatar_id: String,
    },
    SelectProfileTab {
        avatar_id: String,
        tab: AvatarProfileTab,
    },
    RefreshAvatarProfile {
        avatar_id: String,
        tab: Option<AvatarProfileTab>,
    },
    RequestTexture {
        id: AssetID,
        priority: viewer_core::AssetPriority,
    },
    RequestMesh {
        id: String,
        lod: u32,
    },
    ExecuteContinuityProbe {
        queued_at_unix_ms: u64,
    },
    TeleportViaSlurl {
        slurl: String,
        queued_at_unix_ms: u64,
    },
}

#[derive(Debug, Clone)]
struct LiveStartupPlan {
    config: Option<InProcessLiveFeedConfig>,
    enabled: bool,
    startup_status: LiveStartupStatus,
}

pub struct AppLiveTextureProvider {
    command_tx: Sender<LiveFeedCommand>,
    results: Arc<
        std::sync::Mutex<
            BTreeMap<AssetID, viewer_asset::AssetFetchOutcome<viewer_asset::DecodedRgbaImage>>,
        >,
    >,
}

impl viewer_asset::LiveTextureProvider for AppLiveTextureProvider {
    fn request_texture(&mut self, request: &viewer_asset::AssetFetchRequest) -> Result<()> {
        self.command_tx
            .send(LiveFeedCommand::RequestTexture {
                id: request.id.clone(),
                priority: request.priority,
            })
            .map_err(|e| anyhow::anyhow!("failed to send texture request to worker: {}", e))
    }

    fn poll_texture(
        &mut self,
        id: &AssetID,
    ) -> Result<Option<viewer_asset::AssetFetchOutcome<viewer_asset::DecodedRgbaImage>>> {
        let mut results = self.results.lock().unwrap();
        Ok(results.remove(id))
    }
}

#[allow(clippy::too_many_arguments)]
fn compute_recovery_action(
    action: viewer_core::RecoveryAction,
    now_ms: u64,
    last_probe_ms: Option<u64>,
    last_asset_ms: Option<u64>,
    probe_available: bool,
    probe_in_flight: bool,
    probe_cooldown: u64,
    asset_cooldown: u64,
) -> (viewer_core::RecoveryActionResult, Option<u64>, Option<u64>) {
    let mut new_probe = last_probe_ms;
    let mut new_asset = last_asset_ms;

    let (code, cooldown_remaining_ms) = match action {
        viewer_core::RecoveryAction::RetryContinuityProbe => {
            if !probe_available || probe_in_flight {
                (viewer_core::RecoveryResultCode::Unavailable, None)
            } else if let Some(last) = last_probe_ms {
                let elapsed = now_ms.saturating_sub(last);
                if elapsed < probe_cooldown {
                    (
                        viewer_core::RecoveryResultCode::CooldownActive,
                        Some(probe_cooldown - elapsed),
                    )
                } else {
                    new_probe = Some(now_ms);
                    (viewer_core::RecoveryResultCode::Accepted, None)
                }
            } else {
                new_probe = Some(now_ms);
                (viewer_core::RecoveryResultCode::Accepted, None)
            }
        }
        viewer_core::RecoveryAction::RefreshVisibleAssets => {
            if let Some(last) = last_asset_ms {
                let elapsed = now_ms.saturating_sub(last);
                if elapsed < asset_cooldown {
                    (
                        viewer_core::RecoveryResultCode::CooldownActive,
                        Some(asset_cooldown - elapsed),
                    )
                } else {
                    new_asset = Some(now_ms);
                    (viewer_core::RecoveryResultCode::Accepted, None)
                }
            } else {
                new_asset = Some(now_ms);
                (viewer_core::RecoveryResultCode::Accepted, None)
            }
        }
        viewer_core::RecoveryAction::ClearRecoveryBanner => {
            (viewer_core::RecoveryResultCode::Accepted, None)
        }
    };

    let result = viewer_core::RecoveryActionResult {
        action,
        code,
        detail: match action {
            viewer_core::RecoveryAction::RetryContinuityProbe if !probe_available => Some(
                String::from("continuity probe unavailable in current startup mode"),
            ),
            viewer_core::RecoveryAction::RetryContinuityProbe if probe_in_flight => {
                Some(String::from("continuity probe already in flight"))
            }
            _ => None,
        },
        cooldown_remaining_ms,
    };

    (result, new_probe, new_asset)
}

impl LiveVisualState {
    fn from_env() -> Self {
        load_dotenv_file();
        let path = std::env::var("VIEWER_LIVE_VISUAL_SNAPSHOT_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("live_visual_snapshot.json"));
        let startup_plan = live_startup_plan_from_lookup(|key| std::env::var(key).ok());
        let in_process_enabled = startup_plan.enabled;
        let startup_status = startup_plan.startup_status.clone();
        let profile_cache_ttl_secs = startup_plan
            .config
            .as_ref()
            .map(|cfg| cfg.profile_cache_ttl_secs)
            .unwrap_or(120);
        let (in_process_rx, in_process_tx) = if let Some(config) = startup_plan.config {
            let (tx, rx) = spawn_in_process_live_feed(config);
            (Some(rx), Some(tx))
        } else {
            (None, None)
        };

        Self {
            path,
            last_modified: None,
            snapshot: None,
            in_process_rx,
            in_process_tx,
            in_process_enabled,
            startup_status,
            chat_connection: ChatConnectionState::Disabled,
            profile_cache_ttl_secs,
        }
    }

    fn refresh(&mut self) {
        if self.in_process_enabled && self.snapshot.is_some() {
            return;
        }

        let Ok(metadata) = fs::metadata(&self.path) else {
            if !self.in_process_enabled {
                self.last_modified = None;
                self.snapshot = None;
            }
            return;
        };

        let modified = metadata.modified().ok();
        if modified.is_some() && self.last_modified == modified {
            return;
        }

        let Ok(contents) = fs::read_to_string(&self.path) else {
            return;
        };
        let Ok(snapshot) = serde_json::from_str::<LiveVisualSnapshot>(&contents) else {
            return;
        };

        self.snapshot = Some(snapshot);
        self.last_modified = modified;
    }

    fn drain_worker_updates(&mut self) -> Vec<LiveFeedUpdate> {
        let mut updates = Vec::new();
        if let Some(rx) = &self.in_process_rx {
            while let Ok(update) = rx.try_recv() {
                updates.push(update);
            }
        }
        updates
    }

    fn execute_continuity_probe(&mut self) -> bool {
        if let Some(tx) = &self.in_process_tx {
            return tx
                .send(LiveFeedCommand::ExecuteContinuityProbe {
                    queued_at_unix_ms: now_unix_ms(),
                })
                .is_ok();
        }
        false
    }

    fn teleport_via_slurl(&self, slurl: String) {
        if let Some(tx) = &self.in_process_tx {
            let _ = tx.send(LiveFeedCommand::TeleportViaSlurl {
                slurl,
                queued_at_unix_ms: now_unix_ms(),
            });
        }
    }

    fn send_chat(&mut self, text: String) {
        if let Some(tx) = &self.in_process_tx {
            let _ = tx.send(LiveFeedCommand::SendChat {
                text,
                queued_at_unix_ms: now_unix_ms(),
            });
        }
    }

    fn send_direct_im(&self, to_agent_id: String, text: String) {
        if let Some(tx) = &self.in_process_tx {
            let _ = tx.send(LiveFeedCommand::SendDirectIm {
                to_agent_id,
                text,
                queued_at_unix_ms: now_unix_ms(),
            });
        }
    }

    fn open_avatar_profile(&self, avatar_id: String) {
        if let Some(tx) = &self.in_process_tx {
            let _ = tx.send(LiveFeedCommand::OpenAvatarProfile { avatar_id });
        }
    }

    fn select_avatar_profile_tab(&self, avatar_id: String, tab: AvatarProfileTab) {
        if let Some(tx) = &self.in_process_tx {
            let _ = tx.send(LiveFeedCommand::SelectProfileTab { avatar_id, tab });
        }
    }

    fn refresh_avatar_profile(&self, avatar_id: String, tab: Option<AvatarProfileTab>) {
        if let Some(tx) = &self.in_process_tx {
            let _ = tx.send(LiveFeedCommand::RefreshAvatarProfile { avatar_id, tab });
        }
    }
}

fn load_dotenv_file() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        let _ = dotenv();
    });
}

fn in_process_live_feed_config_from_lookup<F>(lookup: F) -> Option<InProcessLiveFeedConfig>
where
    F: Fn(&str) -> Option<String>,
{
    let endpoint = lookup("VIEWER_LOGIN_ENDPOINT")?;
    let username = lookup("VIEWER_LOGIN_USERNAME")?;
    let password = lookup("VIEWER_LOGIN_PASSWORD")?;

    let connect_timeout_secs = lookup("VIEWER_LOGIN_TIMEOUT_SECS")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(15);
    let wire_format = lookup("VIEWER_LOGIN_WIRE_FORMAT")
        .as_deref()
        .map(parse_wire_format)
        .unwrap_or(LoginWireFormat::Llsd);
    let start_location = lookup("VIEWER_LOGIN_START")
        .as_deref()
        .map(parse_start_location)
        .unwrap_or(StartLocationIntent::Saved(StartLocation::Last));
    let agree_to_tos = lookup("VIEWER_LOGIN_AGREE_TOS")
        .map(|v| parse_bool_like(&v))
        .unwrap_or(false);
    let read_critical = lookup("VIEWER_LOGIN_READ_CRITICAL")
        .map(|v| parse_bool_like(&v))
        .unwrap_or(true);
    let mfa_token = lookup("VIEWER_LOGIN_MFA_TOKEN");

    let receive_bind =
        lookup("VIEWER_FIRST_SIM_RECEIVE_BIND").unwrap_or_else(|| String::from("0.0.0.0:0"));
    let receive_timeout_secs = lookup("VIEWER_FIRST_SIM_RECEIVE_TIMEOUT_SECS")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(5);
    let receive_max_packets = lookup("VIEWER_FIRST_SIM_RECEIVE_MAX_PACKETS")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(8);
    let post_movement_tail_packets = lookup("VIEWER_FIRST_SIM_POST_MOVEMENT_TAIL_PACKETS")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(4);
    let post_movement_timeout_secs =
        lookup("VIEWER_FIRST_SIM_POST_MOVEMENT_TIMEOUT_SECS").and_then(|v| v.parse::<u64>().ok());
    let stop_on_region_control = lookup("VIEWER_FIRST_SIM_STOP_ON_REGION_CONTROL")
        .map(|v| parse_bool_like(&v))
        .unwrap_or(false);
    let auto_teleport_slurl = lookup("VIEWER_APP_AUTO_TELEPORT_SLURL")
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let auto_teleport_delay_ticks = lookup("VIEWER_APP_AUTO_TELEPORT_DELAY_TICKS")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(40)
        .min(10_000);
    let region_objects_reprobe_delay_ticks =
        lookup("VIEWER_APP_REGION_OBJECTS_REPROBE_DELAY_TICKS")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(80)
            .clamp(1, 10_000);
    let lludp_startup_parity_bundle = lookup("VIEWER_APP_LLUDP_STARTUP_PARITY_BUNDLE")
        .map(|v| parse_bool_like(&v))
        .unwrap_or(false);
    let require_region_handshake_reply = lookup("VIEWER_APP_REQUIRE_REGION_HANDSHAKE_REPLY")
        .map(|v| parse_bool_like(&v))
        .unwrap_or(false);
    let agent_update_far = lookup("VIEWER_APP_AGENT_UPDATE_FAR")
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(DEFAULT_AGENT_UPDATE_FAR)
        .clamp(16.0, 4096.0);
    let agent_update_keepalive_ticks_override = lookup("VIEWER_APP_AGENT_UPDATE_KEEPALIVE_TICKS")
        .and_then(|v| v.parse::<u64>().ok())
        .map(|ticks| ticks.clamp(1, 60_000));
    let run_probe = lookup("VIEWER_APP_IN_PROCESS_PROBE")
        .map(|v| parse_bool_like(&v))
        .unwrap_or(true);
    let worker_tick_ms = lookup("VIEWER_APP_WORKER_TICK_MS")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(60)
        .max(10);
    let event_queue_poll_timeout_ms = lookup("VIEWER_APP_EVENT_QUEUE_POLL_TIMEOUT_MS")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(45_000)
        .max(100);
    let event_queue_poll_every_ticks = lookup("VIEWER_APP_EVENT_QUEUE_POLL_EVERY_TICKS")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(10)
        .max(1);
    let event_queue_failures_before_reconnect =
        lookup("VIEWER_APP_EVENT_QUEUE_FAILURES_BEFORE_RECONNECT")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0);
    let event_queue_cap_not_found_before_reconnect =
        lookup("VIEWER_APP_EVENT_QUEUE_CAP_NOT_FOUND_BEFORE_RECONNECT")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(3);
    let capability_probes_require_event_queue_ok =
        lookup("VIEWER_APP_CAPABILITY_PROBES_REQUIRE_EVENT_QUEUE_OK")
            .map(|v| parse_bool_like(&v))
            .unwrap_or(true);
    let lane_probe_asset_ids = lookup("VIEWER_APP_LANE_PROBE_ASSET_IDS")
        .map(|v| {
            v.split([',', ';', ' ', '\t', '\n', '\r'])
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let object_uuid_focus = lookup("VIEWER_APP_OBJECT_UUID_FOCUS")
        .map(|v| v.trim().to_ascii_lowercase())
        .filter(|v| !v.is_empty())
        .filter(|v| is_canonical_uuid_like(v));
    let social_poll_timeout_ms = lookup("VIEWER_APP_SOCIAL_POLL_TIMEOUT_MS")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(35)
        .max(5);
    let social_poll_max_packets = lookup("VIEWER_APP_SOCIAL_POLL_MAX_PACKETS")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(4);
    let nearby_poll_timeout_ms = lookup("VIEWER_APP_NEARBY_POLL_TIMEOUT_MS")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(40)
        .max(5);
    let nearby_poll_max_packets = lookup("VIEWER_APP_NEARBY_POLL_MAX_PACKETS")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(2);
    let nearby_send_receive_timeout_ms = lookup("VIEWER_APP_NEARBY_SEND_RECEIVE_TIMEOUT_MS")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(40)
        .max(5);
    let nearby_send_receive_packets = lookup("VIEWER_APP_NEARBY_SEND_RECEIVE_PACKETS")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    let profile_cache_ttl_secs = lookup("VIEWER_APP_PROFILE_CACHE_TTL_SECS")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(120);
    let asset_live_timeout_ms = lookup("VIEWER_ASSET_LIVE_TIMEOUT_MS")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(10_000)
        .clamp(250, 120_000);

    Some(InProcessLiveFeedConfig {
        endpoint,
        username,
        password,
        connect_timeout_secs,
        wire_format,
        start_location,
        agree_to_tos,
        read_critical,
        mfa_token,
        receive_bind,
        receive_timeout_secs,
        receive_max_packets,
        post_movement_tail_packets,
        post_movement_timeout_secs,
        stop_on_region_control,
        auto_teleport_slurl,
        auto_teleport_delay_ticks,
        region_objects_reprobe_delay_ticks,
        lludp_startup_parity_bundle,
        require_region_handshake_reply,
        agent_update_far,
        agent_update_keepalive_ticks_override,
        run_probe,
        worker_tick_ms,
        event_queue_poll_timeout_ms,
        event_queue_poll_every_ticks,
        event_queue_failures_before_reconnect,
        event_queue_cap_not_found_before_reconnect,
        capability_probes_require_event_queue_ok,
        lane_probe_asset_ids,
        object_uuid_focus,
        social_poll_timeout_ms,
        social_poll_max_packets,
        nearby_poll_timeout_ms,
        nearby_poll_max_packets,
        nearby_send_receive_timeout_ms,
        nearby_send_receive_packets,
        profile_cache_ttl_secs,
        asset_live_timeout_ms,
    })
}

fn parse_live_startup_mode(value: Option<&str>) -> LiveStartupMode {
    match value.unwrap_or("auto").trim().to_ascii_lowercase().as_str() {
        "on" | "enabled" | "true" | "1" => LiveStartupMode::On,
        "off" | "disabled" | "false" | "0" => LiveStartupMode::Off,
        _ => LiveStartupMode::Auto,
    }
}

fn parse_asset_source_mode(value: Option<&str>) -> AssetSourceMode {
    match value.unwrap_or("auto").trim().to_ascii_lowercase().as_str() {
        "fixture" => AssetSourceMode::Fixture,
        "live" => AssetSourceMode::Live,
        _ => AssetSourceMode::Auto,
    }
}

fn live_startup_plan_from_lookup<F>(lookup: F) -> LiveStartupPlan
where
    F: Fn(&str) -> Option<String>,
{
    let mode = parse_live_startup_mode(lookup("VIEWER_APP_LIVE_STARTUP").as_deref());
    let config = in_process_live_feed_config_from_lookup(&lookup);
    match mode {
        LiveStartupMode::Off => LiveStartupPlan {
            config: None,
            enabled: false,
            startup_status: LiveStartupStatus::DisabledByConfig,
        },
        LiveStartupMode::On => {
            if let Some(config) = config {
                LiveStartupPlan {
                    config: Some(config),
                    enabled: true,
                    startup_status: LiveStartupStatus::Starting,
                }
            } else {
                LiveStartupPlan {
                    config: None,
                    enabled: false,
                    startup_status: LiveStartupStatus::Failed(LiveStartupFailure {
                        class: LiveStartupFailureClass::MissingConfig,
                        message: String::from("missing required VIEWER_LOGIN_* env vars"),
                    }),
                }
            }
        }
        LiveStartupMode::Auto => {
            if let Some(config) = config {
                LiveStartupPlan {
                    config: Some(config),
                    enabled: true,
                    startup_status: LiveStartupStatus::Starting,
                }
            } else {
                LiveStartupPlan {
                    config: None,
                    enabled: false,
                    startup_status: LiveStartupStatus::DisabledMissingConfig,
                }
            }
        }
    }
}

fn spawn_in_process_live_feed(
    config: InProcessLiveFeedConfig,
) -> (Sender<LiveFeedCommand>, Receiver<LiveFeedUpdate>) {
    let (update_tx, update_rx) = mpsc::channel::<LiveFeedUpdate>();
    let (command_tx, command_rx) = mpsc::channel::<LiveFeedCommand>();
    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        let Ok(runtime) = runtime else {
            return;
        };
        runtime.block_on(async move {
            run_in_process_live_feed(config, update_tx, command_rx).await;
        });
    });
    (command_tx, update_rx)
}

fn classify_asset_fetch_failure_reason(
    error: &viewer_net::ConnectionError,
) -> viewer_asset::AssetFetchFailureReason {
    match error {
        viewer_net::ConnectionError::Http(err) if err.is_timeout() => {
            viewer_asset::AssetFetchFailureReason::Timeout
        }
        viewer_net::ConnectionError::Http(err)
            if err
                .status()
                .map(|status| non_retryable_texture_http_status(status.as_u16()))
                .unwrap_or(false) =>
        {
            viewer_asset::AssetFetchFailureReason::MissingCapability
        }
        viewer_net::ConnectionError::HttpStatus { status, .. }
            if non_retryable_texture_http_status(status.as_u16()) =>
        {
            viewer_asset::AssetFetchFailureReason::MissingCapability
        }
        viewer_net::ConnectionError::MissingCapability(_) => {
            viewer_asset::AssetFetchFailureReason::MissingCapability
        }
        _ => viewer_asset::AssetFetchFailureReason::Transport,
    }
}

fn classify_mesh_http_status_bucket(status_code: u16, body: &str) -> Option<&'static str> {
    if status_code != 403 {
        return None;
    }

    let body = body.to_ascii_lowercase();
    if body.contains("accessdenied") {
        return Some("AccessDenied");
    }
    if body.contains("nosuchkey") || body.contains("not found") {
        return Some("NotFoundOrNoSuchKey");
    }
    if body.contains("signaturedoesnotmatch")
        || body.contains("request has expired")
        || body.contains("expiredtoken")
    {
        return Some("AuthSignatureOrExpiry");
    }
    if body.contains("invalidtoken") || body.contains("access denied") {
        return Some("AuthDenied");
    }

    Some("ForbiddenUnclassified")
}

fn classify_mesh_403_bucket(error: &viewer_net::ConnectionError) -> Option<&'static str> {
    let viewer_net::ConnectionError::HttpStatus { status, body } = error else {
        return None;
    };
    classify_mesh_http_status_bucket(status.as_u16(), body)
}

fn non_retryable_texture_http_status(status_code: u16) -> bool {
    matches!(status_code, 401 | 403 | 404)
}

fn should_retry_live_texture_failure(
    reason: viewer_asset::AssetFetchFailureReason,
    attempt: u8,
) -> bool {
    if attempt >= LIVE_TEXTURE_FETCH_MAX_ATTEMPTS {
        return false;
    }
    matches!(
        reason,
        viewer_asset::AssetFetchFailureReason::Timeout
            | viewer_asset::AssetFetchFailureReason::Transport
    )
}

fn live_texture_retry_backoff_ticks(attempt: u8) -> u64 {
    LIVE_TEXTURE_FETCH_RETRY_BASE_TICKS
        .saturating_mul(1_u64 << u32::from(attempt.saturating_sub(1).min(5)))
}

async fn run_in_process_live_feed(
    config: InProcessLiveFeedConfig,
    tx: mpsc::Sender<LiveFeedUpdate>,
    command_rx: Receiver<LiveFeedCommand>,
) {
    let mut active_start_location = config.start_location.clone();
    let adapter = SecondLifeAdapter;
    let mut reconnect_attempt: u32 = 0;
    let mut auto_teleport_fired = false;
    let mut recent_seed_capability_urls = VecDeque::new();

    let _ = tx.send(LiveFeedUpdate::Status(LiveStartupStatus::Starting));
    let _ = tx.send(LiveFeedUpdate::ChatConnection(
        ChatConnectionState::Connecting,
    ));
    let _ = tx.send(LiveFeedUpdate::Snapshot(offline_snapshot()));
    emit_relay(
        &tx,
        RuntimeRelayLevel::Info,
        "startup",
        "live worker starting",
    );
    emit_relay(
        &tx,
        RuntimeRelayLevel::Info,
        "startup",
        &format!(
            "event queue polling: every_ticks={} timeout_ms={}",
            config.event_queue_poll_every_ticks, config.event_queue_poll_timeout_ms
        ),
    );

    loop {
        let mut connection = Connection::new(ConnectionConfig {
            endpoint: config.endpoint.clone(),
            connect_timeout: std::time::Duration::from_secs(config.connect_timeout_secs),
            wire_format: config.wire_format,
        });
        connection
            .set_require_observed_region_handshake_for_reply(config.require_region_handshake_reply);

        let connect_state = if reconnect_attempt == 0 {
            ChatConnectionState::Connecting
        } else {
            ChatConnectionState::Reconnecting
        };
        let _ = tx.send(LiveFeedUpdate::ChatConnection(connect_state));
        let _ = tx.send(LiveFeedUpdate::Status(LiveStartupStatus::Starting));

        if connection.connect().await.is_err() {
            let failure = LiveStartupFailure {
                class: LiveStartupFailureClass::ConnectTransport,
                message: String::from("connect failed"),
            };
            let _ = tx.send(LiveFeedUpdate::ChatConnection(ChatConnectionState::Failed(
                failure.message.clone(),
            )));
            let _ = tx.send(LiveFeedUpdate::Status(LiveStartupStatus::Failed(failure)));
            reconnect_attempt = reconnect_attempt.saturating_add(1);
            emit_relay(&tx, RuntimeRelayLevel::Warn, "connect", "connect failed");
            tokio::time::sleep(reconnect_backoff_duration(reconnect_attempt)).await;
            continue;
        }

        let intent = LoginIntent {
            username: config.username.clone(),
            password: config.password.clone(),
            start_location: active_start_location.clone(),
            agree_to_tos: config.agree_to_tos,
            read_critical: config.read_critical,
            mfa_token: config.mfa_token.clone(),
        };

        let Ok((result, trace, fallback)) = connection
            .login_with_trace_with_fallback(&adapter, intent)
            .await
        else {
            let failure = LiveStartupFailure {
                class: LiveStartupFailureClass::LoginRequestTransport,
                message: String::from("login request failed"),
            };
            let _ = tx.send(LiveFeedUpdate::ChatConnection(ChatConnectionState::Failed(
                failure.message.clone(),
            )));
            let _ = tx.send(LiveFeedUpdate::Status(LiveStartupStatus::Failed(failure)));
            reconnect_attempt = reconnect_attempt.saturating_add(1);
            emit_relay(
                &tx,
                RuntimeRelayLevel::Warn,
                "login",
                "login request failed",
            );
            tokio::time::sleep(reconnect_backoff_duration(reconnect_attempt)).await;
            continue;
        };
        emit_login_fallback_relay(&tx, &fallback);

        let mut snapshot = build_live_visual_snapshot_from_result(&result);
        let _ = tx.send(LiveFeedUpdate::Snapshot(snapshot.clone()));
        if !snapshot.logged_in {
            let failure = startup_failure_from_login_outcome(&result, &trace, &fallback);
            let _ = tx.send(LiveFeedUpdate::ChatConnection(ChatConnectionState::Failed(
                failure.message.clone(),
            )));
            let _ = tx.send(LiveFeedUpdate::Status(LiveStartupStatus::Failed(
                failure.clone(),
            )));
            reconnect_attempt = reconnect_attempt.saturating_add(1);
            emit_relay(
                &tx,
                RuntimeRelayLevel::Warn,
                "login",
                &format!(
                    "login not successful ({})",
                    startup_failure_class_label(failure.class)
                ),
            );
            tokio::time::sleep(reconnect_backoff_duration(reconnect_attempt)).await;
            continue;
        }
        emit_relay(&tx, RuntimeRelayLevel::Info, "login", "login successful");

        let mut local_agent_id = String::new();
        let mut bootstrap_friends = Vec::new();
        let mut startup_sim_name: Option<String> = None;
        if let GridLoginResult::Success(bootstrap) = &result {
            local_agent_id = bootstrap.agent_id.clone();
            remember_recent_seed_capability_url(
                &mut recent_seed_capability_urls,
                &bootstrap.seed_capability,
            );
            startup_sim_name = bootstrap
                .start_location
                .as_deref()
                .and_then(parse_region_name_from_start_location);
            bootstrap_friends = bootstrap
                .buddy_list
                .iter()
                .filter(|friend| !friend.buddy_id.is_empty())
                .map(|friend| FriendEntry {
                    id: friend.buddy_id.clone(),
                    display_name: None,
                    name_source: None,
                    last_name_resolved_unix_ms: None,
                    online: false,
                    rights_has: friend.rights_has,
                    rights_given: friend.rights_given,
                    last_changed_unix_ms: now_unix_ms(),
                })
                .collect();
        }
        let bootstrap_friend_ids: Vec<String> =
            bootstrap_friends.iter().map(|f| f.id.clone()).collect();
        let bootstrap_friend_id_set: BTreeSet<String> =
            bootstrap_friend_ids.iter().cloned().collect();
        if !bootstrap_friends.is_empty() {
            let _ = tx.send(LiveFeedUpdate::FriendsBootstrap(bootstrap_friends));
        }

        let is_reconnect_session = reconnect_attempt > 0;
        let mut event_ack = 0u64;
        let mut event_queue_consecutive_failures = 0u32;
        let mut event_queue_cap_not_found_failures = 0u32;
        let mut event_queue_poll_task: Option<
            tokio::task::JoinHandle<
                Result<viewer_net::EventQueuePollResult, viewer_net::ConnectionError>,
            >,
        > = None;
        let mut protocol_events = Vec::<String>::new();
        let mut capability_inventory_summary = String::from("none");

        push_protocol_event(&mut protocol_events, "seed_caps:start");
        let capabilities = match connection.fetch_seed_capabilities().await {
            Ok(caps) => {
                let inventory = connection.summarize_seed_capability_inventory(&caps);
                capability_inventory_summary = summarize_seed_capability_inventory(&inventory);
                push_protocol_event(
                    &mut protocol_events,
                    format!("seed_caps:ok {}", capability_inventory_summary),
                );
                let discovered_non_baseline =
                    summarize_non_baseline_caps_by_host(&caps.entries, 16);
                push_protocol_event(
                    &mut protocol_events,
                    format!("seed_caps:non_baseline_by_host {discovered_non_baseline}"),
                );
                emit_relay(
                    &tx,
                    RuntimeRelayLevel::Info,
                    "parallel_protocol",
                    &format!(
                        "seed non-baseline capability names by host: {discovered_non_baseline}"
                    ),
                );
                Some(caps)
            }
            Err(err) => {
                push_protocol_event(&mut protocol_events, format!("seed_caps:err {err}"));
                emit_relay(
                    &tx,
                    RuntimeRelayLevel::Warn,
                    "parallel_protocol",
                    &format!("seed capability fetch failed: {err}"),
                );
                None
            }
        };
        let mut pending_gated_lane_probe_shapes = Vec::<LaneProbeShapeTask>::new();
        if let Some(caps) = capabilities.as_ref() {
            let (mut immediate_lane_probe_shapes, gated_lane_probe_shapes) =
                build_lane_probe_shape_matrix(&caps.entries, &config);
            pending_gated_lane_probe_shapes = gated_lane_probe_shapes;
            run_lane_probe_shape_tasks_once(
                &mut connection,
                &tx,
                &mut protocol_events,
                &mut immediate_lane_probe_shapes,
            )
            .await;
            if pending_gated_lane_probe_shapes.is_empty() {
                push_protocol_event(&mut protocol_events, "LaneProbeShape:gated_none");
            }
        }
        let mut capability_readiness = init_capability_readiness(capabilities.as_ref());
        let mut event_queue_url = capabilities
            .as_ref()
            .and_then(|caps| caps.entries.get("EventQueueGet"))
            .cloned();
        let region_objects_url = capabilities
            .as_ref()
            .and_then(|caps| caps.entries.get("RegionObjects"))
            .cloned();
        let display_names_url = capabilities
            .as_ref()
            .and_then(|caps| caps.entries.get("GetDisplayNames"))
            .cloned();
        let agent_profile_url = capabilities
            .as_ref()
            .and_then(|caps| caps.entries.get("AgentProfile"))
            .cloned();
        let image_cap_url = capabilities
            .as_ref()
            .and_then(|caps| {
                caps.entries
                    .get("GetTexture")
                    .or_else(|| caps.entries.get("ViewerAsset"))
            })
            .cloned();
        let mut pending_interest_list_probe_url = capabilities
            .as_ref()
            .and_then(|caps| caps.entries.get("InterestList"))
            .cloned();
        let mut pending_untrusted_simulator_message_probe_url = capabilities
            .as_ref()
            .and_then(|caps| caps.entries.get("UntrustedSimulatorMessage"))
            .cloned();
        let mut event_queue_has_observed_ok = false;
        let mut event_queue_simulator_target_messages = 0usize;
        let mut event_queue_enable_simulator_messages = 0usize;
        let mut capability_probe_gate_open =
            !config.capability_probes_require_event_queue_ok || event_queue_url.is_none();
        if capability_probe_gate_open {
            let reason = if !config.capability_probes_require_event_queue_ok {
                "config_disabled"
            } else {
                "event_queue_missing"
            };
            push_protocol_event(
                &mut protocol_events,
                format!("probe_gate:open reason={reason}"),
            );
        } else {
            push_protocol_event(
                &mut protocol_events,
                String::from("probe_gate:wait EventQueueGet:ok"),
            );
            if pending_interest_list_probe_url.is_some() {
                push_protocol_event(
                    &mut protocol_events,
                    String::from("InterestList:deferred waiting_for=EventQueueGet:ok"),
                );
            }
            if pending_untrusted_simulator_message_probe_url.is_some() {
                push_protocol_event(
                    &mut protocol_events,
                    String::from("UntrustedSimulatorMessage:deferred waiting_for=EventQueueGet:ok"),
                );
            }
        }
        if let Some(url) = event_queue_url.as_ref() {
            mark_capability_invocation_started(&mut capability_readiness, "EventQueueGet");
            push_protocol_event(
                &mut protocol_events,
                format!(
                    "EventQueueGet:start ack={} {}",
                    event_ack,
                    format_classified_url(url)
                ),
            );
            let timeout = std::time::Duration::from_millis(config.event_queue_poll_timeout_ms);
            event_queue_poll_task =
                Some(spawn_event_queue_poll_task(url.clone(), event_ack, timeout));
        }
        if let Some(url) = region_objects_url.as_deref() {
            mark_capability_invocation_started(&mut capability_readiness, "RegionObjects");
            match connection.fetch_region_objects_once(url).await {
                Ok(inspection) => {
                    mark_capability_invocation_result(
                        &mut capability_readiness,
                        "RegionObjects",
                        Ok("ok"),
                    );
                    let summary = summarize_region_objects_inspection(&inspection);
                    let classified_url = format_classified_url(url);
                    let host_family = format_capability_host_family_tag(url);
                    push_protocol_event(
                        &mut protocol_events,
                        format!("RegionObjects:ok {summary} {host_family}"),
                    );
                    emit_relay(
                        &tx,
                        RuntimeRelayLevel::Info,
                        "region_objects",
                        &format!("primary probe {summary} {classified_url} {host_family}"),
                    );
                }
                Err(err) => {
                    mark_capability_invocation_result(
                        &mut capability_readiness,
                        "RegionObjects",
                        Err(&err.to_string()),
                    );
                    let classified_url = format_classified_url(url);
                    let host_family = format_capability_host_family_tag(url);
                    push_protocol_event(
                        &mut protocol_events,
                        format!("RegionObjects:err {err} {host_family}"),
                    );
                    emit_relay(
                        &tx,
                        RuntimeRelayLevel::Warn,
                        "region_objects",
                        &format!("primary probe failed {err} {classified_url} {host_family}"),
                    );
                }
            }
        }
        if capability_probe_gate_open {
            run_pending_capability_readiness_probes(
                &mut connection,
                &mut capability_readiness,
                &mut protocol_events,
                &mut pending_interest_list_probe_url,
                &mut pending_untrusted_simulator_message_probe_url,
            )
            .await;
            run_lane_probe_shape_tasks_once(
                &mut connection,
                &tx,
                &mut protocol_events,
                &mut pending_gated_lane_probe_shapes,
            )
            .await;
        }
        let mut post_reconnect_region_objects_reprobe_pending =
            is_reconnect_session && region_objects_url.is_some();
        if post_reconnect_region_objects_reprobe_pending {
            emit_relay(
                &tx,
                RuntimeRelayLevel::Info,
                "region_objects",
                &format!(
                    "post-reconnect re-probe armed delay_ticks={}",
                    config.region_objects_reprobe_delay_ticks
                ),
            );
            push_protocol_event(
                &mut protocol_events,
                format!(
                    "RegionObjects:reprobe_armed delay_ticks={}",
                    config.region_objects_reprobe_delay_ticks
                ),
            );
        }

        if config.run_probe && matches!(result, GridLoginResult::Success(_)) {
            let _ = connection
                .probe_first_simulator_handshake_window_with_policy(
                    &config.receive_bind,
                    std::time::Duration::from_secs(config.receive_timeout_secs),
                    config.receive_max_packets,
                    config.post_movement_tail_packets,
                    config
                        .post_movement_timeout_secs
                        .map(std::time::Duration::from_secs),
                    config.stop_on_region_control,
                )
                .await;
            emit_first_simulator_socket_summary(&tx, &connection, "after_probe");
        }

        let mut attempted_profile_image_assets = BTreeSet::new();
        let mut known_avatar_name_ids = BTreeSet::new();
        known_avatar_name_ids.extend(bootstrap_friend_ids.iter().cloned());
        if !local_agent_id.is_empty() {
            known_avatar_name_ids.insert(local_agent_id.clone());
        }
        let (texture_fetch_result_tx, texture_fetch_result_rx) =
            mpsc::channel::<TextureFetchTaskResult>();
        let mut pending_texture_fetches = BTreeMap::<AssetID, ScheduledTextureFetch>::new();
        let mut inflight_texture_fetches = BTreeMap::<AssetID, InFlightTextureFetch>::new();
        let mut worker_tick: u64 = 0;
        let agent_update_keepalive_interval_ticks = agent_update_keepalive_interval_ticks(&config);
        let mut last_agent_update_tick = None;
        let mut region_handshake_reprime_attempts: u32 = 0;
        let mut last_region_handshake_reprime_tick: Option<u64> = None;
        let mut first_sim_socket_steady_state_summary_emitted = false;
        let mut social_circuit: Option<SocialCircuit> = connection
            .open_social_circuit(&config.receive_bind)
            .await
            .ok();
        let mut followed_enable_simulator_ports = BTreeSet::new();
        let mut followed_seed_capability_urls = BTreeSet::new();
        emit_first_simulator_socket_summary(&tx, &connection, "after_open_social_circuit");
        emit_relay(
            &tx,
            RuntimeRelayLevel::Info,
            "social",
            &format!(
                "startup prime mode=lludp_parity_bundle:{}",
                if config.lludp_startup_parity_bundle {
                    "on"
                } else {
                    "off"
                }
            ),
        );
        if let Some(circuit) = social_circuit.as_ref() {
            if let Err(err) = prime_startup_social_circuit(
                &mut connection,
                circuit,
                &tx,
                &local_agent_id,
                &config,
            )
            .await
            {
                emit_relay(
                    &tx,
                    RuntimeRelayLevel::Warn,
                    "social",
                    &format!("startup social activation failed: {err}"),
                );
            } else {
                let _ = flush_pending_first_sim_ack_ids(
                    &mut connection,
                    circuit,
                    &tx,
                    "after_startup_social_prime",
                )
                .await;
                last_agent_update_tick = Some(worker_tick);
            }
            emit_first_simulator_socket_summary(&tx, &connection, "after_startup_social_prime");
        } else {
            emit_relay(
                &tx,
                RuntimeRelayLevel::Warn,
                "social",
                "social circuit unavailable",
            );
        }
        if !bootstrap_friend_ids.is_empty() {
            let mut resolved_count = 0usize;
            if let Some(url) = display_names_url.as_deref() {
                push_protocol_event(
                    &mut protocol_events,
                    format!(
                        "GetDisplayNames:start ids={} {}",
                        bootstrap_friend_ids.len(),
                        format_classified_url(url)
                    ),
                );
                match connection
                    .resolve_avatar_display_names(url, &bootstrap_friend_ids)
                    .await
                {
                    Ok(names) => {
                        resolved_count = names.len();
                        push_protocol_event(
                            &mut protocol_events,
                            format!(
                                "GetDisplayNames:ok resolved={} {}",
                                resolved_count,
                                format_classified_url(url)
                            ),
                        );
                        for entry in names {
                            let _ = tx.send(LiveFeedUpdate::FriendResolvedName {
                                id: entry.id,
                                display_name: entry.display_name,
                                source: String::from("caps.GetDisplayNames"),
                            });
                        }
                    }
                    Err(err) => {
                        push_protocol_event(
                            &mut protocol_events,
                            format!("GetDisplayNames:err {err} {}", format_classified_url(url)),
                        );
                        emit_relay(
                            &tx,
                            RuntimeRelayLevel::Warn,
                            "friend_name",
                            &format!("display name lookup failed: {err}"),
                        );
                    }
                }
            }
            if resolved_count == 0 {
                if let Some(profile_url) = agent_profile_url.as_deref() {
                    push_protocol_event(
                        &mut protocol_events,
                        format!(
                            "AgentProfile:start ids={} {}",
                            bootstrap_friend_ids.len(),
                            format_classified_url(profile_url)
                        ),
                    );
                    let mut profile_count = 0usize;
                    for id in &bootstrap_friend_ids {
                        if let Ok(profile) = connection.fetch_agent_profile(profile_url, id).await
                            && let Some(display_name) = pick_best_avatar_name(&profile)
                        {
                            profile_count = profile_count.saturating_add(1);
                            let _ = tx.send(LiveFeedUpdate::FriendResolvedName {
                                id: id.clone(),
                                display_name,
                                source: String::from("caps.AgentProfile"),
                            });
                        }
                    }
                    push_protocol_event(
                        &mut protocol_events,
                        format!(
                            "AgentProfile:ok resolved={} {}",
                            profile_count,
                            format_classified_url(profile_url)
                        ),
                    );
                    emit_relay(
                        &tx,
                        RuntimeRelayLevel::Info,
                        "friend_name",
                        &format!(
                            "resolved {} friend names (fallback AgentProfile)",
                            profile_count
                        ),
                    );
                } else if display_names_url.is_none() {
                    emit_relay(
                        &tx,
                        RuntimeRelayLevel::Warn,
                        "friend_name",
                        "name capabilities unavailable (GetDisplayNames + AgentProfile)",
                    );
                }
            } else {
                emit_relay(
                    &tx,
                    RuntimeRelayLevel::Info,
                    "friend_name",
                    &format!("resolved {} friend names", resolved_count),
                );
            }
        }

        update_live_visual_from_connection(&mut snapshot, &connection, &config);
        snapshot.source = String::from("viewer_app_in_process:ready");
        emit_object_feed_startup_summary(&tx, &snapshot, &connection);
        emit_parallel_protocol_summary(
            &tx,
            &connection,
            "startup",
            &capability_inventory_summary,
            &summarize_capability_readiness(&capability_readiness),
            &protocol_events,
            event_queue_url.as_deref(),
            event_ack,
            event_queue_consecutive_failures,
            event_queue_cap_not_found_failures,
            event_queue_has_observed_ok,
            event_queue_simulator_target_messages,
            event_queue_enable_simulator_messages,
        );
        let _ = tx.send(LiveFeedUpdate::Snapshot(snapshot.clone()));
        let _ = tx.send(LiveFeedUpdate::WorldAvatars {
            avatars: extract_worker_world_avatar_samples(&connection, &local_agent_id),
            decoded_sim_name: extract_worker_world_sim_name(&connection),
            startup_sim_name: startup_sim_name.clone(),
            self_location: extract_worker_self_location(&connection),
            observed_at_unix_ms: now_unix_ms(),
        });
        let _ = tx.send(LiveFeedUpdate::Status(LiveStartupStatus::Connected));
        let _ = tx.send(LiveFeedUpdate::ChatConnection(
            ChatConnectionState::Connected,
        ));
        reconnect_attempt = 0;
        if !auto_teleport_fired && let Some(slurl) = config.auto_teleport_slurl.as_deref() {
            emit_relay(
                &tx,
                RuntimeRelayLevel::Info,
                "teleport",
                &format!(
                    "auto teleport armed delay_ticks={} input={}",
                    config.auto_teleport_delay_ticks, slurl
                ),
            );
        }

        let mut should_reconnect = false;
        let mut reconnect_reason: Option<String> = None;
        loop {
            if !auto_teleport_fired
                && !should_reconnect
                && worker_tick >= u64::from(config.auto_teleport_delay_ticks)
                && let Some(slurl) = config.auto_teleport_slurl.as_deref()
            {
                emit_relay(
                    &tx,
                    RuntimeRelayLevel::Info,
                    "teleport",
                    &format!(
                        "auto teleport firing worker_tick={} input={}",
                        worker_tick, slurl
                    ),
                );
                auto_teleport_fired = true;
                apply_reconnect_teleport_request(
                    &tx,
                    &mut active_start_location,
                    &mut reconnect_reason,
                    &mut should_reconnect,
                    slurl,
                    &format!("auto worker_tick={worker_tick}"),
                );
            }
            if post_reconnect_region_objects_reprobe_pending
                && !should_reconnect
                && worker_tick >= u64::from(config.region_objects_reprobe_delay_ticks)
            {
                post_reconnect_region_objects_reprobe_pending = false;
                if let Some(url) = region_objects_url.as_deref() {
                    match connection.fetch_region_objects_once(url).await {
                        Ok(inspection) => {
                            let summary = summarize_region_objects_inspection(&inspection);
                            let classified_url = format_classified_url(url);
                            let host_family = format_capability_host_family_tag(url);
                            push_protocol_event(
                                &mut protocol_events,
                                format!("RegionObjects:reprobe_ok {summary} {host_family}"),
                            );
                            emit_relay(
                                &tx,
                                RuntimeRelayLevel::Info,
                                "region_objects",
                                &format!(
                                    "post-reconnect re-probe {summary} {classified_url} {host_family}"
                                ),
                            );
                        }
                        Err(err) => {
                            let classified_url = format_classified_url(url);
                            let host_family = format_capability_host_family_tag(url);
                            push_protocol_event(
                                &mut protocol_events,
                                format!("RegionObjects:reprobe_err {err} {host_family}"),
                            );
                            emit_relay(
                                &tx,
                                RuntimeRelayLevel::Warn,
                                "region_objects",
                                &format!(
                                    "post-reconnect re-probe failed {err} {classified_url} {host_family}"
                                ),
                            );
                        }
                    }
                }
            }

            while let Ok(task_result) = texture_fetch_result_rx.try_recv() {
                let id = task_result.id.clone();
                let id_str = id.to_string();
                let Some(in_flight) = inflight_texture_fetches.remove(&id) else {
                    emit_relay(
                        &tx,
                        RuntimeRelayLevel::Warn,
                        "texture_fetch",
                        &format!("completion for non-inflight texture {id_str} ignored"),
                    );
                    continue;
                };
                let effective_priority = task_result.priority.min(in_flight.priority);
                let effective_first_enqueued_tick = task_result
                    .first_enqueued_tick
                    .min(in_flight.first_enqueued_tick);
                match task_result.outcome {
                    TextureFetchTaskOutcome::Ready(bytes) => {
                        let _ = tx.send(LiveFeedUpdate::TextureAsset {
                            id: id_str.clone(),
                            bytes,
                        });
                        emit_relay(
                            &tx,
                            RuntimeRelayLevel::Info,
                            "texture_fetch",
                            &format!(
                                "ready id={} attempt={} priority={:?}",
                                id_str, task_result.attempt, effective_priority
                            ),
                        );
                    }
                    TextureFetchTaskOutcome::Failed { reason, detail } => {
                        if should_retry_live_texture_failure(reason, task_result.attempt) {
                            let backoff_ticks =
                                live_texture_retry_backoff_ticks(task_result.attempt);
                            pending_texture_fetches
                                .entry(id.clone())
                                .and_modify(|existing| {
                                    if effective_priority < existing.priority {
                                        existing.priority = effective_priority;
                                    }
                                    existing.ready_at_tick =
                                        worker_tick.saturating_add(backoff_ticks);
                                    existing.completed_attempts = task_result.attempt;
                                })
                                .or_insert(ScheduledTextureFetch {
                                    priority: effective_priority,
                                    first_enqueued_tick: effective_first_enqueued_tick,
                                    ready_at_tick: worker_tick.saturating_add(backoff_ticks),
                                    completed_attempts: task_result.attempt,
                                });
                            emit_relay(
                                &tx,
                                RuntimeRelayLevel::Warn,
                                "texture_fetch",
                                &format!(
                                    "retry id={} reason={:?} attempt={} backoff_ticks={} detail={}",
                                    id_str, reason, task_result.attempt, backoff_ticks, detail
                                ),
                            );
                        } else {
                            let _ = tx.send(LiveFeedUpdate::TextureAssetFailed {
                                id: id_str.clone(),
                                reason,
                            });
                            emit_relay(
                                &tx,
                                RuntimeRelayLevel::Warn,
                                "texture_fetch",
                                &format!(
                                    "failed id={} reason={:?} attempt={} detail={}",
                                    id_str, reason, task_result.attempt, detail
                                ),
                            );
                        }
                    }
                }
            }

            let available_texture_slots =
                LIVE_TEXTURE_FETCH_MAX_INFLIGHT.saturating_sub(inflight_texture_fetches.len());
            if available_texture_slots > 0 {
                let mut dispatch_candidates = pending_texture_fetches
                    .iter()
                    .filter(|(_, queued)| queued.ready_at_tick <= worker_tick)
                    .map(|(id, queued)| (id.clone(), queued.priority, queued.first_enqueued_tick))
                    .collect::<Vec<_>>();
                dispatch_candidates.sort_by(|a, b| {
                    a.1.cmp(&b.1)
                        .then_with(|| a.2.cmp(&b.2))
                        .then_with(|| a.0.cmp(&b.0))
                });
                for (id, _, _) in dispatch_candidates
                    .into_iter()
                    .take(available_texture_slots)
                {
                    let Some(queued) = pending_texture_fetches.remove(&id) else {
                        continue;
                    };
                    let attempt = queued.completed_attempts.saturating_add(1);
                    inflight_texture_fetches.insert(
                        id.clone(),
                        InFlightTextureFetch {
                            priority: queued.priority,
                            first_enqueued_tick: queued.first_enqueued_tick,
                            attempt,
                        },
                    );

                    let tx_result = texture_fetch_result_tx.clone();
                    let caps = capabilities.clone();
                    let fetch_timeout =
                        std::time::Duration::from_millis(config.asset_live_timeout_ms);
                    tokio::spawn(async move {
                        let outcome = if let Some(caps) = caps {
                            let urls = viewer_grid::AssetCapabilityPolicy::texture_url_candidates(
                                &caps.entries,
                                &id,
                            );
                            if urls.is_empty() {
                                TextureFetchTaskOutcome::Failed {
                                    reason:
                                        viewer_asset::AssetFetchFailureReason::MissingCapability,
                                    detail: String::from(
                                        "GetTexture/ViewerAsset capability URL candidates missing",
                                    ),
                                }
                            } else {
                                match viewer_net::fetch_texture_asset_bytes(&urls, fetch_timeout)
                                    .await
                                {
                                    Ok(bytes) => TextureFetchTaskOutcome::Ready(bytes),
                                    Err(err) => TextureFetchTaskOutcome::Failed {
                                        reason: classify_asset_fetch_failure_reason(&err),
                                        detail: err.to_string(),
                                    },
                                }
                            }
                        } else {
                            TextureFetchTaskOutcome::Failed {
                                reason: viewer_asset::AssetFetchFailureReason::MissingCapability,
                                detail: String::from(
                                    "active session capabilities unavailable for texture request",
                                ),
                            }
                        };
                        let _ = tx_result.send(TextureFetchTaskResult {
                            id,
                            priority: queued.priority,
                            first_enqueued_tick: queued.first_enqueued_tick,
                            attempt,
                            outcome,
                        });
                    });
                }
            }

            while let Ok(command) = command_rx.try_recv() {
                match command {
                    LiveFeedCommand::SendChat {
                        text,
                        queued_at_unix_ms,
                    } => {
                        let send_started_at = now_unix_ms();
                        emit_relay(
                            &tx,
                            RuntimeRelayLevel::Info,
                            "send_latency",
                            &format!(
                                "nearby queued_ms={}",
                                send_started_at.saturating_sub(queued_at_unix_ms)
                            ),
                        );
                        let _ = tx.send(LiveFeedUpdate::ChatSendStatus(ChatSendStatus::Sending));
                        match send_nearby_chat_with_caps(
                            &mut connection,
                            social_circuit.as_ref(),
                            &text,
                            &config.receive_bind,
                            std::time::Duration::from_millis(config.nearby_send_receive_timeout_ms),
                            config.nearby_send_receive_packets,
                        )
                        .await
                        {
                            Ok(received) => {
                                let sent_at = now_unix_ms();
                                emit_relay(
                                    &tx,
                                    RuntimeRelayLevel::Info,
                                    "send_latency",
                                    &format!(
                                        "nearby send_ms={}",
                                        sent_at.saturating_sub(send_started_at)
                                    ),
                                );
                                let _ =
                                    tx.send(LiveFeedUpdate::ChatSendStatus(ChatSendStatus::Sent));
                                for chat in received {
                                    let _ = tx.send(LiveFeedUpdate::ChatMessage(ChatMessage {
                                        id: now_unix_ms(),
                                        observed_at_unix_ms: now_unix_ms(),
                                        sender: chat.sender,
                                        text: chat.text,
                                        source: chat.source,
                                    }));
                                }
                            }
                            Err(err) => {
                                let _ = tx.send(LiveFeedUpdate::ChatSendStatus(
                                    ChatSendStatus::Failed(err.to_string()),
                                ));
                                emit_relay(
                                    &tx,
                                    RuntimeRelayLevel::Warn,
                                    "send_latency",
                                    &format!(
                                        "nearby send failed after {} ms: {err}",
                                        now_unix_ms().saturating_sub(send_started_at)
                                    ),
                                );
                                if matches!(
                                    err,
                                    ConnectionError::InvalidState(_)
                                        | ConnectionError::FirstSimulatorHandshakeSendFailed { .. }
                                ) {
                                    should_reconnect = true;
                                }
                            }
                        }
                    }
                    LiveFeedCommand::SendDirectIm {
                        to_agent_id,
                        text,
                        queued_at_unix_ms,
                    } => {
                        let send_started_at = now_unix_ms();
                        emit_relay(
                            &tx,
                            RuntimeRelayLevel::Info,
                            "send_latency",
                            &format!(
                                "direct_im queued_ms={}",
                                send_started_at.saturating_sub(queued_at_unix_ms)
                            ),
                        );
                        let Some(circuit) = &social_circuit else {
                            emit_relay(
                                &tx,
                                RuntimeRelayLevel::Warn,
                                "im_send",
                                "social circuit unavailable",
                            );
                            continue;
                        };
                        let Some(session_id) =
                            compute_p2p_session_id(&local_agent_id, &to_agent_id)
                        else {
                            emit_relay(
                                &tx,
                                RuntimeRelayLevel::Warn,
                                "im_send",
                                "invalid agent ids for p2p session id",
                            );
                            continue;
                        };
                        let from_name = config.username.clone();
                        match connection
                            .send_direct_im(circuit, &to_agent_id, &session_id, &from_name, &text)
                            .await
                        {
                            Ok(()) => {
                                let sent_at = now_unix_ms();
                                let _ = tx.send(LiveFeedUpdate::DirectIm(DirectImMessage {
                                    id: now_unix_ms(),
                                    session_id: session_id.clone(),
                                    peer_id: to_agent_id.clone(),
                                    from_id: local_agent_id.clone(),
                                    from_name: String::from("You"),
                                    text: text.clone(),
                                    observed_at_unix_ms: now_unix_ms(),
                                    outgoing: true,
                                }));
                                emit_relay(
                                    &tx,
                                    RuntimeRelayLevel::Info,
                                    "im_send",
                                    &format!("im sent to {}", to_agent_id),
                                );
                                emit_relay(
                                    &tx,
                                    RuntimeRelayLevel::Info,
                                    "send_latency",
                                    &format!(
                                        "direct_im send_ms={}",
                                        sent_at.saturating_sub(send_started_at)
                                    ),
                                );
                            }
                            Err(err) => {
                                emit_relay(
                                    &tx,
                                    RuntimeRelayLevel::Warn,
                                    "im_send",
                                    &format!("im send failed: {err}"),
                                );
                                emit_relay(
                                    &tx,
                                    RuntimeRelayLevel::Warn,
                                    "send_latency",
                                    &format!(
                                        "direct_im send failed after {} ms: {err}",
                                        now_unix_ms().saturating_sub(send_started_at)
                                    ),
                                );
                                if matches!(
                                    err,
                                    ConnectionError::InvalidState(_)
                                        | ConnectionError::FirstSimulatorHandshakeSendFailed { .. }
                                ) {
                                    should_reconnect = true;
                                }
                            }
                        }
                    }
                    LiveFeedCommand::OpenAvatarProfile { avatar_id } => {
                        let avatar_id = avatar_id.trim().to_ascii_lowercase();
                        if avatar_id.is_empty() {
                            continue;
                        }
                        let _ = tx.send(LiveFeedUpdate::ProfileOpenRequested {
                            avatar_id: avatar_id.clone(),
                        });
                        emit_relay(
                            &tx,
                            RuntimeRelayLevel::Info,
                            "profile_open",
                            &format!("open {}", avatar_id),
                        );
                        let tab = AvatarProfileTab::SecondLife;
                        let _ = tx.send(LiveFeedUpdate::ProfileTabLoadStarted {
                            avatar_id: avatar_id.clone(),
                            tab,
                        });
                        emit_relay(
                            &tx,
                            RuntimeRelayLevel::Info,
                            "profile_tab_load_start",
                            &format!("{avatar_id} {tab:?}"),
                        );
                        match fetch_profile_tab_data(
                            &mut connection,
                            agent_profile_url.as_deref(),
                            &avatar_id,
                            &config.receive_bind,
                            tab,
                        )
                        .await
                        {
                            Ok(profile) => {
                                let profile_for_images = profile.clone();
                                let _ = tx.send(LiveFeedUpdate::ProfileData {
                                    avatar_id: avatar_id.clone(),
                                    profile,
                                    requested_tab: tab,
                                });
                                emit_relay(
                                    &tx,
                                    RuntimeRelayLevel::Info,
                                    "profile_tab_load_success",
                                    &format!("{avatar_id} {tab:?}"),
                                );
                                load_profile_images_for_payload(
                                    &mut connection,
                                    &tx,
                                    image_cap_url.as_deref(),
                                    &mut attempted_profile_image_assets,
                                    profile_for_images,
                                )
                                .await;
                            }
                            Err(err) => {
                                let _ = tx.send(LiveFeedUpdate::ProfileTabLoadFailed {
                                    avatar_id: avatar_id.clone(),
                                    tab,
                                    reason: err.to_string(),
                                });
                                emit_relay(
                                    &tx,
                                    RuntimeRelayLevel::Warn,
                                    "profile_fallback_path",
                                    &format!("{avatar_id} {tab:?}: {err}"),
                                );
                            }
                        }
                    }
                    LiveFeedCommand::SelectProfileTab { avatar_id, tab } => {
                        let avatar_id = avatar_id.trim().to_ascii_lowercase();
                        if avatar_id.is_empty() {
                            continue;
                        }
                        let _ = tx.send(LiveFeedUpdate::ProfileTabLoadStarted {
                            avatar_id: avatar_id.clone(),
                            tab,
                        });
                        emit_relay(
                            &tx,
                            RuntimeRelayLevel::Info,
                            "profile_tab_load_start",
                            &format!("{avatar_id} {tab:?}"),
                        );
                        match fetch_profile_tab_data(
                            &mut connection,
                            agent_profile_url.as_deref(),
                            &avatar_id,
                            &config.receive_bind,
                            tab,
                        )
                        .await
                        {
                            Ok(profile) => {
                                let profile_for_images = profile.clone();
                                let _ = tx.send(LiveFeedUpdate::ProfileData {
                                    avatar_id: avatar_id.clone(),
                                    profile,
                                    requested_tab: tab,
                                });
                                emit_relay(
                                    &tx,
                                    RuntimeRelayLevel::Info,
                                    "profile_tab_load_success",
                                    &format!("{avatar_id} {tab:?}"),
                                );
                                load_profile_images_for_payload(
                                    &mut connection,
                                    &tx,
                                    image_cap_url.as_deref(),
                                    &mut attempted_profile_image_assets,
                                    profile_for_images,
                                )
                                .await;
                            }
                            Err(err) => {
                                let _ = tx.send(LiveFeedUpdate::ProfileTabLoadFailed {
                                    avatar_id: avatar_id.clone(),
                                    tab,
                                    reason: err.to_string(),
                                });
                                emit_relay(
                                    &tx,
                                    RuntimeRelayLevel::Warn,
                                    "profile_fallback_path",
                                    &format!("{avatar_id} {tab:?}: {err}"),
                                );
                            }
                        }
                    }
                    LiveFeedCommand::RefreshAvatarProfile { avatar_id, tab } => {
                        let avatar_id = avatar_id.trim().to_ascii_lowercase();
                        if avatar_id.is_empty() {
                            continue;
                        }
                        let requested_tab = tab.unwrap_or(AvatarProfileTab::SecondLife);
                        let _ = tx.send(LiveFeedUpdate::ProfileTabLoadStarted {
                            avatar_id: avatar_id.clone(),
                            tab: requested_tab,
                        });
                        emit_relay(
                            &tx,
                            RuntimeRelayLevel::Info,
                            "profile_tab_load_start",
                            &format!("{avatar_id} {requested_tab:?}"),
                        );
                        match fetch_profile_tab_data(
                            &mut connection,
                            agent_profile_url.as_deref(),
                            &avatar_id,
                            &config.receive_bind,
                            requested_tab,
                        )
                        .await
                        {
                            Ok(profile) => {
                                let profile_for_images = profile.clone();
                                let _ = tx.send(LiveFeedUpdate::ProfileData {
                                    avatar_id: avatar_id.clone(),
                                    profile,
                                    requested_tab,
                                });
                                emit_relay(
                                    &tx,
                                    RuntimeRelayLevel::Info,
                                    "profile_tab_load_success",
                                    &format!("{avatar_id} {requested_tab:?}"),
                                );
                                load_profile_images_for_payload(
                                    &mut connection,
                                    &tx,
                                    image_cap_url.as_deref(),
                                    &mut attempted_profile_image_assets,
                                    profile_for_images,
                                )
                                .await;
                            }
                            Err(err) => {
                                let _ = tx.send(LiveFeedUpdate::ProfileTabLoadFailed {
                                    avatar_id: avatar_id.clone(),
                                    tab: requested_tab,
                                    reason: err.to_string(),
                                });
                                emit_relay(
                                    &tx,
                                    RuntimeRelayLevel::Warn,
                                    "profile_fallback_path",
                                    &format!("{avatar_id} {requested_tab:?}: {err}"),
                                );
                            }
                        }
                    }
                    LiveFeedCommand::RequestTexture { id, priority } => {
                        if id.is_empty() {
                            continue;
                        }
                        let id_str = id.to_string();
                        if let Some(inflight) = inflight_texture_fetches.get_mut(&id) {
                            if priority < inflight.priority {
                                inflight.priority = priority;
                            }
                            emit_relay(
                                &tx,
                                RuntimeRelayLevel::Info,
                                "texture_fetch",
                                &format!(
                                    "inflight id={} priority={:?} attempt={} enqueued_tick={}",
                                    id_str,
                                    inflight.priority,
                                    inflight.attempt,
                                    inflight.first_enqueued_tick
                                ),
                            );
                            continue;
                        }

                        let entry = pending_texture_fetches.entry(id.clone()).or_insert(
                            ScheduledTextureFetch {
                                priority,
                                first_enqueued_tick: worker_tick,
                                ready_at_tick: worker_tick,
                                completed_attempts: 0,
                            },
                        );
                        if priority < entry.priority {
                            entry.priority = priority;
                        }
                        emit_relay(
                            &tx,
                            RuntimeRelayLevel::Info,
                            "texture_fetch",
                            &format!(
                                "queued id={} priority={:?} ready_at_tick={} attempts={}",
                                id_str,
                                entry.priority,
                                entry.ready_at_tick,
                                entry.completed_attempts
                            ),
                        );
                    }
                    LiveFeedCommand::RequestMesh { id, lod } => {
                        let id = id.trim().to_ascii_lowercase();
                        if id.is_empty() {
                            continue;
                        }

                        let Some(caps) = capabilities.as_ref() else {
                            let _ = tx.send(LiveFeedUpdate::MeshAssetFailed {
                                id: id.clone(),
                                lod,
                                reason: viewer_asset::AssetFetchFailureReason::MissingCapability,
                            });
                            emit_relay(
                                &tx,
                                RuntimeRelayLevel::Warn,
                                "mesh_fetch",
                                &format!(
                                    "failed id={} lod={} reason=MissingCapability detail=active session capabilities unavailable for mesh request",
                                    id, lod
                                ),
                            );
                            continue;
                        };

                        let mesh_candidates =
                            viewer_grid::AssetCapabilityPolicy::mesh_request_candidates(
                                &caps.entries,
                                &id,
                            );
                        let urls = mesh_candidates
                            .iter()
                            .map(|candidate| candidate.url.clone())
                            .collect::<Vec<_>>();

                        if urls.is_empty() {
                            let _ = tx.send(LiveFeedUpdate::MeshAssetFailed {
                                id: id.clone(),
                                lod,
                                reason: viewer_asset::AssetFetchFailureReason::MissingCapability,
                            });
                            emit_relay(
                                &tx,
                                RuntimeRelayLevel::Warn,
                                "mesh_fetch",
                                &format!(
                                    "failed id={} lod={} reason=MissingCapability detail=GetMesh/GetMesh2/ViewerAsset candidates missing",
                                    id, lod
                                ),
                            );
                            continue;
                        }

                        emit_relay(
                            &tx,
                            RuntimeRelayLevel::Info,
                            "mesh_fetch",
                            &format!("queued id={} lod={} candidates={}", id, lod, urls.len()),
                        );

                        let fetch_timeout =
                            std::time::Duration::from_millis(config.asset_live_timeout_ms);
                        let (result, attempts) =
                            viewer_net::fetch_mesh_asset_bytes_with_attempts(&urls, fetch_timeout)
                                .await;
                        for attempt in &attempts {
                            let candidate = mesh_candidates
                                .iter()
                                .find(|candidate| candidate.url == attempt.url);
                            emit_relay(
                                &tx,
                                RuntimeRelayLevel::Info,
                                "mesh_fetch",
                                &format_mesh_fetch_attempt_line(&id, lod, candidate, attempt),
                            );
                        }
                        match result {
                            Ok(bytes) => {
                                let _ = tx.send(LiveFeedUpdate::MeshAsset {
                                    id: id.clone(),
                                    lod,
                                    bytes,
                                });
                                emit_relay(
                                    &tx,
                                    RuntimeRelayLevel::Info,
                                    "mesh_fetch",
                                    &format!("ready id={} lod={} attempt=1", id, lod),
                                );
                            }
                            Err(err) => {
                                let reason = classify_asset_fetch_failure_reason(&err);
                                let detail = if let Some(bucket) = classify_mesh_403_bucket(&err) {
                                    format!("{err}; bucket={bucket}")
                                } else {
                                    err.to_string()
                                };
                                let _ = tx.send(LiveFeedUpdate::MeshAssetFailed {
                                    id: id.clone(),
                                    lod,
                                    reason,
                                });
                                emit_relay(
                                    &tx,
                                    RuntimeRelayLevel::Warn,
                                    "mesh_fetch",
                                    &format!(
                                        "failed id={} lod={} reason={:?} detail={}",
                                        id, lod, reason, detail
                                    ),
                                );
                            }
                        }
                    }
                    LiveFeedCommand::ExecuteContinuityProbe { queued_at_unix_ms } => {
                        let started_at = now_unix_ms();
                        emit_relay(
                            &tx,
                            RuntimeRelayLevel::Info,
                            "probe_execution",
                            &format!(
                                "continuity probe queued_ms={}",
                                started_at.saturating_sub(queued_at_unix_ms)
                            ),
                        );
                        let result = connection.execute_continuity_probe().await;
                        let code = match result {
                            Ok(()) => viewer_core::ProbeResultCode::Success,
                            Err(err) => {
                                let is_timeout = err.to_string().contains("timeout")
                                    || match &err {
                                        viewer_net::ConnectionError::Http(e) => e.is_timeout(),
                                        _ => false,
                                    };
                                let status = match &err {
                                    viewer_net::ConnectionError::HttpStatus { status, .. } => {
                                        Some(status.as_u16())
                                    }
                                    viewer_net::ConnectionError::Http(e) => {
                                        e.status().map(|s| s.as_u16())
                                    }
                                    _ => None,
                                };
                                viewer_grid::continuity::classify_probe_outcome(is_timeout, status)
                            }
                        };
                        let _ = tx.send(LiveFeedUpdate::ContinuityProbeResult(code));
                        emit_relay(
                            &tx,
                            RuntimeRelayLevel::Info,
                            "probe_execution",
                            &format!(
                                "continuity probe complete result={:?} delay_ms={}",
                                code,
                                now_unix_ms().saturating_sub(started_at)
                            ),
                        );
                    }
                    LiveFeedCommand::TeleportViaSlurl {
                        slurl,
                        queued_at_unix_ms,
                    } => {
                        let queued_ms = now_unix_ms().saturating_sub(queued_at_unix_ms);
                        apply_reconnect_teleport_request(
                            &tx,
                            &mut active_start_location,
                            &mut reconnect_reason,
                            &mut should_reconnect,
                            &slurl,
                            &format!("manual queued_ms={queued_ms}"),
                        );
                    }
                }
            }

            if let Some(circuit) = &social_circuit
                && config.require_region_handshake_reply
                && !connection.has_observed_region_handshake()
                && region_handshake_reprime_attempts < REGION_HANDSHAKE_REPRIME_MAX_ATTEMPTS
            {
                let can_reprime = match last_region_handshake_reprime_tick {
                    Some(last_tick) => {
                        worker_tick.saturating_sub(last_tick)
                            >= REGION_HANDSHAKE_REPRIME_INTERVAL_TICKS
                    }
                    None => worker_tick > 0,
                };
                if can_reprime {
                    region_handshake_reprime_attempts =
                        region_handshake_reprime_attempts.saturating_add(1);
                    last_region_handshake_reprime_tick = Some(worker_tick);
                    emit_relay(
                        &tx,
                        RuntimeRelayLevel::Info,
                        "social",
                        &format!(
                            "region_handshake_reprime: attempt={}/{} worker_tick={}",
                            region_handshake_reprime_attempts,
                            REGION_HANDSHAKE_REPRIME_MAX_ATTEMPTS,
                            worker_tick
                        ),
                    );
                    match connection.send_handshake_reprime_bundle(circuit).await {
                        Ok(datagrams_sent) => emit_relay(
                            &tx,
                            RuntimeRelayLevel::Info,
                            "social",
                            &format!(
                                "region_handshake_reprime: handshake_bundle_datagrams={datagrams_sent}"
                            ),
                        ),
                        Err(err) => emit_relay(
                            &tx,
                            RuntimeRelayLevel::Warn,
                            "social",
                            &format!(
                                "region_handshake_reprime handshake bundle failed attempt={} error={err}",
                                region_handshake_reprime_attempts
                            ),
                        ),
                    }
                    match connection.send_startup_interest_messages(circuit).await {
                        Ok(()) => {
                            last_agent_update_tick = Some(worker_tick);
                        }
                        Err(err) => emit_relay(
                            &tx,
                            RuntimeRelayLevel::Warn,
                            "social",
                            &format!(
                                "region_handshake_reprime failed attempt={} error={}",
                                region_handshake_reprime_attempts, err
                            ),
                        ),
                    }
                }
            }

            if let Some(circuit) = &social_circuit
                && should_send_agent_update_keepalive(
                    worker_tick,
                    last_agent_update_tick,
                    agent_update_keepalive_interval_ticks,
                )
            {
                let camera_center = connection.current_agent_update_camera_center();
                let reliable = false;
                emit_relay(
                    &tx,
                    RuntimeRelayLevel::Info,
                    "social",
                    &format!(
                        "agent_update_keepalive: period_ticks={} camera_center={:.2},{:.2},{:.2} far={:.2} control_flags={} reliable={reliable}",
                        agent_update_keepalive_interval_ticks,
                        camera_center[0],
                        camera_center[1],
                        camera_center[2],
                        config.agent_update_far,
                        AGENT_UPDATE_CONTROL_FLAGS,
                    ),
                );
                match connection
                    .send_agent_update_custom_on_circuit(
                        circuit,
                        reliable,
                        config.agent_update_far,
                        AGENT_UPDATE_CONTROL_FLAGS,
                    )
                    .await
                {
                    Ok(()) => {
                        last_agent_update_tick = Some(worker_tick);
                    }
                    Err(err) => {
                        emit_relay(
                            &tx,
                            RuntimeRelayLevel::Warn,
                            "social",
                            &format!("agent update keepalive failed: {err}"),
                        );
                        should_reconnect = true;
                    }
                }
            }

            if let Some(circuit) = &social_circuit {
                match connection
                    .poll_social_events(
                        circuit,
                        std::time::Duration::from_millis(config.social_poll_timeout_ms),
                        config.social_poll_max_packets,
                    )
                    .await
                {
                    Ok(events) => {
                        dispatch_social_events(&tx, &local_agent_id, events);
                    }
                    Err(err) => {
                        emit_relay(
                            &tx,
                            RuntimeRelayLevel::Warn,
                            "social",
                            &format!("social poll failed: {err}"),
                        );
                        social_circuit = connection
                            .open_social_circuit(&config.receive_bind)
                            .await
                            .ok();
                        followed_enable_simulator_ports.clear();
                        followed_seed_capability_urls.clear();
                        emit_first_simulator_socket_summary(
                            &tx,
                            &connection,
                            "after_social_circuit_reopen",
                        );
                        if let Some(circuit) = social_circuit.as_ref() {
                            if let Err(prime_err) =
                                connection.send_startup_interest_messages(circuit).await
                            {
                                emit_relay(
                                    &tx,
                                    RuntimeRelayLevel::Warn,
                                    "social",
                                    &format!("social circuit re-prime failed: {prime_err}"),
                                );
                            } else {
                                last_agent_update_tick = Some(worker_tick);
                            }
                            emit_first_simulator_socket_summary(
                                &tx,
                                &connection,
                                "after_reopen_social_prime",
                            );
                        }
                        if social_circuit.is_none() {
                            emit_relay(
                                &tx,
                                RuntimeRelayLevel::Warn,
                                "social",
                                "social circuit reopen failed; reconnecting",
                            );
                            should_reconnect = true;
                        }
                    }
                }
            }

            if let Some(circuit) = &social_circuit
                && let Ok(received) = connection
                    .poll_nearby_chat_on_circuit(
                        circuit,
                        std::time::Duration::from_millis(config.nearby_poll_timeout_ms),
                        config.nearby_poll_max_packets,
                    )
                    .await
            {
                for chat in received {
                    let _ = tx.send(LiveFeedUpdate::ChatMessage(ChatMessage {
                        id: now_unix_ms(),
                        observed_at_unix_ms: now_unix_ms(),
                        sender: chat.sender,
                        text: chat.text,
                        source: chat.source,
                    }));
                }
            }

            if let Some(url) = event_queue_url.as_deref() {
                if event_queue_poll_task.is_none() {
                    let url = url.to_string();
                    let ack = event_ack;
                    mark_capability_invocation_started(&mut capability_readiness, "EventQueueGet");
                    push_protocol_event(
                        &mut protocol_events,
                        format!(
                            "EventQueueGet:start ack={} {}",
                            ack,
                            format_classified_url(&url)
                        ),
                    );
                    let timeout =
                        std::time::Duration::from_millis(config.event_queue_poll_timeout_ms);
                    event_queue_poll_task = Some(spawn_event_queue_poll_task(url, ack, timeout));
                }
                let task_finished = event_queue_poll_task
                    .as_ref()
                    .map(|task| task.is_finished())
                    .unwrap_or(false);
                if task_finished && let Some(task) = event_queue_poll_task.take() {
                    match task.await {
                        Ok(Ok(poll)) => {
                            event_queue_consecutive_failures = 0;
                            event_queue_cap_not_found_failures = 0;
                            if !event_queue_has_observed_ok {
                                event_queue_has_observed_ok = true;
                                if config.capability_probes_require_event_queue_ok
                                    && !capability_probe_gate_open
                                {
                                    capability_probe_gate_open = true;
                                    push_protocol_event(
                                        &mut protocol_events,
                                        String::from("probe_gate:open reason=EventQueueGet:ok"),
                                    );
                                    emit_relay(
                                        &tx,
                                        RuntimeRelayLevel::Info,
                                        "parallel_protocol",
                                        "capability probe gate opened after EventQueueGet:ok",
                                    );
                                }
                            }
                            mark_capability_invocation_result(
                                &mut capability_readiness,
                                "EventQueueGet",
                                Ok("ok"),
                            );
                            let previous_ack = event_ack;
                            if let Some(next_ack) = poll.id {
                                event_ack = next_ack;
                            }
                            push_protocol_event(
                                &mut protocol_events,
                                format!(
                                    "EventQueueGet:ok ack_in={} ack_out={} events={} names={}",
                                    previous_ack,
                                    event_ack,
                                    poll.events.len(),
                                    summarize_event_queue_message_names(&poll, 6),
                                ),
                            );
                            let simulator_targets =
                                connection.extract_event_queue_simulator_targets(&poll);
                            let enable_simulator_count = simulator_targets
                                .iter()
                                .filter(|target| target.message == "EnableSimulator")
                                .count();
                            event_queue_simulator_target_messages =
                                event_queue_simulator_target_messages
                                    .saturating_add(simulator_targets.len());
                            event_queue_enable_simulator_messages =
                                event_queue_enable_simulator_messages
                                    .saturating_add(enable_simulator_count);
                            if enable_simulator_count > 0 {
                                push_protocol_event(
                                    &mut protocol_events,
                                    format!(
                                        "EventQueueGet:EnableSimulator count={} total={}",
                                        enable_simulator_count,
                                        event_queue_enable_simulator_messages
                                    ),
                                );
                            }
                            if !poll.events.is_empty() {
                                emit_relay(
                                    &tx,
                                    RuntimeRelayLevel::Info,
                                    "event_queue",
                                    &format!(
                                        "event queue response ack_in={} ack_out={} events={} names={}",
                                        previous_ack,
                                        event_ack,
                                        poll.events.len(),
                                        summarize_event_queue_message_names(&poll, 8),
                                    ),
                                );
                                emit_event_queue_interesting_details(&tx, &connection, &poll);
                                follow_enable_simulator_ports(
                                    &tx,
                                    &mut connection,
                                    social_circuit.as_ref(),
                                    &poll,
                                    &mut followed_enable_simulator_ports,
                                )
                                .await;
                                follow_region_seed_capabilities(
                                    &tx,
                                    &mut connection,
                                    &poll,
                                    &mut followed_seed_capability_urls,
                                    &mut recent_seed_capability_urls,
                                )
                                .await;
                            }
                            for message in connection.extract_nearby_chat_messages(&poll) {
                                let _ = tx.send(LiveFeedUpdate::ChatMessage(ChatMessage {
                                    id: now_unix_ms(),
                                    observed_at_unix_ms: now_unix_ms(),
                                    sender: message.sender,
                                    text: message.text,
                                    source: message.source,
                                }));
                            }
                        }
                        Ok(Err(err)) => {
                            event_queue_consecutive_failures =
                                event_queue_consecutive_failures.saturating_add(1);
                            let err_text = err.to_string();
                            mark_capability_invocation_result(
                                &mut capability_readiness,
                                "EventQueueGet",
                                Err(&err_text),
                            );
                            push_protocol_event(
                                &mut protocol_events,
                                format!(
                                    "EventQueueGet:err count={} {}",
                                    event_queue_consecutive_failures, err_text
                                ),
                            );
                            if is_event_queue_cap_not_found_error(&err_text) {
                                event_queue_cap_not_found_failures =
                                    event_queue_cap_not_found_failures.saturating_add(1);
                            } else {
                                event_queue_cap_not_found_failures = 0;
                            }
                            let should_log = event_queue_consecutive_failures == 1
                                || event_queue_consecutive_failures.is_multiple_of(10);
                            if should_log {
                                emit_relay(
                                    &tx,
                                    RuntimeRelayLevel::Warn,
                                    "event_queue",
                                    &format!(
                                        "event queue polling failed (count={}): {err}",
                                        event_queue_consecutive_failures
                                    ),
                                );
                            }
                            if config.event_queue_cap_not_found_before_reconnect > 0
                                && event_queue_cap_not_found_failures
                                    >= config.event_queue_cap_not_found_before_reconnect
                            {
                                emit_relay(
                                    &tx,
                                    RuntimeRelayLevel::Warn,
                                    "event_queue",
                                    &format!(
                                        "event queue cap-not-found threshold reached ({}); attempting cap re-prime",
                                        config.event_queue_cap_not_found_before_reconnect
                                    ),
                                );
                                push_protocol_event(
                                    &mut protocol_events,
                                    format!(
                                        "EventQueueGet:cap_reprime:start failures={} threshold={}",
                                        event_queue_cap_not_found_failures,
                                        config.event_queue_cap_not_found_before_reconnect
                                    ),
                                );
                                match reprime_event_queue_capabilities_with_fallback(
                                    &mut connection,
                                    &recent_seed_capability_urls,
                                )
                                .await
                                {
                                    Ok((reprime, reprime_source)) => {
                                        if let Some(refreshed_url) = reprime.event_queue_url {
                                            let previous_url = event_queue_url.take();
                                            event_queue_url = Some(refreshed_url.clone());
                                            pending_interest_list_probe_url =
                                                reprime.pending_interest_list_probe_url;
                                            pending_untrusted_simulator_message_probe_url = reprime
                                                .pending_untrusted_simulator_message_probe_url;
                                            capability_inventory_summary =
                                                reprime.capability_inventory_summary;
                                            capability_readiness = reprime.capability_readiness;
                                            event_queue_cap_not_found_failures = 0;
                                            event_queue_consecutive_failures = 0;
                                            event_ack = 0;
                                            let previous_url_text = previous_url
                                                .as_deref()
                                                .map(format_classified_url)
                                                .unwrap_or_else(|| String::from("none"));
                                            let refreshed_url_text =
                                                format_classified_url(&refreshed_url);
                                            push_protocol_event(
                                                &mut protocol_events,
                                                format!(
                                                    "EventQueueGet:cap_reprime:ok source={} ack_reset=0 prev={} next={} non_baseline={}",
                                                    reprime_source,
                                                    previous_url_text,
                                                    refreshed_url_text,
                                                    reprime.non_baseline_caps_by_host
                                                ),
                                            );
                                            emit_relay(
                                                &tx,
                                                RuntimeRelayLevel::Info,
                                                "event_queue",
                                                &format!(
                                                    "event queue cap re-prime succeeded (source={}): prev={} next={}",
                                                    reprime_source,
                                                    previous_url_text,
                                                    refreshed_url_text
                                                ),
                                            );
                                        } else {
                                            push_protocol_event(
                                                &mut protocol_events,
                                                String::from(
                                                    "EventQueueGet:cap_reprime:err missing_event_queue_cap",
                                                ),
                                            );
                                            emit_relay(
                                                &tx,
                                                RuntimeRelayLevel::Warn,
                                                "event_queue",
                                                "event queue cap re-prime returned no EventQueueGet capability; reconnecting",
                                            );
                                            reconnect_reason = Some(String::from(
                                                "event queue cap re-prime missing EventQueueGet capability",
                                            ));
                                            should_reconnect = true;
                                        }
                                    }
                                    Err(err) => {
                                        let fallback_urls = summarize_recent_seed_capability_urls(
                                            &recent_seed_capability_urls,
                                        );
                                        push_protocol_event(
                                            &mut protocol_events,
                                            format!(
                                                "EventQueueGet:cap_reprime:err {err} fallback_urls={fallback_urls}"
                                            ),
                                        );
                                        emit_relay(
                                            &tx,
                                            RuntimeRelayLevel::Warn,
                                            "event_queue",
                                            &format!(
                                                "event queue cap re-prime failed: {err}; fallback_urls={fallback_urls}; reconnecting"
                                            ),
                                        );
                                        reconnect_reason =
                                            Some(format!("event queue cap re-prime failed: {err}"));
                                        should_reconnect = true;
                                    }
                                }
                            }
                            if config.event_queue_failures_before_reconnect > 0
                                && event_queue_consecutive_failures
                                    >= config.event_queue_failures_before_reconnect
                            {
                                emit_relay(
                                    &tx,
                                    RuntimeRelayLevel::Warn,
                                    "event_queue",
                                    &format!(
                                        "event queue failure threshold reached ({}); reconnecting",
                                        config.event_queue_failures_before_reconnect
                                    ),
                                );
                                should_reconnect = true;
                            }
                        }
                        Err(join_err) => {
                            mark_capability_invocation_result(
                                &mut capability_readiness,
                                "EventQueueGet",
                                Err(&join_err.to_string()),
                            );
                            push_protocol_event(
                                &mut protocol_events,
                                format!("EventQueueGet:task_err {join_err}"),
                            );
                            emit_relay(
                                &tx,
                                RuntimeRelayLevel::Warn,
                                "event_queue",
                                &format!("event queue task failed: {join_err}"),
                            );
                        }
                    }
                }
            }

            if capability_probe_gate_open {
                run_pending_capability_readiness_probes(
                    &mut connection,
                    &mut capability_readiness,
                    &mut protocol_events,
                    &mut pending_interest_list_probe_url,
                    &mut pending_untrusted_simulator_message_probe_url,
                )
                .await;
                run_lane_probe_shape_tasks_once(
                    &mut connection,
                    &tx,
                    &mut protocol_events,
                    &mut pending_gated_lane_probe_shapes,
                )
                .await;
            }

            if should_reconnect {
                let _ = tx.send(LiveFeedUpdate::ChatConnection(
                    ChatConnectionState::Reconnecting,
                ));
                if let Some(detail) = reconnect_reason.as_deref() {
                    let _ = tx.send(LiveFeedUpdate::Status(LiveStartupStatus::Starting));
                    emit_relay(&tx, RuntimeRelayLevel::Info, "reconnect", detail);
                } else {
                    let failure = LiveStartupFailure {
                        class: LiveStartupFailureClass::ConnectionLostReconnecting,
                        message: String::from("connection lost; reconnecting"),
                    };
                    let _ = tx.send(LiveFeedUpdate::Status(LiveStartupStatus::Failed(failure)));
                    emit_relay(
                        &tx,
                        RuntimeRelayLevel::Warn,
                        "reconnect",
                        "connection lost; reconnecting",
                    );
                }
                break;
            }
            update_live_visual_from_connection(&mut snapshot, &connection, &config);
            snapshot.source = String::from("viewer_app_in_process:ready");
            if worker_tick.is_multiple_of(40) {
                emit_object_feed_tick_summary(&tx, &snapshot, &connection);
            }
            let _ = tx.send(LiveFeedUpdate::Snapshot(snapshot.clone()));
            let avatar_samples = extract_worker_world_avatar_samples(&connection, &local_agent_id);
            let _ = tx.send(LiveFeedUpdate::WorldAvatars {
                avatars: avatar_samples.clone(),
                decoded_sim_name: extract_worker_world_sim_name(&connection),
                startup_sim_name: startup_sim_name.clone(),
                self_location: extract_worker_self_location(&connection),
                observed_at_unix_ms: now_unix_ms(),
            });
            if worker_tick.is_multiple_of(40) {
                let unresolved_ids: Vec<String> = avatar_samples
                    .iter()
                    .filter_map(|sample| sample.agent_id.clone())
                    .filter(|id| id != &local_agent_id && !known_avatar_name_ids.contains(id))
                    .collect();
                if !unresolved_ids.is_empty() {
                    let mut resolved_any = false;
                    if let Some(url) = display_names_url.as_deref() {
                        push_protocol_event(
                            &mut protocol_events,
                            format!(
                                "GetDisplayNames:start ids={} {}",
                                unresolved_ids.len(),
                                format_classified_url(url)
                            ),
                        );
                        match connection
                            .resolve_avatar_display_names(url, &unresolved_ids)
                            .await
                        {
                            Ok(names) => {
                                if !names.is_empty() {
                                    resolved_any = true;
                                }
                                push_protocol_event(
                                    &mut protocol_events,
                                    format!(
                                        "GetDisplayNames:ok resolved={} {}",
                                        names.len(),
                                        format_classified_url(url)
                                    ),
                                );
                                for entry in names {
                                    known_avatar_name_ids.insert(entry.id.clone());
                                    if bootstrap_friend_id_set.contains(&entry.id) {
                                        let _ = tx.send(LiveFeedUpdate::FriendResolvedName {
                                            id: entry.id,
                                            display_name: entry.display_name,
                                            source: String::from("caps.GetDisplayNames"),
                                        });
                                    } else {
                                        let _ = tx.send(LiveFeedUpdate::AvatarResolvedName {
                                            id: entry.id,
                                            display_name: entry.display_name,
                                            source: String::from("caps.GetDisplayNames"),
                                        });
                                    }
                                }
                            }
                            Err(err) => {
                                push_protocol_event(
                                    &mut protocol_events,
                                    format!(
                                        "GetDisplayNames:err {err} {}",
                                        format_classified_url(url)
                                    ),
                                );
                                emit_relay(
                                    &tx,
                                    RuntimeRelayLevel::Warn,
                                    "avatar_name",
                                    &format!("display name lookup failed: {err}"),
                                );
                            }
                        }
                    }
                    if !resolved_any && let Some(profile_url) = agent_profile_url.as_deref() {
                        push_protocol_event(
                            &mut protocol_events,
                            format!(
                                "AgentProfile:start ids={} {}",
                                unresolved_ids.len(),
                                format_classified_url(profile_url)
                            ),
                        );
                        let mut profile_resolved = 0usize;
                        for id in &unresolved_ids {
                            if let Ok(profile) =
                                connection.fetch_agent_profile(profile_url, id).await
                                && let Some(display_name) = pick_best_avatar_name(&profile)
                            {
                                profile_resolved = profile_resolved.saturating_add(1);
                                known_avatar_name_ids.insert(id.clone());
                                if bootstrap_friend_id_set.contains(id) {
                                    let _ = tx.send(LiveFeedUpdate::FriendResolvedName {
                                        id: id.clone(),
                                        display_name,
                                        source: String::from("caps.AgentProfile"),
                                    });
                                } else {
                                    let _ = tx.send(LiveFeedUpdate::AvatarResolvedName {
                                        id: id.clone(),
                                        display_name,
                                        source: String::from("caps.AgentProfile"),
                                    });
                                }
                            }
                        }
                        push_protocol_event(
                            &mut protocol_events,
                            format!(
                                "AgentProfile:ok resolved={} {}",
                                profile_resolved,
                                format_classified_url(profile_url)
                            ),
                        );
                        if profile_resolved > 0 {
                            emit_relay(
                                &tx,
                                RuntimeRelayLevel::Info,
                                "avatar_name",
                                &format!(
                                    "resolved {} avatar names (fallback AgentProfile)",
                                    profile_resolved
                                ),
                            );
                        }
                    }
                }
            }
            if !first_sim_socket_steady_state_summary_emitted && social_circuit.is_some() {
                if let Some(circuit) = social_circuit.as_ref() {
                    let _ = flush_pending_first_sim_ack_ids(
                        &mut connection,
                        circuit,
                        &tx,
                        "after_first_steady_state_window",
                    )
                    .await;
                }
                emit_first_simulator_socket_summary(
                    &tx,
                    &connection,
                    "after_first_steady_state_window",
                );
                emit_first_simulator_forensics_summary(
                    &tx,
                    &connection,
                    "after_first_steady_state_window",
                );
                emit_parallel_protocol_summary(
                    &tx,
                    &connection,
                    "after_first_steady_state_window",
                    &capability_inventory_summary,
                    &summarize_capability_readiness(&capability_readiness),
                    &protocol_events,
                    event_queue_url.as_deref(),
                    event_ack,
                    event_queue_consecutive_failures,
                    event_queue_cap_not_found_failures,
                    event_queue_has_observed_ok,
                    event_queue_simulator_target_messages,
                    event_queue_enable_simulator_messages,
                );
                first_sim_socket_steady_state_summary_emitted = true;
            }
            worker_tick = worker_tick.saturating_add(1);
            tokio::time::sleep(std::time::Duration::from_millis(config.worker_tick_ms)).await;
        }

        reconnect_attempt = reconnect_attempt.saturating_add(1);
        tokio::time::sleep(reconnect_backoff_duration(reconnect_attempt)).await;
    }
}

fn offline_snapshot() -> LiveVisualSnapshot {
    LiveVisualSnapshot {
        source: String::from("viewer_app_in_process:start"),
        logged_in: false,
        current_region_name: None,
        first_sim_endpoint: None,
        first_sim_region_x: None,
        first_sim_region_y: None,
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
        decoded_coarse_second_x: None,
        decoded_coarse_second_y: None,
        decoded_coarse_second_z: None,
        decoded_coarse_third_x: None,
        decoded_coarse_third_y: None,
        decoded_coarse_third_z: None,
        decoded_health_updates: 0,
        decoded_health_last_basis_points: None,
        decoded_viewer_time_updates: 0,
        decoded_viewer_time_body_len: None,
        decoded_viewer_time_signature: None,
        decoded_object_feed_update_messages: 0,
        decoded_object_feed_kill_messages: 0,
        decoded_object_feed_decode_dropped: 0,
        decoded_object_feed_evicted: 0,
        decoded_object_feed_total_objects: 0,
        decoded_object_feed_export_truncated: false,
        decoded_object_feed_objects: Vec::new(),
        decoded_object_feed_recent_kills: Vec::new(),
        continuity: viewer_core::RegionContinuitySummary::default(),
        observed_at_unix_ms: now_unix_ms(),
    }
}

fn reconnect_backoff_duration(attempt: u32) -> std::time::Duration {
    let capped = attempt.min(6);
    let base_ms = 500u64.saturating_mul(1u64 << capped);
    let jitter_ms = u64::from(now_unix_ms() as u32 % 350);
    std::time::Duration::from_millis((base_ms + jitter_ms).min(15_000))
}

async fn send_nearby_chat_with_caps(
    connection: &mut Connection,
    circuit: Option<&SocialCircuit>,
    text: &str,
    bind: &str,
    receive_timeout: std::time::Duration,
    receive_max_packets: usize,
) -> std::result::Result<Vec<NearbyChatMessage>, ConnectionError> {
    if let Some(circuit) = circuit {
        connection
            .send_nearby_chat_on_circuit(circuit, text, receive_timeout, receive_max_packets)
            .await
    } else {
        connection
            .send_nearby_chat(text, bind, receive_timeout, receive_max_packets)
            .await
    }
}

async fn fetch_profile_tab_data(
    connection: &mut Connection,
    agent_profile_url: Option<&str>,
    avatar_id: &str,
    receive_bind: &str,
    tab: AvatarProfileTab,
) -> std::result::Result<AgentProfileData, ConnectionError> {
    let mut profile = AgentProfileData {
        id: avatar_id.to_string(),
        ..AgentProfileData::default()
    };
    let mut any_success = false;
    let mut first_error: Option<ConnectionError> = None;

    if let Some(cap_url) = agent_profile_url {
        match connection.fetch_agent_profile(cap_url, avatar_id).await {
            Ok(mut cap_profile) => {
                if cap_profile
                    .profile_url
                    .as_deref()
                    .unwrap_or_default()
                    .is_empty()
                {
                    cap_profile.profile_url =
                        connection.derive_profile_feed_url(&cap_profile, Some(cap_url));
                }
                merge_profile_payload(&mut profile, cap_profile);
                any_success = true;
            }
            Err(err) => {
                first_error = Some(err);
            }
        }
    }

    if let Ok(legacy_circuit) = connection.open_social_circuit(receive_bind).await {
        match connection
            .fetch_agent_profile_legacy(
                &legacy_circuit,
                avatar_id,
                std::time::Duration::from_millis(150),
                256,
            )
            .await
        {
            Ok(legacy_profile) => {
                merge_profile_payload(&mut profile, legacy_profile);
                any_success = true;
            }
            Err(err) => {
                if first_error.is_none() {
                    first_error = Some(err);
                }
            }
        }
    } else if !any_success && first_error.is_none() {
        first_error = Some(ConnectionError::FirstSimulatorHandshakeNotInitialized);
    }
    if profile
        .profile_url
        .as_deref()
        .unwrap_or_default()
        .is_empty()
    {
        profile.profile_url = Some(format!("legacy://profile/{avatar_id}"));
    }
    if any_success {
        Ok(profile)
    } else if let Some(err) = first_error {
        Err(err)
    } else {
        let _ = tab;
        Err(ConnectionError::CapabilityDecode(String::from(
            "profile fetch failed",
        )))
    }
}

fn merge_profile_payload(target: &mut AgentProfileData, incoming: AgentProfileData) {
    if target.id.is_empty() {
        target.id = incoming.id;
    }
    if target.profile_url.is_none() {
        target.profile_url = incoming.profile_url;
    }
    if target.sl_about_text.is_empty() {
        target.sl_about_text = incoming.sl_about_text;
    }
    if target.fl_about_text.is_empty() {
        target.fl_about_text = incoming.fl_about_text;
    }
    if target.notes.is_empty() {
        target.notes = incoming.notes;
    }
    if target.sl_image_id.is_none() {
        target.sl_image_id = incoming.sl_image_id;
    }
    if target.fl_image_id.is_none() {
        target.fl_image_id = incoming.fl_image_id;
    }
    if target.partner_id.is_none() {
        target.partner_id = incoming.partner_id;
    }
    if target.member_since.is_none() {
        target.member_since = incoming.member_since;
    }
    if target.online.is_none() {
        target.online = incoming.online;
    }
    if target.allow_publish.is_none() {
        target.allow_publish = incoming.allow_publish;
    }
    if target.identified.is_none() {
        target.identified = incoming.identified;
    }
    if target.transacted.is_none() {
        target.transacted = incoming.transacted;
    }
    if target.display_name.is_none() {
        target.display_name = incoming.display_name;
    }
    if target.username.is_none() {
        target.username = incoming.username;
    }

    for group in incoming.groups {
        if target.groups.iter().all(|current| current.id != group.id) {
            target.groups.push(group);
        }
    }
    for pick in incoming.picks {
        if target.picks.iter().all(|current| current.id != pick.id) {
            target.picks.push(pick);
        }
    }
    for details in incoming.pick_details {
        if let Some(existing) = target.pick_details.iter_mut().find(|d| d.id == details.id) {
            *existing = details;
        } else {
            target.pick_details.push(details);
        }
    }
    for classified in incoming.classifieds {
        if target
            .classifieds
            .iter()
            .all(|current| current.id != classified.id)
        {
            target.classifieds.push(classified);
        }
    }
    for details in incoming.classified_details {
        if let Some(existing) = target
            .classified_details
            .iter_mut()
            .find(|d| d.id == details.id)
        {
            *existing = details;
        } else {
            target.classified_details.push(details);
        }
    }
}

fn build_live_visual_snapshot_from_result(result: &GridLoginResult) -> LiveVisualSnapshot {
    let mut snapshot = LiveVisualSnapshot {
        source: String::from("viewer_app_in_process:login"),
        logged_in: false,
        current_region_name: None,
        first_sim_endpoint: None,
        first_sim_region_x: None,
        first_sim_region_y: None,
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
        decoded_coarse_second_x: None,
        decoded_coarse_second_y: None,
        decoded_coarse_second_z: None,
        decoded_coarse_third_x: None,
        decoded_coarse_third_y: None,
        decoded_coarse_third_z: None,
        decoded_health_updates: 0,
        decoded_health_last_basis_points: None,
        decoded_viewer_time_updates: 0,
        decoded_viewer_time_body_len: None,
        decoded_viewer_time_signature: None,
        decoded_object_feed_update_messages: 0,
        decoded_object_feed_kill_messages: 0,
        decoded_object_feed_decode_dropped: 0,
        decoded_object_feed_evicted: 0,
        decoded_object_feed_total_objects: 0,
        decoded_object_feed_export_truncated: false,
        decoded_object_feed_objects: Vec::new(),
        decoded_object_feed_recent_kills: Vec::new(),
        continuity: viewer_core::RegionContinuitySummary::default(),
        observed_at_unix_ms: now_unix_ms(),
    };

    if let GridLoginResult::Success(bootstrap) = result {
        snapshot.logged_in = true;
        snapshot.current_region_name = bootstrap
            .start_location
            .as_deref()
            .and_then(parse_region_name_from_start_location);
        snapshot.first_sim_endpoint = Some(format!(
            "{}:{}",
            bootstrap.first_sim.sim_ip, bootstrap.first_sim.sim_port
        ));
        snapshot.first_sim_region_x = Some(bootstrap.first_sim.region_x);
        snapshot.first_sim_region_y = Some(bootstrap.first_sim.region_y);
    }

    snapshot
}

fn update_live_visual_from_connection(
    snapshot: &mut LiveVisualSnapshot,
    connection: &Connection,
    config: &InProcessLiveFeedConfig,
) {
    snapshot.observed_at_unix_ms = now_unix_ms();
    if let Some(decoded_name) = extract_worker_world_sim_name(connection) {
        snapshot.current_region_name = Some(decoded_name);
    }
    snapshot.handshake_agent_movement_complete = connection
        .first_simulator_handshake_state()
        .map(|state| state.stage == viewer_net::FirstSimulatorHandshakeStage::AgentMovementComplete)
        .unwrap_or(false);

    let region = connection.summarize_region_transition_control();
    snapshot.region_transition_control_observations = region.observations as u32;
    snapshot.crossed_region = region.crossed_region as u32;
    snapshot.confirm_enable_simulator = region.confirm_enable_simulator as u32;

    let receive = connection.first_simulator_handshake_receive_diagnostics();
    snapshot.traffic_summary_available = !receive.is_empty();
    snapshot.post_boundary_observations = receive.len() as u32;
    snapshot.likely_broader_traffic = receive
        .iter()
        .filter(|diag| diag.scope == FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic)
        .count() as u32;
    snapshot.unknown = receive
        .iter()
        .filter(|diag| diag.scope == FirstSimulatorInboundTrafficScope::Unknown)
        .count() as u32;

    let decoded = connection.simulator_payload_decode_summary();
    snapshot.decoded_coarse_updates = decoded.coarse_location_updates as u32;
    snapshot.decoded_coarse_location_count = decoded.coarse_location_last_count;
    snapshot.decoded_coarse_first_x = decoded.coarse_location_last_first.map(|xyz| xyz[0]);
    snapshot.decoded_coarse_first_y = decoded.coarse_location_last_first.map(|xyz| xyz[1]);
    snapshot.decoded_coarse_first_z = decoded.coarse_location_last_first.map(|xyz| xyz[2]);
    snapshot.decoded_coarse_second_x = decoded.coarse_location_last_second.map(|xyz| xyz[0]);
    snapshot.decoded_coarse_second_y = decoded.coarse_location_last_second.map(|xyz| xyz[1]);
    snapshot.decoded_coarse_second_z = decoded.coarse_location_last_second.map(|xyz| xyz[2]);
    snapshot.decoded_coarse_third_x = decoded.coarse_location_last_third.map(|xyz| xyz[0]);
    snapshot.decoded_coarse_third_y = decoded.coarse_location_last_third.map(|xyz| xyz[1]);
    snapshot.decoded_coarse_third_z = decoded.coarse_location_last_third.map(|xyz| xyz[2]);
    snapshot.decoded_health_updates = decoded.health_updates as u32;
    snapshot.decoded_health_last_basis_points = decoded.health_last_basis_points;
    snapshot.decoded_viewer_time_updates = decoded.simulator_viewer_time_updates as u32;
    snapshot.decoded_viewer_time_body_len = decoded.simulator_viewer_time_last_body_len;
    snapshot.decoded_viewer_time_signature = decoded.simulator_viewer_time_last_signature;

    snapshot.decoded_object_feed_update_messages = decoded.object_feed_update_messages as u32;
    snapshot.decoded_object_feed_kill_messages = decoded.object_feed_kill_messages as u32;
    snapshot.decoded_object_feed_decode_dropped = decoded.object_feed_decode_dropped as u32;
    snapshot.decoded_object_feed_evicted = decoded.object_feed_evicted as u32;
    let mut decoded_object_feed_objects: Vec<viewer_core::DecodedWorldObjectFeedObject> = decoded
        .object_feed_objects
        .iter()
        .map(|obj| viewer_core::DecodedWorldObjectFeedObject {
            local_id: obj.local_id,
            scale_centi: obj.scale_centi,
            position_centi: obj.position_centi,
            rotation_quat_i16: obj.rotation_quat_i16,
            mesh_id: obj.mesh_id.clone(),
            texture_id: obj
                .texture_id
                .as_deref()
                .map(viewer_core::AssetID::new)
                .filter(|id| !id.is_empty()),
            default_face_material: obj
                .default_face_material
                .as_ref()
                .map(map_net_face_material_to_core),
            face_material_overrides: obj
                .face_material_overrides
                .iter()
                .map(map_net_face_material_to_core)
                .collect(),
            object_id: obj.object_id.clone(),
        })
        .collect();
    let focus_enabled = config.object_uuid_focus.is_some();
    filter_object_feed_objects_by_focus(&mut decoded_object_feed_objects, config);
    snapshot.decoded_object_feed_total_objects = decoded.object_feed_total_objects as u32;
    snapshot.decoded_object_feed_export_truncated = if focus_enabled {
        false
    } else {
        decoded.object_feed_export_truncated
    };
    snapshot.decoded_object_feed_objects = decoded_object_feed_objects;
    snapshot.decoded_object_feed_recent_kills = decoded.object_feed_recent_kills.clone();
    let continuity_summary = connection.continuity_summary();
    snapshot.continuity = map_net_continuity_to_core(&continuity_summary);
}

fn filter_object_feed_objects_by_focus(
    objects: &mut Vec<viewer_core::DecodedWorldObjectFeedObject>,
    config: &InProcessLiveFeedConfig,
) {
    let Some(focus_uuid) = config.object_uuid_focus.as_deref() else {
        return;
    };
    objects.retain(|obj| obj.object_id.as_deref() == Some(focus_uuid));
}

struct EventQueueCapReprime {
    event_queue_url: Option<String>,
    pending_interest_list_probe_url: Option<String>,
    pending_untrusted_simulator_message_probe_url: Option<String>,
    capability_inventory_summary: String,
    capability_readiness: BTreeMap<String, CapabilityReadinessEntry>,
    non_baseline_caps_by_host: String,
}

const MAX_RECENT_SEED_CAPABILITY_URLS: usize = 8;

fn remember_recent_seed_capability_url(recent_seed_urls: &mut VecDeque<String>, seed_url: &str) {
    if seed_url.trim().is_empty() {
        return;
    }
    if let Some(existing_idx) = recent_seed_urls.iter().position(|url| url == seed_url) {
        recent_seed_urls.remove(existing_idx);
    }
    recent_seed_urls.push_front(seed_url.to_string());
    while recent_seed_urls.len() > MAX_RECENT_SEED_CAPABILITY_URLS {
        recent_seed_urls.pop_back();
    }
}

fn summarize_recent_seed_capability_urls(recent_seed_urls: &VecDeque<String>) -> String {
    if recent_seed_urls.is_empty() {
        return String::from("none");
    }
    recent_seed_urls
        .iter()
        .map(|url| format_classified_url(url))
        .collect::<Vec<_>>()
        .join(",")
}

fn map_capability_reprime_from_seed_caps(
    caps: &viewer_net::SeedCapabilityMap,
    inventory_entries: &[viewer_net::SeedCapabilityInventoryEntry],
) -> EventQueueCapReprime {
    EventQueueCapReprime {
        event_queue_url: caps.entries.get("EventQueueGet").cloned(),
        pending_interest_list_probe_url: caps.entries.get("InterestList").cloned(),
        pending_untrusted_simulator_message_probe_url: caps
            .entries
            .get("UntrustedSimulatorMessage")
            .cloned(),
        capability_inventory_summary: summarize_seed_capability_inventory(inventory_entries),
        capability_readiness: init_capability_readiness(Some(caps)),
        non_baseline_caps_by_host: summarize_non_baseline_caps_by_host(&caps.entries, 16),
    }
}

async fn reprime_event_queue_capabilities(
    connection: &mut Connection,
) -> Result<EventQueueCapReprime, String> {
    let caps = connection
        .fetch_seed_capabilities()
        .await
        .map_err(|err| err.to_string())?;
    let inventory = connection.summarize_seed_capability_inventory(&caps);
    Ok(map_capability_reprime_from_seed_caps(&caps, &inventory))
}

async fn reprime_event_queue_capabilities_with_fallback(
    connection: &mut Connection,
    recent_seed_urls: &VecDeque<String>,
) -> Result<(EventQueueCapReprime, String), String> {
    match reprime_event_queue_capabilities(connection).await {
        Ok(reprime) => return Ok((reprime, String::from("session_seed"))),
        Err(session_seed_err) => {
            let mut tried = BTreeSet::<String>::new();
            let mut fallback_errors = Vec::new();
            for seed_url in recent_seed_urls {
                if !tried.insert(seed_url.clone()) {
                    continue;
                }
                match connection.fetch_seed_capabilities_from_url(seed_url).await {
                    Ok(caps) => {
                        let _ = connection.set_session_seed_capability_url(seed_url);
                        let inventory = connection.summarize_seed_capability_inventory(&caps);
                        let reprime = map_capability_reprime_from_seed_caps(&caps, &inventory);
                        return Ok((
                            reprime,
                            format!("recent_seed:{}", format_classified_url(seed_url)),
                        ));
                    }
                    Err(err) => fallback_errors
                        .push(format!("{} => {err}", format_classified_url(seed_url))),
                }
            }

            if fallback_errors.is_empty() {
                return Err(format!(
                    "session seed refresh failed: {session_seed_err}; no recent seed-capability URLs available"
                ));
            }
            Err(format!(
                "session seed refresh failed: {session_seed_err}; fallback attempts exhausted: {}",
                fallback_errors.join(" | ")
            ))
        }
    }
}

fn spawn_event_queue_poll_task(
    url: String,
    ack: u64,
    timeout: std::time::Duration,
) -> tokio::task::JoinHandle<Result<viewer_net::EventQueuePollResult, viewer_net::ConnectionError>>
{
    tokio::spawn(async move { poll_event_queue_url_once(&url, ack, timeout).await })
}

fn is_event_queue_cap_not_found_error(error_text: &str) -> bool {
    let text = error_text.to_ascii_lowercase();
    text.contains("404")
        && (text.contains("cap not found")
            || text.contains("not found")
            || text.contains("http status 404"))
}

fn summarize_event_queue_message_names(
    poll: &viewer_net::EventQueuePollResult,
    limit: usize,
) -> String {
    if poll.events.is_empty() {
        return String::from("none");
    }
    let mut counts = BTreeMap::<String, usize>::new();
    for event in &poll.events {
        *counts.entry(event.message.clone()).or_default() += 1;
    }
    counts
        .into_iter()
        .take(limit)
        .map(|(name, count)| format!("{name}:{count}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn emit_event_queue_interesting_details(
    tx: &mpsc::Sender<LiveFeedUpdate>,
    connection: &Connection,
    poll: &viewer_net::EventQueuePollResult,
) {
    let simulator_targets = connection.extract_event_queue_simulator_targets(poll);
    let simulator_events: Vec<&viewer_net::EventQueueMessage> = poll
        .events
        .iter()
        .filter(|event| {
            event.message == "EnableSimulator" || event.message == "EstablishAgentCommunication"
        })
        .collect();
    for (target, raw_event) in simulator_targets
        .into_iter()
        .zip(simulator_events.into_iter())
        .take(4)
    {
        let mut parts = Vec::new();
        if let Some(handle) = target.handle.as_deref() {
            parts.push(format!("handle={handle}"));
        }
        if let Some(endpoint_ip) = target.endpoint_ip.as_deref() {
            parts.push(format!("endpoint_ip={endpoint_ip}"));
        }
        if let Some(endpoint_port) = target.endpoint_port {
            parts.push(format!("endpoint_port={endpoint_port}"));
        }
        if let Some(endpoint_source) = target.endpoint_source.as_deref() {
            parts.push(format!("endpoint_source={endpoint_source}"));
        }
        if let Some(ip) = target.ip.as_deref() {
            parts.push(format!("ip_raw={ip}"));
        }
        if let Some(port) = target.port.as_deref() {
            parts.push(format!("port_raw={port}"));
        }
        if let Some(sim) = target.sim_ip_and_port.as_deref() {
            parts.push(format!("sim={sim}"));
        }
        if let Some(seed) = target.seed_capability.as_deref() {
            parts.push(format!("seed={}", format_classified_url(seed)));
        }
        let raw_fields = connection.summarize_event_queue_event_fields(raw_event, 12);
        if target.endpoint_ip.is_none() {
            parts.push(String::from("endpoint_unresolved=true"));
        }
        parts.push(format!("raw={raw_fields}"));
        let details = parts.join(" ");
        emit_relay(
            tx,
            RuntimeRelayLevel::Info,
            "event_queue",
            &format!("{} detail {}", target.message, details),
        );
    }

    for parcel in connection
        .extract_event_queue_parcel_summaries(poll)
        .into_iter()
        .take(2)
    {
        let mut parts = Vec::new();
        if let Some(local_id) = parcel.local_id.as_deref() {
            parts.push(format!("local_id={local_id}"));
        }
        if let Some(name) = parcel.name.as_deref() {
            parts.push(format!("name={name}"));
        }
        if let Some(parcel_id) = parcel.parcel_id.as_deref() {
            parts.push(format!("parcel_id={parcel_id}"));
        }
        if let Some(owner_id) = parcel.owner_id.as_deref() {
            parts.push(format!("owner_id={owner_id}"));
        }
        if let Some(area) = parcel.area.as_deref() {
            parts.push(format!("area={area}"));
        }
        let details = if parts.is_empty() {
            poll.events
                .iter()
                .find(|event| event.message == parcel.message)
                .map(|event| connection.summarize_event_queue_event_fields(event, 6))
                .unwrap_or_else(|| String::from("none"))
        } else {
            parts.join(" ")
        };
        emit_relay(
            tx,
            RuntimeRelayLevel::Info,
            "event_queue",
            &format!("{} detail {}", parcel.message, details),
        );
    }
}

async fn follow_enable_simulator_ports(
    tx: &mpsc::Sender<LiveFeedUpdate>,
    connection: &mut Connection,
    circuit: Option<&SocialCircuit>,
    poll: &viewer_net::EventQueuePollResult,
    followed_targets: &mut BTreeSet<String>,
) {
    let Some(circuit) = circuit else {
        return;
    };
    for target in connection.extract_event_queue_simulator_targets(poll) {
        if target.message != "EnableSimulator" {
            continue;
        }
        let Some(endpoint) = extract_enable_simulator_endpoint(&target) else {
            continue;
        };
        let (endpoint_key, endpoint_source) = match &endpoint {
            EnableSimulatorEndpoint::IpPort {
                sim_ip,
                port,
                source,
            } => (format!("{sim_ip}:{port}"), *source),
            EnableSimulatorEndpoint::PortOnly { port } => {
                (format!("*:{}", port), "port_only_fallback")
            }
        };
        if !followed_targets.insert(endpoint_key.clone()) {
            continue;
        }
        let use_circuit_result = match &endpoint {
            EnableSimulatorEndpoint::IpPort { sim_ip, port, .. } => {
                connection
                    .send_use_circuit_code_on_circuit_to_target(circuit, sim_ip, *port)
                    .await
            }
            EnableSimulatorEndpoint::PortOnly { port } => {
                connection
                    .send_use_circuit_code_on_circuit_to_port(circuit, *port)
                    .await
            }
        };
        match use_circuit_result {
            Ok(()) => {
                emit_relay(
                    tx,
                    RuntimeRelayLevel::Info,
                    "event_queue",
                    &format!(
                        "EnableSimulator follow-up sent UseCircuitCode endpoint={endpoint_key} source={endpoint_source}"
                    ),
                );
                let complete_result = match &endpoint {
                    EnableSimulatorEndpoint::IpPort { sim_ip, port, .. } => {
                        connection
                            .send_complete_agent_movement_on_circuit_to_target(
                                circuit, sim_ip, *port,
                            )
                            .await
                    }
                    EnableSimulatorEndpoint::PortOnly { port } => {
                        connection
                            .send_complete_agent_movement_on_circuit_to_port(circuit, *port)
                            .await
                    }
                };
                match complete_result {
                    Ok(()) => emit_relay(
                        tx,
                        RuntimeRelayLevel::Info,
                        "event_queue",
                        &format!(
                            "EnableSimulator follow-up sent CompleteAgentMovement endpoint={endpoint_key} source={endpoint_source}"
                        ),
                    ),
                    Err(err) => emit_relay(
                        tx,
                        RuntimeRelayLevel::Warn,
                        "event_queue",
                        &format!(
                            "EnableSimulator follow-up failed CompleteAgentMovement endpoint={endpoint_key} source={endpoint_source}: {err}"
                        ),
                    ),
                }
            }
            Err(err) => emit_relay(
                tx,
                RuntimeRelayLevel::Warn,
                "event_queue",
                &format!(
                    "EnableSimulator follow-up failed endpoint={endpoint_key} source={endpoint_source}: {err}"
                ),
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum EnableSimulatorEndpoint {
    IpPort {
        sim_ip: String,
        port: u16,
        source: &'static str,
    },
    PortOnly {
        port: u16,
    },
}

fn extract_enable_simulator_endpoint(
    target: &viewer_net::EventQueueSimulatorTarget,
) -> Option<EnableSimulatorEndpoint> {
    if let (Some(sim_ip), Some(port), Some(source)) = (
        target.endpoint_ip.as_deref(),
        target.endpoint_port,
        target.endpoint_source.as_deref(),
    ) {
        return Some(EnableSimulatorEndpoint::IpPort {
            sim_ip: sim_ip.to_string(),
            port,
            source: match source {
                "sim_ip_and_port" => "sim_ip_and_port",
                "ip_port_fields" => "ip_port_fields",
                "simulatorinfo_binary_ip_port" => "simulatorinfo_binary_ip_port",
                _ => "viewer_net_endpoint",
            },
        });
    }

    if let Some(sim) = target.sim_ip_and_port.as_deref()
        && let Ok(parsed) = sim.parse::<std::net::SocketAddr>()
    {
        return Some(EnableSimulatorEndpoint::IpPort {
            sim_ip: parsed.ip().to_string(),
            port: parsed.port(),
            source: "sim_ip_and_port",
        });
    }

    if let (Some(ip), Some(port)) = (
        target.ip.as_deref(),
        target
            .port
            .as_deref()
            .and_then(|value| value.parse::<u16>().ok()),
    ) {
        return Some(EnableSimulatorEndpoint::IpPort {
            sim_ip: ip.to_string(),
            port,
            source: "ip_port_fields",
        });
    }

    if let Some(port) = target
        .port
        .as_deref()
        .and_then(|value| value.parse::<u16>().ok())
    {
        return Some(EnableSimulatorEndpoint::PortOnly { port });
    }

    None
}

async fn follow_region_seed_capabilities(
    tx: &mpsc::Sender<LiveFeedUpdate>,
    connection: &mut Connection,
    poll: &viewer_net::EventQueuePollResult,
    followed_seed_urls: &mut BTreeSet<String>,
    recent_seed_urls: &mut VecDeque<String>,
) {
    for target in connection.extract_event_queue_simulator_targets(poll) {
        let Some(seed_url) = target.seed_capability.as_deref() else {
            continue;
        };
        remember_recent_seed_capability_url(recent_seed_urls, seed_url);
        let message_name = target.message.clone();
        if !followed_seed_urls.insert(seed_url.to_string()) {
            continue;
        }

        match connection.fetch_seed_capabilities_from_url(seed_url).await {
            Ok(caps) => {
                let _ = connection.set_session_seed_capability_url(seed_url);
                let inventory = connection.summarize_seed_capability_inventory(&caps);
                let summary = summarize_seed_capability_inventory(&inventory);
                let discovered_non_baseline =
                    summarize_non_baseline_caps_by_host(&caps.entries, 16);
                emit_relay(
                    tx,
                    RuntimeRelayLevel::Info,
                    "event_queue",
                    &format!(
                        "{message_name} seed caps {}",
                        if summary == "none" {
                            String::from("none")
                        } else {
                            summary
                        }
                    ),
                );
                emit_relay(
                    tx,
                    RuntimeRelayLevel::Info,
                    "event_queue",
                    &format!(
                        "{message_name} seed non-baseline capability names by host: {discovered_non_baseline}"
                    ),
                );

                let important_caps = [
                    "EventQueueGet",
                    "InterestList",
                    "RegionObjects",
                    "UntrustedSimulatorMessage",
                ];
                let mut present = Vec::new();
                for cap_name in important_caps {
                    if let Some(url) = caps.entries.get(cap_name) {
                        present.push(format!("{cap_name}={}", format_classified_url(url)));
                    }
                }
                if present.is_empty() {
                    emit_relay(
                        tx,
                        RuntimeRelayLevel::Info,
                        "event_queue",
                        &format!("{message_name} important caps none"),
                    );
                } else {
                    emit_relay(
                        tx,
                        RuntimeRelayLevel::Info,
                        "event_queue",
                        &format!("{message_name} important caps {}", present.join(";")),
                    );
                }
            }
            Err(err) => emit_relay(
                tx,
                RuntimeRelayLevel::Warn,
                "event_queue",
                &format!(
                    "{message_name} seed capability follow-up failed {}: {err}",
                    format_classified_url(seed_url)
                ),
            ),
        }
    }
}

fn startup_social_drain_packet_budget(config: &InProcessLiveFeedConfig) -> usize {
    config
        .receive_max_packets
        .max(config.social_poll_max_packets)
        .max(config.post_movement_tail_packets)
        .saturating_mul(8)
        .clamp(32, 256)
}

fn dispatch_social_events(
    tx: &mpsc::Sender<LiveFeedUpdate>,
    local_agent_id: &str,
    events: Vec<SocialEvent>,
) {
    for event in events {
        match event {
            SocialEvent::FriendOnline { agent_id } => {
                let _ = tx.send(LiveFeedUpdate::FriendPresence {
                    id: agent_id.clone(),
                    online: true,
                });
                emit_relay(
                    tx,
                    RuntimeRelayLevel::Info,
                    "friend",
                    &format!("{agent_id} online"),
                );
            }
            SocialEvent::FriendOffline { agent_id } => {
                let _ = tx.send(LiveFeedUpdate::FriendPresence {
                    id: agent_id.clone(),
                    online: false,
                });
                emit_relay(
                    tx,
                    RuntimeRelayLevel::Info,
                    "friend",
                    &format!("{agent_id} offline"),
                );
            }
            SocialEvent::FriendRights {
                agent_id,
                related_id,
                rights,
            } => {
                if related_id == local_agent_id {
                    let _ = tx.send(LiveFeedUpdate::FriendRights {
                        id: agent_id.clone(),
                        rights_has: rights,
                        rights_given: 0,
                    });
                } else if agent_id == local_agent_id {
                    let _ = tx.send(LiveFeedUpdate::FriendRights {
                        id: related_id.clone(),
                        rights_has: 0,
                        rights_given: rights,
                    });
                }
            }
            SocialEvent::DirectIm(im) => {
                let participant_id = if im.from_id == local_agent_id {
                    im.to_id.clone()
                } else {
                    im.from_id.clone()
                };
                if !im.from_name.trim().is_empty() && !participant_id.is_empty() {
                    let _ = tx.send(LiveFeedUpdate::FriendResolvedName {
                        id: participant_id.clone(),
                        display_name: im.from_name.clone(),
                        source: String::from("im.from_name"),
                    });
                }
                let session_id = if !im.session_id.is_empty() {
                    im.session_id.clone()
                } else {
                    compute_p2p_session_id(local_agent_id, &participant_id).unwrap_or_default()
                };
                let _ = tx.send(LiveFeedUpdate::DirectIm(DirectImMessage {
                    id: now_unix_ms(),
                    session_id,
                    peer_id: participant_id.clone(),
                    from_id: im.from_id.clone(),
                    from_name: if im.from_name.is_empty() {
                        im.from_id.clone()
                    } else {
                        im.from_name.clone()
                    },
                    text: im.message.clone(),
                    observed_at_unix_ms: now_unix_ms(),
                    outgoing: im.from_id == local_agent_id,
                }));
                emit_relay(
                    tx,
                    RuntimeRelayLevel::Info,
                    "im_recv",
                    &format!("im from {}", im.from_id),
                );
            }
        }
    }
}

async fn drain_startup_social_circuit(
    connection: &mut Connection,
    circuit: &SocialCircuit,
    tx: &mpsc::Sender<LiveFeedUpdate>,
    local_agent_id: &str,
    config: &InProcessLiveFeedConfig,
) -> Result<(), viewer_net::ConnectionError> {
    let events = connection
        .poll_social_events(
            circuit,
            std::time::Duration::from_millis(config.social_poll_timeout_ms),
            startup_social_drain_packet_budget(config),
        )
        .await?;
    dispatch_social_events(tx, local_agent_id, events);
    Ok(())
}

async fn prime_startup_social_circuit(
    connection: &mut Connection,
    circuit: &SocialCircuit,
    tx: &mpsc::Sender<LiveFeedUpdate>,
    local_agent_id: &str,
    config: &InProcessLiveFeedConfig,
) -> Result<(), viewer_net::ConnectionError> {
    let bundle_enabled = config.lludp_startup_parity_bundle;
    // Firestorm-aligned startup ordering: consume the first inbound burst checkpoint
    // before sending the startup interest bundle.
    drain_startup_social_circuit(connection, circuit, tx, local_agent_id, config).await?;
    connection.send_startup_interest_messages(circuit).await?;
    connection
        .send_startup_request_parity_messages(circuit)
        .await?;
    if bundle_enabled {
        flush_pending_first_sim_ack_ids(connection, circuit, tx, "startup_parity_bundle_prime")
            .await?;
    }
    connection.send_retrieve_instant_messages(circuit).await?;
    drain_startup_social_circuit(connection, circuit, tx, local_agent_id, config).await
}

async fn flush_pending_first_sim_ack_ids(
    connection: &mut Connection,
    circuit: &SocialCircuit,
    tx: &mpsc::Sender<LiveFeedUpdate>,
    label: &str,
) -> Result<(), viewer_net::ConnectionError> {
    let flushed = connection.flush_pending_ack_ids_on_circuit(circuit).await?;
    if flushed > 0 {
        emit_relay(
            tx,
            RuntimeRelayLevel::Info,
            "first_sim_ack",
            &format!("{label}: flushed {flushed} pending ack ids"),
        );
    }
    Ok(())
}

fn agent_update_keepalive_interval_ticks(config: &InProcessLiveFeedConfig) -> u64 {
    if let Some(override_ticks) = config.agent_update_keepalive_ticks_override {
        return override_ticks.max(1);
    }
    let worker_tick_ms = config.worker_tick_ms.max(1);
    AGENT_UPDATE_KEEPALIVE_PERIOD_MS.saturating_add(worker_tick_ms.saturating_sub(1))
        / worker_tick_ms
}

fn should_send_agent_update_keepalive(
    worker_tick: u64,
    last_sent_tick: Option<u64>,
    interval_ticks: u64,
) -> bool {
    let interval_ticks = interval_ticks.max(1);
    match last_sent_tick {
        Some(last_sent_tick) => worker_tick.saturating_sub(last_sent_tick) >= interval_ticks,
        None => true,
    }
}

fn map_net_continuity_to_core(
    summary: &viewer_core::RegionContinuitySummary,
) -> viewer_core::RegionContinuitySummary {
    let mut mapped = summary.clone();
    let (outcome, reason) = viewer_grid::continuity::classify_handoff_diagnostics(&mapped);
    mapped.outcome = outcome;
    mapped.reason = reason;
    mapped
}

fn derive_environment_from_snapshot(
    snapshot: Option<&LiveVisualSnapshot>,
) -> viewer_core::EnvironmentState {
    let mut env = viewer_core::EnvironmentState::default();
    let Some(snapshot) = snapshot else {
        return env.sanitized();
    };

    let seconds_of_day = (snapshot.observed_at_unix_ms / 1000) % 86_400;
    env.time_of_day_normalized = seconds_of_day as f32 / 86_400.0;

    match snapshot.continuity.outcome {
        viewer_core::HandoffOutcome::Normal => {}
        viewer_core::HandoffOutcome::Degraded => {
            env.fog.density = (env.fog.density + 0.05).min(1.0);
            env.fog.start = (env.fog.start * 0.8).max(0.0);
            env.fog.end *= 0.85;
        }
        viewer_core::HandoffOutcome::Stalled => {
            env.fog.density = (env.fog.density + 0.1).min(1.0);
            env.fog.start = (env.fog.start * 0.5).max(0.0);
            env.fog.end *= 0.7;
        }
    }

    env.sanitized()
}

/// Derives the transition visual cue contract from the current continuity snapshot.
///
/// Mapping contract (deterministic, no runtime state dependencies):
/// - Normal outcome => Healthy (no-op baseline)
/// - Degraded outcome => Degraded cue with bounded intensity
/// - Stalled outcome => Stalled cue with bounded intensity
/// - If last probe result is Success after degraded/stalled => Recovering cue
fn derive_transition_visual_cue(
    snapshot: Option<&LiveVisualSnapshot>,
) -> viewer_core::TransitionVisualState {
    let Some(snapshot) = snapshot else {
        return viewer_core::TransitionVisualState::default();
    };

    // Check for bounded recovery: probe must be successful and recent relative to snapshot time.
    // Reuse the probe cooldown window to avoid stale success results driving "recovering" cues.
    let has_recent_probe_success = matches!(
        snapshot.continuity.last_probe_result,
        Some(viewer_core::ProbeResultCode::Success)
    ) && snapshot
        .continuity
        .last_probe_time_unix_ms
        .map(|probe_time| {
            snapshot.observed_at_unix_ms.saturating_sub(probe_time) <= RECOVERY_PROBE_COOLDOWN_MS
        })
        .unwrap_or(false);

    let (cue, intensity) = match snapshot.continuity.outcome {
        viewer_core::HandoffOutcome::Normal => {
            if has_recent_probe_success
                && snapshot.continuity.phase != viewer_core::HandoffPhase::None
            {
                // Recovering: probe success during an active (non-None) handoff
                (viewer_core::TransitionVisualCue::Recovering, 0.5_f32)
            } else {
                (viewer_core::TransitionVisualCue::Healthy, 0.0_f32)
            }
        }
        viewer_core::HandoffOutcome::Degraded => {
            if has_recent_probe_success {
                (viewer_core::TransitionVisualCue::Recovering, 0.6_f32)
            } else {
                (viewer_core::TransitionVisualCue::Degraded, 0.6_f32)
            }
        }
        viewer_core::HandoffOutcome::Stalled => {
            (viewer_core::TransitionVisualCue::Stalled, 0.9_f32)
        }
    };

    viewer_core::TransitionVisualState { cue, intensity }.sanitized()
}

fn parse_wire_format(value: &str) -> LoginWireFormat {
    match value.trim().to_ascii_lowercase().as_str() {
        "json" => LoginWireFormat::Json,
        "xmlrpc" | "xml-rpc" => LoginWireFormat::XmlRpc,
        _ => LoginWireFormat::Llsd,
    }
}

fn startup_failure_class_label(class: LiveStartupFailureClass) -> &'static str {
    match class {
        LiveStartupFailureClass::MissingConfig => "missing_config",
        LiveStartupFailureClass::ConnectTransport => "connect_transport",
        LiveStartupFailureClass::LoginRequestTransport => "login_transport",
        LiveStartupFailureClass::LoginRequestShape => "login_request_shape",
        LiveStartupFailureClass::LoginAuth => "login_auth",
        LiveStartupFailureClass::LoginRequiresTos => "login_requires_tos",
        LiveStartupFailureClass::LoginRequiresMfa => "login_requires_mfa",
        LiveStartupFailureClass::LoginUpdateRequired => "login_update_required",
        LiveStartupFailureClass::LoginOther => "login_other",
        LiveStartupFailureClass::ConnectionLostReconnecting => "connection_reconnecting",
    }
}

fn emit_login_fallback_relay(tx: &mpsc::Sender<LiveFeedUpdate>, fallback: &LoginFallbackOutcome) {
    if !fallback.fallback_used {
        return;
    }
    let reason = match fallback.classified_reason {
        Some(LoginFallbackClassifiedReason::RequestShapeMissingPassword) => {
            "request_shape_missing_password"
        }
        None => "unknown",
    };
    emit_relay(
        tx,
        RuntimeRelayLevel::Info,
        "login",
        &format!(
            "wire fallback used: {:?} -> {:?} ({reason})",
            fallback.primary_wire_format, fallback.final_wire_format
        ),
    );
}

fn startup_failure_from_login_outcome(
    result: &GridLoginResult,
    trace: &LoginTrace,
    fallback: &LoginFallbackOutcome,
) -> LiveStartupFailure {
    if fallback.classified_reason
        == Some(LoginFallbackClassifiedReason::RequestShapeMissingPassword)
    {
        return LiveStartupFailure {
            class: LiveStartupFailureClass::LoginRequestShape,
            message: String::from("login request shape rejected (missing password signature)"),
        };
    }

    match result {
        GridLoginResult::RequiresTos { message } => LiveStartupFailure {
            class: LiveStartupFailureClass::LoginRequiresTos,
            message: message
                .clone()
                .unwrap_or_else(|| String::from("terms of service required")),
        },
        GridLoginResult::RequiresMfa { message } => LiveStartupFailure {
            class: LiveStartupFailureClass::LoginRequiresMfa,
            message: message
                .clone()
                .unwrap_or_else(|| String::from("multi-factor token required")),
        },
        GridLoginResult::UpdateRequired { message } => LiveStartupFailure {
            class: LiveStartupFailureClass::LoginUpdateRequired,
            message: message
                .clone()
                .unwrap_or_else(|| String::from("viewer update required")),
        },
        GridLoginResult::Failed(error) => {
            let is_auth = error
                .reason
                .as_deref()
                .map(|reason| reason.eq_ignore_ascii_case("key"))
                .unwrap_or(false);
            LiveStartupFailure {
                class: if is_auth {
                    LiveStartupFailureClass::LoginAuth
                } else {
                    LiveStartupFailureClass::LoginOther
                },
                message: error
                    .message
                    .clone()
                    .or_else(|| trace.final_result.message.clone())
                    .or_else(|| trace.final_response.message.clone())
                    .unwrap_or_else(|| String::from("login not successful")),
            }
        }
        GridLoginResult::Redirect { .. } | GridLoginResult::Success(_) => LiveStartupFailure {
            class: LiveStartupFailureClass::LoginOther,
            message: String::from("login not successful"),
        },
    }
}

fn pick_best_avatar_name(profile: &AgentProfileData) -> Option<String> {
    if let Some(name) = profile.display_name.as_ref().map(|v| v.trim())
        && !name.is_empty()
    {
        return Some(name.to_string());
    }
    if let Some(name) = profile.username.as_ref().map(|v| v.trim())
        && !name.is_empty()
    {
        return Some(name.to_string());
    }
    None
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn coarse_xyz_to_world_position(
    region_coords: Option<[u32; 2]>,
    xyz: [u8; 3],
    lane_offset: f32,
) -> [f32; 3] {
    let (rx, rz) = match region_coords {
        Some([x, y]) => {
            let ox = ((x % 256) as f32 / 255.0 - 0.5) * 4.0;
            let oz = ((y % 256) as f32 / 255.0 - 0.5) * 4.0;
            (3.0 + ox, oz - 1.95)
        }
        None => (3.0, -1.95),
    };
    let x = rx - 1.4 + (f32::from(xyz[0]) / 255.0) * 2.8 + lane_offset;
    let z = rz - 1.2 + (f32::from(xyz[1]) / 255.0) * 2.4;
    let y = 0.55 + (f32::from(xyz[2]) / 255.0) * 1.6;
    [x, y, z]
}

fn uuid_short(value: &str) -> String {
    value.chars().take(8).collect()
}

fn format_avatar_label(agent_id: &str, social_state: &SocialState) -> String {
    if let Some(friend) = social_state.friends.iter().find(|f| f.id == agent_id)
        && let Some(name) = friend.display_name.as_deref()
    {
        return format!("{name} ({})", uuid_short(agent_id));
    }
    uuid_short(agent_id)
}

fn format_avatar_label_with_cache(
    agent_id: &str,
    social_state: &SocialState,
    avatar_name_cache: &BTreeMap<String, String>,
) -> String {
    if let Some(friend) = social_state.friends.iter().find(|f| f.id == agent_id)
        && let Some(name) = friend.display_name.as_deref()
    {
        return format!("{name} ({})", uuid_short(agent_id));
    }
    if let Some(name) = avatar_name_cache.get(agent_id) {
        return format!("{name} ({})", uuid_short(agent_id));
    }
    format_avatar_label(agent_id, social_state)
}

fn normalize_sim_name(sim_name: Option<&str>) -> Option<String> {
    sim_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn resolve_avatar_sim_name(
    sample_sim_name: Option<&str>,
    decoded_world_sim_name: Option<&str>,
    startup_sim_name_fallback: Option<&str>,
) -> String {
    normalize_sim_name(sample_sim_name)
        .or_else(|| normalize_sim_name(decoded_world_sim_name))
        .or_else(|| normalize_sim_name(startup_sim_name_fallback))
        .unwrap_or_else(|| String::from("unknown"))
}

fn update_world_sim_name_state(
    world_sim_name: &mut Option<String>,
    startup_sim_name_fallback: &mut Option<String>,
    decoded_sim_name: Option<&str>,
    startup_sim_name: Option<&str>,
) {
    if let Some(startup_name) = normalize_sim_name(startup_sim_name) {
        *startup_sim_name_fallback = Some(startup_name);
    }
    if let Some(decoded_name) = normalize_sim_name(decoded_sim_name) {
        *world_sim_name = Some(decoded_name);
    } else if world_sim_name.is_none() {
        *world_sim_name = startup_sim_name_fallback.clone();
    }
}

struct MergeWorldAvatarSamplesInput<'a> {
    current: &'a mut Vec<WorldAvatarPlaceholder>,
    social_state: &'a SocialState,
    avatar_name_cache: &'a BTreeMap<String, String>,
    decoded_world_sim_name: Option<&'a str>,
    startup_sim_name_fallback: Option<&'a str>,
    samples: &'a [WorkerWorldAvatarSample],
    region_coords: Option<[u32; 2]>,
    observed_at_unix_ms: u64,
    stale_after_ms: u64,
}

fn merge_world_avatar_samples(input: MergeWorldAvatarSamplesInput<'_>) -> Vec<(String, String)> {
    let MergeWorldAvatarSamplesInput {
        current,
        social_state,
        avatar_name_cache,
        decoded_world_sim_name,
        startup_sim_name_fallback,
        samples,
        region_coords,
        observed_at_unix_ms,
        stale_after_ms,
    } = input;
    let mut relay_events = Vec::new();
    for (idx, sample) in samples.iter().enumerate() {
        let fallback_id = if sample.is_self {
            String::from("self")
        } else {
            format!("coarse-{}", idx + 1)
        };
        let agent_id = sample.agent_id.clone().unwrap_or(fallback_id);
        let world_position =
            coarse_xyz_to_world_position(region_coords, sample.xyz, idx as f32 * 0.06);
        let display_name = if sample.is_self {
            String::from("You")
        } else {
            format_avatar_label_with_cache(&agent_id, social_state, avatar_name_cache)
        };
        let resolved_sim_name = resolve_avatar_sim_name(
            sample.sim_name.as_deref(),
            decoded_world_sim_name,
            startup_sim_name_fallback,
        );
        if let Some(existing) = current
            .iter_mut()
            .find(|avatar| avatar.agent_id == agent_id)
        {
            let moved = existing.world_position != world_position || existing.stale;
            let old_display = existing.display_name.clone();
            existing.world_position = world_position;
            existing.local_position = Some(sample.xyz);
            existing.sim_name = Some(resolved_sim_name.clone());
            existing.display_name = display_name.clone();
            existing.is_self = sample.is_self;
            existing.last_update_unix_ms = observed_at_unix_ms;
            existing.stale = false;
            let updated_avatar = existing.clone();
            let attachments =
                viewer_core::project_avatar_attachments(std::slice::from_ref(&updated_avatar));
            existing.appearance = AvatarAppearanceSummary {
                display_label: display_name.clone(),
                sim_name: Some(resolved_sim_name.clone()),
                is_self: sample.is_self,
                stale: existing.stale,
                last_update_unix_ms: observed_at_unix_ms,
                attachment_count: attachments.len(),
            };
            existing.attachments = attachments;
            if moved {
                relay_events.push((String::from("avatar_updated"), format!("{agent_id} moved")));
            }
            if old_display != display_name {
                relay_events.push((
                    String::from("avatar_name_resolved"),
                    format!("{agent_id} -> {display_name}"),
                ));
            }
        } else {
            current.push(WorldAvatarPlaceholder {
                agent_id: agent_id.clone(),
                world_position,
                local_position: Some(sample.xyz),
                sim_name: Some(resolved_sim_name),
                display_name: display_name.clone(),
                is_self: sample.is_self,
                last_update_unix_ms: observed_at_unix_ms,
                stale: false,
                appearance: AvatarAppearanceSummary {
                    display_label: display_name.clone(),
                    sim_name: Some(resolve_avatar_sim_name(
                        sample.sim_name.as_deref(),
                        decoded_world_sim_name,
                        startup_sim_name_fallback,
                    )),
                    is_self: sample.is_self,
                    stale: false,
                    last_update_unix_ms: observed_at_unix_ms,
                    attachment_count: 0,
                },
                attachments: Vec::new(),
            });
            let last_index = current.len() - 1;
            let attachments =
                viewer_core::project_avatar_attachments(std::slice::from_ref(&current[last_index]));
            current[last_index].appearance.attachment_count = attachments.len();
            current[last_index].attachments = attachments;
            relay_events.push((
                String::from("avatar_seen"),
                format!("{agent_id} {display_name}"),
            ));
        }
    }
    let mut removed = Vec::new();
    for avatar in current.iter_mut() {
        let age = observed_at_unix_ms.saturating_sub(avatar.last_update_unix_ms);
        if age > stale_after_ms {
            if !avatar.stale {
                avatar.stale = true;
                relay_events.push((String::from("avatar_stale"), avatar.agent_id.clone()));
            }
            if age > stale_after_ms.saturating_mul(3) {
                removed.push(avatar.agent_id.clone());
            }
        }
    }
    if !removed.is_empty() {
        current.retain(|avatar| !removed.iter().any(|id| id == &avatar.agent_id));
        for id in removed {
            relay_events.push((String::from("avatar_removed"), id));
        }
    }
    relay_events
}

fn extract_worker_world_avatar_samples(
    connection: &Connection,
    local_agent_id: &str,
) -> Vec<WorkerWorldAvatarSample> {
    let summary = connection.simulator_payload_decode_summary();
    let sample_sim_name = summary.region_handshake_last_sim_name.clone();
    let mut samples = Vec::new();
    for (idx, avatar) in summary.coarse_location_last_avatars.iter().enumerate() {
        let is_self = summary
            .coarse_location_last_self_index
            .map(|self_idx| usize::from(self_idx) == idx)
            .unwrap_or(false)
            || avatar.agent_id.as_deref() == Some(local_agent_id);
        samples.push(WorkerWorldAvatarSample {
            agent_id: avatar.agent_id.clone(),
            xyz: avatar.xyz,
            is_self,
            sim_name: sample_sim_name.clone(),
        });
    }
    if samples.is_empty() {
        if let Some(xyz) = summary.coarse_location_last_first {
            samples.push(WorkerWorldAvatarSample {
                agent_id: None,
                xyz,
                is_self: false,
                sim_name: sample_sim_name.clone(),
            });
        }
        if let Some(xyz) = summary.coarse_location_last_second {
            samples.push(WorkerWorldAvatarSample {
                agent_id: None,
                xyz,
                is_self: false,
                sim_name: sample_sim_name.clone(),
            });
        }
        if let Some(xyz) = summary.coarse_location_last_third {
            samples.push(WorkerWorldAvatarSample {
                agent_id: None,
                xyz,
                is_self: false,
                sim_name: sample_sim_name.clone(),
            });
        }
    }
    if !local_agent_id.is_empty() && !samples.iter().any(|sample| sample.is_self) {
        let fallback_xyz = summary
            .coarse_location_last_first
            .or(summary.coarse_location_last_second)
            .or(summary.coarse_location_last_third)
            .unwrap_or([128, 128, 32]);
        samples.push(WorkerWorldAvatarSample {
            agent_id: Some(local_agent_id.to_string()),
            xyz: fallback_xyz,
            is_self: true,
            sim_name: sample_sim_name,
        });
    }
    samples
}

fn extract_worker_world_sim_name(connection: &Connection) -> Option<String> {
    connection
        .simulator_payload_decode_summary()
        .region_handshake_last_sim_name
        .clone()
}

fn extract_worker_self_location(connection: &Connection) -> Option<[f32; 3]> {
    connection
        .simulator_payload_decode_summary()
        .agent_movement_complete_last_position
        .map(|[x, y, z]| [x as f32, y as f32, z as f32])
}

async fn load_profile_images_for_payload(
    connection: &mut Connection,
    tx: &Sender<LiveFeedUpdate>,
    image_cap_url: Option<&str>,
    attempted_assets: &mut BTreeSet<String>,
    payload: AgentProfileData,
) {
    let Some(image_cap_url) = image_cap_url else {
        emit_relay(
            tx,
            RuntimeRelayLevel::Warn,
            "profile_image",
            "GetTexture/ViewerAsset capability missing",
        );
        return;
    };
    let mut asset_ids = Vec::new();
    if let Some(asset_id) = payload.sl_image_id.as_ref() {
        asset_ids.push(asset_id.clone());
    }
    if let Some(asset_id) = payload.fl_image_id.as_ref() {
        asset_ids.push(asset_id.clone());
    }
    for details in &payload.pick_details {
        if let Some(asset_id) = details.snapshot_id.as_ref() {
            asset_ids.push(asset_id.clone());
        }
    }
    for details in &payload.classified_details {
        if let Some(asset_id) = details.snapshot_id.as_ref() {
            asset_ids.push(asset_id.clone());
        }
    }
    asset_ids.sort();
    asset_ids.dedup();
    for asset_id in asset_ids {
        if !attempted_assets.insert(asset_id.clone()) {
            continue;
        }
        match connection
            .fetch_profile_image_bytes(image_cap_url, &asset_id)
            .await
        {
            Ok(bytes) => {
                let _ = tx.send(LiveFeedUpdate::ProfileImageLoaded {
                    asset_id: asset_id.clone(),
                    bytes,
                });
                emit_relay(
                    tx,
                    RuntimeRelayLevel::Info,
                    "profile_image",
                    &format!("{asset_id}: bytes loaded"),
                );
            }
            Err(err) => {
                let concise = summarize_profile_image_error(&err);
                let _ = tx.send(LiveFeedUpdate::ProfileImageFailed);
                emit_relay(
                    tx,
                    RuntimeRelayLevel::Warn,
                    "profile_image",
                    &format!("{asset_id}: {concise}"),
                );
            }
        }
    }
}

fn summarize_profile_image_error(err: &ConnectionError) -> String {
    match err {
        ConnectionError::HttpStatus { status, .. } => {
            format!("http status {status}")
        }
        other => other.to_string(),
    }
}

fn apply_profile_payload(
    state: &mut AvatarProfileState,
    payload: AgentProfileData,
    at_unix_ms: u64,
) {
    let second_life = SecondLifeProfile {
        avatar_id: payload.id.clone(),
        display_name: payload.display_name.clone(),
        username: payload.username.clone(),
        profile_url: payload.profile_url.clone(),
        about_text: payload.sl_about_text.clone(),
        image_id: payload.sl_image_id.clone(),
        partner_id: payload.partner_id.clone(),
        member_since: payload.member_since.clone(),
        allow_publish: payload.allow_publish,
        online: payload.online,
        identified: payload.identified,
        transacted: payload.transacted,
        groups: payload
            .groups
            .iter()
            .map(|group| viewer_core::ProfileGroup {
                id: group.id.clone(),
                name: group.name.clone(),
                insignia_id: group.image_id.clone(),
            })
            .collect(),
    };
    state.second_life = Some(second_life);
    state.first_life = Some(FirstLifeProfile {
        about_text: payload.fl_about_text.clone(),
        image_id: payload.fl_image_id.clone(),
    });
    state.notes = Some(ProfileNotes {
        text: payload.notes.clone(),
    });
    state.feed.url = payload.profile_url.clone();
    state.feed.source = Some(String::from("AgentProfile"));

    state.picks.items = payload
        .picks
        .iter()
        .map(|entry| ProfilePickSummary {
            id: entry.id.clone(),
            name: entry.name.clone(),
        })
        .collect();
    for entry in &payload.picks {
        state
            .picks
            .details
            .entry(entry.id.clone())
            .or_insert(ProfilePickDetails {
                id: entry.id.clone(),
                name: entry.name.clone(),
                description: None,
                snapshot_id: None,
                parcel_id: None,
                sim_name: None,
                parcel_name: None,
                global_position: None,
            });
    }
    for details in &payload.pick_details {
        state.picks.details.insert(
            details.id.clone(),
            ProfilePickDetails {
                id: details.id.clone(),
                name: details.name.clone().unwrap_or_default(),
                description: details.description.clone(),
                snapshot_id: details.snapshot_id.clone(),
                parcel_id: details.parcel_id.clone(),
                sim_name: details.sim_name.clone(),
                parcel_name: details.parcel_name.clone(),
                global_position: details.global_position,
            },
        );
    }
    if state.picks.selected_pick_id.is_none() {
        state.picks.selected_pick_id = state.picks.items.first().map(|entry| entry.id.clone());
    }

    state.classifieds.items = payload
        .classifieds
        .iter()
        .map(|entry| ProfileClassifiedSummary {
            id: entry.id.clone(),
            name: entry.name.clone(),
        })
        .collect();
    for entry in &payload.classifieds {
        state
            .classifieds
            .details
            .entry(entry.id.clone())
            .or_insert(ProfileClassifiedDetails {
                id: entry.id.clone(),
                name: entry.name.clone(),
                description: None,
                snapshot_id: None,
                parcel_id: None,
                sim_name: None,
                parcel_name: None,
                global_position: None,
                category: None,
                flags: None,
                price_for_listing: None,
            });
    }
    for details in &payload.classified_details {
        state.classifieds.details.insert(
            details.id.clone(),
            ProfileClassifiedDetails {
                id: details.id.clone(),
                name: details.name.clone().unwrap_or_default(),
                description: details.description.clone(),
                snapshot_id: details.snapshot_id.clone(),
                parcel_id: details.parcel_id.clone(),
                sim_name: details.sim_name.clone(),
                parcel_name: details.parcel_name.clone(),
                global_position: details.global_position,
                category: details.category,
                flags: details.flags,
                price_for_listing: details.price_for_listing,
            },
        );
    }
    if state.classifieds.selected_classified_id.is_none() {
        state.classifieds.selected_classified_id = state
            .classifieds
            .items
            .first()
            .map(|entry| entry.id.clone());
    }
    let _ = at_unix_ms;
}

fn profile_tab_has_loaded_data(state: &AvatarProfileState, tab: AvatarProfileTab) -> bool {
    match tab {
        AvatarProfileTab::SecondLife => state.second_life.is_some(),
        AvatarProfileTab::Feed => state.feed.url.is_some() || state.second_life.is_some(),
        AvatarProfileTab::Picks => state.second_life.is_some(),
        AvatarProfileTab::Classifieds => state.second_life.is_some(),
        AvatarProfileTab::FirstLife => state.first_life.is_some(),
        AvatarProfileTab::Notes => state.notes.is_some(),
    }
}

fn should_fetch_profile_tab(
    profile: Option<&AvatarProfileState>,
    avatar_id: &str,
    tab: AvatarProfileTab,
    ttl_secs: u64,
) -> bool {
    let ttl_ms = ttl_secs.saturating_mul(1000).max(1000);
    let Some(profile) = profile else {
        return true;
    };
    if profile.avatar_id != avatar_id {
        return true;
    }
    let load = profile.tab_load(tab);
    if load.status == ProfileLoadStatus::Loading {
        return false;
    }
    if load.status == ProfileLoadStatus::Failed {
        return true;
    }
    let Some(last) = load.last_updated_unix_ms else {
        return true;
    };
    now_unix_ms().saturating_sub(last) > ttl_ms
}

fn open_external_url(url: &str) {
    if url.trim().is_empty() {
        return;
    }
    let try_cmd = |program: &str, arg: &str| -> bool {
        std::process::Command::new(program)
            .arg(arg)
            .spawn()
            .map(|_| true)
            .unwrap_or(false)
    };
    if try_cmd("xdg-open", url) {
        return;
    }
    if try_cmd("open", url) {
        return;
    }
    let _ = try_cmd("cmd", &format!("/C start {}", url));
}

fn should_apply_world_ingestion_seam(
    previous: Option<&WorldObjectIngestionSeam>,
    next: &WorldObjectIngestionSeam,
) -> bool {
    match previous {
        Some(prev) => prev != next,
        None => true,
    }
}

fn should_apply_live_visual_snapshot(
    previous: Option<&LiveVisualSnapshot>,
    next: Option<&LiveVisualSnapshot>,
) -> bool {
    previous != next
}

fn apply_avatar_render_mode(
    scene: &mut Scene,
    social_state: &mut SocialState,
    avatars: &[WorldAvatarPlaceholder],
    mode: AvatarRenderMode,
) {
    social_state.avatar_render_mode = mode;
    scene.apply_world_avatar_placeholders(avatars, mode);
}

impl ViewerApp {
    fn init_window(event_loop: &ActiveEventLoop) -> Result<Arc<Window>> {
        let window = event_loop
            .create_window(
                WindowAttributes::default()
                    .with_title("SL Viewer Rewrite")
                    .with_inner_size(PhysicalSize::new(1280, 720)),
            )
            .context("failed to create window")?;

        Ok(Arc::new(window))
    }

    fn init_state(event_loop: &ActiveEventLoop) -> Result<AppState> {
        let window = Self::init_window(event_loop)?;
        let renderer = RenderBackend::new(window.clone())?;
        let ui = UiSystem::new(&window, renderer.device(), renderer.surface_format());
        let mut social_state = SocialState::default();
        let cache_config = SocialCacheConfig::from_env();
        let mut social_cache = SocialCache::open(&cache_config).ok();
        if let Some(cache) = social_cache.as_ref()
            && let Ok(cached) = cache.load_cached_social_state()
        {
            social_state = cached;
        }
        let avatar_name_cache = social_state
            .friends
            .iter()
            .filter_map(|friend| {
                friend
                    .display_name
                    .as_ref()
                    .map(|name| (friend.id.clone(), name.clone()))
            })
            .collect::<BTreeMap<_, _>>();

        let fixture_texture_ids = fixture_texture_ids_from_env();
        let fixture_mesh_ids = fixture_mesh_ids_from_env();
        let stress_test_mode = StressTestMode::from_env();
        let auto_camera_config = AutoCameraConfig::from_env();
        let screenshot_config =
            screenshot_config_from_lookup(|key| std::env::var(key).ok(), stress_test_mode);
        let mesh_verification = mesh_verification_state_from_lookup(|key| std::env::var(key).ok());

        let live_visual_state = LiveVisualState::from_env();
        let asset_source_mode =
            parse_asset_source_mode(std::env::var("VIEWER_ASSET_SOURCE_MODE").ok().as_deref());
        let live_texture_results = Arc::new(Mutex::new(BTreeMap::new()));
        let mut fixture_texture_cache = viewer_asset::FixtureTextureCache::new();
        let allow_live_provider = match asset_source_mode {
            AssetSourceMode::Fixture => false,
            AssetSourceMode::Auto | AssetSourceMode::Live => true,
        };
        if allow_live_provider && let Some(tx) = &live_visual_state.in_process_tx {
            let provider = AppLiveTextureProvider {
                command_tx: tx.clone(),
                results: Arc::clone(&live_texture_results),
            };
            fixture_texture_cache = fixture_texture_cache.with_live_provider(Box::new(provider));
        }

        let mut state = AppState {
            window,
            renderer,
            ui,
            input: InputState::default(),
            live_visual_state,
            chat_state: ChatState::default(),
            social_state,
            world_avatars: Vec::new(),
            avatar_name_cache,
            world_sim_name: None,
            startup_sim_name_fallback: None,
            world_self_location: None,
            profile_state: None,
            profile_image_bytes: BTreeMap::new(),
            live_texture_results,
            social_cache: social_cache.take(),
            geometry_cache: viewer_asset::GeometryCache::new(),
            fixture_texture_cache,
            fixture_texture_ids,
            fixture_mesh_ids,
            fixture_texture_missing_logged: HashSet::new(),
            live_mesh_assets: BTreeMap::new(),
            camera: Camera::default(),
            scene: Scene::prototype(),
            world_ingestion_seam: WorldObjectIngestionSeam::default(),
            last_applied_live_visual_snapshot: None,
            last_applied_world_ingestion_seam: None,
            smoothed_fps: 60.0,
            smoothed_frame_ms: 16.6,
            avg_scene_update_ms: 0.0,
            last_frame_time: Instant::now(),
            app_start_time: Instant::now(),
            stress_test_mode,
            auto_camera_config,
            screenshot_config,
            frame_counter: 0,
            captured_screenshots: 0,
            environment: viewer_core::EnvironmentState::default(),
            last_probe_retry_ms: None,
            last_asset_refresh_ms: None,
            probe_in_flight: false,
            last_recovery_result: None,
            transition_visual_state: viewer_core::TransitionVisualState::default(),
            network_debug: NetworkDebugState {
                sections: Vec::new(),
                recent_events: viewer_core::RuntimeRelayState {
                    events: Vec::new(),
                    max_events: 200,
                },
            },
            mesh_verification,
        };

        match state.stress_test_mode {
            StressTestMode::SceneStress => state.spawn_stress_test(),
            StressTestMode::GeometryTorture
            | StressTestMode::AutoCamera
            | StressTestMode::Screenshot => state.spawn_geometry_torture_test(),
            StressTestMode::SingleLiveTextureCenter => {
                state.spawn_single_live_texture_center_test()
            }
            StressTestMode::None => {}
        }

        Ok(state)
    }
}

impl AppState {
    fn refresh_network_debug_snapshot_section(&mut self) {
        let mut lines = vec![format!(
            "status={:?} chat={:?}",
            self.live_visual_state.startup_status, self.live_visual_state.chat_connection
        )];
        if let Some(snapshot) = self.live_visual_state.snapshot.as_ref() {
            let object_gate = if snapshot.decoded_object_feed_update_messages > 0 {
                "PASS"
            } else {
                "FAIL"
            };
            lines.push(format!(
                "region={} endpoint={}",
                snapshot.current_region_name.as_deref().unwrap_or("unknown"),
                snapshot.first_sim_endpoint.as_deref().unwrap_or("none")
            ));
            lines.push(format!(
                "logged_in={} amc={} object_gate={}",
                snapshot.logged_in, snapshot.handshake_agent_movement_complete, object_gate
            ));
            lines.push(format!(
                "object_feed updates={} kills={} total={}",
                snapshot.decoded_object_feed_update_messages,
                snapshot.decoded_object_feed_kill_messages,
                snapshot.decoded_object_feed_total_objects
            ));
            lines.push(format!(
                "mesh_queue requested={} fetched={} decoded={} failed={}",
                live_mesh_queue_counts(&self.live_mesh_assets).0,
                live_mesh_queue_counts(&self.live_mesh_assets).1,
                live_mesh_queue_counts(&self.live_mesh_assets).2,
                live_mesh_queue_counts(&self.live_mesh_assets).3
            ));
            lines.push(live_mesh_asset_summary_line(&self.live_mesh_assets));
            lines.push(format!(
                "transition crossed={} confirm_enable={}",
                snapshot.crossed_region, snapshot.confirm_enable_simulator
            ));
        } else {
            lines.push(String::from("snapshot=none"));
        }
        self.network_debug.set_section_lines("Session", lines);
    }

    fn push_local_relay(&mut self, level: RuntimeRelayLevel, category: &str, message: String) {
        tracing::info!("{category}: {message}");
        let event = RuntimeRelayEvent {
            at_unix_ms: now_unix_ms(),
            level,
            category: category.to_string(),
            message,
        };
        self.ingest_network_debug_event(&event);
        self.social_state.relay.push(event);
    }

    fn track_mesh_verification_key(&mut self, key: &(String, u32)) -> bool {
        let Some(config) = self.mesh_verification.config.as_ref() else {
            return false;
        };
        if self.mesh_verification.selected_target.is_none() {
            self.mesh_verification.selected_target =
                Some(config.target_mesh_id.clone().unwrap_or_else(|| key.clone()));
        }
        self.mesh_verification.selected_target.as_ref() == Some(key)
    }

    fn emit_mesh_verification_event(&mut self, stage: &str, key: &(String, u32), detail: String) {
        if !self.track_mesh_verification_key(key) {
            return;
        }
        let Some(config) = self.mesh_verification.config.as_ref() else {
            return;
        };
        let event_key = format!("{stage}:{}:{}", key.0, key.1);
        if !self.mesh_verification.emitted_events.insert(event_key) {
            return;
        }

        if let Some(parent) = config.log_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&config.log_path)
        {
            let json_line = format!(
                "{{\"ts\":{},\"stage\":\"{}\",\"mesh_id\":\"{}\",\"lod\":{},\"detail\":\"{}\"}}\n",
                now_unix_ms(),
                stage.replace('"', "'"),
                key.0.replace('"', "'"),
                key.1,
                detail.replace('"', "'")
            );
            let _ = file.write_all(json_line.as_bytes());
        }

        self.push_local_relay(
            RuntimeRelayLevel::Info,
            "mesh_visibility",
            format!("stage={stage} id={} lod={} {detail}", key.0, key.1),
        );
    }

    fn maybe_emit_mesh_discovered_event(&mut self) {
        let Some(snapshot) = self.live_visual_state.snapshot.as_ref() else {
            return;
        };
        for key in extract_decoded_object_feed_mesh_ids(snapshot, 1) {
            if self.track_mesh_verification_key(&key) {
                self.emit_mesh_verification_event(
                    "object_discovered",
                    &key,
                    String::from("decoded object-feed mesh source observed"),
                );
                break;
            }
        }
    }

    fn schedule_mesh_verification_screenshot(&mut self, key: &(String, u32)) {
        let Some(dir) = self
            .mesh_verification
            .config
            .as_ref()
            .and_then(|config| config.screenshot_dir.clone())
        else {
            return;
        };
        if self.mesh_verification.pending_screenshot.is_some()
            || self.mesh_verification.captured_screenshot.is_some()
        {
            return;
        }
        if !self.track_mesh_verification_key(key) {
            return;
        }
        let _ = fs::create_dir_all(&dir);
        self.mesh_verification.pending_screenshot =
            Some(dir.join(format!("mesh_verify_{}_lod{}.png", key.0, key.1)));
    }

    fn ingest_network_debug_event(&mut self, event: &RuntimeRelayEvent) {
        if !is_network_debug_category(&event.category) {
            return;
        }
        self.network_debug.recent_events.push(event.clone());
        let line = format!("[{}] {}", event.at_unix_ms, event.message);
        match event.category.as_str() {
            "parallel_protocol" => {
                append_network_debug_line(&mut self.network_debug, "Capabilities", line);
            }
            "event_queue" => {
                let title = if event.message.contains("follow-up") {
                    "Follow-Up"
                } else {
                    "EventQueue"
                };
                append_network_debug_line(&mut self.network_debug, title, line);
            }
            "first_sim_socket" | "first_sim_forensics" | "first_sim_ack" | "object_feed" => {
                append_network_debug_line(&mut self.network_debug, "LLUDP", line);
            }
            "region_objects" => {
                append_network_debug_line(&mut self.network_debug, "RegionObjects", line);
            }
            "social" => {
                append_network_debug_line(&mut self.network_debug, "Social Socket", line);
            }
            _ => {
                append_network_debug_line(&mut self.network_debug, "Network", line);
            }
        }
    }

    fn dispatch_recovery_action(
        &mut self,
        action: viewer_core::RecoveryAction,
        now_ms: u64,
    ) -> viewer_core::RecoveryActionResult {
        let (result, new_probe_ms, new_asset_ms) = compute_recovery_action(
            action,
            now_ms,
            self.last_probe_retry_ms,
            self.last_asset_refresh_ms,
            self.live_visual_state.in_process_enabled,
            self.probe_in_flight,
            RECOVERY_PROBE_COOLDOWN_MS,
            RECOVERY_ASSET_REFRESH_COOLDOWN_MS,
        );

        if result.code == viewer_core::RecoveryResultCode::Accepted
            && action == viewer_core::RecoveryAction::RefreshVisibleAssets
        {
            self.fixture_texture_missing_logged.clear();
            self.fixture_texture_cache.clear_failures();
        }

        self.last_probe_retry_ms = new_probe_ms;
        self.last_asset_refresh_ms = new_asset_ms;
        if action == viewer_core::RecoveryAction::RetryContinuityProbe
            && result.code == viewer_core::RecoveryResultCode::Accepted
        {
            self.probe_in_flight = true;
        }

        if action == viewer_core::RecoveryAction::ClearRecoveryBanner {
            self.last_recovery_result = None;
        } else {
            self.last_recovery_result = Some(result.clone());
        }

        result
    }

    fn spawn_geometry_torture_test(&mut self) {
        use viewer_core::{
            GeometrySource, HoleType, InstanceRole, PathType, ProfileType, Transform, VolumeParams,
        };

        let debug_mesh_id = String::from("debug-secondlife-mesh");
        let debug_mesh_lod = 0u32;
        let debug_mesh_bytes = viewer_asset::debug_triangle_second_life_mesh_bytes();
        let debug_mesh_len = debug_mesh_bytes.len();
        let debug_mesh_signature = mesh_byte_signature(&debug_mesh_bytes);
        let debug_mesh_format = viewer_asset::detect_mesh_source_format(&debug_mesh_bytes);
        let debug_mesh_key = (debug_mesh_id.clone(), debug_mesh_lod);
        self.live_mesh_assets.insert(
            debug_mesh_key.clone(),
            LiveMeshAssetState::Fetched(LiveMeshAssetBytes {
                bytes: debug_mesh_bytes,
                byte_len: debug_mesh_len,
                byte_signature: debug_mesh_signature.clone(),
                format_hint: debug_mesh_format,
            }),
        );
        self.emit_mesh_verification_event(
            "mesh_bytes_fetched",
            &debug_mesh_key,
            format!(
                "mode=offline byte_len={} signature={} format={:?}",
                debug_mesh_len, debug_mesh_signature, debug_mesh_format
            ),
        );

        // Grid of diverse procedural prims
        for i in 0..5 {
            for j in 0..5 {
                let mut params = VolumeParams {
                    profile_type: if (i + j) % 2 == 0 {
                        ProfileType::Square
                    } else {
                        ProfileType::Circle
                    },
                    hole_type: HoleType::Same,
                    path_type: if i % 2 == 0 {
                        PathType::Line
                    } else {
                        PathType::Circle
                    },
                    begin_cut: 0.0,
                    end_cut: 1.0,
                    hollow: if j % 2 == 0 { 0.0 } else { 0.5 },
                    twist_begin: 0.0,
                    twist_end: i as f32 * 0.2,
                    taper_x: j as f32 * 0.1,
                    taper_y: j as f32 * 0.1,
                    revolutions: 1.0,
                    skew: 0.0,
                    radius_offset: 0.0,
                    shear_x: 0.0,
                    shear_y: 0.0,
                };

                // Some specific variations
                if i == 4 {
                    params.end_cut = 0.5;
                } // Half prims

                let transform = Transform {
                    position: [i as f32 * 4.0 - 8.0, 5.0, j as f32 * 4.0 - 8.0],
                    ..Transform::default()
                };

                self.scene.insert_instance(
                    GeometrySource::Procedural(params, 1.0),
                    InstanceRole::SceneStatic,
                    transform,
                    [0.2 + i as f32 * 0.1, 0.4 + j as f32 * 0.1, 0.7, 1.0],
                    AlphaMode::Opaque,
                );
            }
        }

        // Add a Sculpted Prim
        let sculpt_trans = Transform {
            position: [0.0, 15.0, 0.0],
            scale: [2.0, 2.0, 2.0],
            ..Transform::default()
        };
        self.scene.insert_instance(
            GeometrySource::Sculpt("dummy-sculpt".to_string(), viewer_core::SculptType::Sphere),
            InstanceRole::SceneStatic,
            sculpt_trans,
            [0.8, 0.2, 0.2, 1.0],
            AlphaMode::Opaque,
        );

        // Add a glTF Mesh (Animation Test)
        let mesh_trans = Transform {
            position: [5.0, 15.0, 5.0],
            ..Transform::default()
        };
        let inst_id = self.scene.insert_instance(
            GeometrySource::Mesh(debug_mesh_id, debug_mesh_lod),
            InstanceRole::SceneStatic,
            mesh_trans,
            [1.0, 1.0, 1.0, 1.0],
            AlphaMode::Opaque,
        );

        if let Some(instance) = self.scene.get_instance_mut(inst_id) {
            // Apply water texture to the first fixture ID
            if let Some(water_id) = self.fixture_texture_ids.first() {
                instance.materials.default =
                    viewer_core::MaterialDescriptor::Legacy(viewer_core::TextureEntry {
                        texture_id: water_id.clone(),
                        ..viewer_core::TextureEntry::default()
                    });
            }

            // Apply smooth horizontal scroll
            instance.texture_anim = viewer_core::TextureAnim {
                mode: viewer_core::material::animation::ANIM_ON
                    | viewer_core::material::animation::SMOOTH
                    | viewer_core::material::animation::LOOP,
                rate: 0.2, // 20% scroll per second
                length: 1.0,
                ..viewer_core::TextureAnim::default()
            };
        }

        // Add a Transparent Validation Cube
        let trans_cube_trans = Transform {
            position: [0.0, 18.0, 0.0],
            scale: [3.0, 3.0, 3.0],
            ..Transform::default()
        };
        self.scene.insert_instance(
            GeometrySource::Diagnostic(MeshKind::Cube),
            InstanceRole::SceneStatic,
            trans_cube_trans,
            [0.2, 0.4, 1.0, 0.5], // Semi-transparent blue
            AlphaMode::Blend,
        );
    }

    fn spawn_single_live_texture_center_test(&mut self) {
        use viewer_core::{GeometrySource, InstanceRole, MeshKind, Transform};

        let transform = Transform {
            position: [0.0, 5.0, 0.0],
            scale: [4.0, 4.0, 4.0],
            ..Transform::default()
        };
        let instance_id = self.scene.insert_instance(
            GeometrySource::Diagnostic(MeshKind::Cube),
            InstanceRole::SceneStatic,
            transform,
            [1.0, 1.0, 1.0, 1.0],
            AlphaMode::Opaque,
        );

        if let Some(texture_id) = self.fixture_texture_ids.first().cloned() {
            if let Some(instance) = self.scene.get_instance_mut(instance_id) {
                instance.materials.default =
                    viewer_core::MaterialDescriptor::Legacy(viewer_core::TextureEntry {
                        texture_id,
                        ..viewer_core::TextureEntry::default()
                    });
            }
        } else {
            tracing::warn!(
                "single live texture center test requested but no fixture texture id is configured"
            );
        }
    }

    fn spawn_stress_test(&mut self) {
        use viewer_core::{GeometrySource, InstanceRole, MeshKind, Transform};
        if std::env::var("STRESS_TEST").as_deref() == Ok("1") {
            for i in 0..10 {
                for j in 0..10 {
                    let transform = Transform {
                        position: [i as f32 * 2.0, 5.0, j as f32 * 2.0],
                        ..Transform::default()
                    };
                    self.scene.insert_instance(
                        GeometrySource::Diagnostic(MeshKind::Cube),
                        InstanceRole::SceneStatic,
                        transform,
                        [0.2, 0.5, 0.8, 1.0],
                        AlphaMode::Opaque,
                    );
                }
            }
            // Small planet/moon system
            let sun_trans = Transform {
                position: [0.0, 10.0, 0.0],
                ..Transform::default()
            };
            let _sun_id = self.scene.insert_instance(
                GeometrySource::Diagnostic(MeshKind::Cube),
                InstanceRole::SceneStatic,
                sun_trans,
                [1.0, 0.8, 0.1, 1.0],
                AlphaMode::Opaque,
            );
        }

        // Spawn a grid of "planets" with "moons"
        for x in -5..5 {
            for z in -5..5 {
                let parent_pos = [x as f32 * 10.0, 5.0, z as f32 * 10.0];
                let parent_id = self.scene.insert_instance(
                    GeometrySource::Diagnostic(MeshKind::Cube),
                    InstanceRole::WorldIngestionProxy,
                    Transform {
                        position: parent_pos,
                        rotation: [0.0, 0.0, 0.0, 1.0],
                        scale: [1.0, 1.0, 1.0],
                    },
                    [0.8, 0.8, 0.2, 1.0],
                    AlphaMode::Opaque,
                );

                // Add 4 moons to each planet
                for i in 0..4 {
                    let angle = (i as f32) * std::f32::consts::PI * 0.5;
                    let moon_pos = [angle.cos() * 3.0, angle.sin() * 3.0, 0.0];
                    let moon_transform = Transform {
                        position: moon_pos,
                        rotation: [0.0, 0.0, 0.0, 1.0],
                        scale: [0.3, 0.3, 0.3],
                    };
                    let moon_id = self.scene.insert_instance(
                        GeometrySource::Diagnostic(MeshKind::Cube),
                        InstanceRole::WorldIngestionProxy,
                        moon_transform,
                        [0.2, 0.6, 0.9, 1.0],
                        AlphaMode::Opaque,
                    );
                    self.scene.get_instance_mut(moon_id).unwrap().parent_id = Some(parent_id);
                }
            }
        }
    }

    fn update_stress_test(&mut self, time: f32) {
        // Rotate parents and their moons
        let (_sin_t, _cos_t) = (time.sin(), time.cos());

        // We iterate through all instances. If they are stress test objects, we rotate them.
        // For Milestone 1, we just find all instances and apply some math if they have a parent or are a big cube.
        // A better way would be tag-based, but for M1 we'll just rotate everything that isn't a known role.

        for inst in self.scene.instances_mut() {
            if inst.role == viewer_core::InstanceRole::WorldIngestionProxy {
                if inst.parent_id.is_none() {
                    // It's a planet - slow rotation
                    let angle = time * 0.2;
                    let s = (angle * 0.5).sin();
                    let c = (angle * 0.5).cos();
                    inst.transform.rotation = [0.0, s, 0.0, c];
                } else {
                    // It's a moon - fast rotation
                    let angle = time * 2.0;
                    let s = (angle * 0.5).sin();
                    let c = (angle * 0.5).cos();
                    inst.transform.rotation = [s, 0.0, 0.0, c];
                }
                inst.dirty_spatial = true;
            }
        }
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.renderer.resize(new_size);
    }

    fn redraw(&mut self) -> Result<()> {
        let now = Instant::now();
        let dt_seconds = (now - self.last_frame_time).as_secs_f32().min(0.1);
        self.last_frame_time = now;
        let frame_ms = (dt_seconds * 1000.0).max(0.0);
        let fps = if dt_seconds > 0.000_001 {
            1.0 / dt_seconds
        } else {
            0.0
        };
        let alpha = 0.15;
        if self.smoothed_fps <= f32::EPSILON {
            self.smoothed_fps = fps;
            self.smoothed_frame_ms = frame_ms;
        } else {
            self.smoothed_fps += (fps - self.smoothed_fps) * alpha;
            self.smoothed_frame_ms += (frame_ms - self.smoothed_frame_ms) * alpha;
        }

        self.social_state.frametime_history.push_back(frame_ms);
        if self.social_state.frametime_history.len() > 200 {
            self.social_state.frametime_history.pop_front();
        }

        let [look_x, look_y] = self.input.take_look_delta();
        let elapsed_seconds = self.app_start_time.elapsed().as_secs_f32();
        if self.stress_test_mode.uses_auto_camera() {
            self.auto_camera_config
                .apply_to_camera(&mut self.camera, elapsed_seconds);
        } else {
            self.camera.add_look_delta(look_x, look_y);
            self.input.update_camera(&mut self.camera, dt_seconds);
        }

        if self.stress_test_mode == StressTestMode::SceneStress {
            self.update_stress_test(elapsed_seconds);
        }

        for update in self.live_visual_state.drain_worker_updates() {
            match update {
                LiveFeedUpdate::ContinuityProbeResult(code) => {
                    self.probe_in_flight = false;
                    self.last_recovery_result = Some(viewer_core::RecoveryActionResult {
                        action: viewer_core::RecoveryAction::RetryContinuityProbe,
                        code: viewer_core::RecoveryResultCode::Completed(code),
                        detail: None,
                        cooldown_remaining_ms: None,
                    });
                    if let Some(snapshot) = &mut self.live_visual_state.snapshot {
                        snapshot.continuity.last_probe_result = Some(code);
                        snapshot.continuity.last_probe_time_unix_ms = Some(now_unix_ms());
                    }
                }
                LiveFeedUpdate::Snapshot(snapshot) => {
                    if let Some(region_name) = snapshot.current_region_name.as_deref() {
                        self.startup_sim_name_fallback
                            .get_or_insert_with(|| region_name.to_string());
                        if self.world_sim_name.is_none() {
                            self.world_sim_name = Some(region_name.to_string());
                        }
                    }
                    self.live_visual_state.snapshot = Some(snapshot);
                }
                LiveFeedUpdate::Status(status) => {
                    self.live_visual_state.startup_status = status;
                }
                LiveFeedUpdate::ChatConnection(status) => {
                    self.live_visual_state.chat_connection = status.clone();
                    self.chat_state.set_connection(status);
                }
                LiveFeedUpdate::ChatMessage(message) => {
                    self.chat_state.push_message(message);
                }
                LiveFeedUpdate::ChatSendStatus(status) => {
                    self.chat_state.send_status = status;
                }
                LiveFeedUpdate::FriendsBootstrap(friends) => {
                    for friend in friends {
                        self.social_state.upsert_friend(friend);
                    }
                }
                LiveFeedUpdate::FriendPresence { id, online } => {
                    self.social_state
                        .set_friend_online(&id, online, now_unix_ms());
                }
                LiveFeedUpdate::FriendRights {
                    id,
                    rights_has,
                    rights_given,
                } => {
                    if let Some(existing) =
                        self.social_state.friends.iter_mut().find(|f| f.id == id)
                    {
                        existing.rights_has = rights_has;
                        existing.rights_given = rights_given;
                        existing.last_changed_unix_ms = now_unix_ms();
                    } else {
                        self.social_state.upsert_friend(FriendEntry {
                            id,
                            display_name: None,
                            name_source: None,
                            last_name_resolved_unix_ms: None,
                            online: false,
                            rights_has,
                            rights_given,
                            last_changed_unix_ms: now_unix_ms(),
                        });
                    }
                }
                LiveFeedUpdate::FriendResolvedName {
                    id,
                    display_name,
                    source,
                } => {
                    let resolved_at = now_unix_ms();
                    self.social_state
                        .resolve_friend_name(&id, &display_name, &source, resolved_at);
                    self.avatar_name_cache
                        .insert(id.clone(), display_name.clone());
                    if let Some(cache) = self.social_cache.as_mut() {
                        let _ = cache.upsert_name_cache(&id, &display_name, &source, resolved_at);
                    }
                    if let Some(avatar) = self.world_avatars.iter_mut().find(|a| a.agent_id == id) {
                        avatar.display_name = format!("{display_name} ({})", uuid_short(&id));
                        self.social_state.relay.push(RuntimeRelayEvent {
                            at_unix_ms: resolved_at,
                            level: RuntimeRelayLevel::Info,
                            category: String::from("avatar_name_resolved"),
                            message: format!("{id} -> {}", avatar.display_name),
                        });
                    }
                }
                LiveFeedUpdate::AvatarResolvedName {
                    id,
                    display_name,
                    source,
                } => {
                    let resolved_at = now_unix_ms();
                    self.avatar_name_cache
                        .insert(id.clone(), display_name.clone());
                    if let Some(cache) = self.social_cache.as_mut() {
                        let _ = cache.upsert_name_cache(&id, &display_name, &source, resolved_at);
                    }
                    if let Some(avatar) = self.world_avatars.iter_mut().find(|a| a.agent_id == id) {
                        avatar.display_name = format!("{display_name} ({})", uuid_short(&id));
                    }
                    self.social_state.relay.push(RuntimeRelayEvent {
                        at_unix_ms: resolved_at,
                        level: RuntimeRelayLevel::Info,
                        category: String::from("avatar_name_resolved"),
                        message: format!("{id} -> {display_name}"),
                    });
                }
                LiveFeedUpdate::WorldAvatars {
                    avatars,
                    decoded_sim_name,
                    startup_sim_name,
                    self_location,
                    observed_at_unix_ms,
                } => {
                    update_world_sim_name_state(
                        &mut self.world_sim_name,
                        &mut self.startup_sim_name_fallback,
                        decoded_sim_name.as_deref(),
                        startup_sim_name.as_deref(),
                    );
                    self.world_self_location = self_location;
                    let region_coords =
                        self.live_visual_state
                            .snapshot
                            .as_ref()
                            .and_then(|snapshot| {
                                Some([snapshot.first_sim_region_x?, snapshot.first_sim_region_y?])
                            });
                    let relay = merge_world_avatar_samples(MergeWorldAvatarSamplesInput {
                        current: &mut self.world_avatars,
                        social_state: &self.social_state,
                        avatar_name_cache: &self.avatar_name_cache,
                        decoded_world_sim_name: self.world_sim_name.as_deref(),
                        startup_sim_name_fallback: self.startup_sim_name_fallback.as_deref(),
                        samples: &avatars,
                        region_coords,
                        observed_at_unix_ms,
                        stale_after_ms: 12_000,
                    });
                    for (category, message) in relay {
                        self.social_state.relay.push(RuntimeRelayEvent {
                            at_unix_ms: now_unix_ms(),
                            level: RuntimeRelayLevel::Info,
                            category,
                            message,
                        });
                    }
                }
                LiveFeedUpdate::DirectIm(message) => {
                    let participant = message.peer_id.clone();
                    if !message.outgoing
                        && !message.from_name.trim().is_empty()
                        && message.from_name != message.from_id
                    {
                        self.social_state.resolve_friend_name(
                            &participant,
                            &message.from_name,
                            "im.from_name",
                            message.observed_at_unix_ms,
                        );
                        if let Some(cache) = self.social_cache.as_mut() {
                            let _ = cache.upsert_name_cache(
                                &participant,
                                &message.from_name,
                                "im.from_name",
                                message.observed_at_unix_ms,
                            );
                        }
                    }
                    if let Some(cache) = self.social_cache.as_mut() {
                        let _ = cache.store_im_message(&message);
                    }
                    self.social_state
                        .upsert_thread_message(message, &participant);
                }
                LiveFeedUpdate::ProfileOpenRequested { avatar_id } => {
                    let mut profile = self.profile_state.take().unwrap_or_default();
                    if profile.avatar_id != avatar_id {
                        profile = AvatarProfileState {
                            avatar_id,
                            ..AvatarProfileState::default()
                        };
                    }
                    profile.selected_tab = AvatarProfileTab::SecondLife;
                    self.profile_state = Some(profile);
                }
                LiveFeedUpdate::ProfileTabLoadStarted { avatar_id, tab } => {
                    let profile = self.ensure_profile_state(&avatar_id);
                    profile.selected_tab = tab;
                    profile.tab_load_mut(tab).mark_loading(now_unix_ms());
                }
                LiveFeedUpdate::ProfileData {
                    avatar_id,
                    profile,
                    requested_tab,
                } => {
                    let at = now_unix_ms();
                    let state = self.ensure_profile_state(&avatar_id);
                    state.selected_tab = requested_tab;
                    apply_profile_payload(state, profile, at);
                    if profile_tab_has_loaded_data(state, requested_tab) {
                        state.tab_load_mut(requested_tab).mark_loaded(at);
                    } else {
                        state
                            .tab_load_mut(requested_tab)
                            .mark_failed(at, "profile payload missing expected tab fields");
                    }
                }
                LiveFeedUpdate::ProfileTabLoadFailed {
                    avatar_id,
                    tab,
                    reason,
                } => {
                    let state = self.ensure_profile_state(&avatar_id);
                    state.selected_tab = tab;
                    state.tab_load_mut(tab).mark_failed(now_unix_ms(), reason);
                }
                LiveFeedUpdate::ProfileImageLoaded { asset_id, bytes } => {
                    self.profile_image_bytes.insert(asset_id, bytes);
                }
                LiveFeedUpdate::ProfileImageFailed => {}
                LiveFeedUpdate::Relay(event) => {
                    self.ingest_network_debug_event(&event);
                    self.social_state.relay.push(event);
                }
                LiveFeedUpdate::TextureAsset { id, bytes } => {
                    let id = AssetID::new(id);
                    let mut results = self.live_texture_results.lock().unwrap();
                    match viewer_asset::decode_texture_rgba8(&bytes) {
                        Ok(img) => {
                            results.insert(
                                id,
                                viewer_asset::AssetFetchOutcome {
                                    status: viewer_asset::AssetStatus::Ready(img),
                                    source: viewer_asset::AssetSourceKind::Live,
                                    failure: None,
                                },
                            );
                        }
                        Err(_) => {
                            results.insert(
                                id,
                                viewer_asset::AssetFetchOutcome {
                                    status: viewer_asset::AssetStatus::Missing,
                                    source: viewer_asset::AssetSourceKind::Live,
                                    failure: Some(viewer_asset::AssetFetchFailureReason::Decode),
                                },
                            );
                        }
                    }
                }
                LiveFeedUpdate::TextureAssetFailed { id, reason } => {
                    let id = AssetID::new(id);
                    let mut results = self.live_texture_results.lock().unwrap();
                    results.insert(
                        id,
                        viewer_asset::AssetFetchOutcome {
                            status: viewer_asset::AssetStatus::Missing,
                            source: viewer_asset::AssetSourceKind::Live,
                            failure: Some(reason),
                        },
                    );
                }
                LiveFeedUpdate::MeshAsset { id, lod, bytes } => {
                    let key = (id, lod);
                    let asset = LiveMeshAssetBytes {
                        byte_len: bytes.len(),
                        byte_signature: mesh_byte_signature(&bytes),
                        format_hint: viewer_asset::detect_mesh_source_format(&bytes),
                        bytes,
                    };
                    self.live_mesh_assets
                        .insert(key.clone(), LiveMeshAssetState::Fetched(asset.clone()));
                    self.push_local_relay(
                        RuntimeRelayLevel::Info,
                        "mesh_asset",
                        format!(
                            "fetched id={} lod={} bytes={} signature={} format_hint={:?}",
                            key.0, key.1, asset.byte_len, asset.byte_signature, asset.format_hint
                        ),
                    );
                    self.emit_mesh_verification_event(
                        "mesh_bytes_fetched",
                        &key,
                        format!(
                            "bytes={} signature={} format_hint={:?}",
                            asset.byte_len, asset.byte_signature, asset.format_hint
                        ),
                    );
                }
                LiveFeedUpdate::MeshAssetFailed { id, lod, reason } => {
                    let key = (id, lod);
                    self.live_mesh_assets.insert(
                        key.clone(),
                        LiveMeshAssetState::Failed {
                            reason,
                            detail: String::from("live mesh fetch failed"),
                            byte_len: None,
                            byte_signature: None,
                            format: viewer_asset::MeshSourceFormat::Unknown,
                        },
                    );
                    self.push_local_relay(
                        RuntimeRelayLevel::Warn,
                        "mesh_asset",
                        format!("failed id={} lod={} reason={:?}", key.0, key.1, reason),
                    );
                }
            }
        }

        let next_live_visual_snapshot = self.live_visual_state.snapshot.clone();
        self.live_visual_state.refresh();
        let next_environment = derive_environment_from_snapshot(next_live_visual_snapshot.as_ref());
        if self.environment != next_environment {
            self.environment = next_environment;
        }
        let next_cue = derive_transition_visual_cue(next_live_visual_snapshot.as_ref());
        if self.transition_visual_state != next_cue {
            self.transition_visual_state = next_cue;
        }
        let next_world_ingestion_seam =
            WorldObjectIngestionAdapter::adapt(next_live_visual_snapshot.as_ref())
                .with_avatar_attachments(&self.world_avatars);
        let scene_update_start = Instant::now();
        if should_apply_live_visual_snapshot(
            self.last_applied_live_visual_snapshot.as_ref(),
            next_live_visual_snapshot.as_ref(),
        ) {
            self.scene
                .apply_live_visual_snapshot(next_live_visual_snapshot.as_ref());
            self.last_applied_live_visual_snapshot = next_live_visual_snapshot.clone();
        }
        if should_apply_world_ingestion_seam(
            self.last_applied_world_ingestion_seam.as_ref(),
            &next_world_ingestion_seam,
        ) {
            self.scene
                .apply_world_object_ingestion_seam(&next_world_ingestion_seam);
            self.last_applied_world_ingestion_seam = Some(next_world_ingestion_seam.clone());
        }
        apply_avatar_render_mode(
            &mut self.scene,
            &mut self.social_state,
            &self.world_avatars,
            self.renderer.avatar_render_mode(),
        );
        let scene_update_dt = scene_update_start.elapsed().as_secs_f32() * 1000.0;
        self.avg_scene_update_ms += (scene_update_dt - self.avg_scene_update_ms) * 0.15;

        self.world_ingestion_seam = next_world_ingestion_seam;
        let screenshot_path = self.next_screenshot_path()?;

        self.scene.sync_spatial();

        let aspect = self.renderer.aspect_ratio();
        let frustum = self.camera.frustum(aspect);
        let visibility_list = self.scene.query_frustum(&frustum);
        self.tick_scene_textures(&visibility_list)?;
        self.tick_scene_meshes(&visibility_list);
        self.maybe_emit_mesh_discovered_event();
        self.refresh_network_debug_snapshot_section();

        // Prepare dynamic geometry
        for &id in &visibility_list {
            let Some(geometry_source) = self
                .scene
                .instances
                .get(&id)
                .map(|instance| instance.geometry.clone())
            else {
                continue;
            };
            if self.renderer.has_dynamic_geometry(&geometry_source) {
                continue;
            }

            let mesh = match &geometry_source {
                GeometrySource::Procedural(params, detail) => {
                    Some(self.geometry_cache.get_procedural(params, *detail))
                }
                GeometrySource::Sculpt(uuid, sculpt_type) => {
                    let dummy_pixels = vec![128u8; 32 * 32 * 3];
                    Some(
                        self.geometry_cache
                            .get_sculpt(uuid, *sculpt_type, &dummy_pixels, 32, 32),
                    )
                }
                GeometrySource::Mesh(uuid, lod) => {
                    let key = (uuid.clone(), *lod);
                    let bytes = self
                        .live_mesh_assets
                        .get(&key)
                        .and_then(LiveMeshAssetState::bytes)
                        .unwrap_or(&[]);
                    let lookup = self.geometry_cache.get_mesh_with_status(uuid, *lod, bytes);
                    match lookup.state {
                        viewer_asset::MeshCacheLookupState::Decoded
                        | viewer_asset::MeshCacheLookupState::CachedReady => {
                            let next_state = match self.live_mesh_assets.get(&key).cloned() {
                                Some(LiveMeshAssetState::Fetched(asset))
                                | Some(LiveMeshAssetState::Decoded { asset, .. }) => {
                                    Some(LiveMeshAssetState::Decoded {
                                        asset,
                                        format: lookup.format,
                                        vertices: lookup.mesh.vertices.len(),
                                        submeshes: lookup.mesh.submeshes.len(),
                                    })
                                }
                                _ => None,
                            };
                            if let Some(next_state) = next_state {
                                let should_log = !matches!(
                                    self.live_mesh_assets.get(&key),
                                    Some(LiveMeshAssetState::Decoded { .. })
                                );
                                self.live_mesh_assets.insert(key.clone(), next_state);
                                if should_log {
                                    self.push_local_relay(
                                        RuntimeRelayLevel::Info,
                                        "mesh_asset",
                                        format!(
                                            "decoded id={} lod={} format={:?} vertices={} submeshes={}",
                                            key.0,
                                            key.1,
                                            lookup.format,
                                            lookup.mesh.vertices.len(),
                                            lookup.mesh.submeshes.len()
                                        ),
                                    );
                                    self.emit_mesh_verification_event(
                                        "mesh_decoded",
                                        &key,
                                        format!(
                                            "format={:?} vertices={} submeshes={}",
                                            lookup.format,
                                            lookup.mesh.vertices.len(),
                                            lookup.mesh.submeshes.len()
                                        ),
                                    );
                                }
                            }
                        }
                        viewer_asset::MeshCacheLookupState::DecodeFailed => {
                            let should_log = !matches!(
                                self.live_mesh_assets.get(&key),
                                Some(LiveMeshAssetState::Failed {
                                    reason: viewer_asset::AssetFetchFailureReason::Decode
                                        | viewer_asset::AssetFetchFailureReason::Unsupported,
                                    ..
                                })
                            );
                            if should_log {
                                self.live_mesh_assets.insert(
                                    key.clone(),
                                    LiveMeshAssetState::Failed {
                                        reason: lookup.failure.unwrap_or(
                                            viewer_asset::AssetFetchFailureReason::Decode,
                                        ),
                                        detail: lookup
                                            .detail
                                            .clone()
                                            .unwrap_or_else(|| String::from("mesh decode failed")),
                                        byte_len: Some(bytes.len()),
                                        byte_signature: Some(mesh_byte_signature(bytes)),
                                        format: lookup.format,
                                    },
                                );
                                self.push_local_relay(
                                    RuntimeRelayLevel::Warn,
                                    "mesh_asset",
                                    format!(
                                        "decode_failed id={} lod={} format={:?} detail={}",
                                        key.0,
                                        key.1,
                                        lookup.format,
                                        lookup.detail.as_deref().unwrap_or("mesh decode failed")
                                    ),
                                );
                            }
                        }
                        viewer_asset::MeshCacheLookupState::EmptyData => {}
                    }
                    Some(lookup.mesh)
                }
                _ => None,
            };

            if let Some(mesh) = mesh {
                if mesh.vertices.is_empty() || mesh.submeshes.is_empty() {
                    continue;
                }
                let mut submeshes = Vec::new();
                let mut index_start = 0;
                let mut all_indices = Vec::new();
                for sm in &mesh.submeshes {
                    if sm.indices.is_empty() {
                        continue;
                    }
                    let count = sm.indices.len() as u32;
                    submeshes.push(viewer_render::SubMeshRange {
                        face_id: sm.face_id,
                        index_start,
                        index_count: count,
                    });
                    all_indices.extend_from_slice(&sm.indices);
                    index_start += count;
                }
                if !all_indices.is_empty() && !submeshes.is_empty() {
                    self.renderer.upsert_geometry(
                        geometry_source.clone(),
                        bytemuck::cast_slice(&mesh.vertices),
                        bytemuck::cast_slice(&all_indices),
                        submeshes,
                    );
                    if let GeometrySource::Mesh(uuid, lod) = &geometry_source {
                        let key = (uuid.clone(), *lod);
                        self.emit_mesh_verification_event(
                            "geometry_uploaded",
                            &key,
                            format!(
                                "vertices={} indices={}",
                                mesh.vertices.len(),
                                all_indices.len()
                            ),
                        );
                        self.emit_mesh_verification_event(
                            "visible",
                            &key,
                            format!("instance_id={} geometry now renderable", id),
                        );
                        self.schedule_mesh_verification_screenshot(&key);
                    }

                    if let Some(instance_mut) = self.scene.get_instance_mut(id) {
                        instance_mut.local_aabb = mesh.aabb;
                        instance_mut.dirty_spatial = true;
                    }
                }
            }
        }
        let metrics = self.scene.metrics_with_visibility(&visibility_list);
        let window = self.window.clone();
        let camera = self.camera;
        let live_visual = next_live_visual_snapshot;
        let session_status = self
            .live_visual_state
            .startup_status
            .to_ux_status(&self.live_visual_state.chat_connection);
        let world_avatars = &self.world_avatars;
        let world_sim_name = self.world_sim_name.clone();
        let world_self_location = self.world_self_location;
        let profile_image_bytes = &self.profile_image_bytes;
        let ui = &mut self.ui;
        let chat_state = &mut self.chat_state;
        let social_state = &mut self.social_state;
        let profile_state = &mut self.profile_state;
        let mut pending_chat_send: Option<String> = None;
        let mut pending_direct_im_send: Option<(String, String)> = None;
        let mut pending_profile_open: Option<String> = None;
        let mut pending_profile_tab_select: Option<(String, AvatarProfileTab)> = None;
        let mut pending_profile_refresh: Option<(String, Option<AvatarProfileTab>)> = None;
        let mut pending_open_external_url: Option<String> = None;
        let mut pending_teleport_via_slurl: Option<String> = None;
        let mut pending_retry_continuity_probe = false;
        let mut pending_refresh_visible_assets = false;
        let mut pending_clear_recovery_banner = false;

        let render_result = self.renderer.render_frame(
            &camera,
            &self.scene,
            &self.environment,
            &self.transition_visual_state,
            &visibility_list,
            self.app_start_time.elapsed().as_secs_f32(),
            screenshot_path.as_deref(),
            |device, queue, encoder, target_view, surface_size| {
                let actions = ui.render(RenderInput {
                    window: &window,
                    device,
                    queue,
                    encoder,
                    target_view,
                    surface_size,
                    camera: &camera,
                    live_visual: live_visual.as_ref(),
                    session_status,
                    chat_state,
                    social_state,
                    world_avatars,
                    world_sim_name: world_sim_name.as_deref(),
                    world_self_location,
                    profile_state,
                    profile_image_bytes,
                    now_unix_ms: now_unix_ms(),
                    profile_cache_ttl_secs: self.live_visual_state.profile_cache_ttl_secs,
                    fps: self.smoothed_fps,
                    frame_ms: self.smoothed_frame_ms,
                    avg_scene_update_ms: self.avg_scene_update_ms,
                    total_instances: metrics.total_instances,
                    visible_proxies: metrics.visible_proxies,
                    fixture_texture_metrics: self.fixture_texture_cache.metrics,
                    environment: &self.environment,
                    transition_visual_state: &self.transition_visual_state,
                    show_chat_window: self.stress_test_mode != StressTestMode::Screenshot,
                    last_recovery_result: self.last_recovery_result.as_ref(),
                    can_retry_probe: match self.last_probe_retry_ms {
                        Some(last) => {
                            self.live_visual_state.in_process_enabled
                                && !self.probe_in_flight
                                && now_unix_ms().saturating_sub(last) >= RECOVERY_PROBE_COOLDOWN_MS
                        }
                        None => self.live_visual_state.in_process_enabled && !self.probe_in_flight,
                    },
                    can_refresh_assets: match self.last_asset_refresh_ms {
                        Some(last) => {
                            now_unix_ms().saturating_sub(last) >= RECOVERY_ASSET_REFRESH_COOLDOWN_MS
                        }
                        None => true,
                    },
                    network_debug: &self.network_debug,
                });
                pending_chat_send = actions.nearby_chat_send;
                pending_direct_im_send = actions.direct_im_send;
                pending_profile_open = actions.open_avatar_profile;
                pending_profile_tab_select = actions.select_avatar_profile_tab;
                pending_profile_refresh = actions.refresh_avatar_profile;
                pending_open_external_url = actions.open_external_url;
                pending_teleport_via_slurl = actions.teleport_via_slurl;
                pending_retry_continuity_probe = actions.retry_continuity_probe;
                pending_refresh_visible_assets = actions.refresh_visible_assets;
                pending_clear_recovery_banner = actions.clear_recovery_banner;
            },
        );

        if pending_retry_continuity_probe {
            let result = self.dispatch_recovery_action(
                viewer_core::RecoveryAction::RetryContinuityProbe,
                now_unix_ms(),
            );
            if result.code == viewer_core::RecoveryResultCode::Accepted
                && !self.live_visual_state.execute_continuity_probe()
            {
                self.probe_in_flight = false;
                self.last_probe_retry_ms = None;
                self.last_recovery_result = Some(viewer_core::RecoveryActionResult {
                    action: viewer_core::RecoveryAction::RetryContinuityProbe,
                    code: viewer_core::RecoveryResultCode::Unavailable,
                    detail: Some(String::from("failed to queue continuity probe command")),
                    cooldown_remaining_ms: None,
                });
            }
        }
        if pending_refresh_visible_assets {
            self.dispatch_recovery_action(
                viewer_core::RecoveryAction::RefreshVisibleAssets,
                now_unix_ms(),
            );
        }
        if pending_clear_recovery_banner {
            self.dispatch_recovery_action(
                viewer_core::RecoveryAction::ClearRecoveryBanner,
                now_unix_ms(),
            );
        }

        if let Some(text) = pending_chat_send {
            self.chat_state.mark_sending();
            self.chat_state.draft.text.clear();
            let local_id = self.chat_state.allocate_local_message_id();
            self.chat_state.push_message(ChatMessage {
                id: local_id,
                observed_at_unix_ms: now_unix_ms(),
                sender: String::from("You"),
                text: text.clone(),
                source: String::from("local"),
            });
            self.live_visual_state.send_chat(text);
        }
        if let Some((to_agent_id, text)) = pending_direct_im_send
            && !text.is_empty()
        {
            self.social_state.im_draft.clear();
            self.live_visual_state.send_direct_im(to_agent_id, text);
        }
        if let Some(avatar_id) = pending_profile_open {
            self.live_visual_state.open_avatar_profile(avatar_id);
        }
        if let Some((avatar_id, tab)) = pending_profile_tab_select
            && should_fetch_profile_tab(
                self.profile_state.as_ref(),
                &avatar_id,
                tab,
                self.live_visual_state.profile_cache_ttl_secs,
            )
        {
            self.live_visual_state
                .select_avatar_profile_tab(avatar_id, tab);
        }
        if let Some((avatar_id, tab)) = pending_profile_refresh {
            let at = now_unix_ms();
            let state = self.ensure_profile_state(&avatar_id);
            if !profile_refresh_on_cooldown(at, state.last_refresh_unix_ms) {
                state.last_refresh_unix_ms = Some(at);
                self.live_visual_state
                    .refresh_avatar_profile(avatar_id, tab);
            }
        }
        if let Some(url) = pending_open_external_url {
            open_external_url(&url);
        }
        if let Some(slurl) = pending_teleport_via_slurl {
            self.live_visual_state.teleport_via_slurl(slurl);
        }

        if let Some(path) = screenshot_path {
            tracing::info!("captured test screenshot: {}", path.display());
            if self.mesh_verification.pending_screenshot.as_ref() == Some(&path) {
                self.mesh_verification.captured_screenshot = Some(path.clone());
                self.mesh_verification.pending_screenshot = None;
                if let Some(target) = self.mesh_verification.selected_target.clone() {
                    self.emit_mesh_verification_event(
                        "screenshot_captured",
                        &target,
                        format!("path={}", path.display()),
                    );
                }
            }
        }

        render_result
    }

    fn next_screenshot_path(&mut self) -> Result<Option<PathBuf>> {
        self.frame_counter = self.frame_counter.saturating_add(1);
        if let Some(path) = self.mesh_verification.pending_screenshot.clone() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!(
                        "failed to create mesh verification screenshot directory: {}",
                        parent.display()
                    )
                })?;
            }
            return Ok(Some(path));
        }
        let Some(config) = &self.screenshot_config else {
            return Ok(None);
        };
        if self.captured_screenshots >= config.max_frames {
            return Ok(None);
        }
        if !self.frame_counter.is_multiple_of(config.every_n_frames) {
            return Ok(None);
        }

        fs::create_dir_all(&config.output_dir).with_context(|| {
            format!(
                "failed to create screenshot output directory: {}",
                config.output_dir.display()
            )
        })?;
        self.captured_screenshots = self.captured_screenshots.saturating_add(1);
        let filename = format!("viewer_test_{:04}.png", self.captured_screenshots);
        Ok(Some(config.output_dir.join(filename)))
    }

    fn tick_scene_textures(&mut self, visibility_list: &[usize]) -> Result<()> {
        // Also extract visible texture IDs from the scene (capped at 64)
        let visible_ids = extract_visible_texture_ids_from_scene(&self.scene, visibility_list, 64);
        let decoded_object_texture_ids = self
            .live_visual_state
            .snapshot
            .as_ref()
            .map(|snapshot| extract_decoded_object_feed_texture_ids(snapshot, 64))
            .unwrap_or_default();
        let mut ids_to_request = merge_texture_request_ids(&self.fixture_texture_ids, &visible_ids);
        ids_to_request = merge_texture_request_ids(&ids_to_request, &decoded_object_texture_ids);

        if ids_to_request.is_empty() {
            return Ok(());
        }

        let _attempted = self.fixture_texture_cache.poll_png_rgba8(2)?;

        let continuity = self
            .live_visual_state
            .snapshot
            .as_ref()
            .map(|s| &s.continuity);
        let prioritized_requests =
            build_asset_priority_hints(&ids_to_request, &visible_ids, continuity);

        for hint in &prioritized_requests {
            let id = &hint.id;
            if self.renderer.has_texture(id) {
                continue;
            }

            match self
                .fixture_texture_cache
                .request_png_rgba8_with_priority(id, hint.priority)?
            {
                viewer_asset::AssetStatus::Loading => {}
                viewer_asset::AssetStatus::Missing => {
                    if self.fixture_texture_missing_logged.insert(id.clone()) {
                        tracing::warn!("fixture texture missing or invalid: {id}");
                    }
                }
                viewer_asset::AssetStatus::Ready(img) => {
                    self.renderer.upsert_texture_rgba8(
                        id.clone(),
                        img.width,
                        img.height,
                        &img.rgba,
                    )?;
                }
            }
        }

        Ok(())
    }

    fn tick_scene_meshes(&mut self, _visibility_list: &[usize]) {
        let Some(tx) = &self.live_visual_state.in_process_tx else {
            return;
        };

        let mut mesh_keys = self
            .fixture_mesh_ids
            .iter()
            .map(|id| (id.clone(), 0u32))
            .collect::<Vec<_>>();
        if let Some(snapshot) = &self.live_visual_state.snapshot {
            for key in extract_decoded_object_feed_mesh_ids(snapshot, 64) {
                if !mesh_keys.contains(&key) {
                    mesh_keys.push(key);
                }
            }
        }

        for (id, lod) in mesh_keys {
            if id.trim().is_empty() {
                continue;
            }
            let key = (id.clone(), lod);
            if matches!(
                self.live_mesh_assets.get(&key),
                Some(
                    LiveMeshAssetState::Requested
                        | LiveMeshAssetState::Fetched(_)
                        | LiveMeshAssetState::Decoded { .. }
                )
            ) {
                continue;
            }

            if tx
                .send(LiveFeedCommand::RequestMesh {
                    id: id.clone(),
                    lod,
                })
                .is_ok()
            {
                self.live_mesh_assets
                    .insert((id, lod), LiveMeshAssetState::Requested);
            }
        }
    }

    fn handle_input_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed
                    && let PhysicalKey::Code(code) = event.physical_key
                {
                    apply_u09_shortcut(
                        code,
                        &mut self.ui.show_diagnostics,
                        &mut self.ui.show_social,
                        &mut self.ui.focus_continuity_requested,
                        &mut self.ui.focus_social_requested,
                    );
                }
                self.input.handle_key_event(event);
            }
            WindowEvent::MouseInput { button, state, .. } => {
                self.input.handle_mouse_button(*button, *state);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.input.handle_cursor_moved(*position);
            }
            _ => {}
        }
    }

    fn ensure_profile_state(&mut self, avatar_id: &str) -> &mut AvatarProfileState {
        let needs_reset = self
            .profile_state
            .as_ref()
            .map(|state| state.avatar_id != avatar_id)
            .unwrap_or(true);
        if needs_reset {
            self.profile_state = Some(AvatarProfileState {
                avatar_id: avatar_id.to_string(),
                ..AvatarProfileState::default()
            });
        }
        self.profile_state
            .as_mut()
            .expect("profile state should be initialized")
    }
}

fn apply_u09_shortcut(
    code: KeyCode,
    show_diagnostics: &mut bool,
    show_social: &mut bool,
    focus_continuity_requested: &mut bool,
    focus_social_requested: &mut bool,
) {
    match code {
        KeyCode::F1 => *show_diagnostics = !*show_diagnostics,
        KeyCode::F2 => {
            *show_diagnostics = true;
            *focus_continuity_requested = true;
        }
        KeyCode::F3 => {
            *show_social = true;
            *focus_social_requested = true;
        }
        _ => {}
    }
}

fn profile_refresh_on_cooldown(now_unix_ms: u64, last_refresh_unix_ms: Option<u64>) -> bool {
    let Some(last_refresh) = last_refresh_unix_ms else {
        return false;
    };
    now_unix_ms < last_refresh.saturating_add(viewer_core::PROFILE_REFRESH_COOLDOWN_MS)
}

fn extract_visible_texture_ids_from_scene(
    scene: &Scene,
    visibility_list: &[usize],
    cap: usize,
) -> Vec<AssetID> {
    if cap == 0 {
        return Vec::new();
    }

    let mut ids = std::collections::BTreeSet::new();
    'instance_loop: for &id in visibility_list {
        if let Some(instance) = scene.instances.get(&id) {
            // Default material
            let default_ids: Vec<AssetID> = instance.materials.default.texture_ids();
            for texture_id in default_ids {
                if !texture_id.is_empty() {
                    ids.insert(texture_id);
                    if ids.len() >= cap {
                        break 'instance_loop;
                    }
                }
            }
            // Per-face materials
            for mat in instance.materials.by_face.values() {
                let face_ids: Vec<AssetID> = mat.texture_ids();
                for texture_id in face_ids {
                    if !texture_id.is_empty() {
                        ids.insert(texture_id);
                        if ids.len() >= cap {
                            break 'instance_loop;
                        }
                    }
                }
            }
        }
    }
    ids.into_iter().take(cap).collect()
}

fn merge_texture_request_ids(fixture_ids: &[AssetID], visible_ids: &[AssetID]) -> Vec<AssetID> {
    let mut ids_to_request = fixture_ids.to_vec();
    for id in visible_ids {
        if !ids_to_request.contains(id) {
            ids_to_request.push(id.clone());
        }
    }
    ids_to_request
}

fn extract_decoded_object_feed_mesh_ids(
    snapshot: &LiveVisualSnapshot,
    cap: usize,
) -> Vec<(String, u32)> {
    if cap == 0 {
        return Vec::new();
    }

    let mut ids = std::collections::BTreeSet::new();
    for obj in &snapshot.decoded_object_feed_objects {
        if let Some(mesh_id) = &obj.mesh_id {
            let normalized = mesh_id.trim().to_ascii_lowercase();
            let object_id_matches = obj
                .object_id
                .as_deref()
                .map(|id| id.trim().eq_ignore_ascii_case(normalized.as_str()))
                .unwrap_or(false);
            if !normalized.is_empty() && !object_id_matches {
                ids.insert((normalized, 0u32));
            }
        }
    }
    ids.into_iter().take(cap).collect()
}

fn extract_decoded_object_feed_texture_ids(
    snapshot: &LiveVisualSnapshot,
    cap: usize,
) -> Vec<AssetID> {
    if cap == 0 {
        return Vec::new();
    }

    let mut ids = std::collections::BTreeSet::new();
    for obj in &snapshot.decoded_object_feed_objects {
        if let Some(texture_id) = obj.texture_id.as_ref() {
            let trimmed = texture_id.as_str().trim();
            if !trimmed.is_empty() {
                ids.insert(AssetID::new(trimmed));
                if ids.len() >= cap {
                    break;
                }
            }
        }
        if let Some(face) = obj.default_face_material.as_ref() {
            insert_face_material_texture_ids(face, &mut ids, cap);
            if ids.len() >= cap {
                break;
            }
        }
        for face in &obj.face_material_overrides {
            insert_face_material_texture_ids(face, &mut ids, cap);
            if ids.len() >= cap {
                break;
            }
        }
    }
    ids.into_iter().take(cap).collect()
}

fn insert_face_material_texture_ids(
    face: &viewer_core::DecodedWorldObjectFaceMaterial,
    ids: &mut std::collections::BTreeSet<AssetID>,
    cap: usize,
) {
    for texture_id in [&face.texture_id, &face.normal_id, &face.specular_id]
        .into_iter()
        .flatten()
    {
        let trimmed = texture_id.as_str().trim();
        if !trimmed.is_empty() {
            ids.insert(AssetID::new(trimmed));
            if ids.len() >= cap {
                return;
            }
        }
    }
}

fn build_asset_priority_hints(
    ids_to_request: &[AssetID],
    visible_ids: &[AssetID],
    continuity: Option<&RegionContinuitySummary>,
) -> Vec<AssetPriorityHint> {
    let visible: HashSet<AssetID> = visible_ids.iter().cloned().collect();
    let has_previous = continuity.and_then(|c| c.previous_region_coords).is_some();
    let neighbor_scope_count = continuity
        .map(|c| {
            c.neighbors
                .len()
                .min(viewer_asset::texture_fixture::A10_MAX_NEIGHBOR_SCOPES)
        })
        .unwrap_or(0);

    let mut active_assigned = 0usize;
    let mut previous_assigned = 0usize;
    let mut neighbor_assigned = 0usize;
    let mut hints = Vec::with_capacity(ids_to_request.len());

    for id in ids_to_request {
        let priority = if visible.contains(id)
            && active_assigned < viewer_asset::texture_fixture::A10_QUOTA_ACTIVE
        {
            active_assigned += 1;
            viewer_core::AssetPriority::Active
        } else if has_previous
            && previous_assigned < viewer_asset::texture_fixture::A10_QUOTA_PREVIOUS
        {
            previous_assigned += 1;
            viewer_core::AssetPriority::Previous
        } else if neighbor_scope_count > 0
            && neighbor_assigned < viewer_asset::texture_fixture::A10_QUOTA_NEIGHBOR
        {
            neighbor_assigned += 1;
            viewer_core::AssetPriority::Neighbor
        } else {
            viewer_core::AssetPriority::Normal
        };

        hints.push(AssetPriorityHint {
            id: id.clone(),
            priority,
        });
    }

    hints
}

impl InputState {
    fn handle_key_event(&mut self, event: &KeyEvent) {
        let pressed = event.state == ElementState::Pressed;

        let PhysicalKey::Code(code) = event.physical_key else {
            return;
        };

        match code {
            KeyCode::KeyW => self.move_forward = pressed,
            KeyCode::KeyS => self.move_backward = pressed,
            KeyCode::KeyA => self.move_left = pressed,
            KeyCode::KeyD => self.move_right = pressed,
            KeyCode::KeyE => self.move_up = pressed,
            KeyCode::KeyQ => self.move_down = pressed,
            _ => {}
        }
    }

    fn handle_mouse_button(&mut self, button: MouseButton, state: ElementState) {
        if button != MouseButton::Right {
            return;
        }

        self.mouse_look_active = state == ElementState::Pressed;
        self.last_cursor_pos = None;
    }

    fn handle_cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        if !self.mouse_look_active {
            self.last_cursor_pos = Some(position);
            return;
        }

        if let Some(last) = self.last_cursor_pos {
            let sensitivity = 0.0025;
            let dx = (position.x - last.x) as f32;
            let dy = (position.y - last.y) as f32;

            self.pending_look_delta[0] += -dx * sensitivity;
            self.pending_look_delta[1] += -dy * sensitivity;
        }

        self.last_cursor_pos = Some(position);
    }

    fn take_look_delta(&mut self) -> [f32; 2] {
        let delta = self.pending_look_delta;
        self.pending_look_delta = [0.0, 0.0];
        delta
    }

    fn update_camera(&self, camera: &mut Camera, dt_seconds: f32) {
        let forward =
            ((self.move_forward as i8 - self.move_backward as i8) as f32) * 3.5 * dt_seconds;
        let right = ((self.move_right as i8 - self.move_left as i8) as f32) * 3.5 * dt_seconds;
        let up = ((self.move_up as i8 - self.move_down as i8) as f32) * 3.5 * dt_seconds;

        camera.move_local(forward, right, up);
    }
}

impl ApplicationHandler for ViewerApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_none() {
            match Self::init_state(event_loop) {
                Ok(state) => self.state = Some(state),
                Err(err) => {
                    eprintln!("startup failed: {err:#}");
                    event_loop.exit();
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = self.state.as_mut() else {
            return;
        };

        if state.window.id() != window_id {
            return;
        }

        let ui_consumed = state.ui.handle_event(&state.window, &event);
        if ui_consumed {
            state.window.request_redraw();
            return;
        }

        state.handle_input_event(&event);

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                state.resize(size);
                state.window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                state.window.request_redraw();
            }
            WindowEvent::KeyboardInput { .. }
            | WindowEvent::MouseInput { .. }
            | WindowEvent::CursorMoved { .. } => {
                state.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                if let Err(err) = state.redraw() {
                    eprintln!("redraw failed: {err:#}");
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = self.state.as_ref() {
            state.window.request_redraw();
        }
    }
}

fn auto_camera_config_from_lookup<F>(lookup: F) -> AutoCameraConfig
where
    F: Fn(&str) -> Option<String>,
{
    let mut config = AutoCameraConfig::default();
    if let Some(center) = lookup("VIEWER_TEST_CAMERA_CENTER")
        .as_deref()
        .and_then(parse_vec3_csv)
    {
        config.center = center;
    }
    if let Some(value) = lookup("VIEWER_TEST_CAMERA_RADIUS").as_deref() {
        config.radius = parse_positive_f32(value, config.radius);
    }
    if let Some(value) = lookup("VIEWER_TEST_CAMERA_HEIGHT").as_deref() {
        config.orbit_height = value.parse::<f32>().ok().unwrap_or(config.orbit_height);
    }
    if let Some(value) = lookup("VIEWER_TEST_CAMERA_LOOK_HEIGHT").as_deref() {
        config.look_height = value.parse::<f32>().ok().unwrap_or(config.look_height);
    }
    if let Some(value) = lookup("VIEWER_TEST_CAMERA_SPEED").as_deref() {
        config.angular_speed_radians =
            parse_positive_f32(value, config.angular_speed_radians).clamp(0.01, 6.0);
    }
    if let Some(value) = lookup("VIEWER_TEST_CAMERA_PHASE").as_deref() {
        config.phase_radians = value.parse::<f32>().ok().unwrap_or(config.phase_radians);
    }
    if let Some(path_file) = lookup("VIEWER_TEST_CAMERA_PATH_FILE").as_deref() {
        match load_camera_path_script(PathBuf::from(path_file)) {
            Ok(script) => config.path_script = Some(script),
            Err(err) => tracing::warn!("invalid VIEWER_TEST_CAMERA_PATH_FILE ({path_file}): {err}"),
        }
    }
    config
}

fn screenshot_config_from_lookup<F>(lookup: F, mode: StressTestMode) -> Option<ScreenshotConfig>
where
    F: Fn(&str) -> Option<String>,
{
    if mode != StressTestMode::Screenshot {
        return None;
    }

    let output_dir = lookup("VIEWER_TEST_SCREENSHOT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("artifacts/screenshots"));
    let every_n_frames = lookup("VIEWER_TEST_SCREENSHOT_EVERY_N_FRAMES")
        .as_deref()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(30)
        .max(1);
    let max_frames = lookup("VIEWER_TEST_SCREENSHOT_MAX_FRAMES")
        .as_deref()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(12)
        .max(1);

    Some(ScreenshotConfig {
        output_dir,
        every_n_frames,
        max_frames,
    })
}

fn mesh_verification_state_from_lookup<F>(lookup: F) -> MeshVerificationState
where
    F: Fn(&str) -> Option<String>,
{
    let enabled = lookup("VIEWER_APP_MESH_VERIFY")
        .as_deref()
        .map(parse_bool_like)
        .unwrap_or(false);
    if !enabled {
        return MeshVerificationState::default();
    }

    let target_mesh_id = lookup("VIEWER_APP_MESH_VERIFY_ID").and_then(|value| {
        let normalized = value.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            None
        } else {
            Some((normalized, 0))
        }
    });
    let log_path = lookup("VIEWER_APP_MESH_VERIFY_LOG_PATH")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| PathBuf::from("artifacts/logs/mesh_visibility_verify.jsonl"));
    let screenshot_dir = lookup("VIEWER_APP_MESH_VERIFY_SCREENSHOT_DIR")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty());

    MeshVerificationState {
        config: Some(MeshVerificationConfig {
            target_mesh_id,
            log_path,
            screenshot_dir,
        }),
        ..Default::default()
    }
}

fn mesh_byte_signature(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::from("empty");
    }
    bytes
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

fn live_mesh_queue_counts(
    assets: &BTreeMap<(String, u32), LiveMeshAssetState>,
) -> (usize, usize, usize, usize) {
    let mut requested = 0usize;
    let mut fetched = 0usize;
    let mut decoded = 0usize;
    let mut failed = 0usize;

    for state in assets.values() {
        match state {
            LiveMeshAssetState::Requested => requested += 1,
            LiveMeshAssetState::Fetched(_) => fetched += 1,
            LiveMeshAssetState::Decoded { .. } => decoded += 1,
            LiveMeshAssetState::Failed { .. } => failed += 1,
        }
    }

    (requested, fetched, decoded, failed)
}

fn live_mesh_asset_summary_line(assets: &BTreeMap<(String, u32), LiveMeshAssetState>) -> String {
    let Some(((id, lod), state)) = assets.iter().next_back() else {
        return String::from("mesh_last=none");
    };
    match state {
        LiveMeshAssetState::Requested => format!("mesh_last=requested id={id} lod={lod}"),
        LiveMeshAssetState::Fetched(asset) => format!(
            "mesh_last=fetched id={id} lod={lod} bytes={} signature={} format_hint={:?}",
            asset.byte_len, asset.byte_signature, asset.format_hint
        ),
        LiveMeshAssetState::Decoded {
            asset,
            format,
            vertices,
            submeshes,
        } => format!(
            "mesh_last=decoded id={id} lod={lod} bytes={} signature={} format={:?} vertices={} submeshes={}",
            asset.byte_len, asset.byte_signature, format, vertices, submeshes
        ),
        LiveMeshAssetState::Failed {
            reason,
            detail,
            byte_len,
            byte_signature,
            format,
        } => format!(
            "mesh_last=failed id={id} lod={lod} reason={:?} format={:?} bytes={} signature={} detail={}",
            reason,
            format,
            byte_len
                .map(|value| value.to_string())
                .unwrap_or_else(|| String::from("none")),
            byte_signature.as_deref().unwrap_or("none"),
            detail
        ),
    }
}

fn parse_positive_f32(value: &str, fallback: f32) -> f32 {
    let parsed = value.parse::<f32>().ok().unwrap_or(fallback);
    if parsed.is_finite() && parsed > 0.0 {
        parsed
    } else {
        fallback
    }
}

fn parse_vec3_csv(value: &str) -> Option<[f32; 3]> {
    let mut parts = value.split(',').map(|part| part.trim().parse::<f32>().ok());
    let x = parts.next().flatten()?;
    let y = parts.next().flatten()?;
    let z = parts.next().flatten()?;
    Some([x, y, z])
}

fn apply_camera_look_at(camera: &mut Camera, position: [f32; 3], target: [f32; 3]) {
    let to_target = [
        target[0] - position[0],
        target[1] - position[1],
        target[2] - position[2],
    ];
    let horizontal = (to_target[0] * to_target[0] + to_target[2] * to_target[2]).sqrt();
    let yaw = to_target[2].atan2(to_target[0]);
    let pitch = to_target[1].atan2(horizontal).clamp(-1.553343, 1.553343);
    camera.position = position;
    camera.yaw = yaw;
    camera.pitch = pitch;
}

fn lerp_vec3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn load_camera_path_script(path: PathBuf) -> Result<CameraPathScript> {
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read camera path file: {}", path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse camera path json: {}", path.display()))?;
    parse_camera_path_script_value(value)
}

fn parse_camera_path_script_value(value: serde_json::Value) -> Result<CameraPathScript> {
    let Some(list) = value.as_array() else {
        anyhow::bail!("camera path must be a JSON array");
    };
    if list.len() < 2 {
        anyhow::bail!("camera path must contain at least 2 waypoints");
    }

    let mut waypoints = Vec::with_capacity(list.len());
    for entry in list {
        let Some(obj) = entry.as_object() else {
            anyhow::bail!("camera waypoint must be a JSON object");
        };
        let time_sec = obj
            .get("time_sec")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32)
            .filter(|v| v.is_finite() && *v >= 0.0)
            .context("waypoint.time_sec must be finite and >= 0")?;
        let position = parse_json_vec3(obj.get("position").context("missing waypoint.position")?)
            .context("invalid waypoint.position")?;
        let look_at = parse_json_vec3(obj.get("look_at").context("missing waypoint.look_at")?)
            .context("invalid waypoint.look_at")?;
        waypoints.push(CameraPathWaypoint {
            time_sec,
            position,
            look_at,
        });
    }

    waypoints.sort_by(|a, b| a.time_sec.total_cmp(&b.time_sec));
    for pair in waypoints.windows(2) {
        if pair[1].time_sec <= pair[0].time_sec {
            anyhow::bail!("camera path waypoint time_sec values must be strictly increasing");
        }
    }

    let duration_sec = waypoints
        .last()
        .map(|w| w.time_sec)
        .filter(|v| *v > 0.0)
        .context("camera path duration must be > 0")?;

    Ok(CameraPathScript {
        waypoints,
        duration_sec,
    })
}

fn parse_json_vec3(value: &serde_json::Value) -> Option<[f32; 3]> {
    let array = value.as_array()?;
    if array.len() != 3 {
        return None;
    }
    let x = array[0].as_f64()? as f32;
    let y = array[1].as_f64()? as f32;
    let z = array[2].as_f64()? as f32;
    if x.is_finite() && y.is_finite() && z.is_finite() {
        Some([x, y, z])
    } else {
        None
    }
}

fn fixture_texture_ids_from_env() -> Vec<viewer_core::AssetID> {
    let raw = std::env::var("VIEWER_FIXTURE_TEXTURES").ok();
    let Some(raw) = raw else {
        return Vec::new();
    };

    let raw = raw.trim();
    if raw.is_empty() || raw == "0" {
        return Vec::new();
    }

    if matches!(raw, "1" | "true" | "yes" | "on") {
        return vec![
            viewer_core::AssetID::new("water_diffuse"),
            viewer_core::AssetID::new("stone_diffuse"),
            viewer_core::AssetID::new("stone_normal"),
        ];
    }

    raw.split(',')
        .map(|s| viewer_core::AssetID::new(s.trim()))
        .filter(|id| !id.is_empty())
        .collect()
}

fn fixture_mesh_ids_from_env() -> Vec<String> {
    let raw = std::env::var("VIEWER_FIXTURE_MESHES").ok();
    let Some(raw) = raw else {
        return Vec::new();
    };

    let raw = raw.trim();
    if raw.is_empty() || raw == "0" {
        return Vec::new();
    }

    fixture_mesh_ids_from_csv(raw)
}

fn fixture_mesh_ids_from_csv(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|id| !id.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use viewer_core::{
        AlphaMode, AssetID, GeometrySource, InstanceRole, MaterialDescriptor, MaterialSet,
        MeshKind, RenderableInstance, TextureEntry, Transform,
    };
    use viewer_grid::{
        GridLoginError, GridLoginErrorClass, LoginIntent, SecondLifeAdapter, StartLocation,
        StartLocationIntent,
    };

    #[test]
    fn in_process_config_requires_endpoint_username_and_password() {
        let vars = HashMap::<String, String>::new();
        let cfg = in_process_live_feed_config_from_lookup(|k| vars.get(k).cloned());
        assert!(cfg.is_none());
    }

    #[test]
    fn in_process_config_parses_defaults_and_overrides() {
        let mut vars = HashMap::<String, String>::new();
        vars.insert(
            String::from("VIEWER_LOGIN_ENDPOINT"),
            String::from("https://example.invalid/login"),
        );
        vars.insert(String::from("VIEWER_LOGIN_USERNAME"), String::from("user"));
        vars.insert(String::from("VIEWER_LOGIN_PASSWORD"), String::from("pass"));
        vars.insert(
            String::from("VIEWER_LOGIN_WIRE_FORMAT"),
            String::from("xmlrpc"),
        );
        vars.insert(
            String::from("VIEWER_FIRST_SIM_RECEIVE_MAX_PACKETS"),
            String::from("16"),
        );
        vars.insert(
            String::from("VIEWER_APP_IN_PROCESS_PROBE"),
            String::from("false"),
        );
        vars.insert(
            String::from("VIEWER_APP_EVENT_QUEUE_FAILURES_BEFORE_RECONNECT"),
            String::from("9"),
        );
        vars.insert(
            String::from("VIEWER_APP_EVENT_QUEUE_CAP_NOT_FOUND_BEFORE_RECONNECT"),
            String::from("4"),
        );
        vars.insert(
            String::from("VIEWER_APP_CAPABILITY_PROBES_REQUIRE_EVENT_QUEUE_OK"),
            String::from("false"),
        );
        vars.insert(
            String::from("VIEWER_APP_LANE_PROBE_ASSET_IDS"),
            String::from(
                "11111111-1111-1111-1111-111111111111,22222222-2222-2222-2222-222222222222",
            ),
        );
        vars.insert(
            String::from("VIEWER_APP_OBJECT_UUID_FOCUS"),
            String::from("10930d3b-1821-c584-a0c7-28a34999800d"),
        );
        vars.insert(
            String::from("VIEWER_APP_WORKER_TICK_MS"),
            String::from("25"),
        );
        vars.insert(
            String::from("VIEWER_APP_AUTO_TELEPORT_SLURL"),
            String::from("secondlife://Ahern/50/60/70"),
        );
        vars.insert(
            String::from("VIEWER_APP_AUTO_TELEPORT_DELAY_TICKS"),
            String::from("77"),
        );
        vars.insert(
            String::from("VIEWER_APP_REGION_OBJECTS_REPROBE_DELAY_TICKS"),
            String::from("99"),
        );
        vars.insert(
            String::from("VIEWER_APP_LLUDP_STARTUP_PARITY_BUNDLE"),
            String::from("true"),
        );
        vars.insert(
            String::from("VIEWER_APP_REQUIRE_REGION_HANDSHAKE_REPLY"),
            String::from("true"),
        );
        vars.insert(
            String::from("VIEWER_APP_AGENT_UPDATE_FAR"),
            String::from("222.5"),
        );
        vars.insert(
            String::from("VIEWER_APP_AGENT_UPDATE_KEEPALIVE_TICKS"),
            String::from("7"),
        );
        vars.insert(String::from("VIEWER_LOGIN_AGREE_TOS"), String::from("true"));
        vars.insert(
            String::from("VIEWER_LOGIN_READ_CRITICAL"),
            String::from("false"),
        );
        vars.insert(
            String::from("VIEWER_LOGIN_MFA_TOKEN"),
            String::from("token123"),
        );
        vars.insert(
            String::from("VIEWER_ASSET_LIVE_TIMEOUT_MS"),
            String::from("9000"),
        );
        let cfg = in_process_live_feed_config_from_lookup(|k| vars.get(k).cloned())
            .expect("config should parse");
        assert_eq!(cfg.wire_format, LoginWireFormat::XmlRpc);
        assert_eq!(cfg.receive_max_packets, 16);
        assert!(!cfg.run_probe);
        assert_eq!(cfg.event_queue_failures_before_reconnect, 9);
        assert_eq!(cfg.event_queue_cap_not_found_before_reconnect, 4);
        assert!(!cfg.capability_probes_require_event_queue_ok);
        assert_eq!(cfg.lane_probe_asset_ids.len(), 2);
        assert_eq!(
            cfg.object_uuid_focus.as_deref(),
            Some("10930d3b-1821-c584-a0c7-28a34999800d")
        );
        assert_eq!(cfg.worker_tick_ms, 25);
        assert_eq!(
            cfg.auto_teleport_slurl.as_deref(),
            Some("secondlife://Ahern/50/60/70")
        );
        assert_eq!(cfg.auto_teleport_delay_ticks, 77);
        assert_eq!(cfg.region_objects_reprobe_delay_ticks, 99);
        assert!(cfg.lludp_startup_parity_bundle);
        assert!(cfg.require_region_handshake_reply);
        assert_eq!(cfg.agent_update_far, 222.5);
        assert_eq!(cfg.agent_update_keepalive_ticks_override, Some(7));
        assert!(cfg.agree_to_tos);
        assert!(!cfg.read_critical);
        assert_eq!(cfg.mfa_token.as_deref(), Some("token123"));
        assert_eq!(cfg.asset_live_timeout_ms, 9000);
    }

    #[test]
    fn agent_update_keepalive_interval_tracks_worker_tick_budget() {
        let config = InProcessLiveFeedConfig {
            worker_tick_ms: 60,
            ..sample_in_process_config()
        };
        assert_eq!(agent_update_keepalive_interval_ticks(&config), 17);

        let fast_config = InProcessLiveFeedConfig {
            worker_tick_ms: 250,
            ..sample_in_process_config()
        };
        assert_eq!(agent_update_keepalive_interval_ticks(&fast_config), 4);

        let override_config = InProcessLiveFeedConfig {
            agent_update_keepalive_ticks_override: Some(3),
            ..sample_in_process_config()
        };
        assert_eq!(agent_update_keepalive_interval_ticks(&override_config), 3);
    }

    #[test]
    fn agent_update_keepalive_scheduler_is_deterministic() {
        assert!(should_send_agent_update_keepalive(0, None, 5));
        assert!(!should_send_agent_update_keepalive(4, Some(0), 5));
        assert!(should_send_agent_update_keepalive(5, Some(0), 5));
        assert!(should_send_agent_update_keepalive(10, Some(5), 0));
    }

    #[test]
    fn stress_test_mode_parses_single_live_texture_center_aliases() {
        assert_eq!(
            StressTestMode::from_value(Some("live_texture")),
            StressTestMode::SingleLiveTextureCenter
        );
        assert_eq!(
            StressTestMode::from_value(Some("single_live_texture")),
            StressTestMode::SingleLiveTextureCenter
        );
        assert_eq!(
            StressTestMode::from_value(Some("7")),
            StressTestMode::SingleLiveTextureCenter
        );
    }

    #[test]
    fn fixture_mesh_ids_from_csv_normalizes_and_filters() {
        let parsed = fixture_mesh_ids_from_csv(" abc ,DEF,, 123 ");
        assert_eq!(parsed, vec!["abc", "def", "123"]);
    }

    #[test]
    fn parse_start_location_maps_home_last_and_uri() {
        assert_eq!(
            parse_start_location("home"),
            StartLocationIntent::Saved(StartLocation::Home)
        );
        assert_eq!(
            parse_start_location("last"),
            StartLocationIntent::Saved(StartLocation::Last)
        );
        assert_eq!(
            parse_start_location("my://region/128/128/25"),
            StartLocationIntent::Uri(String::from("my://region/128/128/25"))
        );
    }

    #[test]
    fn parse_start_location_normalizes_supported_slurls() {
        assert_eq!(
            parse_start_location("secondlife://Ahern/50/60/70"),
            StartLocationIntent::Uri(String::from("uri:Ahern&50&60&70"))
        );
        assert_eq!(
            parse_start_location("secondlife:///app/teleport/A%27ksha%20Oasis/41/166/701"),
            StartLocationIntent::Uri(String::from("uri:A'ksha Oasis&41&166&701"))
        );
        assert_eq!(
            parse_start_location(
                "https://maps.secondlife.com/secondlife/Burning%20Life%20(Hyper)/27/210/30"
            ),
            StartLocationIntent::Uri(String::from("uri:Burning Life (Hyper)&27&210&30"))
        );
    }

    #[test]
    fn normalize_start_location_input_rejects_unsupported_url_scheme() {
        let err = normalize_start_location_input("https://example.com/not-a-slurl")
            .expect_err("non-SLURL http URL should be rejected");
        assert!(err.to_string().contains("unsupported SLURL"));
    }

    #[test]
    fn format_capability_host_family_tag_prefers_host_label() {
        assert_eq!(
            format_capability_host_family_tag(
                "https://simhost-04e63a701b66ed282.agni.secondlife.io:12043/cap/region"
            ),
            String::from("host_family=simhost-04e63a701b66ed282")
        );
    }

    #[test]
    fn summarize_non_baseline_caps_by_host_excludes_object_ingress_baseline() {
        let entries = BTreeMap::from([
            (
                String::from("EventQueueGet"),
                String::from("https://simhost-aaa.agni.secondlife.io:12043/cap/event"),
            ),
            (
                String::from("RegionObjects"),
                String::from("https://simhost-aaa.agni.secondlife.io:12043/cap/regionobjects"),
            ),
            (
                String::from("GetDisplayNames"),
                String::from("https://simhost-aaa.agni.secondlife.io:12043/cap/displaynames"),
            ),
            (
                String::from("GetTexture"),
                String::from("http://asset-cdn.glb.agni.lindenlab.com/?texture_id=test"),
            ),
            (
                String::from("ViewerAsset"),
                String::from("http://asset-cdn.glb.agni.lindenlab.com/cap/viewerasset"),
            ),
        ]);

        let summary = summarize_non_baseline_caps_by_host(&entries, 16);
        assert!(summary.contains("simhost-aaa:count=1"));
        assert!(summary.contains("names=GetDisplayNames"));
        assert!(summary.contains("asset-cdn:count=2"));
        assert!(summary.contains("GetTexture"));
        assert!(summary.contains("ViewerAsset"));
        assert!(!summary.contains("EventQueueGet"));
        assert!(!summary.contains("RegionObjects"));
    }

    #[test]
    fn extract_enable_simulator_endpoint_prefers_sim_ip_and_port() {
        let target = viewer_net::EventQueueSimulatorTarget {
            message: String::from("EnableSimulator"),
            sim_ip_and_port: Some(String::from("16.144.39.130:13001")),
            ip: Some(String::from("1.2.3.4")),
            port: Some(String::from("1234")),
            ..Default::default()
        };
        let endpoint = extract_enable_simulator_endpoint(&target)
            .expect("endpoint should decode from sim_ip_and_port");
        assert_eq!(
            endpoint,
            EnableSimulatorEndpoint::IpPort {
                sim_ip: String::from("16.144.39.130"),
                port: 13001,
                source: "sim_ip_and_port",
            }
        );
    }

    #[test]
    fn extract_enable_simulator_endpoint_falls_back_to_port_only() {
        let target = viewer_net::EventQueueSimulatorTarget {
            message: String::from("EnableSimulator"),
            port: Some(String::from("13028")),
            ..Default::default()
        };
        let endpoint = extract_enable_simulator_endpoint(&target)
            .expect("endpoint should decode from port fallback");
        assert_eq!(endpoint, EnableSimulatorEndpoint::PortOnly { port: 13028 });
    }

    #[test]
    fn extract_enable_simulator_endpoint_uses_viewer_net_decoded_endpoint() {
        let target = viewer_net::EventQueueSimulatorTarget {
            message: String::from("EnableSimulator"),
            endpoint_ip: Some(String::from("16.144.39.130")),
            endpoint_port: Some(13001),
            endpoint_source: Some(String::from("simulatorinfo_binary_ip_port")),
            ..Default::default()
        };
        let endpoint = extract_enable_simulator_endpoint(&target)
            .expect("endpoint should use viewer_net decoded endpoint");
        assert_eq!(
            endpoint,
            EnableSimulatorEndpoint::IpPort {
                sim_ip: String::from("16.144.39.130"),
                port: 13001,
                source: "simulatorinfo_binary_ip_port",
            }
        );
    }

    #[test]
    fn build_lane_probe_shape_matrix_splits_immediate_and_gated_tasks() {
        let entries = BTreeMap::from([
            (
                String::from("SimulatorFeatures"),
                String::from("https://simhost-aaa.agni.secondlife.io:12043/cap/simfeatures"),
            ),
            (
                String::from("InterestList"),
                String::from("https://simhost-aaa.agni.secondlife.io:12043/cap/interest"),
            ),
            (
                String::from("UntrustedSimulatorMessage"),
                String::from("https://simhost-aaa.agni.secondlife.io:12043/cap/untrusted"),
            ),
            (
                String::from("ViewerAsset"),
                String::from("http://asset-cdn.glb.agni.lindenlab.com/cap/viewerasset"),
            ),
        ]);
        let mut cfg = sample_in_process_config();
        cfg.lane_probe_asset_ids = vec![
            String::from("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa"),
            String::from("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb"),
        ];

        let (immediate, gated) = build_lane_probe_shape_matrix(&entries, &cfg);
        assert!(!immediate.is_empty());
        assert!(!gated.is_empty());
        assert!(
            immediate
                .iter()
                .all(|task| task.capability_name == "ViewerAsset")
        );
        assert!(
            gated
                .iter()
                .any(|task| task.capability_name == "SimulatorFeatures")
        );
        assert!(
            gated
                .iter()
                .any(|task| task.capability_name == "InterestList")
        );
        assert!(
            gated
                .iter()
                .any(|task| task.capability_name == "UntrustedSimulatorMessage")
        );
    }

    #[test]
    fn summarize_region_objects_inspection_includes_typed_sample_summary() {
        let mut inspection = RegionObjectsInspection::default();
        inspection
            .typed_object_samples
            .push(viewer_net::RegionObjectsTypedObjectSample {
                object_id: String::from("11111111-1111-1111-1111-111111111111"),
                profile: String::from("pathfinding_linkset"),
                name: Some(String::from("bamboo")),
                owner: Some(String::from("owner-1")),
                position: Some(String::from("3|230|3800")),
                description_shape: Some(String::from("free_text")),
                linkset_use: Some(String::from("dynamic_obstacle")),
                walkability_coefficients: Some([100, 100, 100, 100]),
                landimpact: Some(2),
            });
        inspection.candidate_mesh_asset_ids =
            vec![String::from("947d4505-eb76-2ef5-c049-e7882881d689")];

        let summary = summarize_region_objects_inspection(&inspection);
        assert!(summary.contains("typed_sample="));
        assert!(summary.contains("name=bamboo"));
        assert!(summary.contains("linkset_use=dynamic_obstacle"));
        assert!(summary.contains("walkability=100/100/100/100"));
        assert!(summary.contains("description_shape=free_text"));
        assert!(summary.contains("landimpact=2"));
        assert!(summary.contains("mesh_candidates=947d4505-eb76-2ef5-c049-e7882881d689"));
    }

    fn sample_in_process_config() -> InProcessLiveFeedConfig {
        InProcessLiveFeedConfig {
            endpoint: String::from("https://example.invalid/login"),
            username: String::from("user"),
            password: String::from("pass"),
            connect_timeout_secs: 15,
            wire_format: LoginWireFormat::XmlRpc,
            start_location: StartLocationIntent::Saved(StartLocation::Last),
            agree_to_tos: false,
            read_critical: true,
            mfa_token: None,
            receive_bind: String::from("0.0.0.0:0"),
            receive_timeout_secs: 5,
            receive_max_packets: 8,
            post_movement_tail_packets: 4,
            post_movement_timeout_secs: None,
            stop_on_region_control: false,
            auto_teleport_slurl: None,
            auto_teleport_delay_ticks: 40,
            region_objects_reprobe_delay_ticks: 80,
            lludp_startup_parity_bundle: false,
            require_region_handshake_reply: false,
            agent_update_far: DEFAULT_AGENT_UPDATE_FAR,
            agent_update_keepalive_ticks_override: None,
            run_probe: true,
            worker_tick_ms: 60,
            event_queue_poll_timeout_ms: 45_000,
            event_queue_poll_every_ticks: 10,
            event_queue_failures_before_reconnect: 0,
            event_queue_cap_not_found_before_reconnect: 3,
            capability_probes_require_event_queue_ok: true,
            lane_probe_asset_ids: Vec::new(),
            object_uuid_focus: None,
            social_poll_timeout_ms: 35,
            social_poll_max_packets: 4,
            nearby_poll_timeout_ms: 40,
            nearby_poll_max_packets: 2,
            nearby_send_receive_timeout_ms: 40,
            nearby_send_receive_packets: 0,
            profile_cache_ttl_secs: 120,
            asset_live_timeout_ms: 10_000,
        }
    }

    #[test]
    fn parse_bool_like_accepts_expected_truthy_forms() {
        assert!(parse_bool_like("true"));
        assert!(parse_bool_like("1"));
        assert!(parse_bool_like("YES"));
        assert!(!parse_bool_like("false"));
        assert!(!parse_bool_like("0"));
    }

    #[test]
    fn canonical_uuid_like_validator_is_strict() {
        assert!(is_canonical_uuid_like(
            "10930d3b-1821-c584-a0c7-28a34999800d"
        ));
        assert!(!is_canonical_uuid_like("10930d3b1821c584a0c728a34999800d"));
        assert!(!is_canonical_uuid_like("not-a-uuid"));
    }

    #[test]
    fn focus_filter_keeps_only_matching_object_id() {
        let mut cfg = sample_in_process_config();
        cfg.object_uuid_focus = Some(String::from("10930d3b-1821-c584-a0c7-28a34999800d"));
        let mut objects = vec![
            viewer_core::DecodedWorldObjectFeedObject {
                local_id: 1,
                scale_centi: Some([100, 100, 100]),
                position_centi: None,
                rotation_quat_i16: None,
                mesh_id: None,
                texture_id: None,
                default_face_material: None,
                face_material_overrides: Vec::new(),
                object_id: Some(String::from("10930d3b-1821-c584-a0c7-28a34999800d")),
            },
            viewer_core::DecodedWorldObjectFeedObject {
                local_id: 2,
                scale_centi: Some([100, 100, 100]),
                position_centi: None,
                rotation_quat_i16: None,
                mesh_id: None,
                texture_id: None,
                default_face_material: None,
                face_material_overrides: Vec::new(),
                object_id: Some(String::from("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa")),
            },
            viewer_core::DecodedWorldObjectFeedObject {
                local_id: 3,
                scale_centi: Some([100, 100, 100]),
                position_centi: None,
                rotation_quat_i16: None,
                mesh_id: None,
                texture_id: None,
                default_face_material: None,
                face_material_overrides: Vec::new(),
                object_id: None,
            },
        ];

        filter_object_feed_objects_by_focus(&mut objects, &cfg);

        assert_eq!(objects.len(), 1);
        assert_eq!(
            objects[0].object_id.as_deref(),
            Some("10930d3b-1821-c584-a0c7-28a34999800d")
        );
    }

    #[test]
    fn event_queue_cap_not_found_detection_is_bounded() {
        assert!(is_event_queue_cap_not_found_error(
            "http status 404 Not Found: cap not found: 'abc'"
        ));
        assert!(!is_event_queue_cap_not_found_error(
            "transport error: timeout waiting for response"
        ));
    }

    #[test]
    fn map_capability_reprime_from_seed_caps_extracts_event_queue_and_probe_urls() {
        let mut caps = viewer_net::SeedCapabilityMap::default();
        caps.entries.insert(
            String::from("EventQueueGet"),
            String::from("https://sim.example.invalid/cap/event"),
        );
        caps.entries.insert(
            String::from("InterestList"),
            String::from("https://sim.example.invalid/cap/interest"),
        );
        caps.entries.insert(
            String::from("UntrustedSimulatorMessage"),
            String::from("https://sim.example.invalid/cap/untrusted"),
        );
        caps.entries.insert(
            String::from("GetDisplayNames"),
            String::from("https://display.example.invalid/cap/names"),
        );
        let inventory = vec![viewer_net::SeedCapabilityInventoryEntry {
            name: String::from("EventQueueGet"),
            classification: viewer_net::CapabilityUrlClassification {
                family: viewer_net::CapabilityUrlFamily::SimulatorHost12043,
                host: Some(String::from("sim.example.invalid")),
                port: Some(12043),
            },
        }];

        let reprime = map_capability_reprime_from_seed_caps(&caps, &inventory);

        assert_eq!(
            reprime.event_queue_url.as_deref(),
            Some("https://sim.example.invalid/cap/event")
        );
        assert_eq!(
            reprime.pending_interest_list_probe_url.as_deref(),
            Some("https://sim.example.invalid/cap/interest")
        );
        assert_eq!(
            reprime
                .pending_untrusted_simulator_message_probe_url
                .as_deref(),
            Some("https://sim.example.invalid/cap/untrusted")
        );
        assert!(reprime.capability_inventory_summary.contains("families="));
        assert!(
            reprime
                .non_baseline_caps_by_host
                .contains("GetDisplayNames")
        );
    }

    #[test]
    fn remember_recent_seed_capability_url_is_bounded_and_promotes_duplicates() {
        let mut recent = VecDeque::new();
        for idx in 0..(MAX_RECENT_SEED_CAPABILITY_URLS + 2) {
            remember_recent_seed_capability_url(
                &mut recent,
                &format!("https://seed{}.example.invalid/cap", idx),
            );
        }
        assert_eq!(recent.len(), MAX_RECENT_SEED_CAPABILITY_URLS);
        assert_eq!(
            recent.front().map(String::as_str),
            Some("https://seed9.example.invalid/cap")
        );
        assert_eq!(
            recent.back().map(String::as_str),
            Some("https://seed2.example.invalid/cap")
        );

        remember_recent_seed_capability_url(&mut recent, "https://seed5.example.invalid/cap");
        assert_eq!(
            recent.front().map(String::as_str),
            Some("https://seed5.example.invalid/cap")
        );
        assert_eq!(recent.len(), MAX_RECENT_SEED_CAPABILITY_URLS);
    }

    #[test]
    fn summarize_recent_seed_capability_urls_reports_none_for_empty() {
        let recent = VecDeque::new();
        assert_eq!(summarize_recent_seed_capability_urls(&recent), "none");
    }

    #[test]
    fn lludp_gate_line_reports_fail_without_object_update_evidence() {
        let timeline = viewer_net::FirstSimulatorStartupTimelineSummary {
            first_object_update_index: None,
            ..Default::default()
        };
        let decoded = viewer_net::SimulatorPayloadDecodeSummary {
            object_feed_update_messages: 0,
            object_feed_total_objects: 0,
            object_feed_objects: Vec::new(),
            ..Default::default()
        };
        let line = format_lludp_startup_object_gate_line("startup", &timeline, &decoded);
        assert!(line.contains("verdict=FAIL"));
        assert!(line.contains("object_update=none"));
        assert!(line.contains("local_ids=none"));
    }

    #[test]
    fn lludp_gate_line_reports_pass_with_object_update_and_local_ids() {
        let timeline = viewer_net::FirstSimulatorStartupTimelineSummary {
            first_object_update_index: Some(12),
            ..Default::default()
        };
        let decoded = viewer_net::SimulatorPayloadDecodeSummary {
            object_feed_update_messages: 3,
            object_feed_total_objects: 2,
            object_feed_objects: vec![
                viewer_net::DecodedObjectFeedObject {
                    local_id: 20,
                    scale_centi: None,
                    position_centi: None,
                    rotation_quat_i16: None,
                    mesh_id: None,
                    texture_id: None,
                    default_face_material: None,
                    face_material_overrides: Vec::new(),
                    object_id: None,
                },
                viewer_net::DecodedObjectFeedObject {
                    local_id: 10,
                    scale_centi: None,
                    position_centi: None,
                    rotation_quat_i16: None,
                    mesh_id: None,
                    texture_id: None,
                    default_face_material: None,
                    face_material_overrides: Vec::new(),
                    object_id: None,
                },
            ],
            ..Default::default()
        };
        let line = format_lludp_startup_object_gate_line("startup", &timeline, &decoded);
        assert!(line.contains("verdict=PASS"));
        assert!(line.contains("object_update=12"));
        assert!(line.contains("local_ids=10,20"));
    }

    #[test]
    fn startup_interest_gate_line_reports_missing_requirements() {
        let gate = viewer_net::FirstSimulatorStartupInterestGateSummary {
            passed: false,
            required: vec![viewer_net::FirstSimulatorStartupInterestSendEvidence {
                message: String::from("AgentThrottle"),
                order_index: 1,
                packet_id: 12,
            }],
            missing: vec![
                String::from("AgentUpdate"),
                String::from("AgentHeightWidth"),
            ],
        };
        let line = format_startup_interest_gate_line("startup", &gate);
        assert!(line.contains("startup_interest_gate: verdict=FAIL"));
        assert!(line.contains("required=AgentThrottle@1#12"));
        assert!(line.contains("missing=AgentUpdate,AgentHeightWidth"));
        assert!(line.contains("classification=startup_send_path_defect"));
    }

    #[test]
    fn classify_session_residency_is_degraded_when_event_queue_is_failing() {
        let timeline = viewer_net::FirstSimulatorStartupTimelineSummary {
            first_agent_movement_complete_index: Some(2),
            first_region_handshake_index: Some(3),
            ..Default::default()
        };
        let region = viewer_net::RegionTransitionControlSummary::default();
        let residency = classify_session_residency(&timeline, &region, true, 2, 1, 0, 0);
        assert_eq!(residency.state, "degraded");
    }

    #[test]
    fn classify_session_residency_is_child_likely_when_enable_simulator_is_seen() {
        let timeline = viewer_net::FirstSimulatorStartupTimelineSummary {
            first_agent_movement_complete_index: Some(2),
            ..Default::default()
        };
        let region = viewer_net::RegionTransitionControlSummary {
            observations: 1,
            crossed_region: 0,
            confirm_enable_simulator: 0,
            not_seen_in_run: false,
        };
        let residency = classify_session_residency(&timeline, &region, true, 0, 0, 4, 1);
        assert_eq!(residency.state, "child_likely");
    }

    #[test]
    fn stress_test_mode_parses_legacy_and_new_aliases() {
        assert_eq!(
            StressTestMode::from_value(Some("1")),
            StressTestMode::SceneStress
        );
        assert_eq!(
            StressTestMode::from_value(Some("2")),
            StressTestMode::GeometryTorture
        );
        assert_eq!(
            StressTestMode::from_value(Some("camera")),
            StressTestMode::AutoCamera
        );
        assert_eq!(
            StressTestMode::from_value(Some("screenshots")),
            StressTestMode::Screenshot
        );
        assert_eq!(
            StressTestMode::from_value(Some("unknown")),
            StressTestMode::None
        );
    }

    #[test]
    fn auto_camera_config_parses_env_overrides() {
        let vars = HashMap::<String, String>::from([
            (
                String::from("VIEWER_TEST_CAMERA_CENTER"),
                String::from("1.0,2.0,3.0"),
            ),
            (
                String::from("VIEWER_TEST_CAMERA_RADIUS"),
                String::from("18.5"),
            ),
            (
                String::from("VIEWER_TEST_CAMERA_HEIGHT"),
                String::from("12.0"),
            ),
            (
                String::from("VIEWER_TEST_CAMERA_LOOK_HEIGHT"),
                String::from("4.5"),
            ),
            (
                String::from("VIEWER_TEST_CAMERA_SPEED"),
                String::from("0.8"),
            ),
            (
                String::from("VIEWER_TEST_CAMERA_PHASE"),
                String::from("1.57"),
            ),
        ]);
        let config = auto_camera_config_from_lookup(|k| vars.get(k).cloned());
        assert_eq!(config.center, [1.0, 2.0, 3.0]);
        assert_eq!(config.radius, 18.5);
        assert_eq!(config.orbit_height, 12.0);
        assert_eq!(config.look_height, 4.5);
        assert_eq!(config.angular_speed_radians, 0.8);
        assert_eq!(config.phase_radians, 1.57);
    }

    #[test]
    fn parse_camera_path_script_value_supports_linear_segments() {
        let value = serde_json::json!([
            {"time_sec": 0.0, "position": [0.0, 0.0, 0.0], "look_at": [1.0, 0.0, 0.0]},
            {"time_sec": 2.0, "position": [2.0, 0.0, 0.0], "look_at": [3.0, 0.0, 0.0]}
        ]);
        let script = parse_camera_path_script_value(value).expect("script should parse");
        assert_eq!(script.waypoints.len(), 2);
        let (position, look_at) = script.sample(1.0).expect("sample should exist");
        assert_eq!(position, [1.0, 0.0, 0.0]);
        assert_eq!(look_at, [2.0, 0.0, 0.0]);
    }

    #[test]
    fn auto_camera_config_loads_script_file_when_present() {
        let mut temp_path = std::env::temp_dir();
        temp_path.push(format!(
            "viewer_camera_path_{}_{}.json",
            std::process::id(),
            now_unix_ms()
        ));
        let json = r#"
[
  {"time_sec": 0.0, "position": [0.0, 0.0, 0.0], "look_at": [1.0, 0.0, 0.0]},
  {"time_sec": 1.0, "position": [1.0, 0.0, 0.0], "look_at": [2.0, 0.0, 0.0]}
]
"#;
        fs::write(&temp_path, json).expect("temp file write should succeed");
        let vars = HashMap::<String, String>::from([(
            String::from("VIEWER_TEST_CAMERA_PATH_FILE"),
            temp_path.to_string_lossy().to_string(),
        )]);
        let config = auto_camera_config_from_lookup(|k| vars.get(k).cloned());
        assert!(config.path_script.is_some());
        let _ = fs::remove_file(temp_path);
    }

    #[test]
    fn screenshot_config_is_enabled_only_for_screenshot_mode() {
        let vars = HashMap::<String, String>::from([
            (
                String::from("VIEWER_TEST_SCREENSHOT_DIR"),
                String::from("tmp/shots"),
            ),
            (
                String::from("VIEWER_TEST_SCREENSHOT_EVERY_N_FRAMES"),
                String::from("8"),
            ),
            (
                String::from("VIEWER_TEST_SCREENSHOT_MAX_FRAMES"),
                String::from("3"),
            ),
        ]);

        let disabled =
            screenshot_config_from_lookup(|k| vars.get(k).cloned(), StressTestMode::AutoCamera);
        assert!(disabled.is_none());

        let enabled =
            screenshot_config_from_lookup(|k| vars.get(k).cloned(), StressTestMode::Screenshot)
                .expect("screenshot config should be present");
        assert_eq!(enabled.output_dir, PathBuf::from("tmp/shots"));
        assert_eq!(enabled.every_n_frames, 8);
        assert_eq!(enabled.max_frames, 3);
    }

    #[test]
    fn mesh_verification_state_parses_defaults_and_target() {
        let vars = HashMap::<String, String>::from([
            (String::from("VIEWER_APP_MESH_VERIFY"), String::from("true")),
            (
                String::from("VIEWER_APP_MESH_VERIFY_ID"),
                String::from("947d4505-eb76-2ef5-c049-e7882881d689"),
            ),
            (
                String::from("VIEWER_APP_MESH_VERIFY_LOG_PATH"),
                String::from("artifacts/logs/custom_mesh_verify.jsonl"),
            ),
            (
                String::from("VIEWER_APP_MESH_VERIFY_SCREENSHOT_DIR"),
                String::from("artifacts/screenshots_mesh_verify"),
            ),
        ]);

        let state = mesh_verification_state_from_lookup(|k| vars.get(k).cloned());
        let config = state.config.expect("mesh verification should be enabled");
        assert_eq!(
            config.target_mesh_id,
            Some((String::from("947d4505-eb76-2ef5-c049-e7882881d689"), 0))
        );
        assert_eq!(
            config.log_path,
            PathBuf::from("artifacts/logs/custom_mesh_verify.jsonl")
        );
        assert_eq!(
            config.screenshot_dir,
            Some(PathBuf::from("artifacts/screenshots_mesh_verify"))
        );
    }

    #[test]
    fn live_mesh_asset_summary_reports_decoded_state_details() {
        let mut assets = BTreeMap::new();
        assets.insert(
            (String::from("mesh-a"), 0),
            LiveMeshAssetState::Decoded {
                asset: LiveMeshAssetBytes {
                    bytes: vec![0x7b, 0x00, 0x00],
                    byte_len: 3,
                    byte_signature: String::from("7b0000"),
                    format_hint: viewer_asset::MeshSourceFormat::SecondLifeMesh,
                },
                format: viewer_asset::MeshSourceFormat::SecondLifeMesh,
                vertices: 12,
                submeshes: 2,
            },
        );

        assert_eq!(live_mesh_queue_counts(&assets), (0, 0, 1, 0));
        let line = live_mesh_asset_summary_line(&assets);
        assert!(line.contains("mesh_last=decoded"));
        assert!(line.contains("bytes=3"));
        assert!(line.contains("vertices=12"));
        assert!(line.contains("submeshes=2"));
    }

    #[test]
    fn parse_live_startup_mode_defaults_to_auto_and_parses_values() {
        assert_eq!(parse_live_startup_mode(None), LiveStartupMode::Auto);
        assert_eq!(parse_live_startup_mode(Some("on")), LiveStartupMode::On);
        assert_eq!(parse_live_startup_mode(Some("off")), LiveStartupMode::Off);
    }

    #[test]
    fn parse_asset_source_mode_defaults_to_auto_and_parses_values() {
        assert_eq!(parse_asset_source_mode(None), AssetSourceMode::Auto);
        assert_eq!(
            parse_asset_source_mode(Some("fixture")),
            AssetSourceMode::Fixture
        );
        assert_eq!(parse_asset_source_mode(Some("live")), AssetSourceMode::Live);
        assert_eq!(
            parse_asset_source_mode(Some("unknown")),
            AssetSourceMode::Auto
        );
    }

    #[test]
    fn live_startup_plan_respects_off_mode_even_with_credentials() {
        let mut vars = HashMap::<String, String>::new();
        vars.insert(String::from("VIEWER_APP_LIVE_STARTUP"), String::from("off"));
        vars.insert(
            String::from("VIEWER_LOGIN_ENDPOINT"),
            String::from("https://example.invalid/login"),
        );
        vars.insert(String::from("VIEWER_LOGIN_USERNAME"), String::from("user"));
        vars.insert(String::from("VIEWER_LOGIN_PASSWORD"), String::from("pass"));
        let plan = live_startup_plan_from_lookup(|k| vars.get(k).cloned());
        assert!(!plan.enabled);
        assert!(plan.config.is_none());
        assert_eq!(plan.startup_status, LiveStartupStatus::DisabledByConfig);
    }

    #[test]
    fn live_startup_plan_reports_missing_config_when_forced_on() {
        let vars = HashMap::<String, String>::from([(
            String::from("VIEWER_APP_LIVE_STARTUP"),
            String::from("on"),
        )]);
        let plan = live_startup_plan_from_lookup(|k| vars.get(k).cloned());
        assert!(!plan.enabled);
        assert!(plan.config.is_none());
        assert!(matches!(plan.startup_status, LiveStartupStatus::Failed(_)));
    }

    #[test]
    fn startup_failure_mapping_uses_login_request_shape_class_for_missing_password_signature() {
        let result = GridLoginResult::Failed(GridLoginError {
            class: GridLoginErrorClass::Unknown,
            reason: Some(String::from("viewer-data")),
            message: Some(String::from("Missing password")),
        });
        let trace = LoginTrace {
            initial_request: viewer_net::LoginTraceRequest {
                method: String::from("login_to_simulator"),
                start_location: String::from("last"),
                options: vec![],
                agree_to_tos: true,
                read_critical: true,
                had_mfa_token: false,
            },
            redirect_steps: vec![],
            final_response: viewer_net::LoginTraceResponse {
                login: Some(false),
                reason: Some(String::from("viewer-data")),
                message: Some(String::from("Missing password")),
            },
            final_result: viewer_net::LoginTraceFinalResult {
                outcome: String::from("failed"),
                reason: Some(String::from("viewer-data")),
                message: Some(String::from("Missing password")),
            },
        };
        let fallback = LoginFallbackOutcome {
            primary_wire_format: LoginWireFormat::Llsd,
            fallback_used: true,
            final_wire_format: LoginWireFormat::XmlRpc,
            classified_reason: Some(LoginFallbackClassifiedReason::RequestShapeMissingPassword),
        };

        let failure = startup_failure_from_login_outcome(&result, &trace, &fallback);
        assert_eq!(failure.class, LiveStartupFailureClass::LoginRequestShape);
    }

    #[test]
    fn startup_failure_mapping_uses_auth_class_for_reason_key() {
        let result = GridLoginResult::Failed(GridLoginError {
            class: GridLoginErrorClass::AuthFailed,
            reason: Some(String::from("key")),
            message: Some(String::from("invalid credentials")),
        });
        let trace = LoginTrace {
            initial_request: viewer_net::LoginTraceRequest {
                method: String::from("login_to_simulator"),
                start_location: String::from("last"),
                options: vec![],
                agree_to_tos: true,
                read_critical: true,
                had_mfa_token: false,
            },
            redirect_steps: vec![],
            final_response: viewer_net::LoginTraceResponse {
                login: Some(false),
                reason: Some(String::from("key")),
                message: Some(String::from("invalid credentials")),
            },
            final_result: viewer_net::LoginTraceFinalResult {
                outcome: String::from("failed"),
                reason: Some(String::from("key")),
                message: Some(String::from("invalid credentials")),
            },
        };
        let fallback = LoginFallbackOutcome {
            primary_wire_format: LoginWireFormat::Llsd,
            fallback_used: false,
            final_wire_format: LoginWireFormat::Llsd,
            classified_reason: None,
        };

        let failure = startup_failure_from_login_outcome(&result, &trace, &fallback);
        assert_eq!(failure.class, LiveStartupFailureClass::LoginAuth);
    }

    #[test]
    fn should_apply_world_ingestion_seam_only_when_changed() {
        let empty = WorldObjectIngestionSeam::default();
        assert!(should_apply_world_ingestion_seam(None, &empty));
        assert!(!should_apply_world_ingestion_seam(Some(&empty), &empty));

        let changed = WorldObjectIngestionAdapter::adapt(Some(&LiveVisualSnapshot {
            source: String::from("test"),
            logged_in: true,
            current_region_name: Some(String::from("Test Region")),
            first_sim_endpoint: Some(String::from("198.51.100.42:13009")),
            first_sim_region_x: Some(1024),
            first_sim_region_y: Some(2048),
            handshake_agent_movement_complete: true,
            traffic_summary_available: true,
            post_boundary_observations: 2,
            region_transition_control_observations: 0,
            crossed_region: 0,
            confirm_enable_simulator: 0,
            likely_broader_traffic: 1,
            unknown: 0,
            decoded_coarse_updates: 1,
            decoded_coarse_location_count: Some(2),
            decoded_coarse_first_x: Some(64),
            decoded_coarse_first_y: Some(32),
            decoded_coarse_first_z: Some(12),
            decoded_coarse_second_x: None,
            decoded_coarse_second_y: None,
            decoded_coarse_second_z: None,
            decoded_coarse_third_x: None,
            decoded_coarse_third_y: None,
            decoded_coarse_third_z: None,
            decoded_health_updates: 1,
            decoded_health_last_basis_points: Some(6200),
            decoded_viewer_time_updates: 1,
            decoded_viewer_time_body_len: Some(5),
            decoded_viewer_time_signature: Some(0xDDCCBBAA),
            decoded_object_feed_update_messages: 0,
            decoded_object_feed_kill_messages: 0,
            decoded_object_feed_decode_dropped: 0,
            decoded_object_feed_evicted: 0,
            decoded_object_feed_total_objects: 0,
            decoded_object_feed_export_truncated: false,
            decoded_object_feed_objects: Vec::new(),
            decoded_object_feed_recent_kills: Vec::new(),
            continuity: viewer_core::RegionContinuitySummary::default(),
            observed_at_unix_ms: 1,
        }));
        assert!(should_apply_world_ingestion_seam(Some(&empty), &changed));
        assert!(!should_apply_world_ingestion_seam(Some(&changed), &changed));
        let mut viewer_time_changed_snapshot = LiveVisualSnapshot {
            source: String::from("test"),
            logged_in: true,
            current_region_name: Some(String::from("Test Region")),
            first_sim_endpoint: Some(String::from("198.51.100.42:13009")),
            first_sim_region_x: Some(1024),
            first_sim_region_y: Some(2048),
            handshake_agent_movement_complete: true,
            traffic_summary_available: true,
            post_boundary_observations: 2,
            region_transition_control_observations: 0,
            crossed_region: 0,
            confirm_enable_simulator: 0,
            likely_broader_traffic: 1,
            unknown: 0,
            decoded_coarse_updates: 1,
            decoded_coarse_location_count: Some(2),
            decoded_coarse_first_x: Some(64),
            decoded_coarse_first_y: Some(32),
            decoded_coarse_first_z: Some(12),
            decoded_coarse_second_x: None,
            decoded_coarse_second_y: None,
            decoded_coarse_second_z: None,
            decoded_coarse_third_x: None,
            decoded_coarse_third_y: None,
            decoded_coarse_third_z: None,
            decoded_health_updates: 1,
            decoded_health_last_basis_points: Some(6200),
            decoded_viewer_time_updates: 1,
            decoded_viewer_time_body_len: Some(5),
            decoded_viewer_time_signature: Some(0xDDCCBBAA),
            decoded_object_feed_update_messages: 0,
            decoded_object_feed_kill_messages: 0,
            decoded_object_feed_decode_dropped: 0,
            decoded_object_feed_evicted: 0,
            decoded_object_feed_total_objects: 0,
            decoded_object_feed_export_truncated: false,
            decoded_object_feed_objects: Vec::new(),
            decoded_object_feed_recent_kills: Vec::new(),
            continuity: viewer_core::RegionContinuitySummary::default(),
            observed_at_unix_ms: 1,
        };
        let viewer_time_first =
            WorldObjectIngestionAdapter::adapt(Some(&viewer_time_changed_snapshot));
        viewer_time_changed_snapshot.decoded_viewer_time_updates = 2;
        let viewer_time_second =
            WorldObjectIngestionAdapter::adapt(Some(&viewer_time_changed_snapshot));
        assert!(should_apply_world_ingestion_seam(
            Some(&viewer_time_first),
            &viewer_time_second
        ));
        let mut coarse_neighbor_changed_snapshot = viewer_time_changed_snapshot.clone();
        coarse_neighbor_changed_snapshot.decoded_coarse_second_x = Some(96);
        coarse_neighbor_changed_snapshot.decoded_coarse_second_y = Some(56);
        coarse_neighbor_changed_snapshot.decoded_coarse_second_z = Some(14);
        let coarse_neighbor_changed =
            WorldObjectIngestionAdapter::adapt(Some(&coarse_neighbor_changed_snapshot));
        assert!(should_apply_world_ingestion_seam(
            Some(&viewer_time_second),
            &coarse_neighbor_changed
        ));
    }

    #[test]
    fn map_net_continuity_to_core_maps_all_fields() {
        let mapped = map_net_continuity_to_core(&viewer_core::RegionContinuitySummary {
            phase: viewer_core::HandoffPhase::Confirming,
            outcome: viewer_core::HandoffOutcome::Normal,
            reason: viewer_core::HandoffReason::None,
            phase_age_ms: 0,
            active_region_coords: Some([1024, 2048]),
            previous_region_coords: Some([1023, 2048]),
            neighbors: vec![viewer_core::BoundedNeighborSummary {
                region_handle: 0x0000040000000800,
                region_x: 1024,
                region_y: 2048,
            }],
            ..Default::default()
        });
        assert_eq!(mapped.phase, viewer_core::HandoffPhase::Confirming);
        assert_eq!(mapped.active_region_coords, Some([1024, 2048]));
        assert_eq!(mapped.previous_region_coords, Some([1023, 2048]));
        assert_eq!(mapped.neighbors.len(), 1);
        assert_eq!(mapped.neighbors[0].region_x, 1024);
    }

    #[test]
    fn derive_environment_from_snapshot_uses_defaults_when_absent() {
        let env = derive_environment_from_snapshot(None);
        assert_eq!(env, viewer_core::EnvironmentState::default().sanitized());
    }

    #[test]
    fn derive_environment_from_snapshot_adjusts_fog_for_degraded_outcome() {
        let mut snapshot = offline_snapshot();
        snapshot.observed_at_unix_ms = 43_200_000; // noon UTC
        snapshot.continuity.outcome = viewer_core::HandoffOutcome::Degraded;

        let env = derive_environment_from_snapshot(Some(&snapshot));
        assert!(env.fog.density > viewer_core::EnvironmentState::default().fog.density);
        assert!(env.fog.end > env.fog.start);
        assert!((env.time_of_day_normalized - 0.5).abs() < 0.001);
    }

    #[test]
    fn derive_transition_visual_cue_returns_healthy_when_no_snapshot() {
        let cue = derive_transition_visual_cue(None);
        assert_eq!(cue.cue, viewer_core::TransitionVisualCue::Healthy);
        assert_eq!(cue.intensity, 0.0);
    }

    #[test]
    fn derive_transition_visual_cue_healthy_outcome_yields_healthy() {
        let snapshot = offline_snapshot();
        // Default snapshot has Normal outcome, None phase
        let cue = derive_transition_visual_cue(Some(&snapshot));
        assert_eq!(cue.cue, viewer_core::TransitionVisualCue::Healthy);
        assert_eq!(cue.intensity, 0.0);
    }

    #[test]
    fn derive_transition_visual_cue_degraded_outcome_yields_degraded() {
        let mut snapshot = offline_snapshot();
        snapshot.continuity.outcome = viewer_core::HandoffOutcome::Degraded;
        snapshot.continuity.last_probe_result = None;
        let cue = derive_transition_visual_cue(Some(&snapshot));
        assert_eq!(cue.cue, viewer_core::TransitionVisualCue::Degraded);
        assert!(cue.intensity > 0.0);
        assert!(cue.intensity <= 1.0);
    }

    #[test]
    fn derive_transition_visual_cue_stalled_outcome_yields_stalled() {
        let mut snapshot = offline_snapshot();
        snapshot.continuity.outcome = viewer_core::HandoffOutcome::Stalled;
        let cue = derive_transition_visual_cue(Some(&snapshot));
        assert_eq!(cue.cue, viewer_core::TransitionVisualCue::Stalled);
        assert!(cue.intensity > 0.0);
    }

    #[test]
    fn derive_transition_visual_cue_probe_success_on_degraded_yields_recovering() {
        let mut snapshot = offline_snapshot();
        snapshot.continuity.outcome = viewer_core::HandoffOutcome::Degraded;
        snapshot.continuity.last_probe_result = Some(viewer_core::ProbeResultCode::Success);
        snapshot.continuity.last_probe_time_unix_ms = Some(snapshot.observed_at_unix_ms);
        let cue = derive_transition_visual_cue(Some(&snapshot));
        assert_eq!(cue.cue, viewer_core::TransitionVisualCue::Recovering);
        assert!(cue.intensity > 0.0);
    }

    #[test]
    fn derive_transition_visual_cue_stale_probe_success_on_degraded_stays_degraded() {
        let mut snapshot = offline_snapshot();
        snapshot.continuity.outcome = viewer_core::HandoffOutcome::Degraded;
        snapshot.continuity.last_probe_result = Some(viewer_core::ProbeResultCode::Success);
        snapshot.continuity.last_probe_time_unix_ms =
            Some(snapshot.observed_at_unix_ms - (RECOVERY_PROBE_COOLDOWN_MS + 1));
        let cue = derive_transition_visual_cue(Some(&snapshot));
        assert_eq!(cue.cue, viewer_core::TransitionVisualCue::Degraded);
        assert!(cue.intensity > 0.0);
    }

    #[test]
    fn derive_transition_visual_cue_stale_probe_success_with_active_normal_stays_healthy() {
        let mut snapshot = offline_snapshot();
        snapshot.continuity.outcome = viewer_core::HandoffOutcome::Normal;
        snapshot.continuity.phase = viewer_core::HandoffPhase::Confirming;
        snapshot.continuity.last_probe_result = Some(viewer_core::ProbeResultCode::Success);
        snapshot.continuity.last_probe_time_unix_ms =
            Some(snapshot.observed_at_unix_ms - (RECOVERY_PROBE_COOLDOWN_MS + 1));
        let cue = derive_transition_visual_cue(Some(&snapshot));
        assert_eq!(cue.cue, viewer_core::TransitionVisualCue::Healthy);
        assert_eq!(cue.intensity, 0.0);
    }

    #[test]
    fn apply_u09_shortcut_routes_f1_f2_f3_as_bounded_policy() {
        let mut show_diag = false;
        let mut show_social = false;
        let mut focus_cont = false;
        let mut focus_social = false;

        apply_u09_shortcut(
            KeyCode::F1,
            &mut show_diag,
            &mut show_social,
            &mut focus_cont,
            &mut focus_social,
        );
        assert!(show_diag);
        assert!(!show_social);
        assert!(!focus_cont);
        assert!(!focus_social);

        apply_u09_shortcut(
            KeyCode::F2,
            &mut show_diag,
            &mut show_social,
            &mut focus_cont,
            &mut focus_social,
        );
        assert!(show_diag);
        assert!(focus_cont);
        assert!(!focus_social);

        show_social = false;
        focus_social = false;
        apply_u09_shortcut(
            KeyCode::F3,
            &mut show_diag,
            &mut show_social,
            &mut focus_cont,
            &mut focus_social,
        );
        assert!(show_social);
        assert!(focus_social);
    }

    #[test]
    fn profile_refresh_on_cooldown_enforces_u09_window() {
        let last = 1_000u64;
        assert!(!profile_refresh_on_cooldown(last + 10_000, Some(last)));
        assert!(profile_refresh_on_cooldown(last + 9_999, Some(last)));
        assert!(!profile_refresh_on_cooldown(last + 1, None));
    }

    #[test]
    fn apply_avatar_render_mode_updates_social_diagnostic_and_scene_output() {
        let mut scene = Scene::prototype();
        let mut social = SocialState::default();
        let avatars = vec![WorldAvatarPlaceholder {
            agent_id: String::from("avatar-id"),
            world_position: [1.0, 0.0, 1.0],
            local_position: Some([10, 20, 30]),
            sim_name: Some(String::from("TestSim")),
            display_name: String::from("Avatar"),
            is_self: false,
            last_update_unix_ms: 1,
            stale: false,
            appearance: AvatarAppearanceSummary::default(),
            attachments: Vec::new(),
        }];

        apply_avatar_render_mode(&mut scene, &mut social, &avatars, AvatarRenderMode::Proxy);
        assert_eq!(social.avatar_render_mode, AvatarRenderMode::Proxy);
        assert!(scene.instances.iter().any(|instance| {
            instance.1.role == viewer_core::InstanceRole::WorldAvatarPlaceholderOther
                && instance.1.geometry
                    == viewer_core::GeometrySource::Diagnostic(viewer_core::MeshKind::AvatarProxy)
        }));

        apply_avatar_render_mode(
            &mut scene,
            &mut social,
            &avatars,
            AvatarRenderMode::FallbackBox,
        );
        assert_eq!(social.avatar_render_mode, AvatarRenderMode::FallbackBox);
        assert!(scene.instances.iter().any(|instance| {
            instance.1.role == viewer_core::InstanceRole::WorldAvatarPlaceholderOther
                && instance.1.geometry
                    == viewer_core::GeometrySource::Diagnostic(viewer_core::MeshKind::Cube)
        }));
    }

    #[test]
    fn resolve_avatar_sim_name_uses_priority_order() {
        assert_eq!(
            resolve_avatar_sim_name(Some("SampleSim"), Some("Decoded"), Some("Startup")),
            "SampleSim"
        );
        assert_eq!(
            resolve_avatar_sim_name(None, Some("Decoded"), Some("Startup")),
            "Decoded"
        );
        assert_eq!(
            resolve_avatar_sim_name(None, None, Some("Startup")),
            "Startup"
        );
        assert_eq!(resolve_avatar_sim_name(None, None, None), "unknown");
    }

    #[test]
    fn update_world_sim_name_state_preserves_last_known_non_empty() {
        let mut world_sim_name = Some(String::from("DecodedOne"));
        let mut startup_fallback = Some(String::from("BootstrapOne"));
        update_world_sim_name_state(&mut world_sim_name, &mut startup_fallback, None, None);
        assert_eq!(world_sim_name.as_deref(), Some("DecodedOne"));

        update_world_sim_name_state(
            &mut world_sim_name,
            &mut startup_fallback,
            Some("DecodedTwo"),
            Some("BootstrapTwo"),
        );
        assert_eq!(world_sim_name.as_deref(), Some("DecodedTwo"));
        assert_eq!(startup_fallback.as_deref(), Some("BootstrapTwo"));

        let mut empty_world = None;
        let mut empty_startup = None;
        update_world_sim_name_state(&mut empty_world, &mut empty_startup, None, Some("Fallback"));
        assert_eq!(empty_world.as_deref(), Some("Fallback"));
        assert_eq!(empty_startup.as_deref(), Some("Fallback"));
    }

    #[test]
    fn should_apply_live_visual_snapshot_only_when_changed() {
        assert!(!should_apply_live_visual_snapshot(None, None));
        let first = LiveVisualSnapshot {
            source: String::from("test"),
            logged_in: true,
            current_region_name: Some(String::from("Test Region")),
            first_sim_endpoint: Some(String::from("198.51.100.42:13009")),
            first_sim_region_x: Some(1024),
            first_sim_region_y: Some(2048),
            handshake_agent_movement_complete: true,
            traffic_summary_available: true,
            post_boundary_observations: 2,
            region_transition_control_observations: 0,
            crossed_region: 0,
            confirm_enable_simulator: 0,
            likely_broader_traffic: 1,
            unknown: 0,
            decoded_coarse_updates: 1,
            decoded_coarse_location_count: Some(2),
            decoded_coarse_first_x: Some(64),
            decoded_coarse_first_y: Some(32),
            decoded_coarse_first_z: Some(12),
            decoded_coarse_second_x: None,
            decoded_coarse_second_y: None,
            decoded_coarse_second_z: None,
            decoded_coarse_third_x: None,
            decoded_coarse_third_y: None,
            decoded_coarse_third_z: None,
            decoded_health_updates: 1,
            decoded_health_last_basis_points: Some(6200),
            decoded_viewer_time_updates: 1,
            decoded_viewer_time_body_len: Some(5),
            decoded_viewer_time_signature: Some(0xDDCCBBAA),
            decoded_object_feed_update_messages: 0,
            decoded_object_feed_kill_messages: 0,
            decoded_object_feed_decode_dropped: 0,
            decoded_object_feed_evicted: 0,
            decoded_object_feed_total_objects: 0,
            decoded_object_feed_export_truncated: false,
            decoded_object_feed_objects: Vec::new(),
            decoded_object_feed_recent_kills: Vec::new(),
            continuity: viewer_core::RegionContinuitySummary::default(),
            observed_at_unix_ms: 1,
        };
        assert!(should_apply_live_visual_snapshot(None, Some(&first)));
        assert!(!should_apply_live_visual_snapshot(
            Some(&first),
            Some(&first)
        ));
        let mut second = first.clone();
        second.observed_at_unix_ms = 2;
        assert!(should_apply_live_visual_snapshot(
            Some(&first),
            Some(&second)
        ));
        let mut viewer_time_second = first.clone();
        viewer_time_second.decoded_viewer_time_updates = 2;
        assert!(should_apply_live_visual_snapshot(
            Some(&first),
            Some(&viewer_time_second)
        ));
        assert!(should_apply_live_visual_snapshot(Some(&first), None));
    }

    #[test]
    fn test_extract_visible_texture_ids_is_deterministic_and_capped() {
        let mut scene = Scene::prototype();

        // Instance 1: tex_b, tex_c
        let mut mat1 = MaterialSet {
            default: MaterialDescriptor::Legacy(TextureEntry {
                texture_id: AssetID::new("tex_b"),
                ..TextureEntry::default()
            }),
            ..MaterialSet::default()
        };
        mat1.by_face.insert(
            1,
            MaterialDescriptor::Legacy(TextureEntry {
                texture_id: AssetID::new("tex_c"),
                ..TextureEntry::default()
            }),
        );
        scene.instances.insert(
            1,
            RenderableInstance::new(
                GeometrySource::Diagnostic(MeshKind::Cube),
                InstanceRole::SceneStatic,
                Transform::default(),
                [1.0, 1.0, 1.0, 1.0],
                AlphaMode::Opaque,
            )
            .with_materials(mat1),
        );

        // Instance 2: tex_a
        let mat2 = MaterialSet {
            default: MaterialDescriptor::Legacy(TextureEntry {
                texture_id: AssetID::new("tex_a"),
                ..TextureEntry::default()
            }),
            ..MaterialSet::default()
        };
        scene.instances.insert(
            2,
            RenderableInstance::new(
                GeometrySource::Diagnostic(MeshKind::Cube),
                InstanceRole::SceneStatic,
                Transform::default(),
                [1.0, 1.0, 1.0, 1.0],
                AlphaMode::Opaque,
            )
            .with_materials(mat2),
        );

        // Test determinism (BTreeSet should sort tex_a, tex_b, tex_c regardless of discovery order)
        let ids = extract_visible_texture_ids_from_scene(&scene, &[1, 2], 10);
        assert_eq!(
            ids,
            vec![
                AssetID::new("tex_a"),
                AssetID::new("tex_b"),
                AssetID::new("tex_c")
            ]
        );

        // Test capping
        let ids_capped = extract_visible_texture_ids_from_scene(&scene, &[1, 2], 2);
        // tex_b and tex_c are found in instance 1. tex_a in instance 2.
        // If cap is 2, it should stop after finding tex_b and tex_c from instance 1.
        // Then BTreeSet sorts them -> tex_b, tex_c.
        assert_eq!(ids_capped.len(), 2);
        assert_eq!(
            ids_capped,
            vec![AssetID::new("tex_b"), AssetID::new("tex_c")]
        );
    }

    #[test]
    fn test_merge_texture_request_ids_keeps_visible_when_fixture_empty() {
        let fixture_ids = Vec::<AssetID>::new();
        let visible_ids = vec![AssetID::new("tex_visible_a"), AssetID::new("tex_visible_b")];
        let merged = merge_texture_request_ids(&fixture_ids, &visible_ids);
        assert_eq!(
            merged,
            vec![AssetID::new("tex_visible_a"), AssetID::new("tex_visible_b")]
        );
    }

    #[test]
    fn test_extract_decoded_object_feed_mesh_ids_is_deterministic_and_capped() {
        let mut snapshot = offline_snapshot();
        snapshot.decoded_object_feed_objects = vec![
            viewer_core::DecodedWorldObjectFeedObject {
                local_id: 1,
                scale_centi: None,
                position_centi: None,
                rotation_quat_i16: None,
                mesh_id: Some(String::from("BBB")),
                texture_id: None,
                default_face_material: None,
                face_material_overrides: Vec::new(),
                object_id: None,
            },
            viewer_core::DecodedWorldObjectFeedObject {
                local_id: 2,
                scale_centi: None,
                position_centi: None,
                rotation_quat_i16: None,
                mesh_id: Some(String::from("aaa")),
                texture_id: None,
                default_face_material: None,
                face_material_overrides: Vec::new(),
                object_id: None,
            },
            viewer_core::DecodedWorldObjectFeedObject {
                local_id: 3,
                scale_centi: None,
                position_centi: None,
                rotation_quat_i16: None,
                mesh_id: Some(String::from("bbb")),
                texture_id: None,
                default_face_material: None,
                face_material_overrides: Vec::new(),
                object_id: None,
            },
            viewer_core::DecodedWorldObjectFeedObject {
                local_id: 4,
                scale_centi: None,
                position_centi: None,
                rotation_quat_i16: None,
                mesh_id: Some(String::from("   ")),
                texture_id: None,
                default_face_material: None,
                face_material_overrides: Vec::new(),
                object_id: None,
            },
            viewer_core::DecodedWorldObjectFeedObject {
                local_id: 5,
                scale_centi: None,
                position_centi: None,
                rotation_quat_i16: None,
                mesh_id: Some(String::from("cccccccc-cccc-cccc-cccc-cccccccccccc")),
                texture_id: None,
                default_face_material: None,
                face_material_overrides: Vec::new(),
                object_id: Some(String::from("cccccccc-cccc-cccc-cccc-cccccccccccc")),
            },
        ];

        let ids = extract_decoded_object_feed_mesh_ids(&snapshot, 10);
        assert_eq!(
            ids,
            vec![(String::from("aaa"), 0), (String::from("bbb"), 0)]
        );

        let ids_capped = extract_decoded_object_feed_mesh_ids(&snapshot, 1);
        assert_eq!(ids_capped, vec![(String::from("aaa"), 0)]);
    }

    #[test]
    fn test_extract_decoded_object_feed_texture_ids_is_deterministic_and_capped() {
        let mut snapshot = offline_snapshot();
        snapshot.decoded_object_feed_objects = vec![
            viewer_core::DecodedWorldObjectFeedObject {
                local_id: 1,
                scale_centi: None,
                position_centi: None,
                rotation_quat_i16: None,
                mesh_id: None,
                texture_id: Some(AssetID::new("bbb")),
                default_face_material: None,
                face_material_overrides: Vec::new(),
                object_id: None,
            },
            viewer_core::DecodedWorldObjectFeedObject {
                local_id: 2,
                scale_centi: None,
                position_centi: None,
                rotation_quat_i16: None,
                mesh_id: None,
                texture_id: Some(AssetID::new("aaa")),
                default_face_material: None,
                face_material_overrides: Vec::new(),
                object_id: None,
            },
            viewer_core::DecodedWorldObjectFeedObject {
                local_id: 3,
                scale_centi: None,
                position_centi: None,
                rotation_quat_i16: None,
                mesh_id: None,
                texture_id: Some(AssetID::new("bbb")),
                default_face_material: None,
                face_material_overrides: Vec::new(),
                object_id: None,
            },
            viewer_core::DecodedWorldObjectFeedObject {
                local_id: 4,
                scale_centi: None,
                position_centi: None,
                rotation_quat_i16: None,
                mesh_id: None,
                texture_id: Some(AssetID::new("   ")),
                default_face_material: None,
                face_material_overrides: Vec::new(),
                object_id: None,
            },
        ];

        let ids = extract_decoded_object_feed_texture_ids(&snapshot, 10);
        assert_eq!(ids, vec![AssetID::new("aaa"), AssetID::new("bbb")]);

        let ids_capped = extract_decoded_object_feed_texture_ids(&snapshot, 1);
        assert_eq!(ids_capped, vec![AssetID::new("bbb")]);
    }

    #[test]
    fn test_extract_decoded_object_feed_texture_ids_includes_face_material_ids() {
        let mut snapshot = offline_snapshot();
        snapshot.decoded_object_feed_objects = vec![viewer_core::DecodedWorldObjectFeedObject {
            local_id: 9,
            scale_centi: None,
            position_centi: None,
            rotation_quat_i16: None,
            mesh_id: None,
            texture_id: None,
            default_face_material: Some(viewer_core::DecodedWorldObjectFaceMaterial {
                face_id: 0,
                texture_id: Some(AssetID::new("face_default")),
                normal_id: Some(AssetID::new("face_normal")),
                specular_id: Some(AssetID::new("face_specular")),
                material_id: None,
                rgba: [255, 255, 255, 255],
                offset_s: 0,
                offset_t: 0,
                scale_s: 10_000,
                scale_t: 10_000,
                rotation: 0,
                bump: 0,
                fullbright: false,
                shiny: 0,
                media_flags: 0,
                glow: 0,
            }),
            face_material_overrides: vec![],
            object_id: None,
        }];

        let ids = extract_decoded_object_feed_texture_ids(&snapshot, 10);
        assert_eq!(
            ids,
            vec![
                AssetID::new("face_default"),
                AssetID::new("face_normal"),
                AssetID::new("face_specular"),
            ]
        );
    }

    #[test]
    fn continuity_priority_mapping_assigns_active_previous_neighbor() {
        let ids = vec![
            AssetID::new("a"),
            AssetID::new("b"),
            AssetID::new("c"),
            AssetID::new("d"),
        ];
        let visible = vec![AssetID::new("a")];
        let continuity = RegionContinuitySummary {
            phase: viewer_core::HandoffPhase::Confirming,
            outcome: viewer_core::HandoffOutcome::Normal,
            reason: viewer_core::HandoffReason::None,
            phase_age_ms: 0,
            active_region_coords: Some([1024, 2048]),
            previous_region_coords: Some([1023, 2048]),
            neighbors: vec![viewer_core::BoundedNeighborSummary {
                region_handle: 1,
                region_x: 1025,
                region_y: 2048,
            }],
            ..Default::default()
        };

        let mapped = build_asset_priority_hints(&ids, &visible, Some(&continuity));
        assert_eq!(mapped[0].priority, viewer_core::AssetPriority::Active);
        assert_eq!(mapped[1].priority, viewer_core::AssetPriority::Previous);
        assert_eq!(mapped[2].priority, viewer_core::AssetPriority::Previous);
        assert_eq!(mapped[3].priority, viewer_core::AssetPriority::Previous);
    }

    #[test]
    fn continuity_priority_mapping_falls_back_to_normal_without_continuity() {
        let ids = vec![AssetID::new("a"), AssetID::new("b")];
        let visible = vec![AssetID::new("a")];
        let mapped = build_asset_priority_hints(&ids, &visible, None);
        assert_eq!(mapped[0].priority, viewer_core::AssetPriority::Active);
        assert_eq!(mapped[1].priority, viewer_core::AssetPriority::Normal);
    }

    #[test]
    fn continuity_priority_mapping_assigns_neighbor_when_previous_absent() {
        let ids = vec![AssetID::new("a"), AssetID::new("b")];
        let visible = vec![AssetID::new("a")];
        let continuity = RegionContinuitySummary {
            phase: viewer_core::HandoffPhase::Confirming,
            outcome: viewer_core::HandoffOutcome::Normal,
            reason: viewer_core::HandoffReason::None,
            phase_age_ms: 0,
            active_region_coords: Some([1024, 2048]),
            previous_region_coords: None,
            neighbors: vec![viewer_core::BoundedNeighborSummary {
                region_handle: 1,
                region_x: 1025,
                region_y: 2048,
            }],
            ..Default::default()
        };
        let mapped = build_asset_priority_hints(&ids, &visible, Some(&continuity));
        assert_eq!(mapped[0].priority, viewer_core::AssetPriority::Active);
        assert_eq!(mapped[1].priority, viewer_core::AssetPriority::Neighbor);
    }

    #[test]
    fn recovery_action_respects_cooldowns_and_updates_time() {
        let (result, new_probe, new_asset) = compute_recovery_action(
            viewer_core::RecoveryAction::RefreshVisibleAssets,
            1000,
            None,
            None,
            true,
            false,
            5000,
            2000,
        );
        assert_eq!(result.code, viewer_core::RecoveryResultCode::Accepted);
        assert_eq!(new_probe, None);
        assert_eq!(new_asset, Some(1000));

        // Try again during cooldown
        let (result2, new_probe2, new_asset2) = compute_recovery_action(
            viewer_core::RecoveryAction::RefreshVisibleAssets,
            2000, // 1 second later
            new_probe,
            new_asset,
            true,
            false,
            5000,
            2000,
        );
        assert_eq!(
            result2.code,
            viewer_core::RecoveryResultCode::CooldownActive
        );
        assert_eq!(result2.cooldown_remaining_ms, Some(1000));
        assert_eq!(new_probe2, None);
        assert_eq!(new_asset2, Some(1000)); // Timestamp unchanged

        // Try after cooldown
        let (result3, new_probe3, new_asset3) = compute_recovery_action(
            viewer_core::RecoveryAction::RefreshVisibleAssets,
            4000, // 3 seconds later
            new_probe2,
            new_asset2,
            true,
            false,
            5000,
            2000,
        );
        assert_eq!(result3.code, viewer_core::RecoveryResultCode::Accepted);
        assert_eq!(new_probe3, None);
        assert_eq!(new_asset3, Some(4000)); // Timestamp updated
    }

    #[test]
    fn recovery_action_repeated_spam_blocks_and_does_not_advance_time() {
        let mut last_ms = None;
        for i in 0..10 {
            let now = 1000 + i * 100; // Incrementing by 100ms
            let (result, _, new_asset) = compute_recovery_action(
                viewer_core::RecoveryAction::RefreshVisibleAssets,
                now,
                None,
                last_ms,
                true,
                false,
                5000,
                2000,
            );
            if i == 0 {
                assert_eq!(result.code, viewer_core::RecoveryResultCode::Accepted);
            } else {
                assert_eq!(result.code, viewer_core::RecoveryResultCode::CooldownActive);
            }
            last_ms = new_asset;
        }

        assert_eq!(last_ms, Some(1000)); // Timestamp is still from the first accepted attempt
    }

    #[test]
    fn probe_cooldown_rejection() {
        let mut last_ms = None;
        let mut codes = Vec::new();

        // 0ms: First probe accepted
        let (res, next, _) = compute_recovery_action(
            viewer_core::RecoveryAction::RetryContinuityProbe,
            0,
            last_ms,
            None,
            true,
            false,
            15000,
            5000,
        );
        codes.push(res.code);
        last_ms = next;

        // 10000ms: Still in 15s cooldown
        let (res, next, _) = compute_recovery_action(
            viewer_core::RecoveryAction::RetryContinuityProbe,
            10000,
            last_ms,
            None,
            true,
            false,
            15000,
            5000,
        );
        codes.push(res.code);
        last_ms = next; // Bookkeeping should not change on rejection

        // 16000ms: Cooldown expired
        let (res, next, _) = compute_recovery_action(
            viewer_core::RecoveryAction::RetryContinuityProbe,
            16000,
            last_ms,
            None,
            true,
            false,
            15000,
            5000,
        );
        codes.push(res.code);
        last_ms = next;

        assert_eq!(
            codes,
            vec![
                viewer_core::RecoveryResultCode::Accepted,
                viewer_core::RecoveryResultCode::CooldownActive,
                viewer_core::RecoveryResultCode::Accepted,
            ]
        );
        assert_eq!(last_ms, Some(16000));
    }

    #[test]
    fn probe_retry_rejected_when_unavailable() {
        let (result, new_probe, _) = compute_recovery_action(
            viewer_core::RecoveryAction::RetryContinuityProbe,
            1_000,
            None,
            None,
            false,
            false,
            15_000,
            5_000,
        );
        assert_eq!(result.code, viewer_core::RecoveryResultCode::Unavailable);
        assert_eq!(
            result.detail.as_deref(),
            Some("continuity probe unavailable in current startup mode")
        );
        assert_eq!(new_probe, None);
    }

    #[test]
    fn probe_retry_rejected_when_in_flight() {
        let (result, new_probe, _) = compute_recovery_action(
            viewer_core::RecoveryAction::RetryContinuityProbe,
            2_000,
            Some(1_000),
            None,
            true,
            true,
            15_000,
            5_000,
        );
        assert_eq!(result.code, viewer_core::RecoveryResultCode::Unavailable);
        assert_eq!(
            result.detail.as_deref(),
            Some("continuity probe already in flight")
        );
        assert_eq!(new_probe, Some(1_000));
    }

    #[test]
    fn live_texture_retry_policy_is_bounded_and_reason_aware() {
        assert!(should_retry_live_texture_failure(
            viewer_asset::AssetFetchFailureReason::Transport,
            1
        ));
        assert!(should_retry_live_texture_failure(
            viewer_asset::AssetFetchFailureReason::Timeout,
            2
        ));
        assert!(!should_retry_live_texture_failure(
            viewer_asset::AssetFetchFailureReason::MissingCapability,
            1
        ));
        assert!(!should_retry_live_texture_failure(
            viewer_asset::AssetFetchFailureReason::Transport,
            LIVE_TEXTURE_FETCH_MAX_ATTEMPTS
        ));
    }

    #[test]
    fn live_texture_retry_backoff_grows_monotonically() {
        let first = live_texture_retry_backoff_ticks(1);
        let second = live_texture_retry_backoff_ticks(2);
        let third = live_texture_retry_backoff_ticks(3);
        assert!(first >= LIVE_TEXTURE_FETCH_RETRY_BASE_TICKS);
        assert!(second >= first);
        assert!(third >= second);
    }

    #[test]
    fn non_retryable_texture_http_status_is_bounded() {
        assert!(non_retryable_texture_http_status(401));
        assert!(non_retryable_texture_http_status(403));
        assert!(non_retryable_texture_http_status(404));
        assert!(!non_retryable_texture_http_status(429));
        assert!(!non_retryable_texture_http_status(500));
    }

    #[test]
    fn classify_mesh_403_bucket_detects_access_denied() {
        let err = ConnectionError::HttpStatus {
            status: "403".parse().expect("valid status code"),
            body: String::from("<Error><Code>AccessDenied</Code></Error>"),
        };
        assert_eq!(classify_mesh_403_bucket(&err), Some("AccessDenied"));
    }

    #[test]
    fn classify_mesh_403_bucket_returns_none_for_non_403() {
        let err = ConnectionError::HttpStatus {
            status: "404".parse().expect("valid status code"),
            body: String::from("<Error><Code>NoSuchKey</Code></Error>"),
        };
        assert_eq!(classify_mesh_403_bucket(&err), None);
    }

    #[test]
    fn summarize_decoded_object_feed_mesh_ids_reports_count_and_sorted_sample() {
        let mut snapshot = offline_snapshot();
        snapshot.decoded_object_feed_objects = vec![
            viewer_core::DecodedWorldObjectFeedObject {
                local_id: 1,
                scale_centi: None,
                position_centi: None,
                rotation_quat_i16: None,
                mesh_id: Some(String::from("bbb")),
                texture_id: None,
                default_face_material: None,
                face_material_overrides: Vec::new(),
                object_id: None,
            },
            viewer_core::DecodedWorldObjectFeedObject {
                local_id: 2,
                scale_centi: None,
                position_centi: None,
                rotation_quat_i16: None,
                mesh_id: Some(String::from("aaa")),
                texture_id: None,
                default_face_material: None,
                face_material_overrides: Vec::new(),
                object_id: None,
            },
            viewer_core::DecodedWorldObjectFeedObject {
                local_id: 3,
                scale_centi: None,
                position_centi: None,
                rotation_quat_i16: None,
                mesh_id: Some(String::from("ccc")),
                texture_id: None,
                default_face_material: None,
                face_material_overrides: Vec::new(),
                object_id: None,
            },
        ];

        let (count, sample) = summarize_decoded_object_feed_mesh_ids(&snapshot, 2);
        assert_eq!(count, 3);
        assert_eq!(sample, "aaa,bbb");
    }

    #[test]
    fn format_object_feed_message_family_counts_reports_selected_families() {
        let summary = viewer_net::SimulatorPayloadDecodeSummary {
            object_feed_object_update_messages: 2,
            object_feed_object_update_compressed_messages: 3,
            object_feed_object_extra_params_messages: 4,
            object_feed_improved_terse_messages: 5,
            ..Default::default()
        };

        assert_eq!(
            format_object_feed_message_family_counts(&summary),
            "ObjectUpdate:2,ObjectUpdateCompressed:3,ObjectExtraParams:4,ImprovedTerseObjectUpdate:5"
        );
    }

    #[test]
    fn format_object_feed_mesh_hit_counts_reports_selected_families() {
        let summary = viewer_net::SimulatorPayloadDecodeSummary {
            object_feed_object_update_mesh_hits: 6,
            object_feed_object_update_compressed_mesh_hits: 7,
            object_feed_object_extra_params_mesh_hits: 8,
            ..Default::default()
        };

        assert_eq!(
            format_object_feed_mesh_hit_counts(&summary),
            "ObjectUpdate:6,ObjectUpdateCompressed:7,ObjectExtraParams:8"
        );
    }

    #[test]
    fn format_object_feed_state_export_counts_reports_raw_and_export_views() {
        let mut snapshot = offline_snapshot();
        snapshot.decoded_object_feed_total_objects = 129;
        snapshot.decoded_object_feed_export_truncated = true;
        snapshot.decoded_object_feed_objects = vec![
            viewer_core::DecodedWorldObjectFeedObject {
                local_id: 1,
                scale_centi: None,
                position_centi: None,
                rotation_quat_i16: None,
                mesh_id: Some(String::from("aaa")),
                texture_id: None,
                default_face_material: None,
                face_material_overrides: Vec::new(),
                object_id: None,
            },
            viewer_core::DecodedWorldObjectFeedObject {
                local_id: 2,
                scale_centi: None,
                position_centi: None,
                rotation_quat_i16: None,
                mesh_id: None,
                texture_id: None,
                default_face_material: None,
                face_material_overrides: Vec::new(),
                object_id: None,
            },
        ];
        let summary = viewer_net::SimulatorPayloadDecodeSummary {
            object_feed_state_mesh_objects: 7,
            ..Default::default()
        };

        assert_eq!(
            format_object_feed_state_export_counts(&snapshot, &summary),
            "state_total_objects=129 export_objects=2 state_mesh_objects=7 export_mesh_objects=1 export_truncated=true"
        );
    }

    #[tokio::test]
    async fn update_live_visual_from_connection_preserves_transport_total_object_count() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener should bind");
        let listener_addr = listener.local_addr().expect("listener addr should exist");
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("request should arrive");
            let mut buf = vec![0u8; 4096];
            let _ = stream.read(&mut buf).await.expect("request should read");
            let body = serde_json::json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": 13001,
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })
            .to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body,
            );
            stream
                .write_all(response.as_bytes())
                .await
                .expect("response should write");
        });

        let mut connection = viewer_net::Connection::new(viewer_net::ConnectionConfig {
            endpoint: format!("http://{}/login", listener_addr),
            connect_timeout: Duration::from_secs(5),
            wire_format: LoginWireFormat::Json,
        });
        connection.connect().await.expect("connect should succeed");
        connection
            .login_with_adapter(
                &SecondLifeAdapter,
                LoginIntent {
                    username: String::from("user"),
                    password: String::from("pass"),
                    start_location: StartLocationIntent::Saved(StartLocation::Last),
                    agree_to_tos: false,
                    read_critical: true,
                    mfa_token: None,
                },
            )
            .await
            .expect("login should succeed");

        let mut body = Vec::new();
        body.extend_from_slice(&1u64.to_le_bytes());
        body.extend_from_slice(&0u16.to_le_bytes());
        body.push(129);
        for local_id in 1u32..=129 {
            body.extend_from_slice(&local_id.to_le_bytes());
            body.extend_from_slice(&0u32.to_le_bytes());
            body.extend_from_slice(&0u32.to_le_bytes());
        }
        let mut payload = vec![
            0x00, // flags
            0x00, 0x00, 0x00, 0x01, // packet sequence
            0x00, // extra header offset
            0x0E, // ObjectUpdateCached high-frequency id
        ];
        payload.extend_from_slice(&body);

        connection
            .observe_first_simulator_inbound_payload(&payload)
            .expect("cached update should be observed");

        let mut snapshot = offline_snapshot();
        update_live_visual_from_connection(&mut snapshot, &connection, &sample_in_process_config());

        assert_eq!(snapshot.decoded_object_feed_total_objects, 129);
        assert_eq!(snapshot.decoded_object_feed_objects.len(), 128);
        assert!(snapshot.decoded_object_feed_export_truncated);
    }

    #[test]
    fn format_mesh_fetch_attempt_line_includes_cap_variant_status_and_bucket() {
        let candidate = viewer_grid::MeshCapabilityRequestCandidate {
            capability_name: String::from("ViewerAsset"),
            url_variant: String::from("v1"),
            url: String::from("https://asset-cdn.example.com/viewerasset/?mesh_id=test-mesh"),
        };
        let attempt = viewer_net::AssetFetchAttempt {
            url: candidate.url.clone(),
            range_header: Some(String::from("bytes=0-")),
            status: Some(403),
            error: None,
            response_body: Some(String::from("<Error><Code>AccessDenied</Code></Error>")),
        };

        let line = format_mesh_fetch_attempt_line("test-mesh", 0, Some(&candidate), &attempt);
        assert!(line.contains("id=test-mesh"));
        assert!(line.contains("cap=ViewerAsset"));
        assert!(line.contains("url_variant=v1"));
        assert!(line.contains("status=403"));
        assert!(line.contains("bucket=AccessDenied"));
    }
}
