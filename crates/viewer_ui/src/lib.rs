use egui::NumExt;
use egui_wgpu::{Renderer, ScreenDescriptor};
use egui_winit::State;
use std::collections::{BTreeMap, HashMap};
use viewer_core::{
    AvatarProfileState, AvatarProfileTab, AvatarRenderMode, Camera, ChatConnectionState,
    ChatSendStatus, ChatState, HandoffOutcome, HandoffReason, LiveVisualSnapshot,
    NetworkDebugState, ProbeResultCode, ProfileFreshness, ProfileLoadStatus, RecoveryAction,
    RecoveryActionResult, RecoveryResultCode, RuntimeRelayLevel, SessionUxReason, SessionUxStatus,
    SocialState, TransitionVisualState, WorldAvatarPlaceholder,
};
use wgpu::{
    CommandEncoder, Device, LoadOp, Operations, Queue, RenderPassColorAttachment,
    RenderPassDescriptor, StoreOp, TextureFormat, TextureView,
};
use winit::{dpi::PhysicalSize, event::WindowEvent, window::Window};

fn live_visual_lines(snapshot: Option<&LiveVisualSnapshot>) -> Vec<String> {
    match snapshot {
        Some(snapshot) => {
            let mut lines = vec![
                format!("Source: {}", snapshot.source),
                format!("Logged in: {}", snapshot.logged_in),
                format!(
                    "Current region: {}",
                    snapshot.current_region_name.as_deref().unwrap_or("n/a")
                ),
                format!(
                    "Handshake AMC reached: {}",
                    snapshot.handshake_agent_movement_complete
                ),
                format!(
                    "First sim: {} ({:?}, {:?})",
                    snapshot.first_sim_endpoint.as_deref().unwrap_or("n/a"),
                    snapshot.first_sim_region_x,
                    snapshot.first_sim_region_y
                ),
            ];
            if snapshot.traffic_summary_available {
                lines.push(format!(
                    "Post-boundary: obs={}, region_ctrl={}, broader={}, unknown={}",
                    snapshot.post_boundary_observations,
                    snapshot.region_transition_control_observations,
                    snapshot.likely_broader_traffic,
                    snapshot.unknown
                ));
                lines.push(format!(
                    "CrossedRegion={}, ConfirmEnableSimulator={}",
                    snapshot.crossed_region, snapshot.confirm_enable_simulator
                ));
            } else {
                lines.push(String::from("Traffic summary: unavailable"));
            }

            lines.push(format!(
                "Continuity Phase: {:?} ({:?}, {}, {}ms)",
                snapshot.continuity.phase,
                snapshot.continuity.outcome,
                handoff_reason_label(snapshot.continuity.reason),
                snapshot.continuity.phase_age_ms
            ));
            if let Some([x, y]) = snapshot.continuity.active_region_coords {
                lines.push(format!("Active Region: {}, {}", x, y));
            }
            if let Some([px, py]) = snapshot.continuity.previous_region_coords {
                lines.push(format!("Prev Region: {}, {}", px, py));
            }
            if !snapshot.continuity.neighbors.is_empty() {
                lines.push(format!(
                    "Neighbors: {}",
                    snapshot.continuity.neighbors.len()
                ));
            }
            lines
        }
        None => vec![
            String::from("No live snapshot loaded."),
            String::from("Run viewer_net example to write live_visual_snapshot.json"),
        ],
    }
}

fn render_profile_image(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    image_bytes: &BTreeMap<String, Vec<u8>>,
    texture_cache: &mut HashMap<String, egui::TextureHandle>,
    decode_failures: &mut HashMap<String, String>,
    image_id: Option<&str>,
    tag: &str,
) {
    let Some(image_id) = image_id else {
        ui.label("Image: n/a");
        return;
    };
    if let Some(texture) = texture_cache.get(image_id) {
        let size = texture.size_vec2();
        let scale = (220.0 / size.x.max(1.0))
            .min(160.0 / size.y.max(1.0))
            .min(1.0);
        ui.image((texture.id(), egui::vec2(size.x * scale, size.y * scale)));
        return;
    }
    if let Some(reason) = decode_failures.get(image_id) {
        ui.weak(format!("Image {image_id}: {reason}"));
        return;
    }
    let Some(bytes) = image_bytes.get(image_id) else {
        ui.weak(format!("Image {image_id}: loading"));
        return;
    };
    match decode_profile_image_color(bytes) {
        Ok(color_image) => {
            let handle = ctx.load_texture(
                format!("profile_image_{tag}_{image_id}"),
                color_image,
                egui::TextureOptions::LINEAR,
            );
            texture_cache.insert(image_id.to_string(), handle);
            if let Some(texture) = texture_cache.get(image_id) {
                let size = texture.size_vec2();
                let scale = (220.0 / size.x.max(1.0))
                    .min(160.0 / size.y.max(1.0))
                    .min(1.0);
                ui.image((texture.id(), egui::vec2(size.x * scale, size.y * scale)));
            }
        }
        Err(err) => {
            decode_failures.insert(image_id.to_string(), String::from("unsupported format"));
            ui.weak(format!("Image {image_id}: unsupported format ({err})"));
        }
    }
}

fn decode_profile_image_color(bytes: &[u8]) -> Result<egui::ColorImage, String> {
    if let Ok(decoded) = image::load_from_memory(bytes) {
        let rgba = decoded.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        return Ok(egui::ColorImage::from_rgba_unmultiplied(
            size,
            rgba.as_raw(),
        ));
    }

    if let Ok(j2k) = jpeg2k::Image::from_bytes(bytes)
        && let Ok(decoded) = image::DynamicImage::try_from(&j2k)
    {
        let rgba = decoded.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        return Ok(egui::ColorImage::from_rgba_unmultiplied(
            size,
            rgba.as_raw(),
        ));
    }

    let jp2 = justjp2::decode(bytes).map_err(|err| err.to_string())?;
    if jp2.components.is_empty() || jp2.width == 0 || jp2.height == 0 {
        return Err(String::from("empty JP2 image"));
    }
    let width = jp2.width as usize;
    let height = jp2.height as usize;
    let mut rgba = vec![0u8; width * height * 4];
    for y in 0..height {
        for x in 0..width {
            let r = sample_jp2_component_u8(&jp2.components, 0, x, y, width, height);
            let g = sample_jp2_component_u8(&jp2.components, 1, x, y, width, height);
            let b = sample_jp2_component_u8(&jp2.components, 2, x, y, width, height);
            let alpha = sample_jp2_component_u8(&jp2.components, 3, x, y, width, height);
            let idx = (y * width + x) * 4;
            rgba[idx] = r;
            rgba[idx + 1] = g;
            rgba[idx + 2] = b;
            rgba[idx + 3] = if jp2.components.len() >= 4 {
                alpha
            } else {
                255
            };
        }
    }
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        [width, height],
        &rgba,
    ))
}

fn sample_jp2_component_u8(
    components: &[justjp2::Component],
    component_idx: usize,
    x: usize,
    y: usize,
    out_width: usize,
    out_height: usize,
) -> u8 {
    let component = components
        .get(component_idx)
        .or_else(|| components.first())
        .expect("jp2 components non-empty");
    let comp_width = component.width.max(1) as usize;
    let comp_height = component.height.max(1) as usize;
    let sx = (x * comp_width) / out_width.max(1);
    let sy = (y * comp_height) / out_height.max(1);
    let idx = sy.saturating_mul(comp_width).saturating_add(sx);
    let sample = *component.data.get(idx).unwrap_or(&0);
    let precision = component.precision.clamp(1, 31);
    let max = ((1i64 << precision) - 1).max(1);
    let normalized = if component.signed {
        let bias = 1i64 << (precision - 1);
        (i64::from(sample) + bias).clamp(0, max)
    } else {
        i64::from(sample).clamp(0, max)
    };
    ((normalized * 255) / max) as u8
}

fn total_live_failures(metrics: &viewer_core::AssetContinuityMetrics) -> usize {
    metrics.live_requests_failed_transport
        + metrics.live_requests_failed_decode
        + metrics.live_requests_failed_timeout
        + metrics.live_requests_failed_missing_capability
        + metrics.live_requests_failed_other
}

