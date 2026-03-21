use egui_wgpu::{Renderer, ScreenDescriptor};
use egui_winit::State;
use std::collections::{BTreeMap, HashMap};
use viewer_core::{
    AvatarProfileState, AvatarProfileTab, Camera, ChatConnectionState, ChatSendStatus, ChatState,
    LiveVisualSnapshot, ProfileLoadStatus, RuntimeRelayLevel, SocialState, WorldAvatarPlaceholder,
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

    if let Ok(j2k) = jpeg2k::Image::from_bytes(bytes) {
        if let Ok(decoded) = image::DynamicImage::try_from(&j2k) {
            let rgba = decoded.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            return Ok(egui::ColorImage::from_rgba_unmultiplied(
                size,
                rgba.as_raw(),
            ));
        }
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
        }
    }

    /// Forwards window events to egui and returns whether the event was consumed.
    pub fn handle_event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        self.egui_state.on_window_event(window, event).consumed
    }

    /// Draws the persistent overlay that existed in the prior viewer_app implementation.
    /// The caller owns the encoder and render target so that it can merge this draw call
    /// with other renderer work.
    pub fn render(
        &mut self,
        window: &Window,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        target_view: &TextureView,
        surface_size: PhysicalSize<u32>,
        camera: &Camera,
        live_visual: Option<&LiveVisualSnapshot>,
        live_startup_status: &str,
        chat_state: &mut ChatState,
        social_state: &mut SocialState,
        world_avatars: &[WorldAvatarPlaceholder],
        world_sim_name: Option<&str>,
        world_self_location: Option<[f32; 3]>,
        profile_state: &mut Option<AvatarProfileState>,
        profile_image_bytes: &BTreeMap<String, Vec<u8>>,
        fps: f32,
        frame_ms: f32,
    ) -> UiActions {
        if surface_size.width == 0 || surface_size.height == 0 {
            return UiActions::default();
        }

        let raw_input = self.egui_state.take_egui_input(window);
        let mut actions = UiActions::default();

        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
                ui.heading("SL Viewer Rewrite");
            });

            egui::Window::new("Debug")
                .default_pos(egui::pos2(16.0, 48.0))
                .resizable(false)
                .show(ctx, |ui| {
                    egui::CollapsingHeader::new("Runtime")
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.label(format!(
                                "Surface: {}x{}",
                                surface_size.width, surface_size.height
                            ));
                            ui.label(format!("Live startup: {live_startup_status}"));
                            ui.label(format!("Sim: {}", world_sim_name.unwrap_or("unknown")));
                            if let Some([x, y, z]) = world_self_location {
                                ui.label(format!("Location: {:.1}, {:.1}, {:.1}", x, y, z));
                            } else {
                                ui.label("Location: unknown");
                            }
                        });
                    egui::CollapsingHeader::new("Camera")
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.label(format!(
                                "pos: x={:.2} y={:.2} z={:.2}",
                                camera.position[0], camera.position[1], camera.position[2]
                            ));
                            ui.label(format!("yaw/pitch: {:.2} / {:.2} rad", camera.yaw, camera.pitch));
                        });
                    egui::CollapsingHeader::new("Live Visual Snapshot")
                        .default_open(false)
                        .show(ui, |ui| {
                            for line in live_visual_lines(live_visual) {
                                ui.label(line);
                            }
                        });
                });

            egui::Window::new("Performance")
                .default_pos(egui::pos2(16.0, 252.0))
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(format!("FPS: {:.1}", fps));
                    ui.label(format!("Frame: {:.2} ms", frame_ms));
                });

            egui::Window::new("Chat + IM")
                .default_pos(egui::pos2(16.0, 340.0))
                .default_size(egui::vec2(760.0, 360.0))
                .min_size(egui::vec2(520.0, 260.0))
                .max_size(egui::vec2(1100.0, 760.0))
                .resizable(true)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        let (connection_text, color) = connection_chip(&chat_state.connection);
                        ui.colored_label(color, format!(" connection: {connection_text} "));
                        if let Some((send_text, send_color)) = send_chip(&chat_state.send_status) {
                            ui.colored_label(send_color, format!(" nearby: {send_text} "));
                        }
                        if !social_state.im_draft.trim().is_empty() {
                            ui.colored_label(
                                egui::Color32::LIGHT_BLUE,
                                " im draft unsent ",
                            );
                        }
                    });
                    if let ChatConnectionState::Failed(reason) = &chat_state.connection {
                        ui.colored_label(egui::Color32::RED, format!("Connection error: {reason}"));
                    }
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Threads:");
                        ui.selectable_value(&mut self.thread_filter, ThreadFilter::Recent, "Recent");
                        ui.selectable_value(&mut self.thread_filter, ThreadFilter::Online, "Online");
                        ui.selectable_value(&mut self.thread_filter, ThreadFilter::All, "All");
                        if ui.button("Clear all unread").clicked() {
                            social_state.clear_all_unread(0);
                        }
                    });
                    ui.separator();

                    ui.columns(2, |cols| {
                        let total_width = cols[0].available_width() + cols[1].available_width();
                        cols[0].set_width(total_width * 0.34);
                        cols[0].vertical(|ui| {
                            let nearby_selected = self.active_chat_target == ChatTarget::Nearby;
                            if ui.selectable_label(nearby_selected, "Nearby").clicked() {
                                self.active_chat_target = ChatTarget::Nearby;
                            }
                            let friend_ids = sorted_friend_ids_for_filter(social_state, self.thread_filter);
                            egui::ScrollArea::vertical()
                                .id_salt("chat_thread_sidebar")
                                .show(ui, |ui| {
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
                                            social_state.selected_friend_id = Some(friend_id.clone());
                                            social_state.mark_thread_read_by_participant(&friend_id, 0);
                                        }
                                    }
                                });
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
                                let text_edit = ui.add(
                                    egui::TextEdit::multiline(&mut chat_state.draft.text)
                                        .desired_rows(3)
                                        .hint_text("Enter sends, Shift+Enter newline"),
                                );
                                let can_send = !chat_state.draft.text.trim().is_empty();
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
                                    social_state.selected_friend_id =
                                        social_state.friends.first().map(|friend| friend.id.clone());
                                }
                                if let Some(selected) = social_state.selected_friend_id.clone() {
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
                                            actions.open_avatar_profile = Some(selected.clone());
                                        }
                                        if ui.button("Mark read").clicked() {
                                            social_state.mark_thread_read_by_participant(&selected, 0);
                                        }
                                    });
                                    if let Some(thread) = social_state.thread_for_participant(&selected) {
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
                                    let text_edit = ui.add(
                                        egui::TextEdit::multiline(&mut social_state.im_draft)
                                            .desired_rows(3)
                                            .hint_text("Enter sends, Shift+Enter newline"),
                                    );
                                    let can_send = !social_state.im_draft.trim().is_empty();
                                    let enter_send = text_edit.has_focus()
                                        && ui.input(|input| {
                                            should_submit_on_enter(
                                                input.key_pressed(egui::Key::Enter),
                                                input.modifiers.shift,
                                            )
                                        });
                                    let click_send = ui
                                        .add_enabled(can_send, egui::Button::new("Send IM"))
                                        .clicked()
                                        ;
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
                            });
                            ui.horizontal(|ui| {
                                ui.weak(format!(
                                    "tab: {:?} | updated: {}",
                                    profile.selected_tab,
                                    load.last_updated_unix_ms
                                        .map(|v| v.to_string())
                                        .unwrap_or_else(|| String::from("n/a"))
                                ));
                                if let Some(err) = load.error.as_ref() {
                                    ui.colored_label(egui::Color32::RED, err);
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
                            if ui.button("Refresh").clicked() {
                                actions.refresh_avatar_profile =
                                    Some((profile.avatar_id.clone(), Some(profile.selected_tab)));
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
                                if let Some(url) = profile.feed.url.as_ref() {
                                    if ui.button("Open in Browser").clicked() {
                                        actions.open_external_url = Some(url.clone());
                                    }
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
                                    {
                                        if let Some(details) =
                                            profile.picks.details.get(selected_id)
                                        {
                                            let pick_title = if details.name.is_empty() {
                                                short_id(&details.id)
                                            } else {
                                                details.name.clone()
                                            };
                                            ui.label(format!("Pick: {}", pick_title));
                                            ui.label(format!("ID: {}", details.id));
                                            if let Some(description) = details.description.as_ref()
                                            {
                                                ui.label(format!("Description: {description}"));
                                            }
                                            if let Some(sim_name) = details.sim_name.as_ref() {
                                                ui.label(format!("Region: {sim_name}"));
                                            }
                                            if let Some(parcel_name) = details.parcel_name.as_ref()
                                            {
                                                ui.label(format!("Parcel: {parcel_name}"));
                                            }
                                            if let Some(snapshot_id) = details.snapshot_id.as_ref()
                                            {
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
                                    {
                                        if let Some(details) =
                                            profile.classifieds.details.get(selected_id)
                                        {
                                            let classified_title = if details.name.is_empty() {
                                                short_id(&details.id)
                                            } else {
                                                details.name.clone()
                                            };
                                            ui.label(format!("Classified: {}", classified_title));
                                            ui.label(format!("ID: {}", details.id));
                                            if let Some(description) = details.description.as_ref()
                                            {
                                                ui.label(format!("Description: {description}"));
                                            }
                                            if let Some(sim_name) = details.sim_name.as_ref() {
                                                ui.label(format!("Region: {sim_name}"));
                                            }
                                            if let Some(parcel_name) = details.parcel_name.as_ref()
                                            {
                                                ui.label(format!("Parcel: {parcel_name}"));
                                            }
                                            if let Some(price) = details.price_for_listing {
                                                ui.label(format!("Price: {price}"));
                                            }
                                            if let Some(snapshot_id) = details.snapshot_id.as_ref()
                                            {
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

            egui::Window::new("Runtime Relay")
                .default_pos(egui::pos2(920.0, 48.0))
                .default_size(egui::vec2(340.0, 320.0))
                .show(ctx, |ui| {
                    ui.label(format!("events: {}", social_state.relay.events.len()));
                    egui::CollapsingHeader::new("Show Relay Events")
                        .default_open(true)
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .stick_to_bottom(true)
                                .show(ui, |ui| {
                                    for event in &social_state.relay.events {
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
    use viewer_core::{DirectImMessage, FriendEntry};

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
            observed_at_unix_ms: 1,
        };
        let lines = live_visual_lines(Some(&snapshot));
        assert!(lines.iter().any(|line| line.contains("Logged in: true")));
        assert!(lines.iter().any(|line| line.contains("obs=5")));
        assert!(lines.iter().any(|line| line.contains("CrossedRegion=1")));
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
}
