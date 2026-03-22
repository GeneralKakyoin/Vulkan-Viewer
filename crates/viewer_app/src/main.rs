use anyhow::{Context, Result};
use dotenvy::dotenv;
mod social_cache;
use social_cache::{SocialCache, SocialCacheConfig};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tracing_subscriber::FmtSubscriber;
use viewer_core::{
    AvatarProfileState, AvatarProfileTab, AvatarRenderMode, Camera, ChatConnectionState,
    ChatMessage, ChatSendStatus, ChatState, DirectImMessage, FirstLifeProfile, FriendEntry,
    LiveVisualSnapshot,
    ProfileClassifiedDetails, ProfileClassifiedSummary, ProfileLoadStatus, ProfileNotes,
    ProfilePickDetails, ProfilePickSummary, RuntimeRelayEvent, RuntimeRelayLevel, Scene,
    SecondLifeProfile, SocialState, WorldAvatarPlaceholder, WorldObjectIngestionAdapter,
    WorldObjectIngestionSeam, compute_p2p_session_id,
};
use viewer_grid::{
    GridLoginResult, LoginIntent, SecondLifeAdapter, StartLocation, StartLocationIntent,
};
use viewer_net::{
    AgentProfileData, Connection, ConnectionConfig, ConnectionError, LoginFallbackClassifiedReason,
    LoginFallbackOutcome, LoginTrace, LoginWireFormat, FirstSimulatorInboundTrafficScope,
    NearbyChatMessage, SocialCircuit, SocialEvent, poll_event_queue_url_once,
};
use viewer_render::RenderBackend;
use viewer_ui::UiSystem;
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
    last_frame_time: Instant,
    smoothed_fps: f32,
    smoothed_frame_ms: f32,
    avg_scene_update_ms: f32,
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
    run_probe: bool,
    worker_tick_ms: u64,
    event_queue_poll_timeout_ms: u64,
    event_queue_poll_every_ticks: u32,
    event_queue_failures_before_reconnect: u32,
    social_poll_timeout_ms: u64,
    social_poll_max_packets: usize,
    nearby_poll_timeout_ms: u64,
    nearby_poll_max_packets: usize,
    nearby_send_receive_timeout_ms: u64,
    nearby_send_receive_packets: usize,
    profile_cache_ttl_secs: u64,
}

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
}

#[derive(Debug, Clone)]
struct LiveStartupPlan {
    config: Option<InProcessLiveFeedConfig>,
    enabled: bool,
    startup_status: LiveStartupStatus,
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