fn short_id(value: &str) -> String {
    value.chars().take(8).collect()
}

/// Thin wrapper for all egui pieces that the viewer exposes.
pub struct UiSystem {
    egui_ctx: egui::Context,
    egui_state: State,
    egui_renderer: Renderer,
    active_chat_target: ChatTarget,
    thread_filter: ThreadFilter,
    profile_texture_cache: HashMap<String, egui::TextureHandle>,
    profile_texture_failures: HashMap<String, String>,
    network_debug_teleport_target: String,
    relay_filter_text: String,
    relay_level_filter: Option<RuntimeRelayLevel>,
    pub show_diagnostics: bool,
    pub show_network_debug: bool,
    pub show_social: bool,
    pub focus_continuity_requested: bool,
    pub focus_social_requested: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChatTarget {
    Nearby,
    DirectIm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ThreadFilter {
    All,
    Online,
    Recent,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UiActions {
    pub nearby_chat_send: Option<String>,
    pub direct_im_send: Option<(String, String)>,
    pub open_avatar_profile: Option<String>,
    pub select_avatar_profile_tab: Option<(String, AvatarProfileTab)>,
    pub refresh_avatar_profile: Option<(String, Option<AvatarProfileTab>)>,
    pub open_external_url: Option<String>,
    pub teleport_via_slurl: Option<String>,
    pub retry_continuity_probe: bool,
    pub refresh_visible_assets: bool,
    pub clear_recovery_banner: bool,
}

pub struct RenderInput<'a> {
    pub window: &'a Window,
    pub device: &'a Device,
    pub queue: &'a Queue,
    pub encoder: &'a mut CommandEncoder,
    pub target_view: &'a TextureView,
    pub surface_size: PhysicalSize<u32>,
    pub camera: &'a Camera,
    pub live_visual: Option<&'a LiveVisualSnapshot>,
    pub session_status: SessionUxStatus,
    pub chat_state: &'a mut ChatState,
    pub social_state: &'a mut SocialState,
    pub world_avatars: &'a [WorldAvatarPlaceholder],
    pub world_sim_name: Option<&'a str>,
    pub world_self_location: Option<[f32; 3]>,
    pub profile_state: &'a mut Option<AvatarProfileState>,
    pub profile_image_bytes: &'a BTreeMap<String, Vec<u8>>,
    pub now_unix_ms: u64,
    pub profile_cache_ttl_secs: u64,
    pub fps: f32,
    pub frame_ms: f32,
    pub avg_scene_update_ms: f32,
    pub total_instances: usize,
    pub visible_proxies: usize,
    pub fixture_texture_metrics: viewer_core::AssetContinuityMetrics,
    pub environment: &'a viewer_core::EnvironmentState,
    pub transition_visual_state: &'a TransitionVisualState,
    pub last_recovery_result: Option<&'a RecoveryActionResult>,
    pub can_retry_probe: bool,
    pub can_refresh_assets: bool,
    pub show_chat_window: bool,
    pub network_debug: &'a NetworkDebugState,
}

fn should_submit_on_enter(enter_pressed: bool, shift_held: bool) -> bool {
    enter_pressed && !shift_held
}

fn connection_chip(connection: &ChatConnectionState) -> (&'static str, egui::Color32) {
    match connection {
        ChatConnectionState::Disabled => ("disabled", egui::Color32::GRAY),
        ChatConnectionState::Connecting => ("connecting", egui::Color32::YELLOW),
        ChatConnectionState::Connected => ("connected", egui::Color32::GREEN),
        ChatConnectionState::Reconnecting => ("reconnecting", egui::Color32::YELLOW),
        ChatConnectionState::Failed(_) => ("failed", egui::Color32::RED),
    }
}

fn send_chip(send_status: &ChatSendStatus) -> Option<(&'static str, egui::Color32)> {
    match send_status {
        ChatSendStatus::Idle => None,
        ChatSendStatus::Sending => Some(("sending", egui::Color32::YELLOW)),
        ChatSendStatus::Sent => Some(("sent", egui::Color32::LIGHT_GREEN)),
        ChatSendStatus::Failed(_) => Some(("send failed", egui::Color32::RED)),
    }
}

fn avatar_render_mode_label(mode: AvatarRenderMode) -> &'static str {
    match mode {
        AvatarRenderMode::Proxy => "proxy",
        AvatarRenderMode::FallbackBox => "fallback-box",
    }
}

fn session_reason_label(reason: SessionUxReason) -> &'static str {
    match reason {
        SessionUxReason::DisabledByConfig => "disabled-by-config",
        SessionUxReason::MissingConfig => "missing-config",
        SessionUxReason::ConnectTransport => "connect-transport",
        SessionUxReason::LoginTransport => "login-transport",
        SessionUxReason::LoginAuth => "login-auth",
        SessionUxReason::LoginRequiresTos => "login-requires-tos",
        SessionUxReason::LoginRequiresMfa => "login-requires-mfa",
        SessionUxReason::LoginUpdateRequired => "login-update-required",
        SessionUxReason::ConnectionLost => "connection-lost",
        SessionUxReason::Other => "other",
    }
}

fn probe_result_label(code: ProbeResultCode) -> &'static str {
    match code {
        ProbeResultCode::Success => "success",
        ProbeResultCode::Timeout => "timeout",
        ProbeResultCode::TransportError => "transport-error",
        ProbeResultCode::HttpFailure => "http-failure",
        ProbeResultCode::Unavailable => "unavailable",
    }
}

fn recovery_result_label(result: &RecoveryActionResult) -> String {
    let action = match result.action {
        RecoveryAction::RetryContinuityProbe => "retry continuity probe",
        RecoveryAction::RefreshVisibleAssets => "refresh visible assets",
        RecoveryAction::ClearRecoveryBanner => "clear recovery banner",
    };
    let status = match result.code {
        RecoveryResultCode::Accepted => "accepted".to_string(),
        RecoveryResultCode::CooldownActive => {
            if let Some(remaining) = result.cooldown_remaining_ms {
                format!("cooldown ({} ms remaining)", remaining)
            } else {
                String::from("cooldown")
            }
        }
        RecoveryResultCode::Unavailable => String::from("unavailable"),
        RecoveryResultCode::Completed(code) => {
            format!("completed ({})", probe_result_label(code))
        }
    };
    if let Some(detail) = &result.detail {
        format!("{action}: {status} - {detail}")
    } else {
        format!("{action}: {status}")
    }
}

fn session_status_chip(
    status: &SessionUxStatus,
) -> (&'static str, egui::Color32, Option<SessionUxReason>) {
    match status {
        SessionUxStatus::Disabled { reason } => ("disabled", egui::Color32::GRAY, *reason),
        SessionUxStatus::Starting => ("starting", egui::Color32::YELLOW, None),
        SessionUxStatus::Connected => ("connected", egui::Color32::GREEN, None),
        SessionUxStatus::Reconnecting { reason } => {
            ("reconnecting", egui::Color32::YELLOW, *reason)
        }
        SessionUxStatus::Failed { reason } => ("failed", egui::Color32::RED, Some(*reason)),
    }
}

fn handoff_outcome_chip(outcome: HandoffOutcome) -> (&'static str, egui::Color32) {
    match outcome {
        HandoffOutcome::Normal => ("normal", egui::Color32::GREEN),
        HandoffOutcome::Degraded => ("degraded", egui::Color32::YELLOW),
        HandoffOutcome::Stalled => ("STALLED", egui::Color32::RED),
    }
}

fn handoff_reason_label(reason: HandoffReason) -> &'static str {
    match reason {
        HandoffReason::None => "none",
        HandoffReason::LateConfirmation => "late_confirmation",
        HandoffReason::StaleWindowExceeded => "stale_window",
        HandoffReason::MissingCrossedRegion => "missing_crossed",
        HandoffReason::NetworkJitter => "jitter",
    }
}

