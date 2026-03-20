use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tracing_subscriber::FmtSubscriber;
use viewer_core::{
    Camera, LiveVisualSnapshot, Scene, WorldObjectIngestionAdapter, WorldObjectIngestionSeam,
};
use viewer_grid::{GridLoginResult, LoginIntent, SecondLifeAdapter, StartLocation, StartLocationIntent};
use viewer_net::{Connection, ConnectionConfig, FirstSimulatorInboundTrafficScope, LoginWireFormat};
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
    input: InputState,
    live_visual_state: LiveVisualState,
    last_frame_time: Instant,
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
    in_process_rx: Option<Receiver<LiveVisualSnapshot>>,
    in_process_enabled: bool,
}

#[derive(Debug, Clone)]
struct InProcessLiveFeedConfig {
    endpoint: String,
    username: String,
    password: String,
    connect_timeout_secs: u64,
    wire_format: LoginWireFormat,
    start_location: StartLocationIntent,
    receive_bind: String,
    receive_timeout_secs: u64,
    receive_max_packets: usize,
    post_movement_tail_packets: usize,
    post_movement_timeout_secs: Option<u64>,
    stop_on_region_control: bool,
    run_probe: bool,
}

impl LiveVisualState {
    fn from_env() -> Self {
        let path = std::env::var("VIEWER_LIVE_VISUAL_SNAPSHOT_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("live_visual_snapshot.json"));
        let in_process_config =
            in_process_live_feed_config_from_lookup(|key| std::env::var(key).ok());
        let in_process_enabled = in_process_config.is_some();
        let in_process_rx = in_process_config.map(spawn_in_process_live_feed);

        Self {
            path,
            last_modified: None,
            snapshot: None,
            in_process_rx,
            in_process_enabled,
        }
    }

    fn refresh(&mut self) {
        let mut got_in_process_update = false;
        if let Some(rx) = &self.in_process_rx {
            while let Ok(snapshot) = rx.try_recv() {
                self.snapshot = Some(snapshot);
                got_in_process_update = true;
            }
        }

        if got_in_process_update || self.in_process_enabled {
            if self.snapshot.is_some() {
                return;
            }
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
    let post_movement_timeout_secs = lookup("VIEWER_FIRST_SIM_POST_MOVEMENT_TIMEOUT_SECS")
        .and_then(|v| v.parse::<u64>().ok());
    let stop_on_region_control = lookup("VIEWER_FIRST_SIM_STOP_ON_REGION_CONTROL")
        .map(|v| parse_bool_like(&v))
        .unwrap_or(false);
    let run_probe = lookup("VIEWER_APP_IN_PROCESS_PROBE")
        .map(|v| parse_bool_like(&v))
        .unwrap_or(true);

    Some(InProcessLiveFeedConfig {
        endpoint,
        username,
        password,
        connect_timeout_secs,
        wire_format,
        start_location,
        receive_bind,
        receive_timeout_secs,
        receive_max_packets,
        post_movement_tail_packets,
        post_movement_timeout_secs,
        stop_on_region_control,
        run_probe,
    })
}

fn spawn_in_process_live_feed(config: InProcessLiveFeedConfig) -> Receiver<LiveVisualSnapshot> {
    let (tx, rx) = mpsc::channel::<LiveVisualSnapshot>();
    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        let Ok(runtime) = runtime else {
            return;
        };
        runtime.block_on(async move {
            run_in_process_live_feed(config, tx).await;
        });
    });
    rx
}

async fn run_in_process_live_feed(
    config: InProcessLiveFeedConfig,
    tx: mpsc::Sender<LiveVisualSnapshot>,
) {
    let _ = tx.send(LiveVisualSnapshot {
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
        observed_at_unix_ms: now_unix_ms(),
    });

    let mut connection = Connection::new(ConnectionConfig {
        endpoint: config.endpoint.clone(),
        connect_timeout: std::time::Duration::from_secs(config.connect_timeout_secs),
        wire_format: config.wire_format,
    });
    let adapter = SecondLifeAdapter;

    if connection.connect().await.is_err() {
        return;
    }

    let intent = LoginIntent {
        username: config.username.clone(),
        password: config.password.clone(),
        start_location: config.start_location.clone(),
        agree_to_tos: false,
        read_critical: true,
        mfa_token: None,
    };

    let Ok((result, _trace)) = connection.login_with_trace(&adapter, intent).await else {
        return;
    };
    let mut snapshot = build_live_visual_snapshot_from_result(&result);
    let _ = tx.send(snapshot.clone());

    if config.run_probe && matches!(result, GridLoginResult::Success(_)) {
        let _ = connection
            .probe_first_simulator_handshake_window_with_policy(
                &config.receive_bind,
                std::time::Duration::from_secs(config.receive_timeout_secs),
                config.receive_max_packets,
                config.post_movement_tail_packets,
                config.post_movement_timeout_secs
                    .map(std::time::Duration::from_secs),
                config.stop_on_region_control,
            )
            .await;
    }

    update_live_visual_from_connection(&mut snapshot, &connection);
    snapshot.source = String::from("viewer_app_in_process:ready");
    let _ = tx.send(snapshot);
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
}

fn parse_wire_format(value: &str) -> LoginWireFormat {
    match value.trim().to_ascii_lowercase().as_str() {
        "json" => LoginWireFormat::Json,
        "xmlrpc" | "xml-rpc" => LoginWireFormat::XmlRpc,
        _ => LoginWireFormat::Llsd,
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

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
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

        Ok(AppState {
            window,
            renderer,
            ui,
            camera: Camera::default(),
            scene: Scene::prototype(),
            world_ingestion_seam: WorldObjectIngestionSeam::default(),
            input: InputState::default(),
            live_visual_state: LiveVisualState::from_env(),
            last_frame_time: Instant::now(),
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

        let [look_x, look_y] = self.input.take_look_delta();
        self.camera.add_look_delta(look_x, look_y);
        self.input.update_camera(&mut self.camera, dt_seconds);
        self.live_visual_state.refresh();
        self.world_ingestion_seam =
            WorldObjectIngestionAdapter::adapt(self.live_visual_state.snapshot.as_ref());
        self.scene
            .apply_live_visual_snapshot(self.live_visual_state.snapshot.as_ref());
        self.scene
            .apply_world_object_ingestion_seam(&self.world_ingestion_seam);

        let window = self.window.clone();
        let ui = &mut self.ui;
        let camera = self.camera;
        let live_visual = self.live_visual_state.snapshot.clone();

        self.renderer.render_frame(
            &camera,
            &self.scene,
            |device, queue, encoder, target_view, surface_size| {
                ui.render(
                    &window,
                    device,
                    queue,
                    encoder,
                    target_view,
                    surface_size,
                    &camera,
                    live_visual.as_ref(),
                );
            },
        )
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
        let cfg = in_process_live_feed_config_from_lookup(|k| vars.get(k).cloned())
            .expect("config should parse");
        assert_eq!(cfg.wire_format, LoginWireFormat::XmlRpc);
        assert_eq!(cfg.receive_max_packets, 16);
        assert!(!cfg.run_probe);
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
}
