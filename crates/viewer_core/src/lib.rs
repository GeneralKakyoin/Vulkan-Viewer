use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    AvatarProxy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AvatarRenderMode {
    #[default]
    Proxy,
    FallbackBox,
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
    WorldIngestionDecodedCoarseNeighborhoodPayload,
    WorldIngestionDecodedCoarseNeighborhoodSatelliteA,
    WorldIngestionDecodedCoarseNeighborhoodSatelliteB,
    WorldIngestionDecodedHealthPayload,
    WorldIngestionDecodedViewerTimePayload,
    WorldIngestionDecodedCompositeBeacon,
    WorldObjectStateEntityBody,
    WorldObjectStateEntityAura,
    WorldObjectStateEntityWingBody,
    WorldObjectStateEntityWingAura,
    WorldObjectStateEntityGuardBody,
    WorldObjectStateEntityGuardAura,
    WorldObjectStateEntityClusterCore,
    WorldObjectStateEntityPulse,
    WorldObjectStateEntityStability,
    WorldSemanticRelationConnector,
    WorldSemanticContextRing,
    WorldAvatarPlaceholderSelf,
    WorldAvatarPlaceholderOther,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    #[serde(default)]
    pub decoded_coarse_second_x: Option<u8>,
    #[serde(default)]
    pub decoded_coarse_second_y: Option<u8>,
    #[serde(default)]
    pub decoded_coarse_second_z: Option<u8>,
    #[serde(default)]
    pub decoded_coarse_third_x: Option<u8>,
    #[serde(default)]
    pub decoded_coarse_third_y: Option<u8>,
    #[serde(default)]
    pub decoded_coarse_third_z: Option<u8>,
    #[serde(default)]
    pub decoded_health_updates: u32,
    #[serde(default)]
    pub decoded_health_last_basis_points: Option<u16>,
    #[serde(default)]
    pub decoded_viewer_time_updates: u32,
    #[serde(default)]
    pub decoded_viewer_time_body_len: Option<u16>,
    #[serde(default)]
    pub decoded_viewer_time_signature: Option<u32>,
    pub observed_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    pub id: u64,
    pub observed_at_unix_ms: u64,
    pub sender: String,
    pub text: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChatDraft {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ChatSendStatus {
    #[default]
    Idle,
    Sending,
    Sent,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatConnectionState {
    Disabled,
    Connecting,
    Connected,
    Reconnecting,
    Failed(String),
}

impl Default for ChatConnectionState {
    fn default() -> Self {
        Self::Disabled
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChatState {
    pub connection: ChatConnectionState,
    pub draft: ChatDraft,
    pub send_status: ChatSendStatus,
    pub messages: Vec<ChatMessage>,
    next_local_message_id: u64,
}

impl ChatState {
    pub fn set_connection(&mut self, connection: ChatConnectionState) {
        self.connection = connection;
    }

    pub fn set_draft(&mut self, draft: impl Into<String>) {
        self.draft.text = draft.into();
    }

    pub fn mark_sending(&mut self) {
        self.send_status = ChatSendStatus::Sending;
    }

    pub fn mark_sent(&mut self) {
        self.send_status = ChatSendStatus::Sent;
    }

    pub fn mark_send_failed(&mut self, reason: impl Into<String>) {
        self.send_status = ChatSendStatus::Failed(reason.into());
    }

    pub fn reset_send_status(&mut self) {
        self.send_status = ChatSendStatus::Idle;
    }

    pub fn allocate_local_message_id(&mut self) -> u64 {
        self.next_local_message_id = self.next_local_message_id.saturating_add(1);
        self.next_local_message_id
    }

    pub fn push_message(&mut self, message: ChatMessage) {
        if self
            .messages
            .iter()
            .any(|existing| existing.id == message.id)
        {
            return;
        }
        self.messages.push(message);
        if self.messages.len() > 200 {
            let keep_from = self.messages.len() - 200;
            self.messages.drain(0..keep_from);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FriendEntry {
    pub id: String,
    pub display_name: Option<String>,
    pub name_source: Option<String>,
    pub last_name_resolved_unix_ms: Option<u64>,
    pub online: bool,
    pub rights_has: i32,
    pub rights_given: i32,
    pub last_changed_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectImMessage {
    pub id: u64,
    pub session_id: String,
    pub peer_id: String,
    pub from_id: String,
    pub from_name: String,
    pub text: String,
    pub observed_at_unix_ms: u64,
    pub outgoing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DirectImThread {
    pub session_id: String,
    pub participant_id: String,
    pub messages: Vec<DirectImMessage>,
    pub last_activity_unix_ms: u64,
    pub unread_count: u32,
    pub last_read_unix_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeRelayLevel {
    Trace,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeRelayEvent {
    pub at_unix_ms: u64,
    pub level: RuntimeRelayLevel,
    pub category: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuntimeRelayState {
    pub events: Vec<RuntimeRelayEvent>,
    pub max_events: usize,
}

impl RuntimeRelayState {
    pub fn push(&mut self, event: RuntimeRelayEvent) {
        if self.max_events == 0 {
            self.max_events = 500;
        }
        self.events.push(event);
        if self.events.len() > self.max_events {
            let keep_from = self.events.len() - self.max_events;
            self.events.drain(0..keep_from);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SocialState {
    pub friends: Vec<FriendEntry>,
    pub selected_friend_id: Option<String>,
    pub im_draft: String,
    pub im_threads: Vec<DirectImThread>,
    pub avatar_render_mode: AvatarRenderMode,
    pub relay: RuntimeRelayState,
    next_im_message_id: u64,
}

impl SocialState {
    pub fn upsert_friend(&mut self, friend: FriendEntry) {
        if let Some(existing) = self.friends.iter_mut().find(|f| f.id == friend.id) {
            existing.online = friend.online;
            existing.rights_has = friend.rights_has;
            existing.rights_given = friend.rights_given;
            existing.last_changed_unix_ms = friend.last_changed_unix_ms;
            if friend.display_name.is_some() {
                existing.display_name = friend.display_name;
            }
            if friend.name_source.is_some() {
                existing.name_source = friend.name_source;
            }
            if friend.last_name_resolved_unix_ms.is_some() {
                existing.last_name_resolved_unix_ms = friend.last_name_resolved_unix_ms;
            }
        } else {
            self.friends.push(friend);
            self.friends.sort_by(|a, b| a.id.cmp(&b.id));
        }
    }

    pub fn set_friend_online(&mut self, id: &str, online: bool, at_unix_ms: u64) {
        if let Some(existing) = self.friends.iter_mut().find(|f| f.id == id) {
            existing.online = online;
            existing.last_changed_unix_ms = at_unix_ms;
        } else {
            self.upsert_friend(FriendEntry {
                id: id.to_string(),
                display_name: None,
                name_source: None,
                last_name_resolved_unix_ms: None,
                online,
                rights_has: 0,
                rights_given: 0,
                last_changed_unix_ms: at_unix_ms,
            });
        }
    }

    pub fn resolve_friend_name(
        &mut self,
        id: &str,
        display_name: &str,
        source: &str,
        at_unix_ms: u64,
    ) {
        if display_name.trim().is_empty() {
            return;
        }
        if let Some(existing) = self.friends.iter_mut().find(|f| f.id == id) {
            existing.display_name = Some(display_name.trim().to_string());
            existing.name_source = Some(source.to_string());
            existing.last_name_resolved_unix_ms = Some(at_unix_ms);
            return;
        }
        self.upsert_friend(FriendEntry {
            id: id.to_string(),
            display_name: Some(display_name.trim().to_string()),
            name_source: Some(source.to_string()),
            last_name_resolved_unix_ms: Some(at_unix_ms),
            online: false,
            rights_has: 0,
            rights_given: 0,
            last_changed_unix_ms: at_unix_ms,
        });
    }

    pub fn friend_display_label(friend: &FriendEntry) -> String {
        if let Some(name) = friend.display_name.as_deref() {
            let short = short_uuid(&friend.id);
            if is_probably_unreadable_label(name) {
                return format!("{short} ({})", friend.id);
            }
            return format!("{name} ({short})");
        }
        friend.id.clone()
    }

    pub fn upsert_thread_message(&mut self, message: DirectImMessage, participant_id: &str) {
        let session_id = message.session_id.clone();
        let message_at = message.observed_at_unix_ms;
        let incoming = !message.outgoing;
        let thread = if let Some(existing) = self
            .im_threads
            .iter_mut()
            .find(|t| t.session_id == session_id)
        {
            existing
        } else {
            self.im_threads.push(DirectImThread {
                session_id: session_id.clone(),
                participant_id: participant_id.to_string(),
                messages: Vec::new(),
                last_activity_unix_ms: 0,
                unread_count: 0,
                last_read_unix_ms: None,
            });
            self.im_threads
                .iter_mut()
                .find(|t| t.session_id == session_id)
                .expect("newly pushed thread should exist")
        };
        if !thread.messages.iter().any(|m| m.id == message.id) {
            thread.messages.push(message);
            thread.last_activity_unix_ms = thread.last_activity_unix_ms.max(message_at);
            if incoming {
                thread.unread_count = thread.unread_count.saturating_add(1);
            }
            if thread.messages.len() > 300 {
                let keep_from = thread.messages.len() - 300;
                thread.messages.drain(0..keep_from);
            }
        }
    }

    pub fn mark_thread_read_by_participant(&mut self, participant_id: &str, at_unix_ms: u64) {
        if let Some(thread) = self
            .im_threads
            .iter_mut()
            .find(|t| t.participant_id == participant_id || t.session_id == participant_id)
        {
            thread.unread_count = 0;
            thread.last_read_unix_ms = Some(at_unix_ms);
        }
    }

    pub fn clear_all_unread(&mut self, at_unix_ms: u64) {
        for thread in &mut self.im_threads {
            thread.unread_count = 0;
            thread.last_read_unix_ms = Some(at_unix_ms);
        }
    }

    pub fn thread_for_participant(&self, participant_id: &str) -> Option<&DirectImThread> {
        self.im_threads
            .iter()
            .find(|t| t.participant_id == participant_id || t.session_id == participant_id)
    }

    pub fn sorted_thread_participants_by_recent(&self) -> Vec<String> {
        let mut pairs: Vec<(String, u64)> = self
            .im_threads
            .iter()
            .map(|thread| (thread.participant_id.clone(), thread.last_activity_unix_ms))
            .collect();
        pairs.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        pairs.into_iter().map(|(id, _)| id).collect()
    }

    pub fn next_im_message_id(&mut self) -> u64 {
        self.next_im_message_id = self.next_im_message_id.saturating_add(1);
        self.next_im_message_id
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorldAvatarPlaceholder {
    pub agent_id: String,
    pub world_position: [f32; 3],
    pub local_position: Option<[u8; 3]>,
    pub sim_name: Option<String>,
    pub display_name: String,
    pub is_self: bool,
    pub last_update_unix_ms: u64,
    pub stale: bool,
}

impl WorldAvatarPlaceholder {
    pub fn short_agent_id(&self) -> String {
        short_uuid(&self.agent_id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AvatarProfileTab {
    SecondLife,
    Feed,
    Picks,
    Classifieds,
    FirstLife,
    Notes,
}

impl Default for AvatarProfileTab {
    fn default() -> Self {
        Self::SecondLife
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileLoadStatus {
    Idle,
    Loading,
    Loaded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileLoadState {
    pub status: ProfileLoadStatus,
    pub last_updated_unix_ms: Option<u64>,
    pub error: Option<String>,
}

impl Default for ProfileLoadState {
    fn default() -> Self {
        Self {
            status: ProfileLoadStatus::Idle,
            last_updated_unix_ms: None,
            error: None,
        }
    }
}

impl ProfileLoadState {
    pub fn mark_loading(&mut self, at_unix_ms: u64) {
        self.status = ProfileLoadStatus::Loading;
        self.last_updated_unix_ms = Some(at_unix_ms);
        self.error = None;
    }

    pub fn mark_loaded(&mut self, at_unix_ms: u64) {
        self.status = ProfileLoadStatus::Loaded;
        self.last_updated_unix_ms = Some(at_unix_ms);
        self.error = None;
    }

    pub fn mark_failed(&mut self, at_unix_ms: u64, reason: impl Into<String>) {
        self.status = ProfileLoadStatus::Failed;
        self.last_updated_unix_ms = Some(at_unix_ms);
        self.error = Some(reason.into());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProfileGroup {
    pub id: String,
    pub name: String,
    pub insignia_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SecondLifeProfile {
    pub avatar_id: String,
    pub display_name: Option<String>,
    pub username: Option<String>,
    pub profile_url: Option<String>,
    pub about_text: String,
    pub image_id: Option<String>,
    pub partner_id: Option<String>,
    pub member_since: Option<String>,
    pub allow_publish: Option<bool>,
    pub online: Option<bool>,
    pub identified: Option<bool>,
    pub transacted: Option<bool>,
    pub groups: Vec<ProfileGroup>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FirstLifeProfile {
    pub about_text: String,
    pub image_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProfileNotes {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProfilePickSummary {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProfilePickDetails {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub snapshot_id: Option<String>,
    pub parcel_id: Option<String>,
    pub sim_name: Option<String>,
    pub parcel_name: Option<String>,
    pub global_position: Option<[i32; 3]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProfilePicksList {
    pub items: Vec<ProfilePickSummary>,
    pub selected_pick_id: Option<String>,
    pub details: BTreeMap<String, ProfilePickDetails>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProfileClassifiedSummary {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProfileClassifiedDetails {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub snapshot_id: Option<String>,
    pub parcel_id: Option<String>,
    pub sim_name: Option<String>,
    pub parcel_name: Option<String>,
    pub global_position: Option<[i32; 3]>,
    pub category: Option<u32>,
    pub flags: Option<u8>,
    pub price_for_listing: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProfileClassifiedsList {
    pub items: Vec<ProfileClassifiedSummary>,
    pub selected_classified_id: Option<String>,
    pub details: BTreeMap<String, ProfileClassifiedDetails>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProfileFeedLinkState {
    pub url: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AvatarProfileState {
    pub avatar_id: String,
    pub selected_tab: AvatarProfileTab,
    pub second_life: Option<SecondLifeProfile>,
    pub first_life: Option<FirstLifeProfile>,
    pub notes: Option<ProfileNotes>,
    pub picks: ProfilePicksList,
    pub classifieds: ProfileClassifiedsList,
    pub feed: ProfileFeedLinkState,
    pub load_second_life: ProfileLoadState,
    pub load_feed: ProfileLoadState,
    pub load_picks: ProfileLoadState,
    pub load_classifieds: ProfileLoadState,
    pub load_first_life: ProfileLoadState,
    pub load_notes: ProfileLoadState,
}

impl AvatarProfileState {
    pub fn display_label(&self) -> String {
        if let Some(sl) = self.second_life.as_ref() {
            if let Some(name) = sl.display_name.as_ref() {
                if is_probably_unreadable_label(name) {
                    if let Some(username) = sl.username.as_ref() {
                        return format!("{username} ({})", short_uuid(&self.avatar_id));
                    }
                    return self.avatar_id.clone();
                }
                return format!("{name} ({})", short_uuid(&self.avatar_id));
            }
            if let Some(username) = sl.username.as_ref() {
                return format!("{username} ({})", short_uuid(&self.avatar_id));
            }
        }
        self.avatar_id.clone()
    }

    pub fn tab_load_mut(&mut self, tab: AvatarProfileTab) -> &mut ProfileLoadState {
        match tab {
            AvatarProfileTab::SecondLife => &mut self.load_second_life,
            AvatarProfileTab::Feed => &mut self.load_feed,
            AvatarProfileTab::Picks => &mut self.load_picks,
            AvatarProfileTab::Classifieds => &mut self.load_classifieds,
            AvatarProfileTab::FirstLife => &mut self.load_first_life,
            AvatarProfileTab::Notes => &mut self.load_notes,
        }
    }

    pub fn tab_load(&self, tab: AvatarProfileTab) -> &ProfileLoadState {
        match tab {
            AvatarProfileTab::SecondLife => &self.load_second_life,
            AvatarProfileTab::Feed => &self.load_feed,
            AvatarProfileTab::Picks => &self.load_picks,
            AvatarProfileTab::Classifieds => &self.load_classifieds,
            AvatarProfileTab::FirstLife => &self.load_first_life,
            AvatarProfileTab::Notes => &self.load_notes,
        }
    }
}

pub fn compute_p2p_session_id(agent_id: &str, other_agent_id: &str) -> Option<String> {
    let agent = parse_uuid_bytes(agent_id)?;
    let other = parse_uuid_bytes(other_agent_id)?;
    if agent == other {
        return Some(agent_id.to_ascii_lowercase());
    }
    let mut xored = [0u8; 16];
    for idx in 0..16 {
        xored[idx] = agent[idx] ^ other[idx];
    }
    Some(format_uuid_bytes(xored))
}

fn parse_uuid_bytes(raw: &str) -> Option<[u8; 16]> {
    let compact = raw.replace('-', "");
    if compact.len() != 32 {
        return None;
    }
    let mut bytes = [0u8; 16];
    for (idx, slot) in bytes.iter_mut().enumerate() {
        let start = idx * 2;
        let end = start + 2;
        *slot = u8::from_str_radix(&compact[start..end], 16).ok()?;
    }
    Some(bytes)
}

fn short_uuid(uuid: &str) -> String {
    uuid.chars().take(8).collect()
}

fn is_probably_unreadable_label(value: &str) -> bool {
    if value.trim().is_empty() {
        return true;
    }
    let non_ascii = value.chars().filter(|ch| !ch.is_ascii()).count();
    non_ascii > 0
}

fn format_uuid_bytes(bytes: [u8; 16]) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15]
    )
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
    DecodedCoarseNeighborhoodPayload,
    DecodedHealthPayload,
    DecodedViewerTimePayload,
    ObjectStateEntitySeedPayload,
    ObjectStateEntityLifecyclePayload,
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
    pub decoded_coarse_second_xyz: Option<[u8; 3]>,
    pub decoded_coarse_third_xyz: Option<[u8; 3]>,
    pub decoded_coarse_updates: Option<u32>,
    pub decoded_health_updates: Option<u32>,
    pub decoded_health_basis_points: Option<u16>,
    pub decoded_viewer_time_updates: Option<u32>,
    pub decoded_viewer_time_body_len: Option<u16>,
    pub decoded_viewer_time_signature: Option<u32>,
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
            decoded_coarse_second_xyz: None,
            decoded_coarse_third_xyz: None,
            decoded_coarse_updates: None,
            decoded_health_updates: None,
            decoded_health_basis_points: None,
            decoded_viewer_time_updates: None,
            decoded_viewer_time_body_len: None,
            decoded_viewer_time_signature: None,
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
                decoded_coarse_second_xyz: None,
                decoded_coarse_third_xyz: None,
                decoded_coarse_updates: None,
                decoded_health_updates: None,
                decoded_health_basis_points: None,
                decoded_viewer_time_updates: None,
                decoded_viewer_time_body_len: None,
                decoded_viewer_time_signature: None,
            });
        }
        Self { items }
    }

    pub fn from_live_snapshot(snapshot: Option<&LiveVisualSnapshot>) -> Self {
        let mut seam =
            Self::from_diagnostic_slice(WorldDiagnosticSlice::from_live_snapshot(snapshot));
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
                decoded_coarse_second_xyz: None,
                decoded_coarse_third_xyz: None,
                decoded_coarse_updates: None,
                decoded_health_updates: None,
                decoded_health_basis_points: None,
                decoded_viewer_time_updates: None,
                decoded_viewer_time_body_len: None,
                decoded_viewer_time_signature: None,
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
            let second_xyz = snapshot.and_then(|s| {
                Some([
                    s.decoded_coarse_second_x?,
                    s.decoded_coarse_second_y?,
                    s.decoded_coarse_second_z?,
                ])
            });
            let third_xyz = snapshot.and_then(|s| {
                Some([
                    s.decoded_coarse_third_x?,
                    s.decoded_coarse_third_y?,
                    s.decoded_coarse_third_z?,
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
                decoded_coarse_second_xyz: second_xyz,
                decoded_coarse_third_xyz: third_xyz,
                decoded_coarse_updates: snapshot.map(|s| s.decoded_coarse_updates),
                decoded_health_updates: None,
                decoded_health_basis_points: None,
                decoded_viewer_time_updates: None,
                decoded_viewer_time_body_len: None,
                decoded_viewer_time_signature: None,
            });
            if let Some(coarse_second_xyz) = second_xyz {
                seam.items.push(WorldObjectIngestionItem {
                    lane: WorldObjectIngestionLane::DecodedCoarseNeighborhoodPayload,
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
                    decoded_coarse_second_xyz: Some(coarse_second_xyz),
                    decoded_coarse_third_xyz: third_xyz,
                    decoded_coarse_updates: snapshot.map(|s| s.decoded_coarse_updates),
                    decoded_health_updates: None,
                    decoded_health_basis_points: None,
                    decoded_viewer_time_updates: None,
                    decoded_viewer_time_body_len: None,
                    decoded_viewer_time_signature: None,
                });
            }
        }
        if let Some(health_basis_points) = snapshot.and_then(|s| s.decoded_health_last_basis_points)
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
                lane: WorldObjectIngestionLane::DecodedHealthPayload,
                stage,
                region_coords,
                simulator_target_present: false,
                traffic_broader_count: 0,
                traffic_unknown_count: 0,
                traffic_region_control_count: 0,
                decoded_endpoint_port: None,
                decoded_endpoint_host_tail: None,
                decoded_coarse_location_count: None,
                decoded_coarse_first_xyz: None,
                decoded_coarse_second_xyz: None,
                decoded_coarse_third_xyz: None,
                decoded_coarse_updates: None,
                decoded_health_updates: snapshot.map(|s| s.decoded_health_updates),
                decoded_health_basis_points: Some(health_basis_points),
                decoded_viewer_time_updates: None,
                decoded_viewer_time_body_len: None,
                decoded_viewer_time_signature: None,
            });
        }
        if let Some(body_len) = snapshot.and_then(|s| s.decoded_viewer_time_body_len) {
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
                lane: WorldObjectIngestionLane::DecodedViewerTimePayload,
                stage,
                region_coords,
                simulator_target_present: false,
                traffic_broader_count: 0,
                traffic_unknown_count: 0,
                traffic_region_control_count: 0,
                decoded_endpoint_port: None,
                decoded_endpoint_host_tail: None,
                decoded_coarse_location_count: None,
                decoded_coarse_first_xyz: None,
                decoded_coarse_second_xyz: None,
                decoded_coarse_third_xyz: None,
                decoded_coarse_updates: None,
                decoded_health_updates: None,
                decoded_health_basis_points: None,
                decoded_viewer_time_updates: snapshot.map(|s| s.decoded_viewer_time_updates),
                decoded_viewer_time_body_len: Some(body_len),
                decoded_viewer_time_signature: snapshot
                    .and_then(|s| s.decoded_viewer_time_signature),
            });
        }
        if let Some(state) = snapshot {
            let coarse_xyz = match (
                state.decoded_coarse_first_x,
                state.decoded_coarse_first_y,
                state.decoded_coarse_first_z,
            ) {
                (Some(x), Some(y), Some(z)) => Some([x, y, z]),
                _ => None,
            };
            if let (Some(health_basis_points), Some(coarse_count), Some(coarse_first_xyz)) = (
                state.decoded_health_last_basis_points,
                state.decoded_coarse_location_count,
                coarse_xyz,
            ) {
                let stage = if state.logged_in && state.handshake_agent_movement_complete {
                    WorldEntryStage::EnteredFirstRegion
                } else if state.logged_in {
                    WorldEntryStage::Connected
                } else {
                    WorldEntryStage::Offline
                };
                let region_coords = match (state.first_sim_region_x, state.first_sim_region_y) {
                    (Some(x), Some(y)) => Some([x, y]),
                    _ => None,
                };
                seam.items.push(WorldObjectIngestionItem {
                    lane: WorldObjectIngestionLane::ObjectStateEntitySeedPayload,
                    stage,
                    region_coords,
                    simulator_target_present: state.first_sim_endpoint.is_some(),
                    traffic_broader_count: 0,
                    traffic_unknown_count: 0,
                    traffic_region_control_count: 0,
                    decoded_endpoint_port: None,
                    decoded_endpoint_host_tail: None,
                    decoded_coarse_location_count: Some(coarse_count),
                    decoded_coarse_first_xyz: Some(coarse_first_xyz),
                    decoded_coarse_second_xyz: match (
                        state.decoded_coarse_second_x,
                        state.decoded_coarse_second_y,
                        state.decoded_coarse_second_z,
                    ) {
                        (Some(x), Some(y), Some(z)) => Some([x, y, z]),
                        _ => None,
                    },
                    decoded_coarse_third_xyz: match (
                        state.decoded_coarse_third_x,
                        state.decoded_coarse_third_y,
                        state.decoded_coarse_third_z,
                    ) {
                        (Some(x), Some(y), Some(z)) => Some([x, y, z]),
                        _ => None,
                    },
                    decoded_coarse_updates: Some(state.decoded_coarse_updates),
                    decoded_health_updates: Some(state.decoded_health_updates),
                    decoded_health_basis_points: Some(health_basis_points),
                    decoded_viewer_time_updates: Some(state.decoded_viewer_time_updates),
                    decoded_viewer_time_body_len: state.decoded_viewer_time_body_len,
                    decoded_viewer_time_signature: state.decoded_viewer_time_signature,
                });
                seam.items.push(WorldObjectIngestionItem {
                    lane: WorldObjectIngestionLane::ObjectStateEntityLifecyclePayload,
                    stage,
                    region_coords,
                    simulator_target_present: state.first_sim_endpoint.is_some(),
                    traffic_broader_count: 0,
                    traffic_unknown_count: 0,
                    traffic_region_control_count: 0,
                    decoded_endpoint_port: None,
                    decoded_endpoint_host_tail: None,
                    decoded_coarse_location_count: Some(coarse_count),
                    decoded_coarse_first_xyz: Some(coarse_first_xyz),
                    decoded_coarse_second_xyz: match (
                        state.decoded_coarse_second_x,
                        state.decoded_coarse_second_y,
                        state.decoded_coarse_second_z,
                    ) {
                        (Some(x), Some(y), Some(z)) => Some([x, y, z]),
                        _ => None,
                    },
                    decoded_coarse_third_xyz: match (
                        state.decoded_coarse_third_x,
                        state.decoded_coarse_third_y,
                        state.decoded_coarse_third_z,
                    ) {
                        (Some(x), Some(y), Some(z)) => Some([x, y, z]),
                        _ => None,
                    },
                    decoded_coarse_updates: Some(state.decoded_coarse_updates),
                    decoded_health_updates: Some(state.decoded_health_updates),
                    decoded_health_basis_points: Some(health_basis_points),
                    decoded_viewer_time_updates: Some(state.decoded_viewer_time_updates),
                    decoded_viewer_time_body_len: state.decoded_viewer_time_body_len,
                    decoded_viewer_time_signature: state.decoded_viewer_time_signature,
                });
            }
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
        let Some(cube) = self.instances.iter_mut().find(|instance| {
            instance.mesh == MeshKind::Cube && instance.role == InstanceRole::SceneStatic
        }) else {
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
            remove_instance(
                &mut self.instances,
                InstanceRole::WorldIngestionTrafficPayload,
            );
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

        if let Some(item) =
            seam.items.iter().copied().find(|item| {
                item.lane == WorldObjectIngestionLane::DecodedCoarseNeighborhoodPayload
            })
        {
            upsert_instance(
                &mut self.instances,
                InstanceRole::WorldIngestionDecodedCoarseNeighborhoodPayload,
                MeshKind::Cube,
                world_ingestion_decoded_coarse_neighborhood_transform(item),
                world_ingestion_decoded_coarse_neighborhood_color(item),
            );
            upsert_instance(
                &mut self.instances,
                InstanceRole::WorldIngestionDecodedCoarseNeighborhoodSatelliteA,
                MeshKind::AxisMarker,
                world_ingestion_decoded_coarse_neighborhood_satellite_a_transform(item),
                world_ingestion_decoded_coarse_neighborhood_satellite_a_color(item),
            );
            upsert_instance(
                &mut self.instances,
                InstanceRole::WorldIngestionDecodedCoarseNeighborhoodSatelliteB,
                MeshKind::AxisMarker,
                world_ingestion_decoded_coarse_neighborhood_satellite_b_transform(item),
                world_ingestion_decoded_coarse_neighborhood_satellite_b_color(item),
            );
        } else {
            remove_instance(
                &mut self.instances,
                InstanceRole::WorldIngestionDecodedCoarseNeighborhoodPayload,
            );
            remove_instance(
                &mut self.instances,
                InstanceRole::WorldIngestionDecodedCoarseNeighborhoodSatelliteA,
            );
            remove_instance(
                &mut self.instances,
                InstanceRole::WorldIngestionDecodedCoarseNeighborhoodSatelliteB,
            );
        }

        if let Some(item) = seam
            .items
            .iter()
            .copied()
            .find(|item| item.lane == WorldObjectIngestionLane::DecodedHealthPayload)
        {
            upsert_instance(
                &mut self.instances,
                InstanceRole::WorldIngestionDecodedHealthPayload,
                MeshKind::Cube,
                world_ingestion_decoded_health_transform(item),
                world_ingestion_decoded_health_color(item),
            );
        } else {
            remove_instance(
                &mut self.instances,
                InstanceRole::WorldIngestionDecodedHealthPayload,
            );
        }

        if let Some(item) = seam
            .items
            .iter()
            .copied()
            .find(|item| item.lane == WorldObjectIngestionLane::DecodedViewerTimePayload)
        {
            upsert_instance(
                &mut self.instances,
                InstanceRole::WorldIngestionDecodedViewerTimePayload,
                MeshKind::AxisMarker,
                world_ingestion_decoded_viewer_time_transform(item),
                world_ingestion_decoded_viewer_time_color(item),
            );
        } else {
            remove_instance(
                &mut self.instances,
                InstanceRole::WorldIngestionDecodedViewerTimePayload,
            );
        }

        let coarse_item = seam
            .items
            .iter()
            .copied()
            .find(|item| item.lane == WorldObjectIngestionLane::DecodedCoarseLocationPayload);
        let health_item = seam
            .items
            .iter()
            .copied()
            .find(|item| item.lane == WorldObjectIngestionLane::DecodedHealthPayload);
        if let (Some(coarse), Some(health)) = (coarse_item, health_item) {
            upsert_instance(
                &mut self.instances,
                InstanceRole::WorldIngestionDecodedCompositeBeacon,
                MeshKind::Cube,
                world_ingestion_decoded_composite_transform(coarse, health),
                world_ingestion_decoded_composite_color(health),
            );
        } else {
            remove_instance(
                &mut self.instances,
                InstanceRole::WorldIngestionDecodedCompositeBeacon,
            );
        }

        if let Some(item) = seam
            .items
            .iter()
            .copied()
            .find(|item| item.lane == WorldObjectIngestionLane::ObjectStateEntitySeedPayload)
        {
            let entity_count = world_object_state_entity_count(item);
            upsert_instance(
                &mut self.instances,
                InstanceRole::WorldObjectStateEntityBody,
                MeshKind::Cube,
                world_object_state_entity_body_transform(item, 0),
                world_object_state_entity_body_color(item, 0),
            );
            upsert_instance(
                &mut self.instances,
                InstanceRole::WorldObjectStateEntityAura,
                MeshKind::AxisMarker,
                world_object_state_entity_aura_transform(item, 0),
                world_object_state_entity_aura_color(item, 0),
            );
            if entity_count >= 2 {
                upsert_instance(
                    &mut self.instances,
                    InstanceRole::WorldObjectStateEntityWingBody,
                    MeshKind::Cube,
                    world_object_state_entity_body_transform(item, 1),
                    world_object_state_entity_body_color(item, 1),
                );
                upsert_instance(
                    &mut self.instances,
                    InstanceRole::WorldObjectStateEntityWingAura,
                    MeshKind::AxisMarker,
                    world_object_state_entity_aura_transform(item, 1),
                    world_object_state_entity_aura_color(item, 1),
                );
            } else {
                remove_instance(
                    &mut self.instances,
                    InstanceRole::WorldObjectStateEntityWingBody,
                );
                remove_instance(
                    &mut self.instances,
                    InstanceRole::WorldObjectStateEntityWingAura,
                );
            }
            if entity_count >= 3 {
                upsert_instance(
                    &mut self.instances,
                    InstanceRole::WorldObjectStateEntityGuardBody,
                    MeshKind::Cube,
                    world_object_state_entity_body_transform(item, 2),
                    world_object_state_entity_body_color(item, 2),
                );
                upsert_instance(
                    &mut self.instances,
                    InstanceRole::WorldObjectStateEntityGuardAura,
                    MeshKind::AxisMarker,
                    world_object_state_entity_aura_transform(item, 2),
                    world_object_state_entity_aura_color(item, 2),
                );
            } else {
                remove_instance(
                    &mut self.instances,
                    InstanceRole::WorldObjectStateEntityGuardBody,
                );
                remove_instance(
                    &mut self.instances,
                    InstanceRole::WorldObjectStateEntityGuardAura,
                );
            }
            upsert_instance(
                &mut self.instances,
                InstanceRole::WorldObjectStateEntityClusterCore,
                MeshKind::AxisMarker,
                world_object_state_entity_cluster_core_transform(item, entity_count),
                world_object_state_entity_cluster_core_color(item, entity_count),
            );
        } else {
            remove_instance(
                &mut self.instances,
                InstanceRole::WorldObjectStateEntityBody,
            );
            remove_instance(
                &mut self.instances,
                InstanceRole::WorldObjectStateEntityAura,
            );
            remove_instance(
                &mut self.instances,
                InstanceRole::WorldObjectStateEntityWingBody,
            );
            remove_instance(
                &mut self.instances,
                InstanceRole::WorldObjectStateEntityWingAura,
            );
            remove_instance(
                &mut self.instances,
                InstanceRole::WorldObjectStateEntityGuardBody,
            );
            remove_instance(
                &mut self.instances,
                InstanceRole::WorldObjectStateEntityGuardAura,
            );
            remove_instance(
                &mut self.instances,
                InstanceRole::WorldObjectStateEntityClusterCore,
            );
        }

        if let Some(item) =
            seam.items.iter().copied().find(|item| {
                item.lane == WorldObjectIngestionLane::ObjectStateEntityLifecyclePayload
            })
        {
            upsert_instance(
                &mut self.instances,
                InstanceRole::WorldObjectStateEntityPulse,
                MeshKind::AxisMarker,
                world_object_state_entity_pulse_transform(item),
                world_object_state_entity_pulse_color(item),
            );
            upsert_instance(
                &mut self.instances,
                InstanceRole::WorldObjectStateEntityStability,
                MeshKind::Cube,
                world_object_state_entity_stability_transform(item),
                world_object_state_entity_stability_color(item),
            );
        } else {
            remove_instance(
                &mut self.instances,
                InstanceRole::WorldObjectStateEntityPulse,
            );
            remove_instance(
                &mut self.instances,
                InstanceRole::WorldObjectStateEntityStability,
            );
        }
    }

    pub fn apply_world_avatar_placeholders(
        &mut self,
        avatars: &[WorldAvatarPlaceholder],
        mode: AvatarRenderMode,
    ) {
        self.instances.retain(|instance| {
            instance.role != InstanceRole::WorldAvatarPlaceholderSelf
                && instance.role != InstanceRole::WorldAvatarPlaceholderOther
        });
        for avatar in avatars {
            let (mesh, scale, color) = match (mode, avatar.is_self, avatar.stale) {
                (AvatarRenderMode::Proxy, true, _) => {
                    (MeshKind::AvatarProxy, [0.38, 1.30, 0.38], [0.22, 0.86, 0.40])
                }
                (AvatarRenderMode::Proxy, false, true) => {
                    (MeshKind::AvatarProxy, [0.34, 1.16, 0.34], [0.66, 0.50, 0.38])
                }
                (AvatarRenderMode::Proxy, false, false) => {
                    (MeshKind::AvatarProxy, [0.36, 1.22, 0.36], [0.30, 0.74, 0.98])
                }
                (AvatarRenderMode::FallbackBox, true, _) => {
                    (MeshKind::Cube, [0.32, 1.20, 0.32], [0.22, 0.86, 0.40])
                }
                (AvatarRenderMode::FallbackBox, false, true) => {
                    (MeshKind::Cube, [0.30, 1.10, 0.30], [0.66, 0.50, 0.38])
                }
                (AvatarRenderMode::FallbackBox, false, false) => {
                    (MeshKind::Cube, [0.30, 1.10, 0.30], [0.30, 0.74, 0.98])
                }
            };
            self.instances.push(RenderableInstance {
                mesh,
                role: if avatar.is_self {
                    InstanceRole::WorldAvatarPlaceholderSelf
                } else {
                    InstanceRole::WorldAvatarPlaceholderOther
                },
                transform: Transform {
                    position: avatar.world_position,
                    scale,
                },
                color,
            });
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

fn world_cluster_base(region_coords: Option<[u32; 2]>) -> [f32; 2] {
    let [offset_x, offset_z] = world_presence_offset(region_coords);
    [3.0 + offset_x, offset_z - 1.95]
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

fn world_traffic_pillar_transform(
    slice: WorldDiagnosticSlice,
    kind: TrafficPillarKind,
) -> Transform {
    let [base_x, base_z] = world_cluster_base(slice.presence.region_coords);
    let (x_shift, z_shift) = match kind {
        TrafficPillarKind::Broader => (-1.05, 0.95),
        TrafficPillarKind::Unknown => (0.0, 1.05),
        TrafficPillarKind::RegionControl => (1.05, 0.95),
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
    let [base_x, base_z] = world_cluster_base(item.region_coords);
    let (y, scale) = match item.stage {
        WorldEntryStage::EnteredFirstRegion => (0.36, [0.22, 0.22, 0.22]),
        WorldEntryStage::Connected => (0.28, [0.18, 0.18, 0.18]),
        WorldEntryStage::Offline => (0.22, [0.12, 0.12, 0.12]),
    };
    Transform {
        position: [base_x, y, base_z + 0.22],
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
    let [base_x, base_z] = world_cluster_base(item.region_coords);
    let weighted = item
        .traffic_broader_count
        .saturating_add(item.traffic_unknown_count.saturating_mul(2))
        .saturating_add(item.traffic_region_control_count.saturating_mul(3))
        .min(30) as f32;
    let y = 0.50 + weighted * 0.012;
    let scale = 0.24 + weighted * 0.005;
    Transform {
        position: [base_x + 0.92, y, base_z + 0.18],
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
    let [base_x, base_z] = world_cluster_base(item.region_coords);
    let host_tail = f32::from(item.decoded_endpoint_host_tail.unwrap_or(0));
    let port = f32::from(item.decoded_endpoint_port.unwrap_or(0));
    let x = base_x - 0.96 + ((host_tail / 255.0) - 0.5) * 0.85;
    let z = base_z - 0.18;
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
    let [base_x, base_z] = world_cluster_base(item.region_coords);
    let [coarse_x, coarse_y, coarse_z] = item.decoded_coarse_first_xyz.unwrap_or([128, 128, 0]);
    let x = base_x - 0.42 + ((f32::from(coarse_x) / 255.0) - 0.5) * 1.4;
    let z = base_z - 0.72 + ((f32::from(coarse_y) / 255.0) - 0.5) * 1.2;
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

fn world_ingestion_decoded_coarse_neighborhood_transform(
    item: WorldObjectIngestionItem,
) -> Transform {
    let [base_x, base_z] = world_cluster_base(item.region_coords);
    let [coarse_x, coarse_y, coarse_z] = item.decoded_coarse_second_xyz.unwrap_or([128, 128, 0]);
    let x = base_x + 1.05 + ((f32::from(coarse_x) / 255.0) - 0.5) * 1.8;
    let z = base_z - 0.10 + ((f32::from(coarse_y) / 255.0) - 0.5) * 1.6;
    let y = 0.30 + (f32::from(coarse_z) / 255.0) * 1.2;
    let count = f32::from(item.decoded_coarse_location_count.unwrap_or(0));
    let scale = (0.18 + count * 0.012).clamp(0.18, 0.52);
    Transform {
        position: [x, y, z],
        scale: [scale, scale, scale],
    }
}

fn world_ingestion_decoded_coarse_neighborhood_color(item: WorldObjectIngestionItem) -> [f32; 3] {
    let updates = item.decoded_coarse_updates.unwrap_or(0) as f32;
    let intensity = (updates.min(10.0) / 10.0).clamp(0.0, 1.0);
    [0.30, 0.64 + intensity * 0.30, 0.98]
}

fn world_ingestion_decoded_coarse_neighborhood_satellite_a_transform(
    item: WorldObjectIngestionItem,
) -> Transform {
    let [base_x, base_z] = world_cluster_base(item.region_coords);
    let [coarse_x, coarse_y, coarse_z] = item.decoded_coarse_first_xyz.unwrap_or([128, 128, 0]);
    let x = base_x + 1.55 + ((f32::from(coarse_x) / 255.0) - 0.5) * 1.0;
    let z = base_z + 0.35 + ((f32::from(coarse_y) / 255.0) - 0.5) * 1.0;
    let y = 0.34 + (f32::from(coarse_z) / 255.0) * 0.9;
    Transform {
        position: [x, y, z],
        scale: [0.18, 0.18, 0.18],
    }
}

fn world_ingestion_decoded_coarse_neighborhood_satellite_a_color(
    item: WorldObjectIngestionItem,
) -> [f32; 3] {
    let count = item.decoded_coarse_location_count.unwrap_or(0) as f32;
    let c = (count / 12.0).clamp(0.0, 1.0);
    [0.26, 0.74 + c * 0.20, 0.84]
}

fn world_ingestion_decoded_coarse_neighborhood_satellite_b_transform(
    item: WorldObjectIngestionItem,
) -> Transform {
    let [base_x, base_z] = world_cluster_base(item.region_coords);
    let [coarse_x, coarse_y, coarse_z] = item
        .decoded_coarse_third_xyz
        .or(item.decoded_coarse_second_xyz)
        .unwrap_or([128, 128, 0]);
    let x = base_x + 0.78 + ((f32::from(coarse_x) / 255.0) - 0.5) * 1.0;
    let z = base_z + 0.95 + ((f32::from(coarse_y) / 255.0) - 0.5) * 1.1;
    let y = 0.30 + (f32::from(coarse_z) / 255.0) * 0.85;
    Transform {
        position: [x, y, z],
        scale: [0.15, 0.15, 0.15],
    }
}

fn world_ingestion_decoded_coarse_neighborhood_satellite_b_color(
    item: WorldObjectIngestionItem,
) -> [f32; 3] {
    let updates = item.decoded_coarse_updates.unwrap_or(0) as f32;
    let i = (updates.min(12.0) / 12.0).clamp(0.0, 1.0);
    [0.24, 0.58 + i * 0.34, 0.96]
}

fn world_ingestion_decoded_health_transform(item: WorldObjectIngestionItem) -> Transform {
    let [base_x, base_z] = world_cluster_base(item.region_coords);
    let basis_points = f32::from(item.decoded_health_basis_points.unwrap_or(0));
    let normalized = (basis_points / 10_000.0).clamp(0.0, 1.0);
    let y = 0.25 + normalized * 1.25;
    let scale = 0.14 + normalized * 0.34;
    Transform {
        position: [base_x + 0.62, y, base_z - 0.92],
        scale: [scale, scale, scale],
    }
}

fn world_ingestion_decoded_health_color(item: WorldObjectIngestionItem) -> [f32; 3] {
    let basis_points = f32::from(item.decoded_health_basis_points.unwrap_or(0));
    let normalized = (basis_points / 10_000.0).clamp(0.0, 1.0);
    [1.0 - normalized * 0.72, 0.28 + normalized * 0.66, 0.24]
}

fn world_ingestion_decoded_viewer_time_transform(item: WorldObjectIngestionItem) -> Transform {
    let [base_x, base_z] = world_cluster_base(item.region_coords);
    let updates = item.decoded_viewer_time_updates.unwrap_or(0);
    let body_len = f32::from(item.decoded_viewer_time_body_len.unwrap_or(0));
    let signature = item.decoded_viewer_time_signature.unwrap_or(0);
    let heading = ((signature % 3600) as f32 / 3600.0) * core::f32::consts::TAU;
    let radius = 1.05 + (body_len / 256.0).clamp(0.0, 0.36);
    let y = 0.64 + ((updates % 12) as f32 / 12.0) * 0.58;
    let scale = (0.22 + (body_len / 255.0) * 0.24).clamp(0.20, 0.48);
    Transform {
        position: [
            base_x + heading.cos() * radius,
            y,
            base_z + heading.sin() * radius,
        ],
        scale: [scale, scale, scale],
    }
}

fn world_ingestion_decoded_viewer_time_color(item: WorldObjectIngestionItem) -> [f32; 3] {
    let updates = item.decoded_viewer_time_updates.unwrap_or(0) as f32;
    let body_len = f32::from(item.decoded_viewer_time_body_len.unwrap_or(0));
    let intensity = (updates.min(12.0) / 12.0).clamp(0.0, 1.0);
    let len_norm = (body_len / 255.0).clamp(0.0, 1.0);
    [0.36 + len_norm * 0.44, 0.34 + intensity * 0.48, 0.92]
}

fn world_ingestion_decoded_composite_transform(
    coarse: WorldObjectIngestionItem,
    health: WorldObjectIngestionItem,
) -> Transform {
    let [base_x, base_z] = world_cluster_base(coarse.region_coords.or(health.region_coords));
    let [cx, cy, cz] = coarse.decoded_coarse_first_xyz.unwrap_or([128, 128, 0]);
    let health_norm = f32::from(health.decoded_health_basis_points.unwrap_or(0)) / 10_000.0;
    let x = base_x + ((f32::from(cx) / 255.0) - 0.5) * 0.72;
    let z = base_z - 1.35 + ((f32::from(cy) / 255.0) - 0.5) * 0.72;
    let y = 0.32 + (f32::from(cz) / 255.0) * 0.55 + health_norm.clamp(0.0, 1.0) * 0.45;
    let coarse_count = f32::from(coarse.decoded_coarse_location_count.unwrap_or(0));
    let scale = (0.18 + coarse_count * 0.01 + health_norm * 0.18).clamp(0.18, 0.46);
    Transform {
        position: [x, y, z],
        scale: [scale, scale, scale],
    }
}

fn world_ingestion_decoded_composite_color(health: WorldObjectIngestionItem) -> [f32; 3] {
    let h = (f32::from(health.decoded_health_basis_points.unwrap_or(0)) / 10_000.0).clamp(0.0, 1.0);
    [0.92 - h * 0.52, 0.35 + h * 0.55, 0.86 - h * 0.62]
}

fn world_object_state_entity_count(item: WorldObjectIngestionItem) -> usize {
    match item.decoded_coarse_location_count.unwrap_or(0) {
        0..=1 => 1,
        2..=4 => 2,
        _ => 3,
    }
}

fn object_state_cluster_center(item: WorldObjectIngestionItem) -> [f32; 3] {
    let [base_x, base_z] = world_cluster_base(item.region_coords);
    let [cx, cy, cz] = item.decoded_coarse_first_xyz.unwrap_or([128, 128, 0]);
    let health =
        (f32::from(item.decoded_health_basis_points.unwrap_or(0)) / 10_000.0).clamp(0.0, 1.0);
    [
        base_x + ((f32::from(cx) / 255.0) - 0.5) * 0.95,
        0.34 + (f32::from(cz) / 255.0) * 0.82 + health * 0.26,
        base_z - 1.18 + ((f32::from(cy) / 255.0) - 0.5) * 0.95,
    ]
}

fn world_object_state_entity_body_transform(
    item: WorldObjectIngestionItem,
    variant: usize,
) -> Transform {
    let [center_x, center_y, center_z] = object_state_cluster_center(item);
    let health =
        (f32::from(item.decoded_health_basis_points.unwrap_or(0)) / 10_000.0).clamp(0.0, 1.0);
    let spread = match object_state_lifecycle_phase(item) {
        ObjectStateLifecyclePhase::Dormant => 0.75,
        ObjectStateLifecyclePhase::Warming => 0.92,
        ObjectStateLifecyclePhase::Active => 1.18,
        ObjectStateLifecyclePhase::Strained => 1.05,
    };
    let phase =
        ((item.decoded_coarse_updates.unwrap_or(0) % 32) as f32 / 32.0) * core::f32::consts::TAU;
    let (orbit_radius, orbit_angle, y_bias, scale_bias) = match variant {
        1 => (0.56 * spread, phase + 0.8, 0.05, -0.03),
        2 => (0.84 * spread, phase + 2.35, 0.10, -0.05),
        _ => (0.0, phase, 0.0, 0.0),
    };
    let x = center_x + orbit_radius * orbit_angle.cos();
    let z = center_z + orbit_radius * orbit_angle.sin();
    let y = center_y;
    let scale = (0.22 + health * 0.18 + scale_bias).clamp(0.18, 0.44);
    Transform {
        position: [x, y + y_bias, z],
        scale: [scale, scale * 1.25, scale],
    }
}

fn world_object_state_entity_body_color(
    item: WorldObjectIngestionItem,
    variant: usize,
) -> [f32; 3] {
    let health =
        (f32::from(item.decoded_health_basis_points.unwrap_or(0)) / 10_000.0).clamp(0.0, 1.0);
    match variant {
        1 => [
            0.28 + health * 0.26,
            0.24 + health * 0.44,
            0.58 + health * 0.20,
        ],
        2 => [
            0.42 + health * 0.22,
            0.30 + health * 0.34,
            0.30 + health * 0.18,
        ],
        _ => [
            0.24 + health * 0.32,
            0.30 + health * 0.58,
            0.36 + health * 0.22,
        ],
    }
}

fn world_object_state_entity_aura_transform(
    item: WorldObjectIngestionItem,
    variant: usize,
) -> Transform {
    let body = world_object_state_entity_body_transform(item, variant);
    let coarse_count = f32::from(item.decoded_coarse_location_count.unwrap_or(0));
    let lifecycle_boost = match object_state_lifecycle_phase(item) {
        ObjectStateLifecyclePhase::Dormant => 0.02,
        ObjectStateLifecyclePhase::Warming => 0.06,
        ObjectStateLifecyclePhase::Active => 0.12,
        ObjectStateLifecyclePhase::Strained => 0.08,
    };
    let aura_scale_bias = match variant {
        1 => 0.08,
        2 => 0.12,
        _ => 0.18,
    };
    let aura_scale =
        (body.scale[0] + aura_scale_bias + coarse_count * 0.01 + lifecycle_boost).clamp(0.28, 0.78);
    Transform {
        position: [body.position[0], body.position[1] + 0.42, body.position[2]],
        scale: [aura_scale, aura_scale, aura_scale],
    }
}

fn world_object_state_entity_aura_color(
    item: WorldObjectIngestionItem,
    variant: usize,
) -> [f32; 3] {
    let health =
        (f32::from(item.decoded_health_basis_points.unwrap_or(0)) / 10_000.0).clamp(0.0, 1.0);
    match variant {
        1 => [0.44 + health * 0.38, 0.36 + health * 0.48, 0.96],
        2 => [0.96, 0.54 + health * 0.28, 0.34 + health * 0.42],
        _ => [0.96, 0.42 + health * 0.44, 0.26 + health * 0.52],
    }
}

fn world_object_state_entity_cluster_core_transform(
    item: WorldObjectIngestionItem,
    entity_count: usize,
) -> Transform {
    let [center_x, center_y, center_z] = object_state_cluster_center(item);
    let phase_scale = match object_state_lifecycle_phase(item) {
        ObjectStateLifecyclePhase::Dormant => 0.0,
        ObjectStateLifecyclePhase::Warming => 0.03,
        ObjectStateLifecyclePhase::Active => 0.08,
        ObjectStateLifecyclePhase::Strained => 0.05,
    };
    let scale = (0.16 + entity_count as f32 * 0.06 + phase_scale).clamp(0.22, 0.50);
    Transform {
        position: [center_x, center_y + 0.92, center_z],
        scale: [scale, scale, scale],
    }
}

fn world_object_state_entity_cluster_core_color(
    item: WorldObjectIngestionItem,
    entity_count: usize,
) -> [f32; 3] {
    let health =
        (f32::from(item.decoded_health_basis_points.unwrap_or(0)) / 10_000.0).clamp(0.0, 1.0);
    let richness = (entity_count as f32 / 3.0).clamp(0.33, 1.0);
    [
        0.24 + richness * 0.46,
        0.52 + health * 0.38,
        0.94 - richness * 0.34,
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ObjectStateLifecyclePhase {
    Dormant,
    Warming,
    Active,
    Strained,
}

fn object_state_lifecycle_phase(item: WorldObjectIngestionItem) -> ObjectStateLifecyclePhase {
    let health = item.decoded_health_basis_points.unwrap_or(0);
    let coarse_updates = item.decoded_coarse_updates.unwrap_or(0);
    let health_updates = item.decoded_health_updates.unwrap_or(0);
    let viewer_time_updates = item.decoded_viewer_time_updates.unwrap_or(0);
    let activity = coarse_updates + health_updates + viewer_time_updates;
    if health < 2_800 {
        ObjectStateLifecyclePhase::Strained
    } else if activity >= 6 {
        ObjectStateLifecyclePhase::Active
    } else if activity >= 1 {
        ObjectStateLifecyclePhase::Warming
    } else {
        ObjectStateLifecyclePhase::Dormant
    }
}

fn world_object_state_entity_pulse_transform(item: WorldObjectIngestionItem) -> Transform {
    let core = world_object_state_entity_cluster_core_transform(
        item,
        world_object_state_entity_count(item),
    );
    let phase = object_state_lifecycle_phase(item);
    let phase_lift = match phase {
        ObjectStateLifecyclePhase::Dormant => 0.18,
        ObjectStateLifecyclePhase::Warming => 0.26,
        ObjectStateLifecyclePhase::Active => 0.34,
        ObjectStateLifecyclePhase::Strained => 0.30,
    };
    let cadence = ((item.decoded_coarse_updates.unwrap_or(0)
        + item.decoded_health_updates.unwrap_or(0))
        % 10) as f32
        / 10.0;
    let phase_scale = match phase {
        ObjectStateLifecyclePhase::Dormant => 0.06,
        ObjectStateLifecyclePhase::Warming => 0.12,
        ObjectStateLifecyclePhase::Active => 0.20,
        ObjectStateLifecyclePhase::Strained => 0.16,
    };
    let scale = (core.scale[0] + 0.08 + phase_scale + cadence * 0.14).clamp(0.24, 0.72);
    Transform {
        position: [
            core.position[0],
            core.position[1] + phase_lift,
            core.position[2],
        ],
        scale: [scale, scale, scale],
    }
}

fn world_object_state_entity_pulse_color(item: WorldObjectIngestionItem) -> [f32; 3] {
    match object_state_lifecycle_phase(item) {
        ObjectStateLifecyclePhase::Dormant => [0.48, 0.52, 0.66],
        ObjectStateLifecyclePhase::Warming => [0.62, 0.74, 0.96],
        ObjectStateLifecyclePhase::Active => [0.26, 0.92, 0.62],
        ObjectStateLifecyclePhase::Strained => [0.98, 0.42, 0.30],
    }
}

fn world_object_state_entity_stability_transform(item: WorldObjectIngestionItem) -> Transform {
    let [center_x, _, center_z] = object_state_cluster_center(item);
    let health_norm =
        (f32::from(item.decoded_health_basis_points.unwrap_or(0)) / 10_000.0).clamp(0.0, 1.0);
    let instability = 1.0 - health_norm;
    let phase = object_state_lifecycle_phase(item);
    let phase_instability = match phase {
        ObjectStateLifecyclePhase::Dormant => 0.08,
        ObjectStateLifecyclePhase::Warming => 0.0,
        ObjectStateLifecyclePhase::Active => -0.06,
        ObjectStateLifecyclePhase::Strained => 0.14,
    };
    let height = 0.16 + instability * 0.72;
    let width =
        0.10 + (item.decoded_coarse_location_count.unwrap_or(0) as f32 / 24.0).clamp(0.0, 0.24);
    let adjusted_height = (height + phase_instability).clamp(0.14, 0.86);
    Transform {
        position: [
            center_x + 0.26,
            0.12 + adjusted_height * 0.5,
            center_z + 0.18,
        ],
        scale: [width, adjusted_height, width],
    }
}

fn world_object_state_entity_stability_color(item: WorldObjectIngestionItem) -> [f32; 3] {
    let health_norm =
        (f32::from(item.decoded_health_basis_points.unwrap_or(0)) / 10_000.0).clamp(0.0, 1.0);
    [0.96 - health_norm * 0.56, 0.26 + health_norm * 0.58, 0.38]
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

    pub fn project_world_to_screen(
        &self,
        world: [f32; 3],
        surface_size: [f32; 2],
    ) -> Option<[f32; 2]> {
        let width = surface_size[0].max(1.0);
        let height = surface_size[1].max(1.0);
        let aspect = width / height;
        let projection = perspective_rh_zo(60.0_f32.to_radians(), aspect, 0.1, 100.0);
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let forward = [cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw];
        let view = look_to_rh(self.position, forward, [0.0, 1.0, 0.0]);
        let view_projection = mat4_mul(projection, view);
        let clip = mat4_mul_vec4(view_projection, [world[0], world[1], world[2], 1.0]);
        if clip[3] <= 0.0001 {
            return None;
        }
        let ndc_x = clip[0] / clip[3];
        let ndc_y = clip[1] / clip[3];
        let ndc_z = clip[2] / clip[3];
        if !(-1.0..=1.0).contains(&ndc_x) || !(-1.0..=1.0).contains(&ndc_y) || ndc_z < 0.0 {
            return None;
        }
        let screen_x = (ndc_x * 0.5 + 0.5) * width;
        let screen_y = (1.0 - (ndc_y * 0.5 + 0.5)) * height;
        Some([screen_x, screen_y])
    }
}

fn perspective_rh_zo(fovy_radians: f32, aspect: f32, znear: f32, zfar: f32) -> [[f32; 4]; 4] {
    let f = 1.0 / (0.5 * fovy_radians).tan();
    [
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, zfar / (znear - zfar), -1.0],
        [0.0, 0.0, (zfar * znear) / (znear - zfar), 0.0],
    ]
}

fn look_to_rh(eye: [f32; 3], direction: [f32; 3], up: [f32; 3]) -> [[f32; 4]; 4] {
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

fn mat4_mul(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0; 4]; 4];
    for c in 0..4 {
        for r in 0..4 {
            out[c][r] =
                a[0][r] * b[c][0] + a[1][r] * b[c][1] + a[2][r] * b[c][2] + a[3][r] * b[c][3];
        }
    }
    out
}

fn mat4_mul_vec4(m: [[f32; 4]; 4], v: [f32; 4]) -> [f32; 4] {
    [
        m[0][0] * v[0] + m[1][0] * v[1] + m[2][0] * v[2] + m[3][0] * v[3],
        m[0][1] * v[0] + m[1][1] * v[1] + m[2][1] * v[2] + m[3][1] * v[3],
        m[0][2] * v[0] + m[1][2] * v[1] + m[2][2] * v[2] + m[3][2] * v[3],
        m[0][3] * v[0] + m[1][3] * v[1] + m[2][3] * v[2] + m[3][3] * v[3],
    ]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (dot(v, v)).sqrt();
    if len > 0.0 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        [0.0, 0.0, -1.0]
    }
}

#[cfg(test)]
mod social_tests {
    use super::*;

    #[test]
    fn compute_p2p_session_id_xors_uuid_bytes() {
        let a = "00000000-0000-0000-0000-000000000001";
        let b = "00000000-0000-0000-0000-000000000003";
        let session = compute_p2p_session_id(a, b).expect("session id should be computed");
        assert_eq!(session, "00000000-0000-0000-0000-000000000002");
    }

    #[test]
    fn runtime_relay_state_is_bounded() {
        let mut relay = RuntimeRelayState {
            events: Vec::new(),
            max_events: 3,
        };
        for idx in 0..5 {
            relay.push(RuntimeRelayEvent {
                at_unix_ms: idx,
                level: RuntimeRelayLevel::Info,
                category: String::from("test"),
                message: idx.to_string(),
            });
        }
        assert_eq!(relay.events.len(), 3);
        assert_eq!(relay.events[0].message, "2");
        assert_eq!(relay.events[2].message, "4");
    }

    #[test]
    fn friend_name_resolution_sets_display_label_and_preserves_id() {
        let mut social = SocialState::default();
        social.upsert_friend(FriendEntry {
            id: String::from("11111111-2222-3333-4444-555555555555"),
            display_name: None,
            name_source: None,
            last_name_resolved_unix_ms: None,
            online: false,
            rights_has: 0,
            rights_given: 0,
            last_changed_unix_ms: 1,
        });
        social.resolve_friend_name(
            "11111111-2222-3333-4444-555555555555",
            "Alpha Resident",
            "im.from_name",
            25,
        );
        let friend = social
            .friends
            .iter()
            .find(|f| f.id == "11111111-2222-3333-4444-555555555555")
            .expect("friend should exist");
        assert_eq!(friend.display_name.as_deref(), Some("Alpha Resident"));
        assert_eq!(
            SocialState::friend_display_label(friend),
            "Alpha Resident (11111111)"
        );
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
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldIngestionDecodedHealthPayload)
        );
        assert!(scene.instances.iter().all(|instance| {
            instance.role != InstanceRole::WorldIngestionDecodedCompositeBeacon
        }));
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityBody)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityAura)
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
        assert_eq!(seam_proxy.transform.position[1], 0.28);
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
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldIngestionDecodedHealthPayload)
        );
        assert!(scene.instances.iter().all(|instance| {
            instance.role != InstanceRole::WorldIngestionDecodedCompositeBeacon
        }));
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityBody)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityAura)
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
        assert_eq!(seam_proxy.transform.position[1], 0.36);
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
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldIngestionDecodedHealthPayload)
        );
        assert!(scene.instances.iter().all(|instance| {
            instance.role != InstanceRole::WorldIngestionDecodedCompositeBeacon
        }));
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityBody)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityAura)
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
            decoded_coarse_updates: 1,
            decoded_coarse_location_count: Some(2),
            decoded_coarse_first_x: Some(64),
            decoded_coarse_first_y: Some(32),
            decoded_coarse_first_z: Some(1),
            decoded_coarse_second_x: None,
            decoded_coarse_second_y: None,
            decoded_coarse_second_z: None,
            decoded_coarse_third_x: None,
            decoded_coarse_third_y: None,
            decoded_coarse_third_z: None,
            decoded_health_updates: 1,
            decoded_health_last_basis_points: Some(6700),
            decoded_viewer_time_updates: 0,
            decoded_viewer_time_body_len: None,
            decoded_viewer_time_signature: None,
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
        let decoded_health_payload = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldIngestionDecodedHealthPayload)
            .expect("decoded health payload marker should exist");
        assert_eq!(decoded_health_payload.mesh, MeshKind::Cube);
        assert!(decoded_health_payload.transform.position[1] > 0.25);
        let decoded_coarse_payload = scene
            .instances
            .iter()
            .find(|instance| {
                instance.role == InstanceRole::WorldIngestionDecodedCoarseLocationPayload
            })
            .expect("decoded coarse payload marker should exist");
        assert_eq!(decoded_coarse_payload.mesh, MeshKind::AxisMarker);
        let composite = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldIngestionDecodedCompositeBeacon)
            .expect("decoded composite beacon should exist");
        assert_eq!(composite.mesh, MeshKind::Cube);
        assert!(composite.transform.position[1] > 0.3);
        let entity_body = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldObjectStateEntityBody)
            .expect("object-state entity body should exist");
        assert_eq!(entity_body.mesh, MeshKind::Cube);
        let entity_aura = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldObjectStateEntityAura)
            .expect("object-state entity aura should exist");
        assert_eq!(entity_aura.mesh, MeshKind::AxisMarker);
        assert!(entity_aura.transform.position[1] > entity_body.transform.position[1]);
        let wing_body = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldObjectStateEntityWingBody)
            .expect("object-state wing body should exist");
        assert_eq!(wing_body.mesh, MeshKind::Cube);
        let wing_aura = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldObjectStateEntityWingAura)
            .expect("object-state wing aura should exist");
        assert_eq!(wing_aura.mesh, MeshKind::AxisMarker);
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityGuardBody)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityGuardAura)
        );
        let cluster_core = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldObjectStateEntityClusterCore)
            .expect("object-state cluster core should exist");
        assert_eq!(cluster_core.mesh, MeshKind::AxisMarker);
        assert!(cluster_core.transform.position[1] > entity_aura.transform.position[1]);
        let pulse = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldObjectStateEntityPulse)
            .expect("object-state pulse should exist");
        assert_eq!(pulse.mesh, MeshKind::AxisMarker);
        let stability = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldObjectStateEntityStability)
            .expect("object-state stability should exist");
        assert_eq!(stability.mesh, MeshKind::Cube);
        assert!(stability.transform.scale[1] > stability.transform.scale[0]);
    }

    #[test]
    fn first_region_presence_maps_snapshot_stages() {
        let offline = FirstRegionPresence::from_live_snapshot(None);
        assert_eq!(offline.stage, WorldEntryStage::Offline);
        assert!(!offline.has_sim_endpoint);
        assert_eq!(offline.region_coords, None);

        let connected =
            FirstRegionPresence::from_live_snapshot(Some(&sample_snapshot(true, false)));
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
            decoded_coarse_first_z: Some(0),
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
            decoded_coarse_first_z: Some(0),
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

        let connected =
            WorldObjectIngestionSeam::from_live_snapshot(Some(&sample_snapshot(true, false)));
        assert_eq!(connected.items.len(), 1);
        let first = connected.items[0];
        assert_eq!(
            first.lane,
            WorldObjectIngestionLane::FirstRegionPresenceProxy
        );
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
            decoded_coarse_first_z: Some(0),
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
            observed_at_unix_ms: 9,
        };
        let seam = WorldObjectIngestionSeam::from_live_snapshot(Some(&snapshot));
        assert!(
            seam.items
                .iter()
                .any(|item| item.lane == WorldObjectIngestionLane::FirstRegionPresenceProxy)
        );
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
            decoded_coarse_first_z: Some(0),
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
            decoded_coarse_first_z: Some(0),
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
            observed_at_unix_ms: 9,
        };
        let seam = WorldObjectIngestionSeam::from_live_snapshot(Some(&snapshot));
        assert!(
            seam.items
                .iter()
                .all(|item| item.lane != WorldObjectIngestionLane::DecodedSimulatorEndpointPayload)
        );
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
    fn world_object_ingestion_seam_includes_coarse_neighborhood_payload_when_second_sample_present()
    {
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
            decoded_coarse_updates: 2,
            decoded_coarse_location_count: Some(3),
            decoded_coarse_first_x: Some(64),
            decoded_coarse_first_y: Some(32),
            decoded_coarse_first_z: Some(12),
            decoded_coarse_second_x: Some(96),
            decoded_coarse_second_y: Some(56),
            decoded_coarse_second_z: Some(14),
            decoded_coarse_third_x: Some(110),
            decoded_coarse_third_y: Some(62),
            decoded_coarse_third_z: Some(18),
            decoded_health_updates: 0,
            decoded_health_last_basis_points: None,
            decoded_viewer_time_updates: 0,
            decoded_viewer_time_body_len: None,
            decoded_viewer_time_signature: None,
            observed_at_unix_ms: 9,
        };
        let seam = WorldObjectIngestionSeam::from_live_snapshot(Some(&snapshot));
        let neighborhood = seam
            .items
            .iter()
            .find(|item| item.lane == WorldObjectIngestionLane::DecodedCoarseNeighborhoodPayload)
            .expect("decoded coarse neighborhood payload should exist");
        assert_eq!(neighborhood.decoded_coarse_second_xyz, Some([96, 56, 14]));
        assert_eq!(neighborhood.decoded_coarse_third_xyz, Some([110, 62, 18]));
    }

    #[test]
    fn world_object_ingestion_seam_omits_coarse_neighborhood_payload_without_second_sample() {
        let mut snapshot = sample_snapshot(true, true);
        snapshot.decoded_coarse_location_count = Some(3);
        snapshot.decoded_coarse_first_x = Some(64);
        snapshot.decoded_coarse_first_y = Some(32);
        snapshot.decoded_coarse_first_z = Some(12);
        snapshot.decoded_coarse_second_x = None;
        snapshot.decoded_coarse_second_y = None;
        snapshot.decoded_coarse_second_z = None;
        snapshot.decoded_coarse_third_x = Some(110);
        snapshot.decoded_coarse_third_y = Some(62);
        snapshot.decoded_coarse_third_z = Some(18);

        let seam = WorldObjectIngestionSeam::from_live_snapshot(Some(&snapshot));
        assert!(seam
            .items
            .iter()
            .all(|item| item.lane != WorldObjectIngestionLane::DecodedCoarseNeighborhoodPayload));
    }

    #[test]
    fn world_object_ingestion_seam_includes_decoded_health_payload_when_present() {
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
            decoded_coarse_second_x: None,
            decoded_coarse_second_y: None,
            decoded_coarse_second_z: None,
            decoded_coarse_third_x: None,
            decoded_coarse_third_y: None,
            decoded_coarse_third_z: None,
            decoded_health_updates: 2,
            decoded_health_last_basis_points: Some(6200),
            decoded_viewer_time_updates: 0,
            decoded_viewer_time_body_len: None,
            decoded_viewer_time_signature: None,
            observed_at_unix_ms: 9,
        };
        let seam = WorldObjectIngestionSeam::from_live_snapshot(Some(&snapshot));
        let decoded = seam
            .items
            .iter()
            .find(|item| item.lane == WorldObjectIngestionLane::DecodedHealthPayload)
            .expect("decoded health payload should exist");
        assert_eq!(decoded.decoded_health_basis_points, Some(6200));
        let entity_seed = seam
            .items
            .iter()
            .find(|item| item.lane == WorldObjectIngestionLane::ObjectStateEntitySeedPayload)
            .expect("object-state entity seed payload should exist when coarse+health are present");
        assert_eq!(entity_seed.decoded_health_basis_points, Some(6200));
        assert_eq!(entity_seed.decoded_coarse_location_count, Some(2));
        assert_eq!(entity_seed.decoded_coarse_first_xyz, Some([64, 32, 12]));
        assert_eq!(entity_seed.decoded_coarse_updates, Some(1));
        assert_eq!(entity_seed.decoded_health_updates, Some(2));
        let lifecycle = seam
            .items
            .iter()
            .find(|item| item.lane == WorldObjectIngestionLane::ObjectStateEntityLifecyclePayload)
            .expect("object-state lifecycle payload should exist when coarse+health are present");
        assert_eq!(lifecycle.decoded_health_updates, Some(2));
        assert_eq!(lifecycle.decoded_coarse_updates, Some(1));
    }

    #[test]
    fn world_object_ingestion_seam_includes_decoded_viewer_time_payload_when_present() {
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
            decoded_coarse_updates: 2,
            decoded_coarse_location_count: Some(3),
            decoded_coarse_first_x: Some(80),
            decoded_coarse_first_y: Some(48),
            decoded_coarse_first_z: Some(1),
            decoded_coarse_second_x: None,
            decoded_coarse_second_y: None,
            decoded_coarse_second_z: None,
            decoded_coarse_third_x: None,
            decoded_coarse_third_y: None,
            decoded_coarse_third_z: None,
            decoded_health_updates: 1,
            decoded_health_last_basis_points: Some(6500),
            decoded_viewer_time_updates: 5,
            decoded_viewer_time_body_len: Some(28),
            decoded_viewer_time_signature: Some(0xDDCCBBAA),
            observed_at_unix_ms: 9,
        };
        let seam = WorldObjectIngestionSeam::from_live_snapshot(Some(&snapshot));
        let decoded = seam
            .items
            .iter()
            .find(|item| item.lane == WorldObjectIngestionLane::DecodedViewerTimePayload)
            .expect("decoded viewer-time payload should exist");
        assert_eq!(decoded.decoded_viewer_time_updates, Some(5));
        assert_eq!(decoded.decoded_viewer_time_body_len, Some(28));
        assert_eq!(decoded.decoded_viewer_time_signature, Some(0xDDCCBBAA));

        let lifecycle = seam
            .items
            .iter()
            .find(|item| item.lane == WorldObjectIngestionLane::ObjectStateEntityLifecyclePayload)
            .expect("object-state lifecycle payload should exist when coarse+health are present");
        assert_eq!(lifecycle.decoded_viewer_time_updates, Some(5));
        assert_eq!(lifecycle.decoded_viewer_time_body_len, Some(28));
        assert_eq!(lifecycle.decoded_viewer_time_signature, Some(0xDDCCBBAA));
    }

    #[test]
    fn world_object_ingestion_seam_omits_decoded_viewer_time_payload_without_body_len() {
        let mut snapshot = sample_snapshot(true, true);
        snapshot.decoded_viewer_time_updates = 3;
        snapshot.decoded_viewer_time_body_len = None;
        snapshot.decoded_viewer_time_signature = Some(0x11223344);
        let seam = WorldObjectIngestionSeam::from_live_snapshot(Some(&snapshot));
        assert!(
            seam.items
                .iter()
                .all(|item| item.lane != WorldObjectIngestionLane::DecodedViewerTimePayload)
        );
    }

    #[test]
    fn scene_applies_and_removes_decoded_viewer_time_payload_role() {
        let mut scene = Scene::prototype();
        let mut snapshot = sample_snapshot(true, true);
        snapshot.decoded_coarse_location_count = Some(4);
        snapshot.decoded_coarse_first_x = Some(64);
        snapshot.decoded_coarse_first_y = Some(32);
        snapshot.decoded_coarse_first_z = Some(12);
        snapshot.decoded_health_last_basis_points = Some(7000);
        snapshot.decoded_viewer_time_updates = 2;
        snapshot.decoded_viewer_time_body_len = Some(22);
        snapshot.decoded_viewer_time_signature = Some(0x01020304);
        apply_scene_from_snapshot(&mut scene, Some(&snapshot));
        assert!(scene.instances.iter().any(|instance| {
            instance.role == InstanceRole::WorldIngestionDecodedViewerTimePayload
        }));

        snapshot.decoded_viewer_time_body_len = None;
        snapshot.decoded_viewer_time_signature = None;
        apply_scene_from_snapshot(&mut scene, Some(&snapshot));
        assert!(scene.instances.iter().all(|instance| {
            instance.role != InstanceRole::WorldIngestionDecodedViewerTimePayload
        }));
    }

    #[test]
    fn scene_applies_and_removes_decoded_coarse_neighborhood_payload_role() {
        let mut scene = Scene::prototype();
        let mut snapshot = sample_snapshot(true, true);
        snapshot.decoded_coarse_location_count = Some(4);
        snapshot.decoded_coarse_first_x = Some(64);
        snapshot.decoded_coarse_first_y = Some(32);
        snapshot.decoded_coarse_first_z = Some(12);
        snapshot.decoded_coarse_second_x = Some(92);
        snapshot.decoded_coarse_second_y = Some(48);
        snapshot.decoded_coarse_second_z = Some(20);
        snapshot.decoded_coarse_third_x = Some(106);
        snapshot.decoded_coarse_third_y = Some(60);
        snapshot.decoded_coarse_third_z = Some(22);
        apply_scene_from_snapshot(&mut scene, Some(&snapshot));
        assert!(scene.instances.iter().any(|instance| {
            instance.role == InstanceRole::WorldIngestionDecodedCoarseNeighborhoodPayload
        }));
        assert!(scene.instances.iter().any(|instance| {
            instance.role == InstanceRole::WorldIngestionDecodedCoarseNeighborhoodSatelliteA
        }));
        assert!(scene.instances.iter().any(|instance| {
            instance.role == InstanceRole::WorldIngestionDecodedCoarseNeighborhoodSatelliteB
        }));

        snapshot.decoded_coarse_second_x = None;
        snapshot.decoded_coarse_second_y = None;
        snapshot.decoded_coarse_second_z = None;
        snapshot.decoded_coarse_third_x = None;
        snapshot.decoded_coarse_third_y = None;
        snapshot.decoded_coarse_third_z = None;
        apply_scene_from_snapshot(&mut scene, Some(&snapshot));
        assert!(scene.instances.iter().all(|instance| {
            instance.role != InstanceRole::WorldIngestionDecodedCoarseNeighborhoodPayload
        }));
        assert!(scene.instances.iter().all(|instance| {
            instance.role != InstanceRole::WorldIngestionDecodedCoarseNeighborhoodSatelliteA
        }));
        assert!(scene.instances.iter().all(|instance| {
            instance.role != InstanceRole::WorldIngestionDecodedCoarseNeighborhoodSatelliteB
        }));
        assert!(scene.instances.iter().all(|instance| {
            instance.role != InstanceRole::WorldIngestionDecodedCoarseNeighborhoodSatelliteA
        }));
        assert!(scene.instances.iter().all(|instance| {
            instance.role != InstanceRole::WorldIngestionDecodedCoarseNeighborhoodSatelliteB
        }));
    }

    #[test]
    fn coarse_neighborhood_family_positions_are_structured() {
        let mut scene = Scene::prototype();
        let mut snapshot = sample_snapshot(true, true);
        snapshot.first_sim_region_x = Some(1024);
        snapshot.first_sim_region_y = Some(2048);
        snapshot.decoded_coarse_location_count = Some(5);
        snapshot.decoded_coarse_first_x = Some(64);
        snapshot.decoded_coarse_first_y = Some(32);
        snapshot.decoded_coarse_first_z = Some(12);
        snapshot.decoded_coarse_second_x = Some(92);
        snapshot.decoded_coarse_second_y = Some(48);
        snapshot.decoded_coarse_second_z = Some(20);
        snapshot.decoded_coarse_third_x = Some(108);
        snapshot.decoded_coarse_third_y = Some(62);
        snapshot.decoded_coarse_third_z = Some(24);
        apply_scene_from_snapshot(&mut scene, Some(&snapshot));

        let hub = scene
            .instances
            .iter()
            .find(|instance| {
                instance.role == InstanceRole::WorldIngestionDecodedCoarseNeighborhoodPayload
            })
            .expect("coarse neighborhood hub should exist");
        let sat_a = scene
            .instances
            .iter()
            .find(|instance| {
                instance.role == InstanceRole::WorldIngestionDecodedCoarseNeighborhoodSatelliteA
            })
            .expect("coarse neighborhood satellite A should exist");
        let sat_b = scene
            .instances
            .iter()
            .find(|instance| {
                instance.role == InstanceRole::WorldIngestionDecodedCoarseNeighborhoodSatelliteB
            })
            .expect("coarse neighborhood satellite B should exist");

        assert!(hub.transform.position[0] > 0.0);
        assert!(sat_a.transform.position[0] > hub.transform.position[0]);
        assert!(sat_b.transform.position[2] > hub.transform.position[2]);
    }

    #[test]
    fn scene_composite_beacon_requires_both_decoded_inputs() {
        let mut scene = Scene::prototype();
        let coarse_only = LiveVisualSnapshot {
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
            decoded_coarse_updates: 1,
            decoded_coarse_location_count: Some(2),
            decoded_coarse_first_x: Some(64),
            decoded_coarse_first_y: Some(32),
            decoded_coarse_first_z: Some(0),
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
            observed_at_unix_ms: 9,
        };
        apply_scene_from_snapshot(&mut scene, Some(&coarse_only));
        assert!(scene.instances.iter().all(|instance| {
            instance.role != InstanceRole::WorldIngestionDecodedCompositeBeacon
        }));
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityBody)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityAura)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityWingBody)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityWingAura)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityGuardBody)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityGuardAura)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityClusterCore)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityPulse)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityStability)
        );

        let mut health_only = coarse_only.clone();
        health_only.decoded_coarse_location_count = None;
        health_only.decoded_coarse_first_x = None;
        health_only.decoded_coarse_first_y = None;
        health_only.decoded_coarse_first_z = None;
        health_only.decoded_health_updates = 1;
        health_only.decoded_health_last_basis_points = Some(6200);
        apply_scene_from_snapshot(&mut scene, Some(&health_only));
        assert!(scene.instances.iter().all(|instance| {
            instance.role != InstanceRole::WorldIngestionDecodedCompositeBeacon
        }));
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityBody)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityAura)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityWingBody)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityWingAura)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityGuardBody)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityGuardAura)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityClusterCore)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityPulse)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityStability)
        );
    }

    #[test]
    fn scene_object_state_entity_family_count_is_gated_by_coarse_count() {
        let mut scene = Scene::prototype();
        let mut snapshot = sample_snapshot(true, true);
        snapshot.decoded_coarse_location_count = Some(1);
        snapshot.decoded_coarse_first_x = Some(64);
        snapshot.decoded_coarse_first_y = Some(32);
        snapshot.decoded_coarse_first_z = Some(12);
        snapshot.decoded_coarse_updates = 3;
        snapshot.decoded_health_last_basis_points = Some(7000);
        apply_scene_from_snapshot(&mut scene, Some(&snapshot));
        assert!(
            scene
                .instances
                .iter()
                .any(|instance| instance.role == InstanceRole::WorldObjectStateEntityBody)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityWingBody)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityGuardBody)
        );
        assert!(
            scene
                .instances
                .iter()
                .any(|instance| instance.role == InstanceRole::WorldObjectStateEntityClusterCore)
        );
        assert!(
            scene
                .instances
                .iter()
                .any(|instance| instance.role == InstanceRole::WorldObjectStateEntityPulse)
        );
        assert!(
            scene
                .instances
                .iter()
                .any(|instance| instance.role == InstanceRole::WorldObjectStateEntityStability)
        );

        snapshot.decoded_coarse_location_count = Some(3);
        snapshot.decoded_coarse_updates = 4;
        apply_scene_from_snapshot(&mut scene, Some(&snapshot));
        assert!(
            scene
                .instances
                .iter()
                .any(|instance| instance.role == InstanceRole::WorldObjectStateEntityWingBody)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityGuardBody)
        );

        snapshot.decoded_coarse_location_count = Some(7);
        snapshot.decoded_coarse_updates = 5;
        apply_scene_from_snapshot(&mut scene, Some(&snapshot));
        assert!(
            scene
                .instances
                .iter()
                .any(|instance| instance.role == InstanceRole::WorldObjectStateEntityWingBody)
        );
        assert!(
            scene
                .instances
                .iter()
                .any(|instance| instance.role == InstanceRole::WorldObjectStateEntityGuardBody)
        );
    }

    #[test]
    fn object_state_lifecycle_phase_tracks_update_and_health_signals() {
        let base = WorldObjectIngestionItem {
            lane: WorldObjectIngestionLane::ObjectStateEntityLifecyclePayload,
            stage: WorldEntryStage::EnteredFirstRegion,
            region_coords: Some([1024, 2048]),
            simulator_target_present: true,
            traffic_broader_count: 0,
            traffic_unknown_count: 0,
            traffic_region_control_count: 0,
            decoded_endpoint_port: None,
            decoded_endpoint_host_tail: None,
            decoded_coarse_location_count: Some(4),
            decoded_coarse_first_xyz: Some([64, 32, 12]),
            decoded_coarse_second_xyz: None,
            decoded_coarse_third_xyz: None,
            decoded_coarse_updates: Some(0),
            decoded_health_updates: Some(0),
            decoded_health_basis_points: Some(6400),
            decoded_viewer_time_updates: Some(0),
            decoded_viewer_time_body_len: None,
            decoded_viewer_time_signature: None,
        };
        assert_eq!(
            object_state_lifecycle_phase(base),
            ObjectStateLifecyclePhase::Dormant
        );
        assert_eq!(
            object_state_lifecycle_phase(WorldObjectIngestionItem {
                decoded_coarse_updates: Some(1),
                ..base
            }),
            ObjectStateLifecyclePhase::Warming
        );
        assert_eq!(
            object_state_lifecycle_phase(WorldObjectIngestionItem {
                decoded_coarse_updates: Some(3),
                decoded_health_updates: Some(2),
                decoded_viewer_time_updates: Some(1),
                ..base
            }),
            ObjectStateLifecyclePhase::Active
        );
        assert_eq!(
            object_state_lifecycle_phase(WorldObjectIngestionItem {
                decoded_health_basis_points: Some(2200),
                decoded_coarse_updates: Some(6),
                decoded_health_updates: Some(4),
                ..base
            }),
            ObjectStateLifecyclePhase::Strained
        );
    }

    #[test]
    fn scene_world_cluster_hierarchy_is_spatially_coherent() {
        let mut scene = Scene::prototype();
        let mut snapshot = sample_snapshot(true, true);
        snapshot.first_sim_endpoint = Some(String::from("198.51.100.42:13009"));
        snapshot.first_sim_region_x = Some(1024);
        snapshot.first_sim_region_y = Some(2048);
        snapshot.traffic_summary_available = true;
        snapshot.likely_broader_traffic = 6;
        snapshot.unknown = 2;
        snapshot.decoded_coarse_location_count = Some(5);
        snapshot.decoded_coarse_first_x = Some(80);
        snapshot.decoded_coarse_first_y = Some(64);
        snapshot.decoded_coarse_first_z = Some(20);
        snapshot.decoded_coarse_updates = 4;
        snapshot.decoded_health_updates = 3;
        snapshot.decoded_health_last_basis_points = Some(7300);
        apply_scene_from_snapshot(&mut scene, Some(&snapshot));

        let anchor = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldRegionAnchor)
            .expect("region anchor should exist");
        let body = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldObjectStateEntityBody)
            .expect("entity body should exist");
        let core = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldObjectStateEntityClusterCore)
            .expect("cluster core should exist");
        let pulse = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldObjectStateEntityPulse)
            .expect("pulse should exist");
        let stability = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldObjectStateEntityStability)
            .expect("stability should exist");
        let traffic = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldIngestionTrafficPayload)
            .expect("traffic payload should exist");

        assert!(body.transform.position[2] < anchor.transform.position[2]);
        assert!(core.transform.position[1] > body.transform.position[1]);
        assert!(pulse.transform.position[1] > core.transform.position[1]);
        assert!(stability.transform.position[1] < body.transform.position[1]);
        assert!(traffic.transform.position[0] > body.transform.position[0]);
    }

    #[test]
    fn lifecycle_progression_changes_pulse_and_satellite_spacing() {
        let mut scene = Scene::prototype();
        let mut snapshot = sample_snapshot(true, true);
        snapshot.first_sim_endpoint = Some(String::from("198.51.100.42:13009"));
        snapshot.first_sim_region_x = Some(1024);
        snapshot.first_sim_region_y = Some(2048);
        snapshot.decoded_coarse_location_count = Some(6);
        snapshot.decoded_coarse_first_x = Some(96);
        snapshot.decoded_coarse_first_y = Some(72);
        snapshot.decoded_coarse_first_z = Some(18);
        snapshot.decoded_health_last_basis_points = Some(7200);

        snapshot.decoded_coarse_updates = 0;
        snapshot.decoded_health_updates = 0;
        apply_scene_from_snapshot(&mut scene, Some(&snapshot));
        let dormant_pulse = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldObjectStateEntityPulse)
            .expect("dormant pulse should exist");
        let dormant_body = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldObjectStateEntityBody)
            .expect("dormant body should exist");
        let dormant_wing = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldObjectStateEntityWingBody)
            .expect("dormant wing should exist");
        let dormant_spacing =
            (dormant_wing.transform.position[0] - dormant_body.transform.position[0]).abs()
                + (dormant_wing.transform.position[2] - dormant_body.transform.position[2]).abs();
        let dormant_pulse_scale = dormant_pulse.transform.scale[0];

        snapshot.decoded_coarse_updates = 6;
        snapshot.decoded_health_updates = 4;
        apply_scene_from_snapshot(&mut scene, Some(&snapshot));
        let active_pulse = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldObjectStateEntityPulse)
            .expect("active pulse should exist");
        let active_body = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldObjectStateEntityBody)
            .expect("active body should exist");
        let active_wing = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldObjectStateEntityWingBody)
            .expect("active wing should exist");
        let active_spacing =
            (active_wing.transform.position[0] - active_body.transform.position[0]).abs()
                + (active_wing.transform.position[2] - active_body.transform.position[2]).abs();

        assert!(active_pulse.transform.scale[0] > dormant_pulse_scale);
        assert!(active_spacing > dormant_spacing);
    }

    #[test]
    fn viewer_time_updates_drive_lifecycle_visual_progression() {
        let mut scene = Scene::prototype();
        let mut snapshot = sample_snapshot(true, true);
        snapshot.first_sim_endpoint = Some(String::from("198.51.100.42:13009"));
        snapshot.first_sim_region_x = Some(1024);
        snapshot.first_sim_region_y = Some(2048);
        snapshot.decoded_coarse_location_count = Some(4);
        snapshot.decoded_coarse_first_x = Some(64);
        snapshot.decoded_coarse_first_y = Some(32);
        snapshot.decoded_coarse_first_z = Some(12);
        snapshot.decoded_health_last_basis_points = Some(6800);
        snapshot.decoded_coarse_updates = 0;
        snapshot.decoded_health_updates = 0;
        snapshot.decoded_viewer_time_updates = 0;
        snapshot.decoded_viewer_time_body_len = None;
        snapshot.decoded_viewer_time_signature = None;
        apply_scene_from_snapshot(&mut scene, Some(&snapshot));
        let dormant_pulse = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldObjectStateEntityPulse)
            .expect("dormant pulse should exist");
        let dormant_pulse_scale = dormant_pulse.transform.scale[0];

        snapshot.decoded_viewer_time_updates = 4;
        snapshot.decoded_viewer_time_body_len = Some(24);
        snapshot.decoded_viewer_time_signature = Some(0xAABBCCDD);
        apply_scene_from_snapshot(&mut scene, Some(&snapshot));
        let active_pulse = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldObjectStateEntityPulse)
            .expect("active pulse should exist");
        let viewer_time_marker = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldIngestionDecodedViewerTimePayload)
            .expect("viewer-time payload marker should exist");

        assert!(active_pulse.transform.scale[0] > dormant_pulse_scale);
        assert!(viewer_time_marker.transform.scale[0] > 0.0);
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
            decoded_coarse_first_z: Some(0),
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
        assert!(scene.instances.iter().all(|instance| {
            instance.role != InstanceRole::WorldIngestionDecodedCoarseLocationPayload
        }));
        assert!(scene.instances.iter().all(|instance| {
            instance.role != InstanceRole::WorldIngestionDecodedCoarseNeighborhoodPayload
        }));
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldIngestionDecodedHealthPayload)
        );
        assert!(scene.instances.iter().all(|instance| {
            instance.role != InstanceRole::WorldIngestionDecodedViewerTimePayload
        }));
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityBody)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityAura)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityWingBody)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityWingAura)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityGuardBody)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityGuardAura)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityClusterCore)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityPulse)
        );
        assert!(
            scene
                .instances
                .iter()
                .all(|instance| instance.role != InstanceRole::WorldObjectStateEntityStability)
        );
    }

    #[test]
    fn chat_state_dedupes_messages_by_id() {
        let mut chat = ChatState::default();
        chat.push_message(ChatMessage {
            id: 42,
            observed_at_unix_ms: 1,
            sender: String::from("A"),
            text: String::from("hello"),
            source: String::from("event_queue"),
        });
        chat.push_message(ChatMessage {
            id: 42,
            observed_at_unix_ms: 2,
            sender: String::from("B"),
            text: String::from("duplicate"),
            source: String::from("event_queue"),
        });
        assert_eq!(chat.messages.len(), 1);
        assert_eq!(chat.messages[0].text, "hello");
    }

    #[test]
    fn scene_uses_avatar_proxy_mesh_in_proxy_mode() {
        let mut scene = Scene::prototype();
        let avatars = vec![
            WorldAvatarPlaceholder {
                agent_id: String::from("self-id"),
                world_position: [1.0, 0.0, 2.0],
                local_position: Some([1, 2, 3]),
                sim_name: Some(String::from("TestSim")),
                display_name: String::from("Self"),
                is_self: true,
                last_update_unix_ms: 10,
                stale: false,
            },
            WorldAvatarPlaceholder {
                agent_id: String::from("other-id"),
                world_position: [2.0, 0.0, 3.0],
                local_position: Some([2, 3, 4]),
                sim_name: Some(String::from("TestSim")),
                display_name: String::from("Other"),
                is_self: false,
                last_update_unix_ms: 10,
                stale: true,
            },
        ];

        scene.apply_world_avatar_placeholders(&avatars, AvatarRenderMode::Proxy);

        let self_avatar = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldAvatarPlaceholderSelf)
            .expect("self avatar should be present");
        assert_eq!(self_avatar.mesh, MeshKind::AvatarProxy);
        assert_eq!(self_avatar.color, [0.22, 0.86, 0.40]);
        assert_eq!(self_avatar.transform.scale, [0.38, 1.30, 0.38]);

        let other_avatar = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldAvatarPlaceholderOther)
            .expect("other avatar should be present");
        assert_eq!(other_avatar.mesh, MeshKind::AvatarProxy);
        assert_eq!(other_avatar.color, [0.66, 0.50, 0.38]);
        assert_eq!(other_avatar.transform.scale, [0.34, 1.16, 0.34]);
    }

    #[test]
    fn scene_falls_back_to_box_mesh_in_fallback_mode() {
        let mut scene = Scene::prototype();
        let avatars = vec![WorldAvatarPlaceholder {
            agent_id: String::from("other-id"),
            world_position: [2.0, 0.0, 3.0],
            local_position: Some([2, 3, 4]),
            sim_name: Some(String::from("TestSim")),
            display_name: String::from("Other"),
            is_self: false,
            last_update_unix_ms: 10,
            stale: false,
        }];

        scene.apply_world_avatar_placeholders(&avatars, AvatarRenderMode::FallbackBox);

        let avatar = scene
            .instances
            .iter()
            .find(|instance| instance.role == InstanceRole::WorldAvatarPlaceholderOther)
            .expect("other avatar should be present");
        assert_eq!(avatar.mesh, MeshKind::Cube);
        assert_eq!(avatar.color, [0.30, 0.74, 0.98]);
        assert_eq!(avatar.transform.scale, [0.30, 1.10, 0.30]);
    }

    #[test]
    fn chat_state_send_status_progresses() {
        let mut chat = ChatState::default();
        assert_eq!(chat.send_status, ChatSendStatus::Idle);
        chat.mark_sending();
        assert_eq!(chat.send_status, ChatSendStatus::Sending);
        chat.mark_send_failed("network");
        assert_eq!(
            chat.send_status,
            ChatSendStatus::Failed(String::from("network"))
        );
        chat.mark_sent();
        assert_eq!(chat.send_status, ChatSendStatus::Sent);
        chat.reset_send_status();
        assert_eq!(chat.send_status, ChatSendStatus::Idle);
    }

    #[test]
    fn social_state_tracks_unread_and_mark_read() {
        let mut social = SocialState::default();
        social.upsert_thread_message(
            DirectImMessage {
                id: 1,
                session_id: String::from("session-a"),
                peer_id: String::from("friend-a"),
                from_id: String::from("friend-a"),
                from_name: String::from("Friend A"),
                text: String::from("hello"),
                observed_at_unix_ms: 10,
                outgoing: false,
            },
            "friend-a",
        );
        let thread = social
            .thread_for_participant("friend-a")
            .expect("thread should exist");
        assert_eq!(thread.unread_count, 1);
        assert_eq!(thread.last_activity_unix_ms, 10);

        social.mark_thread_read_by_participant("friend-a", 20);
        let thread = social
            .thread_for_participant("friend-a")
            .expect("thread should exist");
        assert_eq!(thread.unread_count, 0);
        assert_eq!(thread.last_read_unix_ms, Some(20));
    }

    #[test]
    fn social_state_sorts_thread_participants_by_recent_activity() {
        let mut social = SocialState::default();
        social.upsert_thread_message(
            DirectImMessage {
                id: 1,
                session_id: String::from("session-a"),
                peer_id: String::from("friend-a"),
                from_id: String::from("friend-a"),
                from_name: String::from("Friend A"),
                text: String::from("older"),
                observed_at_unix_ms: 10,
                outgoing: false,
            },
            "friend-a",
        );
        social.upsert_thread_message(
            DirectImMessage {
                id: 2,
                session_id: String::from("session-b"),
                peer_id: String::from("friend-b"),
                from_id: String::from("friend-b"),
                from_name: String::from("Friend B"),
                text: String::from("newer"),
                observed_at_unix_ms: 50,
                outgoing: false,
            },
            "friend-b",
        );
        let sorted = social.sorted_thread_participants_by_recent();
        assert_eq!(sorted, vec![String::from("friend-b"), String::from("friend-a")]);
    }
}