fn sorted_friend_ids_for_filter(social_state: &SocialState, filter: ThreadFilter) -> Vec<String> {
    let mut ids: Vec<String> = social_state.friends.iter().map(|f| f.id.clone()).collect();
    match filter {
        ThreadFilter::All => {
            ids.sort_by(|a, b| {
                let a_name = social_state
                    .friends
                    .iter()
                    .find(|f| &f.id == a)
                    .map(SocialState::friend_display_label)
                    .unwrap_or_else(|| a.clone());
                let b_name = social_state
                    .friends
                    .iter()
                    .find(|f| &f.id == b)
                    .map(SocialState::friend_display_label)
                    .unwrap_or_else(|| b.clone());
                a_name.cmp(&b_name)
            });
        }
        ThreadFilter::Online => {
            ids.retain(|id| {
                social_state
                    .friends
                    .iter()
                    .find(|f| &f.id == id)
                    .map(|f| f.online)
                    .unwrap_or(false)
            });
            ids.sort_by(|a, b| {
                let a_last = social_state
                    .thread_for_participant(a)
                    .map(|t| t.last_activity_unix_ms)
                    .unwrap_or(0);
                let b_last = social_state
                    .thread_for_participant(b)
                    .map(|t| t.last_activity_unix_ms)
                    .unwrap_or(0);
                b_last.cmp(&a_last).then_with(|| a.cmp(b))
            });
        }
        ThreadFilter::Recent => {
            let mut by_recent = social_state.sorted_thread_participants_by_recent();
            for id in ids {
                if !by_recent.iter().any(|existing| existing == &id) {
                    by_recent.push(id);
                }
            }
            ids = by_recent;
        }
    }
    ids
}

fn render_compact_message_row(
    ui: &mut egui::Ui,
    header: &str,
    body: &str,
    timestamp: u64,
    outgoing: bool,
    grouped_with_prev: bool,
) {
    let fill = if outgoing {
        egui::Color32::from_rgb(50, 70, 105)
    } else {
        egui::Color32::from_rgb(46, 52, 58)
    };
    let frame = egui::Frame::new()
        .fill(fill)
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(8, 6));
    frame.show(ui, |ui| {
        if !grouped_with_prev {
            ui.horizontal(|ui| {
                ui.strong(header);
                ui.weak(format!("#{timestamp}"));
            });
        }
        ui.label(body);
    });
}

impl UiSystem {
    /// Builds the UI stack that will drive input handling and rendering.
    pub fn new(window: &Window, device: &Device, surface_format: TextureFormat) -> Self {
        let egui_ctx = egui::Context::default();
        let egui_state = State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let egui_renderer = Renderer::new(device, surface_format, None, 1, false);

        Self {
            egui_ctx,
            egui_state,
            egui_renderer,
            active_chat_target: ChatTarget::Nearby,
            thread_filter: ThreadFilter::Recent,
            profile_texture_cache: HashMap::new(),
            profile_texture_failures: HashMap::new(),
            network_debug_teleport_target: String::new(),
            relay_filter_text: String::new(),
            relay_level_filter: None,
            show_diagnostics: true,
            show_network_debug: true,
            show_social: false,
            focus_continuity_requested: false,
            focus_social_requested: false,
        }
    }