    fn send_chat(&self, text: String) {
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

    fn startup_status_line(&self) -> String {
        match &self.startup_status {
            LiveStartupStatus::DisabledByConfig => {
                String::from("disabled (VIEWER_APP_LIVE_STARTUP=off)")
            }
            LiveStartupStatus::DisabledMissingConfig => {
                String::from("disabled (missing login env); using fallback snapshot path")
            }
            LiveStartupStatus::Starting => String::from("starting"),
            LiveStartupStatus::Connected => String::from("connected"),
            LiveStartupStatus::Failed(failure) => {
                let class = startup_failure_class_label(failure.class);
                format!("failed ({class}: {})", failure.message)
            }
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
        run_probe,
        worker_tick_ms,
        event_queue_poll_timeout_ms,
        event_queue_poll_every_ticks,
        event_queue_failures_before_reconnect,
        social_poll_timeout_ms,
        social_poll_max_packets,
        nearby_poll_timeout_ms,
        nearby_poll_max_packets,
        nearby_send_receive_timeout_ms,
        nearby_send_receive_packets,
        profile_cache_ttl_secs,
    })
}

fn parse_live_startup_mode(value: Option<&str>) -> LiveStartupMode {
    match value.unwrap_or("auto").trim().to_ascii_lowercase().as_str() {
        "on" | "enabled" | "true" | "1" => LiveStartupMode::On,
        "off" | "disabled" | "false" | "0" => LiveStartupMode::Off,
        _ => LiveStartupMode::Auto,
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

async fn run_in_process_live_feed(
    config: InProcessLiveFeedConfig,
    tx: mpsc::Sender<LiveFeedUpdate>,
    command_rx: Receiver<LiveFeedCommand>,
) {
    let adapter = SecondLifeAdapter;
    let mut reconnect_attempt: u32 = 0;

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

    loop {
        let mut connection = Connection::new(ConnectionConfig {
            endpoint: config.endpoint.clone(),
            connect_timeout: std::time::Duration::from_secs(config.connect_timeout_secs),
            wire_format: config.wire_format,
        });

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
            start_location: config.start_location.clone(),
            agree_to_tos: config.agree_to_tos,
            read_critical: config.read_critical,
            mfa_token: config.mfa_token.clone(),
        };

        let Ok((result, trace, fallback)) =
            connection.login_with_trace_with_fallback(&adapter, intent).await
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
            let _ = tx.send(LiveFeedUpdate::Status(LiveStartupStatus::Failed(failure.clone())));
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
        }

        let capabilities = connection.fetch_seed_capabilities().await.ok();
        let event_queue_url = capabilities
            .as_ref()
            .and_then(|caps| caps.entries.get("EventQueueGet"))
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
        if !bootstrap_friend_ids.is_empty() {
            let mut resolved_count = 0usize;
            if let Some(url) = display_names_url.as_deref() {
                match connection
                    .resolve_avatar_display_names(url, &bootstrap_friend_ids)
                    .await
                {
                    Ok(names) => {
                        resolved_count = names.len();
                        for entry in names {
                            let _ = tx.send(LiveFeedUpdate::FriendResolvedName {
                                id: entry.id,
                                display_name: entry.display_name,
                                source: String::from("caps.GetDisplayNames"),
                            });
                        }
                    }
                    Err(err) => {
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
                    let mut profile_count = 0usize;
                    for id in &bootstrap_friend_ids {
                        if let Ok(profile) = connection.fetch_agent_profile(profile_url, id).await {
                            if let Some(display_name) = pick_best_avatar_name(&profile) {
                                profile_count = profile_count.saturating_add(1);
                                let _ = tx.send(LiveFeedUpdate::FriendResolvedName {
                                    id: id.clone(),
                                    display_name,
                                    source: String::from("caps.AgentProfile"),
                                });
                            }
                        }
                    }
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
        let mut event_ack = 0u64;
        let mut attempted_profile_image_assets = BTreeSet::new();
        let mut known_avatar_name_ids = BTreeSet::new();
        known_avatar_name_ids.extend(bootstrap_friend_ids.iter().cloned());
        if !local_agent_id.is_empty() {
            known_avatar_name_ids.insert(local_agent_id.clone());
        }
        let mut social_circuit: Option<SocialCircuit> = connection
            .open_social_circuit(&config.receive_bind)
            .await
            .ok();
        if let Some(circuit) = &social_circuit {
            let _ = connection.send_retrieve_instant_messages(circuit).await;
        } else {
            emit_relay(
                &tx,
                RuntimeRelayLevel::Warn,
                "social",
                "social circuit unavailable",
            );
        }

        update_live_visual_from_connection(&mut snapshot, &connection);
        snapshot.source = String::from("viewer_app_in_process:ready");
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

        let mut should_reconnect = false;
        let mut event_queue_consecutive_failures = 0u32;
        let mut worker_tick: u64 = 0;
        let mut event_queue_poll_task = None;
        loop {
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
                        for event in events {
                            match event {
                                SocialEvent::FriendOnline { agent_id } => {
                                    let _ = tx.send(LiveFeedUpdate::FriendPresence {
                                        id: agent_id.clone(),
                                        online: true,
                                    });
                                    emit_relay(
                                        &tx,
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
                                        &tx,
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
                                    if !im.from_name.trim().is_empty() && !participant_id.is_empty()
                                    {
                                        let _ = tx.send(LiveFeedUpdate::FriendResolvedName {
                                            id: participant_id.clone(),
                                            display_name: im.from_name.clone(),
                                            source: String::from("im.from_name"),
                                        });
                                    }
                                    let session_id = if !im.session_id.is_empty() {
                                        im.session_id.clone()
                                    } else {
                                        compute_p2p_session_id(&local_agent_id, &participant_id)
                                            .unwrap_or_default()
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
                                        &tx,
                                        RuntimeRelayLevel::Info,
                                        "im_recv",
                                        &format!("im from {}", im.from_id),
                                    );
                                }
                            }
                        }
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

            if let Ok(received) = connection
                .poll_nearby_chat_udp(
                    &config.receive_bind,
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
                if event_queue_poll_task.is_none()
                    && worker_tick % u64::from(config.event_queue_poll_every_ticks) == 0
                {
                    let url = url.to_string();
                    let ack = event_ack;
                    let timeout =
                        std::time::Duration::from_millis(config.event_queue_poll_timeout_ms);
                    event_queue_poll_task = Some(tokio::spawn(async move {
                        poll_event_queue_url_once(&url, ack, timeout).await
                    }));
                }
                let task_finished = event_queue_poll_task
                    .as_ref()
                    .map(|task| task.is_finished())
                    .unwrap_or(false);
                if task_finished {
                    if let Some(task) = event_queue_poll_task.take() {
                        match task.await {
                            Ok(Ok(poll)) => {
                                event_queue_consecutive_failures = 0;
                                if let Some(next_ack) = poll.id {
                                    event_ack = next_ack;
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
                                let should_log = event_queue_consecutive_failures == 1
                                    || event_queue_consecutive_failures % 10 == 0;
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
            }

            if should_reconnect {
                let failure = LiveStartupFailure {
                    class: LiveStartupFailureClass::ConnectionLostReconnecting,
                    message: String::from("connection lost; reconnecting"),
                };
                let _ = tx.send(LiveFeedUpdate::ChatConnection(
                    ChatConnectionState::Reconnecting,
                ));
                let _ = tx.send(LiveFeedUpdate::Status(LiveStartupStatus::Failed(failure)));
                emit_relay(
                    &tx,
                    RuntimeRelayLevel::Warn,
                    "reconnect",
                    "connection lost; reconnecting",
                );
                break;
            }
            update_live_visual_from_connection(&mut snapshot, &connection);
            snapshot.source = String::from("viewer_app_in_process:ready");
            let _ = tx.send(LiveFeedUpdate::Snapshot(snapshot.clone()));
            let avatar_samples = extract_worker_world_avatar_samples(&connection, &local_agent_id);
            let _ = tx.send(LiveFeedUpdate::WorldAvatars {
                avatars: avatar_samples.clone(),
                decoded_sim_name: extract_worker_world_sim_name(&connection),
                startup_sim_name: startup_sim_name.clone(),
                self_location: extract_worker_self_location(&connection),
                observed_at_unix_ms: now_unix_ms(),
            });
            if worker_tick % 40 == 0 {
                let unresolved_ids: Vec<String> = avatar_samples
                    .iter()
                    .filter_map(|sample| sample.agent_id.clone())
                    .filter(|id| id != &local_agent_id && !known_avatar_name_ids.contains(id))
                    .collect();
                if !unresolved_ids.is_empty() {
                    let mut resolved_any = false;
                    if let Some(url) = display_names_url.as_deref() {
                        match connection
                            .resolve_avatar_display_names(url, &unresolved_ids)
                            .await
                        {
                            Ok(names) => {
                                if !names.is_empty() {
                                    resolved_any = true;
                                }
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
                                emit_relay(
                                    &tx,
                                    RuntimeRelayLevel::Warn,
                                    "avatar_name",
                                    &format!("display name lookup failed: {err}"),
                                );
                            }
                        }
                    }
                    if !resolved_any {
                        if let Some(profile_url) = agent_profile_url.as_deref() {
                            let mut profile_resolved = 0usize;
                            for id in &unresolved_ids {
                                if let Ok(profile) =
                                    connection.fetch_agent_profile(profile_url, id).await
                                {
                                    if let Some(display_name) = pick_best_avatar_name(&profile) {
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
                            }
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
    text: &str,
    bind: &str,
    receive_timeout: std::time::Duration,
    receive_max_packets: usize,
) -> std::result::Result<Vec<NearbyChatMessage>, ConnectionError> {
    connection
        .send_nearby_chat(text, bind, receive_timeout, receive_max_packets)
        .await
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
        observed_at_unix_ms: now_unix_ms(),
    };

    if let GridLoginResult::Success(bootstrap) = result {
        snapshot.logged_in = true;
        snapshot.first_sim_endpoint = Some(format!(
            "{}:{}",
            bootstrap.first_sim.sim_ip, bootstrap.first_sim.sim_port
        ));
        snapshot.first_sim_region_x = Some(bootstrap.first_sim.region_x);
        snapshot.first_sim_region_y = Some(bootstrap.first_sim.region_y);
    }

    snapshot
}

fn update_live_visual_from_connection(snapshot: &mut LiveVisualSnapshot, connection: &Connection) {
    snapshot.observed_at_unix_ms = now_unix_ms();
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
        Some(LoginFallbackClassifiedReason::RequestShapeMissingPassword) => "request_shape_missing_password",
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
    if fallback.classified_reason == Some(LoginFallbackClassifiedReason::RequestShapeMissingPassword)
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

fn parse_start_location(value: &str) -> StartLocationIntent {
    if value.eq_ignore_ascii_case("home") {
        StartLocationIntent::Saved(StartLocation::Home)
    } else if value.eq_ignore_ascii_case("last") {
        StartLocationIntent::Saved(StartLocation::Last)
    } else {
        StartLocationIntent::Uri(value.to_string())
    }
}

fn parse_bool_like(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn parse_region_name_from_start_location(value: &str) -> Option<String> {
    let raw = value.trim();
    if raw.is_empty() || raw.eq_ignore_ascii_case("last") || raw.eq_ignore_ascii_case("home") {
        return None;
    }
    if let Some(pos) = raw.find("secondlife://") {
        let tail = &raw[pos + "secondlife://".len()..];
        let name = tail.split('/').next().unwrap_or("").trim();
        if !name.is_empty() {
            return Some(name.to_string());
        }
    }
    if let Some(pos) = raw.find("uri:") {
        let tail = &raw[pos + 4..];
        let name = tail.split('&').next().unwrap_or("").trim();
        if !name.is_empty() {
            return Some(name.to_string());
        }
    }
    None
}

fn pick_best_avatar_name(profile: &AgentProfileData) -> Option<String> {
    if let Some(name) = profile.display_name.as_ref().map(|v| v.trim()) {
        if !name.is_empty() {
            return Some(name.to_string());
        }
    }
    if let Some(name) = profile.username.as_ref().map(|v| v.trim()) {
        if !name.is_empty() {
            return Some(name.to_string());
        }
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
    if let Some(friend) = social_state.friends.iter().find(|f| f.id == agent_id) {
        if let Some(name) = friend.display_name.as_deref() {
            return format!("{name} ({})", uuid_short(agent_id));
        }
    }
    uuid_short(agent_id)
}

fn format_avatar_label_with_cache(
    agent_id: &str,
    social_state: &SocialState,
    avatar_name_cache: &BTreeMap<String, String>,
) -> String {
    if let Some(friend) = social_state.friends.iter().find(|f| f.id == agent_id) {
        if let Some(name) = friend.display_name.as_deref() {
            return format!("{name} ({})", uuid_short(agent_id));
        }
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

fn merge_world_avatar_samples(
    current: &mut Vec<WorldAvatarPlaceholder>,
    social_state: &SocialState,
    avatar_name_cache: &BTreeMap<String, String>,
    decoded_world_sim_name: Option<&str>,
    startup_sim_name_fallback: Option<&str>,
    samples: &[WorkerWorldAvatarSample],
    region_coords: Option<[u32; 2]>,
    observed_at_unix_ms: u64,
    stale_after_ms: u64,
) -> Vec<(String, String)> {
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
            });
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

fn emit_relay(
    tx: &mpsc::Sender<LiveFeedUpdate>,
    level: RuntimeRelayLevel,
    category: &str,
    message: &str,
) {
    let line = format!("[{}] {category}: {message}", now_unix_ms());
    println!("{line}");
    let _ = fs::create_dir_all("logs");
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("logs/viewer_app_runtime.jsonl")
    {
        let json_line = format!(
            "{{\"ts\":{},\"category\":\"{}\",\"message\":\"{}\"}}\n",
            now_unix_ms(),
            category.replace('"', "'"),
            message.replace('"', "'")
        );
        let _ = file.write_all(json_line.as_bytes());
    }
    let _ = tx.send(LiveFeedUpdate::Relay(RuntimeRelayEvent {
        at_unix_ms: now_unix_ms(),
        level,
        category: category.to_string(),
        message: message.to_string(),
    }));
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
        if let Some(cache) = social_cache.as_ref() {
            if let Ok(cached) = cache.load_cached_social_state() {
                social_state = cached;
            }
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

        Ok(AppState {
            window,
            renderer,
            ui,
            camera: Camera::default(),
            scene: Scene::prototype(),
            world_ingestion_seam: WorldObjectIngestionSeam::default(),
            last_applied_live_visual_snapshot: None,
            last_applied_world_ingestion_seam: None,
            input: InputState::default(),
            live_visual_state: LiveVisualState::from_env(),
            chat_state: ChatState::default(),
            social_state,
            world_avatars: Vec::new(),
            avatar_name_cache,
            world_sim_name: None,
            startup_sim_name_fallback: None,
            world_self_location: None,
            profile_state: None,
            profile_image_bytes: BTreeMap::new(),
            social_cache: social_cache.take(),
            last_frame_time: Instant::now(),
            smoothed_fps: 0.0,
            smoothed_frame_ms: 0.0,
            avg_scene_update_ms: 0.0,
        })
    }
}

impl AppState {
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

        let [look_x, look_y] = self.input.take_look_delta();
        self.camera.add_look_delta(look_x, look_y);
        self.input.update_camera(&mut self.camera, dt_seconds);

        for update in self.live_visual_state.drain_worker_updates() {
            match update {
                LiveFeedUpdate::Snapshot(snapshot) => {
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
                    let relay = merge_world_avatar_samples(
                        &mut self.world_avatars,
                        &self.social_state,
                        &self.avatar_name_cache,
                        self.world_sim_name.as_deref(),
                        self.startup_sim_name_fallback.as_deref(),
                        &avatars,
                        region_coords,
                        observed_at_unix_ms,
                        12_000,
                    );
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
                    self.social_state.relay.push(event);
                }
            }
        }
        self.live_visual_state.refresh();
        let next_live_visual_snapshot = self.live_visual_state.snapshot.clone();
        let next_world_ingestion_seam =
            WorldObjectIngestionAdapter::adapt(next_live_visual_snapshot.as_ref());
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

        let window = self.window.clone();
        let ui = &mut self.ui;
        let camera = self.camera;
        let live_visual = next_live_visual_snapshot;
        let live_startup_status = self.live_visual_state.startup_status_line();
        let chat_state = &mut self.chat_state;
        let social_state = &mut self.social_state;
        let world_avatars = &self.world_avatars;
        let world_sim_name = self.world_sim_name.clone();
        let world_self_location = self.world_self_location;
        let profile_state = &mut self.profile_state;
        let profile_image_bytes = &self.profile_image_bytes;
        let mut pending_chat_send: Option<String> = None;
        let mut pending_direct_im_send: Option<(String, String)> = None;
        let mut pending_profile_open: Option<String> = None;
        let mut pending_profile_tab_select: Option<(String, AvatarProfileTab)> = None;
        let mut pending_profile_refresh: Option<(String, Option<AvatarProfileTab>)> = None;
        let mut pending_open_external_url: Option<String> = None;

        let aspect = self.renderer.aspect_ratio();
        let frustum = self.camera.frustum(aspect);
        let visibility_list = self.scene.query_frustum(&frustum);
        let metrics = self.scene.metrics_with_visibility(&visibility_list);

        let render_result = self.renderer.render_frame(
            &camera,
            &self.scene,
            &visibility_list,
            |device, queue, encoder, target_view, surface_size| {
                let actions = ui.render(
                    &window,
                    device,
                    queue,
                    encoder,
                    target_view,
                    surface_size,
                    &camera,
                    live_visual.as_ref(),
                    &live_startup_status,
                    chat_state,
                    social_state,
                    world_avatars,
                    world_sim_name.as_deref(),
                    world_self_location,
                    profile_state,
                    profile_image_bytes,
                    self.smoothed_fps,
                    self.smoothed_frame_ms,
                    self.avg_scene_update_ms,
                    metrics.total_instances,
                    metrics.visible_proxies,
                );
                pending_chat_send = actions.nearby_chat_send;
                pending_direct_im_send = actions.direct_im_send;
                pending_profile_open = actions.open_avatar_profile;
                pending_profile_tab_select = actions.select_avatar_profile_tab;
                pending_profile_refresh = actions.refresh_avatar_profile;
                pending_open_external_url = actions.open_external_url;
            },
        );

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
        if let Some((to_agent_id, text)) = pending_direct_im_send {
            if !text.is_empty() {
                self.social_state.im_draft.clear();
                self.live_visual_state.send_direct_im(to_agent_id, text);
            }
        }
        if let Some(avatar_id) = pending_profile_open {
            self.live_visual_state.open_avatar_profile(avatar_id);
        }
        if let Some((avatar_id, tab)) = pending_profile_tab_select {
            if should_fetch_profile_tab(
                self.profile_state.as_ref(),
                &avatar_id,
                tab,
                self.live_visual_state.profile_cache_ttl_secs,
            ) {
                self.live_visual_state
                    .select_avatar_profile_tab(avatar_id, tab);
            }
        }
        if let Some((avatar_id, tab)) = pending_profile_refresh {
            self.live_visual_state
                .refresh_avatar_profile(avatar_id, tab);
        }
        if let Some(url) = pending_open_external_url {
            open_external_url(&url);
        }

        render_result
    }

    fn handle_input_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use viewer_grid::{GridLoginError, GridLoginErrorClass};

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
            String::from("VIEWER_APP_WORKER_TICK_MS"),
            String::from("25"),
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
        let cfg = in_process_live_feed_config_from_lookup(|k| vars.get(k).cloned())
            .expect("config should parse");
        assert_eq!(cfg.wire_format, LoginWireFormat::XmlRpc);
        assert_eq!(cfg.receive_max_packets, 16);
        assert!(!cfg.run_probe);
        assert_eq!(cfg.event_queue_failures_before_reconnect, 9);
        assert_eq!(cfg.worker_tick_ms, 25);
        assert!(cfg.agree_to_tos);
        assert!(!cfg.read_critical);
        assert_eq!(cfg.mfa_token.as_deref(), Some("token123"));
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
    fn parse_bool_like_accepts_expected_truthy_forms() {
        assert!(parse_bool_like("true"));
        assert!(parse_bool_like("1"));
        assert!(parse_bool_like("YES"));
        assert!(!parse_bool_like("false"));
        assert!(!parse_bool_like("0"));
    }

    #[test]
    fn parse_live_startup_mode_defaults_to_auto_and_parses_values() {
        assert_eq!(parse_live_startup_mode(None), LiveStartupMode::Auto);
        assert_eq!(parse_live_startup_mode(Some("on")), LiveStartupMode::On);
        assert_eq!(parse_live_startup_mode(Some("off")), LiveStartupMode::Off);
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
            observed_at_unix_ms: 1,
        }));
        assert!(should_apply_world_ingestion_seam(Some(&empty), &changed));
        assert!(!should_apply_world_ingestion_seam(Some(&changed), &changed));
        let mut viewer_time_changed_snapshot = LiveVisualSnapshot {
            source: String::from("test"),
            logged_in: true,
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
        }];

        apply_avatar_render_mode(&mut scene, &mut social, &avatars, AvatarRenderMode::Proxy);
        assert_eq!(social.avatar_render_mode, AvatarRenderMode::Proxy);
        assert!(
            scene.instances.iter().any(|instance| {
                instance.role == viewer_core::InstanceRole::WorldAvatarPlaceholderOther
                    && instance.mesh == viewer_core::MeshKind::AvatarProxy
            })
        );

        apply_avatar_render_mode(
            &mut scene,
            &mut social,
            &avatars,
            AvatarRenderMode::FallbackBox,
        );
        assert_eq!(social.avatar_render_mode, AvatarRenderMode::FallbackBox);
        assert!(
            scene.instances.iter().any(|instance| {
                instance.role == viewer_core::InstanceRole::WorldAvatarPlaceholderOther
                    && instance.mesh == viewer_core::MeshKind::Cube
            })
        );
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
}