    /// Forwards window events to egui and returns whether the event was consumed.
    pub fn handle_event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        self.egui_state.on_window_event(window, event).consumed
    }

    /// Draws the persistent overlay that existed in the prior viewer_app implementation.
    /// The caller owns the encoder and render target so that it can merge this draw call
    /// with other renderer work.
    pub fn render(&mut self, input: RenderInput<'_>) -> UiActions {
        let RenderInput {
            window,
            device,
            queue,
            encoder,
            target_view,
            surface_size,
            camera,
            live_visual,
            session_status,
            chat_state,
            social_state,
            world_avatars,
            world_sim_name,
            world_self_location,
            profile_state,
            profile_image_bytes,
            now_unix_ms,
            profile_cache_ttl_secs,
            fps,
            frame_ms,
            avg_scene_update_ms,
            total_instances,
            visible_proxies,
            fixture_texture_metrics,
            environment,
            transition_visual_state,
            last_recovery_result,
            can_retry_probe,
            can_refresh_assets,
            show_chat_window: _show_chat_window,
            network_debug,
        } = input;
        if surface_size.width == 0 || surface_size.height == 0 {
            return UiActions::default();
        }

        let raw_input = self.egui_state.take_egui_input(window);
        let mut actions = UiActions::default();

        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
                ui.heading("SL Viewer Rewrite");
            });

            egui::Window::new("Session")
                .default_pos(egui::pos2(16.0, 48.0))
                .resizable(false)
                .show(ctx, |ui| {
                    let (status_text, status_color, status_reason) =
                        session_status_chip(&session_status);
                    ui.horizontal(|ui| {
                        ui.label("Status:");
                        ui.colored_label(status_color, status_text);
                        if let Some(reason) = status_reason {
                            ui.weak(format!(" ({})", session_reason_label(reason)));
                        }
                    });
                    ui.separator();
                    ui.label(format!("Sim: {}", world_sim_name.unwrap_or("unknown")));
                    if let Some([x, y, z]) = world_self_location {
                        ui.label(format!("Pos: {:.1}, {:.1}, {:.1}", x, y, z));
                    }
                    ui.label(format!(
                        "Avatar Render: {}",
                        avatar_render_mode_label(social_state.avatar_render_mode)
                    ));
                });

            let mut diag_open = self.show_diagnostics;
            let mut focus_continuity = self.focus_continuity_requested;
            egui::Window::new("Diagnostics")
                .default_pos(egui::pos2(surface_size.width as f32 - 260.0, 48.0))
                .resizable(false)
                .open(&mut diag_open)
                .show(ctx, |ui| {
                    egui::CollapsingHeader::new("Performance & Metrics")
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.label(format!("FPS: {:.1}", fps));
                            ui.label(format!("Frame: {:.2} ms", frame_ms));

                            // Frametime Sparkline
                            let history = &social_state.frametime_history;
                            if !history.is_empty() {
                                let height = 30.0;
                                let width = ui.available_width().at_least(100.0);
                                let (rect, _response) = ui.allocate_at_least(
                                    egui::vec2(width, height),
                                    egui::Sense::hover(),
                                );
                                let painter = ui.painter();
                                painter.rect_filled(rect, 2.0, egui::Color32::from_gray(30));

                                let count = history.len();
                                let bar_width = width / (count as f32).max(1.0);
                                let max_ms = 33.3; // Scale to 30fps baseline, but allow overflow

                                for (i, &ms) in history.iter().enumerate() {
                                    let h_frac = (ms / max_ms).at_most(1.0);
                                    let h = h_frac * height;
                                    let x = rect.min.x + i as f32 * bar_width;
                                    let y = rect.max.y - h;
                                    let color = if ms > 20.0 {
                                        egui::Color32::from_rgb(200, 100, 100) // Reddish for spike
                                    } else {
                                        egui::Color32::from_rgb(100, 200, 100) // Greenish
                                    };
                                    painter.rect_filled(
                                        egui::Rect::from_min_max(
                                            egui::pos2(x, y),
                                            egui::pos2(x + bar_width.at_least(1.0), rect.max.y),
                                        ),
                                        0.0,
                                        color,
                                    );
                                }
                            }
                            ui.separator();
                            ui.label(format!("Scene Update: {:.2} ms", avg_scene_update_ms));
                            ui.separator();
                            ui.label(format!("Instances: {}", total_instances));
                            ui.label(format!("Visible: {}", visible_proxies));
                        });

                    egui::CollapsingHeader::new("Avatar Surface")
                        .default_open(false)
                        .show(ui, |ui| {
                            let total_attachment_proxies: usize = world_avatars
                                .iter()
                                .map(|avatar| avatar.attachments.len())
                                .sum();
                            ui.label(format!("Avatars: {}", world_avatars.len()));
                            ui.label(format!(
                                "Attachment proxies: {} (cap {} per avatar)",
                                total_attachment_proxies,
                                viewer_core::MAX_R08_ATTACHMENTS_PER_AVATAR
                            ));
                        });

                    egui::CollapsingHeader::new("Environment")
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.label(format!(
                                "Time of day: {:.3}",
                                environment.time_of_day_normalized
                            ));
                            ui.label(format!(
                                "Sky/Fog enabled: {}/{}",
                                environment.sky_enabled, environment.fog_enabled
                            ));
                            ui.label(format!(
                                "Ambient: [{:.2}, {:.2}, {:.2}]",
                                environment.ambient.color[0],
                                environment.ambient.color[1],
                                environment.ambient.color[2]
                            ));
                            ui.label(format!(
                                "Sky Top: [{:.2}, {:.2}, {:.2}]",
                                environment.sky.top_color[0],
                                environment.sky.top_color[1],
                                environment.sky.top_color[2]
                            ));
                            ui.label(format!(
                                "Sky Bottom: [{:.2}, {:.2}, {:.2}]",
                                environment.sky.bottom_color[0],
                                environment.sky.bottom_color[1],
                                environment.sky.bottom_color[2]
                            ));
                            ui.label(format!(
                                "Fog: density={:.3}, start={:.1}, end={:.1}",
                                environment.fog.density, environment.fog.start, environment.fog.end
                            ));
                            ui.label(format!(
                                "Render Cue: {:?} ({:.2})",
                                transition_visual_state.cue, transition_visual_state.intensity
                            ));
                        });

                    egui::CollapsingHeader::new("Camera")
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.label(format!(
                                "pos: {:.1}, {:.1}, {:.1}",
                                camera.position[0], camera.position[1], camera.position[2]
                            ));
                            ui.label(format!(
                                "yaw/pitch: {:.2} / {:.2} rad",
                                camera.yaw, camera.pitch
                            ));
                        });

                    egui::CollapsingHeader::new("System")
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.label(format!(
                                "Surface: {}x{}",
                                surface_size.width, surface_size.height
                            ));
                        });

                    egui::CollapsingHeader::new("Presence & Continuity")
                        .default_open(true)
                        .show(ui, |ui| {
                            if focus_continuity {
                                ui.label("Focus: Presence & Continuity");
                                ui.scroll_to_cursor(Some(egui::Align::TOP));
                                focus_continuity = false;
                            }
                            if let Some(snapshot) = live_visual {
                                let (outcome_text, outcome_color) =
                                    handoff_outcome_chip(snapshot.continuity.outcome);
                                ui.horizontal(|ui| {
                                    ui.label("Handoff health:");
                                    ui.colored_label(outcome_color, outcome_text);
                                });
                            }
                            for line in live_visual_lines(live_visual) {
                                ui.label(line);
                            }
                            ui.separator();
                            if ui
                                .add_enabled(
                                    can_retry_probe,
                                    egui::Button::new("Retry Continuity Probe"),
                                )
                                .clicked()
                            {
                                actions.retry_continuity_probe = true;
                            }
                            if ui
                                .add_enabled(
                                    can_refresh_assets,
                                    egui::Button::new("Refresh Visible Assets"),
                                )
                                .clicked()
                            {
                                actions.refresh_visible_assets = true;
                            }
                            if last_recovery_result.is_some()
                                && ui.button("Clear Recovery Banner").clicked()
                            {
                                actions.clear_recovery_banner = true;
                            }
                            if let Some(result) = last_recovery_result {
                                ui.separator();
                                ui.label(format!(
                                    "Recovery status: {}",
                                    recovery_result_label(result)
                                ));
                            }
                        });

                    egui::CollapsingHeader::new("Asset Streaming Continuity")
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.label(format!(
                                "Retained Active: {}",
                                fixture_texture_metrics.continuity_retained_active
                            ));
                            ui.label(format!(
                                "Retained Previous: {}",
                                fixture_texture_metrics.continuity_retained_previous
                            ));
                            ui.label(format!(
                                "Retained Neighbor: {}",
                                fixture_texture_metrics.continuity_retained_neighbor
                            ));
                            ui.label(format!(
                                "Promoted Neighbor: {}",
                                fixture_texture_metrics.continuity_promoted_neighbor
                            ));
                            ui.separator();
                            ui.label(format!(
                                "Evicted Expired: {}",
                                fixture_texture_metrics.continuity_evicted_expired
                            ));
                            ui.label(format!(
                                "Evicted Budget: {}",
                                fixture_texture_metrics.continuity_evicted_budget
                            ));
                            ui.label(format!(
                                "Requests Enqueued: {}",
                                fixture_texture_metrics.continuity_requests_enqueued
                            ));
                            ui.label(format!(
                                "Requests Dropped (Cap): {}",
                                fixture_texture_metrics.continuity_requests_dropped_cap
                            ));
                            ui.label(format!(
                                "Pressure Events: {}",
                                fixture_texture_metrics.continuity_budget_pressure_events
                            ));
                            ui.separator();
                            ui.label("Experimental: Live Asset Bridge (A13)");
                            ui.label(format!(
                                "Live Enqueued / Ready: {} / {}",
                                fixture_texture_metrics.live_requests_enqueued,
                                fixture_texture_metrics.live_requests_ready
                            ));
                            let total_failed = total_live_failures(&fixture_texture_metrics);
                            ui.label(format!("Live Failed: {}", total_failed));
                            if total_failed > 0 {
                                ui.indent("live_failures", |ui| {
                                    ui.label(format!(
                                        "Transport: {}",
                                        fixture_texture_metrics.live_requests_failed_transport
                                    ));
                                    ui.label(format!(
                                        "Decode: {}",
                                        fixture_texture_metrics.live_requests_failed_decode
                                    ));
                                    ui.label(format!(
                                        "Timeout: {}",
                                        fixture_texture_metrics.live_requests_failed_timeout
                                    ));
                                    ui.label(format!(
                                        "MissingCapability: {}",
                                        fixture_texture_metrics
                                            .live_requests_failed_missing_capability
                                    ));
                                });
                            }
                            ui.label(format!(
                                "Fixture Fallbacks: {}",
                                fixture_texture_metrics.fixture_fallbacks_used
                            ));
                        });

                    egui::CollapsingHeader::new("Runtime Relay")
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Search:");
                                ui.text_edit_singleline(&mut self.relay_filter_text);
                                if ui.button("x").clicked() {
                                    self.relay_filter_text.clear();
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Min Level:");
                                ui.selectable_value(&mut self.relay_level_filter, None, "All");
                                ui.selectable_value(
                                    &mut self.relay_level_filter,
                                    Some(RuntimeRelayLevel::Info),
                                    "Info",
                                );
                                ui.selectable_value(
                                    &mut self.relay_level_filter,
                                    Some(RuntimeRelayLevel::Warn),
                                    "Warn",
                                );
                                ui.selectable_value(
                                    &mut self.relay_level_filter,
                                    Some(RuntimeRelayLevel::Error),
                                    "Error",
                                );
                            });
                            ui.separator();

                            let filtered_events: Vec<_> = social_state
                                .relay
                                .events
                                .iter()
                                .filter(|e| {
                                    if let Some(min_level) = self.relay_level_filter
                                        && e.level < min_level
                                    {
                                        return false;
                                    }
                                    if !self.relay_filter_text.is_empty() {
                                        let filter = self.relay_filter_text.to_lowercase();
                                        if !e.category.to_lowercase().contains(&filter)
                                            && !e.message.to_lowercase().contains(&filter)
                                        {
                                            return false;
                                        }
                                    }
                                    true
                                })
                                .collect();

                            ui.label(format!(
                                "showing {}/{} events",
                                filtered_events.len(),
                                social_state.relay.events.len()
                            ));

                            egui::ScrollArea::vertical()
                                .id_salt("relay_scroll")
                                .max_height(200.0)
                                .stick_to_bottom(true)
                                .show(ui, |ui| {
                                    for event in filtered_events {
                                        let color = match event.level {
                                            RuntimeRelayLevel::Trace => egui::Color32::GRAY,
                                            RuntimeRelayLevel::Info => egui::Color32::WHITE,
                                            RuntimeRelayLevel::Warn => egui::Color32::YELLOW,
                                            RuntimeRelayLevel::Error => egui::Color32::RED,
                                        };
                                        ui.colored_label(
                                            color,
                                            format!(
                                                "[{}] {}: {}",
                                                event.at_unix_ms, event.category, event.message
                                            ),
                                        );
                                    }
                                });
                        });
                });
            self.show_diagnostics = diag_open;
            self.focus_continuity_requested = focus_continuity;

            if self.show_network_debug {
                let mut network_open = self.show_network_debug;
                egui::Window::new("Network Debug")
                    .default_pos(egui::pos2(surface_size.width as f32 - 520.0, 48.0))
                    .default_size(egui::vec2(420.0, 420.0))
                    .resizable(true)
                    .open(&mut network_open)
                    .show(ctx, |ui| {
                        ui.strong("Reconnect Teleport");
                        ui.label(format!(
                            "Current region: {}",
                            world_sim_name.unwrap_or("unknown")
                        ));
                        ui.horizontal(|ui| {
                            let response =
                                ui.text_edit_singleline(&mut self.network_debug_teleport_target);
                            let submit_on_enter = response.lost_focus()
                                && ui.input(|input| input.key_pressed(egui::Key::Enter));
                            let can_submit =
                                !self.network_debug_teleport_target.trim().is_empty();
                            let clicked = ui
                                .add_enabled(
                                    can_submit,
                                    egui::Button::new("Reconnect via SLURL"),
                                )
                                .clicked();
                            if (clicked || submit_on_enter) && can_submit {
                                actions.teleport_via_slurl = Some(
                                    self.network_debug_teleport_target.trim().to_string(),
                                );
                            }
                        });
                        ui.small(
                            "Accepted: secondlife://..., secondlife:///app/teleport/..., maps.secondlife.com URLs, or uri:Region&x&y&z.",
                        );
                        ui.separator();

                        for section in &network_debug.sections {
                            egui::CollapsingHeader::new(section.title.as_str())
                                .default_open(true)
                                .show(ui, |ui| {
                                    if section.lines.is_empty() {
                                        ui.label("none");
                                    } else {
                                        for line in &section.lines {
                                            ui.label(line);
                                        }
                                    }
                                });
                        }

                        egui::CollapsingHeader::new("Recent Network Events")
                            .default_open(true)
                            .show(ui, |ui| {
                                ui.label(format!(
                                    "showing {} events",
                                    network_debug.recent_events.events.len()
                                ));
                                egui::ScrollArea::vertical()
                                    .id_salt("network_debug_scroll")
                                    .max_height(220.0)
                                    .stick_to_bottom(true)
                                    .show(ui, |ui| {
                                        for event in &network_debug.recent_events.events {
                                            let color = match event.level {
                                                RuntimeRelayLevel::Trace => egui::Color32::GRAY,
                                                RuntimeRelayLevel::Info => egui::Color32::WHITE,
                                                RuntimeRelayLevel::Warn => egui::Color32::YELLOW,
                                                RuntimeRelayLevel::Error => egui::Color32::RED,
                                            };
                                            ui.colored_label(
                                                color,
                                                format!(
                                                    "[{}] {}: {}",
                                                    event.at_unix_ms, event.category, event.message
                                                ),
                                            );
                                        }
                                    });
                            });
                    });
                self.show_network_debug = network_open;
            }

            let mut social_open = self.show_social;
            let mut focus_social = self.focus_social_requested;
            if social_open {
                egui::Window::new("Social")
                    .default_pos(egui::pos2(16.0, 340.0))
                    .default_size(egui::vec2(760.0, 360.0))
                    .min_size(egui::vec2(520.0, 260.0))
                    .max_size(egui::vec2(1100.0, 760.0))
                    .resizable(true)
                    .open(&mut social_open)
                    .show(ctx, |ui| {
                        if focus_social {
                            ui.label("Focus: Social Workspace");
                            ui.scroll_to_cursor(Some(egui::Align::TOP));
                            focus_social = false;
                        }
                        ui.horizontal(|ui| {
                            let (connection_text, color) = connection_chip(&chat_state.connection);
                            ui.colored_label(color, format!(" connection: {connection_text} "));
                            if let Some((send_text, send_color)) =
                                send_chip(&chat_state.send_status)
                            {
                                ui.colored_label(send_color, format!(" nearby: {send_text} "));
                            }
                            if !social_state.im_draft.trim().is_empty() {
                                ui.colored_label(egui::Color32::LIGHT_BLUE, " im draft unsent ");
                            }
                        });
                        if let ChatConnectionState::Failed(reason) = &chat_state.connection {
                            ui.colored_label(
                                egui::Color32::RED,
                                format!("Connection error: {reason}"),
                            );
                        }
                        ui.separator();

                        ui.columns(2, |cols| {
                            let total_width = cols[0].available_width() + cols[1].available_width();
                            cols[0].set_width(total_width * 0.34);
                            cols[0].vertical(|ui| {
                                ui.strong("Conversations");
                                let nearby_selected = self.active_chat_target == ChatTarget::Nearby;
                                if ui.selectable_label(nearby_selected, "Nearby").clicked() {
                                    self.active_chat_target = ChatTarget::Nearby;
                                }
                                ui.separator();
                                ui.horizontal(|ui| {
                                    ui.strong("Threads");
                                    ui.selectable_value(
                                        &mut self.thread_filter,
                                        ThreadFilter::Recent,
                                        "Recent",
                                    );
                                    ui.selectable_value(
                                        &mut self.thread_filter,
                                        ThreadFilter::Online,
                                        "Online",
                                    );
                                    ui.selectable_value(
                                        &mut self.thread_filter,
                                        ThreadFilter::All,
                                        "All",
                                    );
                                });
                                let friend_ids =
                                    sorted_friend_ids_for_filter(social_state, self.thread_filter);
                                egui::ScrollArea::vertical()
                                    .id_salt("chat_thread_sidebar")
                                    .show(ui, |ui| {
                                        if friend_ids.is_empty() {
                                            ui.weak("No friends loaded");
                                        }
                                        for friend_id in friend_ids {
                                            let label = social_state
                                                .friends
                                                .iter()
                                                .find(|f| f.id == friend_id)
                                                .map(SocialState::friend_display_label)
                                                .unwrap_or_else(|| friend_id.clone());
                                            let unread = social_state
                                                .thread_for_participant(&friend_id)
                                                .map(|thread| thread.unread_count)
                                                .unwrap_or(0);
                                            let hinted = if unread > 0 {
                                                format!("{label} [{unread}]")
                                            } else {
                                                label
                                            };
                                            let selected = social_state
                                                .selected_friend_id
                                                .as_ref()
                                                .map(|id| id == &friend_id)
                                                .unwrap_or(false)
                                                && self.active_chat_target == ChatTarget::DirectIm;
                                            if ui.selectable_label(selected, hinted).clicked() {
                                                self.active_chat_target = ChatTarget::DirectIm;
                                                social_state.selected_friend_id =
                                                    Some(friend_id.clone());
                                                social_state
                                                    .mark_thread_read_by_participant(&friend_id, 0);
                                            }
                                        }
                                    });
                                if ui.button("Clear all unread").clicked() {
                                    social_state.clear_all_unread(0);
                                }
                            });

                            cols[1].vertical(|ui| match self.active_chat_target {
                                ChatTarget::Nearby => {
                                    ui.strong("Nearby Chat");
                                    egui::ScrollArea::vertical()
                                        .id_salt("nearby_messages_compact")
                                        .stick_to_bottom(true)
                                        .show(ui, |ui| {
                                            let mut prev_sender = String::new();
                                            for message in &chat_state.messages {
                                                let grouped = prev_sender == message.sender;
                                                render_compact_message_row(
                                                    ui,
                                                    &message.sender,
                                                    &message.text,
                                                    message.observed_at_unix_ms,
                                                    message.sender == "You",
                                                    grouped,
                                                );
                                                prev_sender = message.sender.clone();
                                            }
                                        });
                                    ui.separator();
                                    let is_connected =
                                        matches!(session_status, SessionUxStatus::Connected);
                                    let text_edit = ui.add_enabled(
                                        is_connected,
                                        egui::TextEdit::multiline(&mut chat_state.draft.text)
                                            .desired_rows(3)
                                            .hint_text(if is_connected {
                                                "Enter sends, Shift+Enter newline"
                                            } else {
                                                "Connect to send nearby chat"
                                            }),
                                    );
                                    let can_send =
                                        is_connected && !chat_state.draft.text.trim().is_empty();
                                    let enter_send = text_edit.has_focus()
                                        && ui.input(|input| {
                                            should_submit_on_enter(
                                                input.key_pressed(egui::Key::Enter),
                                                input.modifiers.shift,
                                            )
                                        });
                                    let click_send = ui
                                        .add_enabled(can_send, egui::Button::new("Send nearby"))
                                        .clicked();
                                    if can_send && (enter_send || click_send) {
                                        actions.nearby_chat_send =
                                            Some(chat_state.draft.text.trim().to_string());
                                    }
                                }
                                ChatTarget::DirectIm => {
                                    if social_state.selected_friend_id.is_none() {
                                        social_state.selected_friend_id = social_state
                                            .friends
                                            .first()
                                            .map(|friend| friend.id.clone());
                                    }
                                    if let Some(selected) = social_state.selected_friend_id.clone()
                                    {
                                        social_state.mark_thread_read_by_participant(&selected, 0);
                                        let label = social_state
                                            .friends
                                            .iter()
                                            .find(|f| f.id == selected)
                                            .map(SocialState::friend_display_label)
                                            .unwrap_or_else(|| selected.clone());
                                        ui.horizontal(|ui| {
                                            ui.strong(format!("Direct IM: {label}"));
                                            if ui.button("Profile").clicked() {
                                                actions.open_avatar_profile =
                                                    Some(selected.clone());
                                            }
                                            if ui.button("Mark read").clicked() {
                                                social_state
                                                    .mark_thread_read_by_participant(&selected, 0);
                                            }
                                        });
                                        if let Some(thread) =
                                            social_state.thread_for_participant(&selected)
                                        {
                                            egui::ScrollArea::vertical()
                                                .id_salt("direct_im_messages_compact")
                                                .stick_to_bottom(true)
                                                .show(ui, |ui| {
                                                    let mut prev_sender = String::new();
                                                    for msg in &thread.messages {
                                                        let prefix = if msg.outgoing {
                                                            "You"
                                                        } else {
                                                            msg.from_name.as_str()
                                                        };
                                                        let grouped = prev_sender == prefix;
                                                        render_compact_message_row(
                                                            ui,
                                                            prefix,
                                                            &msg.text,
                                                            msg.observed_at_unix_ms,
                                                            msg.outgoing,
                                                            grouped,
                                                        );
                                                        prev_sender = prefix.to_string();
                                                    }
                                                });
                                        } else {
                                            ui.label("No IM history yet.");
                                        }
                                        ui.separator();
                                        let is_connected =
                                            matches!(session_status, SessionUxStatus::Connected);
                                        let text_edit = ui.add_enabled(
                                            is_connected,
                                            egui::TextEdit::multiline(&mut social_state.im_draft)
                                                .desired_rows(3)
                                                .hint_text(if is_connected {
                                                    "Enter sends, Shift+Enter newline"
                                                } else {
                                                    "Connect to send IM"
                                                }),
                                        );
                                        let can_send = is_connected
                                            && !social_state.im_draft.trim().is_empty();
                                        let enter_send = text_edit.has_focus()
                                            && ui.input(|input| {
                                                should_submit_on_enter(
                                                    input.key_pressed(egui::Key::Enter),
                                                    input.modifiers.shift,
                                                )
                                            });
                                        let click_send = ui
                                            .add_enabled(can_send, egui::Button::new("Send IM"))
                                            .clicked();
                                        if can_send && (enter_send || click_send) {
                                            actions.direct_im_send = Some((
                                                selected.clone(),
                                                social_state.im_draft.trim().to_string(),
                                            ));
                                        }
                                    } else {
                                        ui.label("No friends available for direct IM.");
                                    }
                                }
                            });
                        });
                    });
                self.show_social = social_open;
            }
            self.focus_social_requested = focus_social;

            if let Some(profile) = profile_state.as_mut() {
                egui::Window::new("Avatar Profile")
                    .default_pos(egui::pos2(800.0, 340.0))
                    .default_size(egui::vec2(640.0, 420.0))
                    .min_size(egui::vec2(400.0, 260.0))
                    .resizable(true)
                    .show(ctx, |ui| {
                        let load = profile.tab_load(profile.selected_tab);
                        let status_text = match load.status {
                            ProfileLoadStatus::Idle => "idle",
                            ProfileLoadStatus::Loading => "loading",
                            ProfileLoadStatus::Loaded => "loaded",
                            ProfileLoadStatus::Failed => "failed",
                        };
                        let status_color = match load.status {
                            ProfileLoadStatus::Idle => egui::Color32::GRAY,
                            ProfileLoadStatus::Loading => egui::Color32::YELLOW,
                            ProfileLoadStatus::Loaded => egui::Color32::GREEN,
                            ProfileLoadStatus::Failed => egui::Color32::RED,
                        };

                        egui::Frame::group(ui.style()).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.strong(profile.display_label());
                                ui.weak(format!("#{}", short_id(profile.avatar_id.as_str())));
                                ui.colored_label(status_color, status_text);

                                // Freshness indicator
                                let (freshness, age_str) = viewer_core::compute_profile_freshness(
                                    now_unix_ms,
                                    load.last_updated_unix_ms,
                                    profile_cache_ttl_secs,
                                );
                                let (freshness_label, freshness_color) = match freshness {
                                    ProfileFreshness::Fresh => {
                                        ("Fresh", egui::Color32::from_rgb(100, 200, 100))
                                    }
                                    ProfileFreshness::Stale => {
                                        ("Stale", egui::Color32::from_rgb(200, 150, 50))
                                    }
                                    ProfileFreshness::Unknown => ("Unknown", egui::Color32::GRAY),
                                };
                                ui.colored_label(
                                    freshness_color,
                                    format!("{freshness_label} ({age_str})"),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.weak(format!("tab: {:?}", profile.selected_tab));
                                if let Some(err) = load.error.as_ref() {
                                    ui.colored_label(egui::Color32::RED, format!(" Error: {err}"));
                                }
                            });
                        });
                        ui.add_space(4.0);

                        ui.horizontal(|ui| {
                            let mut jumped_tab = profile.selected_tab;
                            for (label, tab) in [
                                ("2nd Life", AvatarProfileTab::SecondLife),
                                ("Feed", AvatarProfileTab::Feed),
                                ("Picks", AvatarProfileTab::Picks),
                                ("Classifieds", AvatarProfileTab::Classifieds),
                                ("1st Life", AvatarProfileTab::FirstLife),
                                ("Notes", AvatarProfileTab::Notes),
                            ] {
                                if ui
                                    .selectable_label(profile.selected_tab == tab, label)
                                    .clicked()
                                {
                                    profile.selected_tab = tab;
                                    actions.select_avatar_profile_tab =
                                        Some((profile.avatar_id.clone(), tab));
                                }
                            }
                            let cooldown_ms = viewer_core::PROFILE_REFRESH_COOLDOWN_MS;
                            let last_refresh = profile.last_refresh_unix_ms.unwrap_or(0);
                            let on_cooldown = now_unix_ms < last_refresh + cooldown_ms;
                            let remaining_secs = if on_cooldown {
                                (last_refresh + cooldown_ms - now_unix_ms).div_ceil(1000)
                            } else {
                                0
                            };

                            let refresh_btn =
                                ui.add_enabled(!on_cooldown, egui::Button::new("Refresh"));
                            if refresh_btn.clicked() {
                                actions.refresh_avatar_profile =
                                    Some((profile.avatar_id.clone(), Some(profile.selected_tab)));
                            }
                            if on_cooldown {
                                ui.weak(format!("({remaining_secs}s)"));
                            }
                            egui::ComboBox::from_id_salt("profile_tab_jump")
                                .selected_text(format!("{:?}", profile.selected_tab))
                                .show_ui(ui, |ui| {
                                    for tab in [
                                        AvatarProfileTab::SecondLife,
                                        AvatarProfileTab::Feed,
                                        AvatarProfileTab::Picks,
                                        AvatarProfileTab::Classifieds,
                                        AvatarProfileTab::FirstLife,
                                        AvatarProfileTab::Notes,
                                    ] {
                                        ui.selectable_value(
                                            &mut jumped_tab,
                                            tab,
                                            format!("{tab:?}"),
                                        );
                                    }
                                });
                            if jumped_tab != profile.selected_tab {
                                profile.selected_tab = jumped_tab;
                                actions.select_avatar_profile_tab =
                                    Some((profile.avatar_id.clone(), jumped_tab));
                            }
                        });
                        ui.separator();

                        match profile.selected_tab {
                            AvatarProfileTab::SecondLife => {
                                if let Some(second_life) = profile.second_life.as_ref() {
                                    render_profile_image(
                                        ui,
                                        &self.egui_ctx,
                                        profile_image_bytes,
                                        &mut self.profile_texture_cache,
                                        &mut self.profile_texture_failures,
                                        second_life.image_id.as_deref(),
                                        "second_life",
                                    );
                                    ui.label(format!(
                                        "Display: {}",
                                        second_life.display_name.as_deref().unwrap_or("unknown")
                                    ));
                                    ui.label(format!(
                                        "Username: {}",
                                        second_life.username.as_deref().unwrap_or("unknown")
                                    ));
                                    ui.label(format!(
                                        "Member since: {}",
                                        second_life.member_since.as_deref().unwrap_or("n/a")
                                    ));
                                    ui.label(format!(
                                        "Online: {}",
                                        second_life
                                            .online
                                            .map(|v| v.to_string())
                                            .unwrap_or_else(|| String::from("unknown"))
                                    ));
                                    ui.separator();
                                    ui.label("About:");
                                    egui::ScrollArea::vertical()
                                        .id_salt("profile_second_life_about")
                                        .max_height((ui.available_height() * 0.45).max(72.0))
                                        .show(ui, |ui| {
                                            ui.label(second_life.about_text.as_str());
                                        });
                                    ui.separator();
                                    ui.label("Groups:");
                                    egui::ScrollArea::vertical()
                                        .id_salt("profile_second_life_groups")
                                        .max_height((ui.available_height() * 0.35).max(72.0))
                                        .show(ui, |ui| {
                                            for group in &second_life.groups {
                                                let label = if group.name.trim().is_empty() {
                                                    group.id.clone()
                                                } else {
                                                    format!(
                                                        "{} ({})",
                                                        group.name,
                                                        short_id(&group.id)
                                                    )
                                                };
                                                ui.add(egui::Label::new(label).truncate());
                                            }
                                        });
                                } else {
                                    ui.label("No 2nd Life profile data loaded yet.");
                                }
                            }
                            AvatarProfileTab::Feed => {
                                let feed_url = profile.feed.url.as_deref().unwrap_or("n/a");
                                ui.label(format!("Feed URL: {feed_url}"));
                                if let Some(url) = profile.feed.url.as_ref()
                                    && ui.button("Open in Browser").clicked()
                                {
                                    actions.open_external_url = Some(url.clone());
                                }
                            }
                            AvatarProfileTab::Picks => {
                                if profile.picks.items.is_empty() {
                                    ui.label("No picks found.");
                                } else {
                                    let mut picked_id: Option<String> = None;
                                    egui::ScrollArea::vertical()
                                        .id_salt("profile_picks_list")
                                        .max_height((ui.available_height() * 0.42).max(72.0))
                                        .show(ui, |ui| {
                                            for pick in &profile.picks.items {
                                                let selected = profile
                                                    .picks
                                                    .selected_pick_id
                                                    .as_deref()
                                                    .map(|id| id == pick.id.as_str())
                                                    .unwrap_or(false);
                                                if ui
                                                    .selectable_label(
                                                        selected,
                                                        format!(
                                                            "{} ({})",
                                                            pick.name,
                                                            short_id(&pick.id)
                                                        ),
                                                    )
                                                    .clicked()
                                                {
                                                    picked_id = Some(pick.id.clone());
                                                }
                                            }
                                        });
                                    if let Some(selected) = picked_id {
                                        profile.picks.selected_pick_id = Some(selected);
                                    }
                                    ui.separator();
                                    if let Some(selected_id) =
                                        profile.picks.selected_pick_id.as_ref()
                                        && let Some(details) =
                                            profile.picks.details.get(selected_id)
                                    {
                                        let pick_title = if details.name.is_empty() {
                                            short_id(&details.id)
                                        } else {
                                            details.name.clone()
                                        };
                                        ui.label(format!("Pick: {}", pick_title));
                                        ui.label(format!("ID: {}", details.id));
                                        if let Some(description) = details.description.as_ref() {
                                            ui.label(format!("Description: {description}"));
                                        }
                                        if let Some(sim_name) = details.sim_name.as_ref() {
                                            ui.label(format!("Region: {sim_name}"));
                                        }
                                        if let Some(parcel_name) = details.parcel_name.as_ref() {
                                            ui.label(format!("Parcel: {parcel_name}"));
                                        }
                                        if let Some(snapshot_id) = details.snapshot_id.as_ref() {
                                            render_profile_image(
                                                ui,
                                                &self.egui_ctx,
                                                profile_image_bytes,
                                                &mut self.profile_texture_cache,
                                                &mut self.profile_texture_failures,
                                                Some(snapshot_id.as_str()),
                                                "pick",
                                            );
                                        }
                                    }
                                }
                            }
                            AvatarProfileTab::Classifieds => {
                                if profile.classifieds.items.is_empty() {
                                    ui.label("No classifieds found.");
                                } else {
                                    let mut picked_id: Option<String> = None;
                                    egui::ScrollArea::vertical()
                                        .id_salt("profile_classified_list")
                                        .max_height((ui.available_height() * 0.42).max(72.0))
                                        .show(ui, |ui| {
                                            for classified in &profile.classifieds.items {
                                                let selected = profile
                                                    .classifieds
                                                    .selected_classified_id
                                                    .as_deref()
                                                    .map(|id| id == classified.id.as_str())
                                                    .unwrap_or(false);
                                                if ui
                                                    .selectable_label(
                                                        selected,
                                                        format!(
                                                            "{} ({})",
                                                            classified.name,
                                                            short_id(&classified.id)
                                                        ),
                                                    )
                                                    .clicked()
                                                {
                                                    picked_id = Some(classified.id.clone());
                                                }
                                            }
                                        });
                                    if let Some(selected) = picked_id {
                                        profile.classifieds.selected_classified_id = Some(selected);
                                    }
                                    ui.separator();
                                    if let Some(selected_id) =
                                        profile.classifieds.selected_classified_id.as_ref()
                                        && let Some(details) =
                                            profile.classifieds.details.get(selected_id)
                                    {
                                        let classified_title = if details.name.is_empty() {
                                            short_id(&details.id)
                                        } else {
                                            details.name.clone()
                                        };
                                        ui.label(format!("Classified: {}", classified_title));
                                        ui.label(format!("ID: {}", details.id));
                                        if let Some(description) = details.description.as_ref() {
                                            ui.label(format!("Description: {description}"));
                                        }
                                        if let Some(sim_name) = details.sim_name.as_ref() {
                                            ui.label(format!("Region: {sim_name}"));
                                        }
                                        if let Some(parcel_name) = details.parcel_name.as_ref() {
                                            ui.label(format!("Parcel: {parcel_name}"));
                                        }
                                        if let Some(price) = details.price_for_listing {
                                            ui.label(format!("Price: {price}"));
                                        }
                                        if let Some(snapshot_id) = details.snapshot_id.as_ref() {
                                            render_profile_image(
                                                ui,
                                                &self.egui_ctx,
                                                profile_image_bytes,
                                                &mut self.profile_texture_cache,
                                                &mut self.profile_texture_failures,
                                                Some(snapshot_id.as_str()),
                                                "classified",
                                            );
                                        }
                                    }
                                }
                            }
                            AvatarProfileTab::FirstLife => {
                                if let Some(first_life) = profile.first_life.as_ref() {
                                    render_profile_image(
                                        ui,
                                        &self.egui_ctx,
                                        profile_image_bytes,
                                        &mut self.profile_texture_cache,
                                        &mut self.profile_texture_failures,
                                        first_life.image_id.as_deref(),
                                        "first_life",
                                    );
                                    ui.label("About:");
                                    egui::ScrollArea::vertical()
                                        .id_salt("profile_first_life_about")
                                        .max_height((ui.available_height() * 0.6).max(72.0))
                                        .show(ui, |ui| {
                                            ui.label(first_life.about_text.as_str());
                                        });
                                } else {
                                    ui.label("No 1st Life data loaded yet.");
                                }
                            }
                            AvatarProfileTab::Notes => {
                                if let Some(notes) = profile.notes.as_ref() {
                                    egui::ScrollArea::vertical()
                                        .id_salt("profile_notes")
                                        .max_height((ui.available_height() * 0.7).max(72.0))
                                        .show(ui, |ui| {
                                            ui.label(notes.text.as_str());
                                        });
                                } else {
                                    ui.label("No notes loaded yet.");
                                }
                            }
                        }
                    });
            }

            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("world_avatar_labels_layer"),
            ));
            for avatar in world_avatars {
                let label_position = [
                    avatar.world_position[0],
                    avatar.world_position[1] + 0.95,
                    avatar.world_position[2],
                ];
                let Some([sx, sy]) = camera.project_world_to_screen(
                    label_position,
                    [surface_size.width as f32, surface_size.height as f32],
                ) else {
                    continue;
                };
                let text = if avatar.display_name.trim().is_empty() {
                    avatar.short_agent_id()
                } else {
                    let mut base = avatar.display_name.clone();
                    if let Some(sim_name) = avatar.sim_name.as_deref() {
                        base = format!("{base} @ {sim_name}");
                    }
                    if let Some([x, y, z]) = avatar.local_position {
                        base = format!("{base} ({x}, {y}, {z})");
                    }
                    base
                };
                let font_id = egui::FontId::proportional(14.0);
                let galley = painter.layout_no_wrap(text, font_id.clone(), egui::Color32::WHITE);
                let rect = egui::Rect::from_center_size(
                    egui::pos2(sx, sy),
                    galley.size() + egui::vec2(12.0, 6.0),
                );
                painter.rect_filled(rect, 4.0, egui::Color32::from_black_alpha(180));
                painter.galley(
                    egui::pos2(rect.left() + 6.0, rect.top() + 3.0),
                    galley,
                    egui::Color32::WHITE,
                );
            }
        });

        self.egui_state
            .handle_platform_output(window, full_output.platform_output);

        let pixels_per_point = window.scale_factor() as f32;
        let paint_jobs = self
            .egui_ctx
            .tessellate(full_output.shapes, pixels_per_point);

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [surface_size.width, surface_size.height],
            pixels_per_point,
        };

        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer
                .update_texture(device, queue, *id, image_delta);
        }

        self.egui_renderer
            .update_buffers(device, queue, encoder, &paint_jobs, &screen_descriptor);

        {
            let render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("viewer_ui_render_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            let mut render_pass = render_pass.forget_lifetime();
            self.egui_renderer
                .render(&mut render_pass, &paint_jobs, &screen_descriptor);
        }

        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use viewer_core::{DirectImMessage, FriendEntry, RegionContinuitySummary};

    #[test]
    fn live_visual_lines_reports_absent_snapshot() {
        let lines = live_visual_lines(None);
        assert!(
            lines
                .iter()
                .any(|line| line.contains("No live snapshot loaded"))
        );
    }

    #[test]
    fn live_visual_lines_reports_present_snapshot_fields() {
        let snapshot = LiveVisualSnapshot {
            source: String::from("viewer_app_in_process:ready"),
            logged_in: true,
            current_region_name: Some(String::from("Test Region")),
            first_sim_endpoint: Some(String::from("127.0.0.1:13009")),
            first_sim_region_x: Some(1000),
            first_sim_region_y: Some(1001),
            handshake_agent_movement_complete: true,
            traffic_summary_available: true,
            post_boundary_observations: 5,
            region_transition_control_observations: 1,
            crossed_region: 1,
            confirm_enable_simulator: 0,
            likely_broader_traffic: 3,
            unknown: 1,
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
            continuity: RegionContinuitySummary::default(),
            observed_at_unix_ms: 1,
        };
        let lines = live_visual_lines(Some(&snapshot));
        assert!(lines.iter().any(|line| line.contains("Logged in: true")));
        assert!(lines.iter().any(|line| line.contains("obs=5")));
        assert!(lines.iter().any(|line| line.contains("CrossedRegion=1")));
        assert!(
            lines
                .iter()
                .any(|line| line.contains("Continuity Phase: None"))
        );
    }

    #[test]
    fn should_submit_on_enter_respects_shift_modifier() {
        assert!(should_submit_on_enter(true, false));
        assert!(!should_submit_on_enter(true, true));
        assert!(!should_submit_on_enter(false, false));
    }

    #[test]
    fn sorted_friend_ids_for_recent_prioritizes_thread_activity() {
        let mut social = SocialState::default();
        social.friends.push(FriendEntry {
            id: String::from("a"),
            display_name: Some(String::from("A")),
            name_source: None,
            last_name_resolved_unix_ms: None,
            online: true,
            rights_has: 0,
            rights_given: 0,
            last_changed_unix_ms: 1,
        });
        social.friends.push(FriendEntry {
            id: String::from("b"),
            display_name: Some(String::from("B")),
            name_source: None,
            last_name_resolved_unix_ms: None,
            online: true,
            rights_has: 0,
            rights_given: 0,
            last_changed_unix_ms: 1,
        });
        social.upsert_thread_message(
            DirectImMessage {
                id: 1,
                session_id: String::from("sa"),
                peer_id: String::from("a"),
                from_id: String::from("a"),
                from_name: String::from("A"),
                text: String::from("first"),
                observed_at_unix_ms: 20,
                outgoing: false,
            },
            "a",
        );
        social.upsert_thread_message(
            DirectImMessage {
                id: 2,
                session_id: String::from("sb"),
                peer_id: String::from("b"),
                from_id: String::from("b"),
                from_name: String::from("B"),
                text: String::from("newer"),
                observed_at_unix_ms: 30,
                outgoing: false,
            },
            "b",
        );
        let ids = sorted_friend_ids_for_filter(&social, ThreadFilter::Recent);
        assert_eq!(ids, vec![String::from("b"), String::from("a")]);
    }

    #[test]
    fn session_status_chip_labels() {
        use viewer_core::{SessionUxReason, SessionUxStatus};

        let (label, color, reason) = session_status_chip(&SessionUxStatus::Connected);
        assert_eq!(label, "connected");
        assert_eq!(color, egui::Color32::GREEN);
        assert!(reason.is_none());

        let (label, color, reason) = session_status_chip(&SessionUxStatus::Failed {
            reason: SessionUxReason::LoginAuth,
        });
        assert_eq!(label, "failed");
        assert_eq!(color, egui::Color32::RED);
        assert_eq!(reason, Some(SessionUxReason::LoginAuth));
    }

    #[test]
    fn total_live_failures_counts_missing_capability() {
        let metrics = viewer_core::AssetContinuityMetrics {
            live_requests_failed_transport: 1,
            live_requests_failed_decode: 2,
            live_requests_failed_timeout: 3,
            live_requests_failed_missing_capability: 4,
            live_requests_failed_other: 5,
            ..Default::default()
        };
        assert_eq!(total_live_failures(&metrics), 15);
    }
}
