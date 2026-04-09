use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use reqwest::{
    Method, StatusCode,
    header::{ACCEPT, CONTENT_TYPE},
};
use roxmltree::{Document, Node};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::net::UdpSocket;
use tokio::time::timeout;
use viewer_core::{HandoffPhase, RegionContinuitySummary};
use viewer_grid::legacy_login::{
    classify_legacy_login_name, normalize_legacy_passwd, split_legacy_name,
};
use viewer_grid::{
    FriendBootstrapEntry, GridAdapterError, GridLoginAdapter, GridLoginRequest, GridLoginResponse,
    GridLoginResult, LoginIntent, SessionBootstrap,
};

mod asset_fetch;
mod cookie_utils;
mod login_codec_utils;
mod object_decode_utils;
pub use crate::asset_fetch::{
    fetch_asset_bytes, fetch_mesh_asset_bytes, fetch_mesh_asset_bytes_with_attempts,
    fetch_texture_asset_bytes,
};
use crate::cookie_utils::{extract_cookie_header_from_set_cookie, merge_cookie_header};
use crate::login_codec_utils::{
    XmlRpcValue, escape_xml, first_child_with_tag, llsd_boolean, llsd_string, llsd_to_bool,
    llsd_to_string, llsd_to_u16, llsd_to_u32, parse_llsd_buddy_list, parse_llsd_map,
    parse_xmlrpc_value, xmlrpc_as_bool, xmlrpc_as_buddy_list, xmlrpc_as_string, xmlrpc_as_u16,
    xmlrpc_as_u32, xmlrpc_member_array_of_strings, xmlrpc_member_bool, xmlrpc_member_string,
};
use crate::object_decode_utils::*;

const MAX_LOGIN_REDIRECTS: usize = 4;
const MAX_EVENT_QUEUE_ONE_SHOT_ATTEMPTS: usize = 3;
const EVENT_QUEUE_ONE_SHOT_MIN_TIMEOUT: Duration = Duration::from_secs(35);
const EVENT_QUEUE_ONE_SHOT_RETRY_BASE_DELAY: Duration = Duration::from_millis(250);
const EVENT_QUEUE_ONE_SHOT_RETRY_MAX_DELAY: Duration = Duration::from_secs(2);
const LLSD_XML_CONTENT_TYPE: &str = "application/llsd+xml";
const TEXTURE_FETCH_ACCEPT_HEADER: &str = "image/x-j2c,image/jp2,image/*,*/*";
const MESH_FETCH_ACCEPT_HEADER: &str = "application/vnd.ll.mesh";
const MESH_FETCH_RANGE_HEADER_FIRESTORM: &str = "bytes=0-4095";
const MESH_FETCH_RANGE_HEADER_FULL: &str = "bytes=0-";
const FIRESTORM_USER_AGENT_HEADER: &str = "Firestorm";
const LLUDP_PACKET_ID_SIZE: usize = 6;
const LLUDP_MINIMUM_VALID_PACKET_SIZE: usize = LLUDP_PACKET_ID_SIZE + 1;
const LLUDP_MESSAGE_PREFIX: u8 = 0xFF;
const LLUDP_RELIABLE_FLAG: u8 = 0x40;
const LLUDP_ACK_FLAG: u8 = 0x10;
const LLUDP_ZERO_CODE_FLAG: u8 = 0x80;
const LLUDP_LOW_FREQUENCY_PREFIX: u32 = 0xFFFF0000;
const LLUDP_MTU_BYTES: usize = 1200;
const LLUDP_MAX_APPENDED_ACKS: usize = 250;
const MAX_PENDING_FIRST_SIMULATOR_ACK_IDS: usize = 512;
const MAX_FIRST_SIMULATOR_SOCKET_DIAGNOSTICS: usize = 128;
const LLUDP_USE_CIRCUIT_CODE_LOW_ID: u16 = 3;
const LLUDP_CHAT_FROM_VIEWER_LOW_ID: u16 = 80;
const LLUDP_AGENT_THROTTLE_LOW_ID: u16 = 81;
const LLUDP_AGENT_HEIGHT_WIDTH_LOW_ID: u16 = 83;
const LLUDP_SET_ALWAYS_RUN_LOW_ID: u16 = 88;
const LLUDP_COMPLETE_AGENT_MOVEMENT_LOW_ID: u16 = 249;
const LLUDP_TEST_MESSAGE_LOW_ID: u16 = 1;
const LLUDP_OBJECT_EXTRA_PARAMS_LOW_ID: u16 = 99;
const LLUDP_REGION_HANDSHAKE_LOW_ID: u16 = 148;
const LLUDP_REGION_HANDSHAKE_REPLY_LOW_ID: u16 = 149;
const LLUDP_HEALTH_MESSAGE_LOW_ID: u16 = 138;
const LLUDP_CHAT_FROM_SIMULATOR_LOW_ID: u16 = 139;
const LLUDP_SIMULATOR_VIEWER_TIME_LOW_ID: u16 = 150;
const LLUDP_ENABLE_SIMULATOR_LOW_ID: u16 = 151;
const LLUDP_AGENT_MOVEMENT_COMPLETE_LOW_ID: u16 = 250;
const LLUDP_MUTE_LIST_REQUEST_LOW_ID: u16 = 262;
const LLUDP_MONEY_BALANCE_REQUEST_LOW_ID: u16 = 313;
const LLUDP_AGENT_DATA_UPDATE_LOW_ID: u16 = 387;
const LLUDP_AGENT_DATA_UPDATE_REQUEST_LOW_ID: u16 = 386;
const LLUDP_PACKET_ACK_LOW_ID: u16 = 0xFFFB;
const LLUDP_ONLINE_NOTIFICATION_LOW_ID: u16 = 322;
const LLUDP_CHANGE_USER_RIGHTS_LOW_ID: u16 = 321;
const LLUDP_OFFLINE_NOTIFICATION_LOW_ID: u16 = 323;
const LLUDP_IMPROVED_INSTANT_MESSAGE_LOW_ID: u16 = 254;
const LLUDP_RETRIEVE_INSTANT_MESSAGES_LOW_ID: u16 = 255;
const LLUDP_AVATAR_PROPERTIES_REQUEST_LOW_ID: u16 = 169;
const LLUDP_AVATAR_PROPERTIES_REPLY_LOW_ID: u16 = 171;
const LLUDP_AVATAR_GROUPS_REPLY_LOW_ID: u16 = 173;
const LLUDP_AVATAR_CLASSIFIED_REPLY_LOW_ID: u16 = 42;
const LLUDP_CLASSIFIED_INFO_REQUEST_LOW_ID: u16 = 43;
const LLUDP_CLASSIFIED_INFO_REPLY_LOW_ID: u16 = 44;
const LLUDP_AVATAR_NOTES_REPLY_LOW_ID: u16 = 176;
const LLUDP_AVATAR_PICKS_REPLY_LOW_ID: u16 = 178;
const LLUDP_PICK_INFO_REPLY_LOW_ID: u16 = 184;
const LLUDP_GENERIC_MESSAGE_LOW_ID: u16 = 261;
const LLUDP_LAYER_DATA_HIGH_ID: u8 = 11;
const LLUDP_OBJECT_UPDATE_HIGH_ID: u8 = 12;
const LLUDP_OBJECT_UPDATE_COMPRESSED_HIGH_ID: u8 = 13;
const LLUDP_OBJECT_UPDATE_CACHED_HIGH_ID: u8 = 14;
const LLUDP_IMPROVED_TERSE_OBJECT_UPDATE_HIGH_ID: u8 = 15;
const LLUDP_KILL_OBJECT_HIGH_ID: u8 = 16;
const LLUDP_CAMERA_CONSTRAINT_HIGH_ID: u8 = 22;
const LLUDP_AGENT_UPDATE_HIGH_ID: u8 = 4;
const LLUDP_AGENT_ANIMATION_HIGH_ID: u8 = 5;
const LLUDP_VIEWER_EFFECT_MEDIUM_ID: u8 = 17;
const LLUDP_REQUEST_MULTIPLE_OBJECTS_MEDIUM_ID: u8 = 3;
const LLUDP_COARSE_LOCATION_UPDATE_MEDIUM_ID: u8 = 6;
const LLUDP_ATTACHED_SOUND_MEDIUM_ID: u8 = 13;
const LLUDP_CROSSED_REGION_MEDIUM_ID: u8 = 7;
const LLUDP_CONFIRM_ENABLE_SIMULATOR_MEDIUM_ID: u8 = 8;
const LLUDP_CACHE_MISS_TYPE_TOTAL: u8 = 0;
const MAX_OBJECT_FEED_OBJECTS: usize = 1024;
const MAX_OBJECT_FEED_EXPORT_OBJECTS: usize = 128;
const MAX_OBJECT_FEED_RECENT_KILLS: usize = 128;
const MAX_PENDING_OBJECT_CACHE_MISS_IDS: usize = 2048;
const LLUDP_MAX_REQUEST_MULTIPLE_OBJECTS_BLOCKS: usize = 255;
const MAX_ZEROCODED_BODY_BYTES: usize = 256 * 1024;
const MAX_CONTINUITY_NEIGHBORS: usize = 8;
const MAX_CONTINUITY_OBSERVATIONS: usize = 16;
// Firestorm reference capture `artifacts/logs/firestorm_agvproto_capture_2026-03-29_211128.log`
// shows this throttle mix immediately after AMC, before object ingress starts.
const STARTUP_AGENT_THROTTLES: [f32; 7] = [450.0, 310.0, 62.0, 62.0, 1528.0, 1528.0, 560.0];
const STARTUP_AGENT_HEIGHT: u16 = 720;
const STARTUP_AGENT_WIDTH: u16 = 1280;
const STARTUP_AGENT_UPDATE_FAR: f32 = 96.0;
const STARTUP_AGENT_ANIMATION_ID: &str = "efcf670c-2d18-8128-973a-034ebc806b67";
// Firestorm sends viewer capability bits here (not echoed simulator RegionFlags).
// OpenSim gates initial data on bit 0x1000 in RegionHandshakeReply.
const REGION_HANDSHAKE_REPLY_FLAG_SUPPORTS_SELF_APPEARANCE: u32 = 0x0000_1000;
// Firestorm reference (behavior only): llviewerregion.cpp capabilityNames.append(...)
// This broader request set improves capability-lane discovery on simulator-host routes where
// object-relevant data may surface outside the minimal baseline probes.
const DEFAULT_SEED_CAPABILITY_REQUEST: &[&str] = &[
    "AbuseCategories",
    "AcceptFriendship",
    "AcceptGroupInvite",
    "AgentPreferences",
    "AgentProfile",
    "AgentState",
    "AttachmentResources",
    "AvatarPickerSearch",
    "AvatarRenderInfo",
    "CharacterProperties",
    "ChatSessionRequest",
    "CopyInventoryFromNotecard",
    "CreateInventoryCategory",
    "DeclineFriendship",
    "DeclineGroupInvite",
    "DispatchRegionInfo",
    "DirectDelivery",
    "EnvironmentSettings",
    "EstateAccess",
    "DispatchOpenRegionSettings",
    "EstateChangeInfo",
    "EventQueueGet",
    "ExtEnvironment",
    "FetchLib2",
    "FetchLibDescendents2",
    "FetchInventory2",
    "FetchInventoryDescendents2",
    "IncrementCOFVersion",
    "RequestTaskInventory",
    "InterestList",
    "InventoryThumbnailUpload",
    "GetDisplayNames",
    "GetExperiences",
    "AgentExperiences",
    "FindExperienceByName",
    "GetExperienceInfo",
    "GetAdminExperiences",
    "GetCreatorExperiences",
    "ExperiencePreferences",
    "GroupExperiences",
    "UpdateExperience",
    "IsExperienceAdmin",
    "IsExperienceContributor",
    "RegionExperiences",
    "ExperienceQuery",
    "GetMesh",
    "GetMesh2",
    "GetMetadata",
    "GetObjectCost",
    "GetObjectPhysicsData",
    "GetTexture",
    "GroupAPIv1",
    "GroupMemberData",
    "GroupProposalBallot",
    "HomeLocation",
    "LandResources",
    "LSLSyntax",
    "MapLayer",
    "MapLayerGod",
    "MeshUploadFlag",
    "ModifyMaterialParams",
    "ModifyRegion",
    "NavMeshGenerationStatus",
    "NewFileAgentInventory",
    "ObjectAnimation",
    "ObjectMedia",
    "ObjectMediaNavigate",
    "ObjectNavMeshProperties",
    "ParcelPropertiesUpdate",
    "ParcelVoiceInfoRequest",
    "ProductInfoRequest",
    "ProvisionVoiceAccountRequest",
    "VoiceSignalingRequest",
    "ReadOfflineMsgs",
    "RegionObjects",
    "RegionSchedule",
    "RemoteParcelRequest",
    "RenderMaterials",
    "RequestTextureDownload",
    "ResourceCostSelected",
    "RetrieveNavMeshSrc",
    "SearchStatRequest",
    "SearchStatTracking",
    "SendPostcard",
    "SendUserReport",
    "SendUserReportWithScreenshot",
    "ServerReleaseNotes",
    "SetDisplayName",
    "SimConsoleAsync",
    "SimulatorFeatures",
    "StartGroupProposal",
    "TerrainNavMeshProperties",
    "TextureStats",
    "UntrustedSimulatorMessage",
    "UpdateAgentInformation",
    "UpdateAgentLanguage",
    "UpdateAvatarAppearance",
    "UpdateGestureAgentInventory",
    "UpdateGestureTaskInventory",
    "UpdateNotecardAgentInventory",
    "UpdateNotecardTaskInventory",
    "UpdateScriptAgent",
    "UpdateScriptTask",
    "UpdateSettingsAgentInventory",
    "UpdateSettingsTaskInventory",
    "UploadAgentProfileImage",
    "UpdateMaterialAgentInventory",
    "UpdateMaterialTaskInventory",
    "UploadBakedTexture",
    "UserInfo",
    "ViewerAsset",
    "ViewerBenefits",
    "ViewerMetrics",
    "ViewerStartAuction",
    "ViewerStats",
    "VETPBR",
];

pub trait LoginCodec {
    fn content_type(&self) -> &str;
    fn encode_request(&self, request: &GridLoginRequest) -> Result<Vec<u8>, CodecError>;
    fn decode_response(&self, body: &[u8]) -> Result<GridLoginResponse, CodecError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LoginWireFormat {
    #[default]
    Json,
    Llsd,
    XmlRpc,
}

impl LoginWireFormat {
    fn codec(&self) -> Box<dyn LoginCodec> {
        match self {
            LoginWireFormat::Json => Box::new(JsonLoginCodec),
            LoginWireFormat::Llsd => Box::new(LlsdLoginCodec),
            LoginWireFormat::XmlRpc => Box::new(XmlRpcLoginCodec),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct JsonLoginCodec;

#[derive(Debug, Error)]
pub enum CodecError {
    #[error("request codec error: {0}")]
    Serialize(String),
    #[error("response codec error: {0}")]
    Deserialize(String),
}

impl LoginCodec for JsonLoginCodec {
    fn content_type(&self) -> &str {
        "application/json"
    }

    fn encode_request(&self, request: &GridLoginRequest) -> Result<Vec<u8>, CodecError> {
        serde_json::to_vec(request).map_err(|err| CodecError::Serialize(err.to_string()))
    }

    fn decode_response(&self, body: &[u8]) -> Result<GridLoginResponse, CodecError> {
        let value: Value =
            serde_json::from_slice(body).map_err(|err| CodecError::Deserialize(err.to_string()))?;
        let normalized = normalize_login_response(value);
        serde_json::from_value(normalized).map_err(|err| CodecError::Deserialize(err.to_string()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub endpoint: String,
    pub connect_timeout: Duration,
    pub wire_format: LoginWireFormat,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            endpoint: String::from("https://example.invalid"),
            connect_timeout: Duration::from_secs(10),
            wire_format: LoginWireFormat::Json,
        }
    }
}

pub struct LlsdLoginCodec;
pub struct XmlRpcLoginCodec;

impl LoginCodec for LlsdLoginCodec {
    fn content_type(&self) -> &str {
        "application/llsd+xml"
    }

    fn encode_request(&self, request: &GridLoginRequest) -> Result<Vec<u8>, CodecError> {
        let mut xml = String::from("<llsd><map>");
        xml.push_str(&format!(
            "<key>method</key><string>{}</string>",
            escape_xml(&request.method)
        ));
        xml.push_str("<key>params</key><map>");
        xml.push_str(&llsd_string("username", &request.params.username));
        xml.push_str(&llsd_string(
            "passwd",
            &normalize_legacy_passwd(&request.params.password),
        ));
        if let Some((first, last)) = split_legacy_name(&request.params.username) {
            xml.push_str(&llsd_string("first", &first));
            xml.push_str(&llsd_string("last", &last));
        }
        xml.push_str(&llsd_string("start", &request.params.start));
        xml.push_str(&llsd_boolean("agree_to_tos", request.params.agree_to_tos));
        xml.push_str(&llsd_boolean("read_critical", request.params.read_critical));
        if let Some(token) = &request.params.token {
            xml.push_str(&llsd_string("token", token));
        }
        xml.push_str(&llsd_string("channel", &request.params.channel));
        xml.push_str(&llsd_string("version", &request.params.version));
        xml.push_str(&llsd_string("platform", &request.params.platform));
        xml.push_str(&llsd_string(
            "platform_version",
            &request.params.platform_version,
        ));
        xml.push_str(&llsd_string("host_id", &request.params.host_id));
        xml.push_str(&llsd_string("machine_hash", &request.params.machine_hash));
        xml.push_str("</map>");
        xml.push_str("<key>options</key><array>");
        for opt in &request.options {
            xml.push_str(&format!("<string>{}</string>", escape_xml(opt)));
        }
        xml.push_str("</array></map></llsd>");
        Ok(xml.into_bytes())
    }

    fn decode_response(&self, body: &[u8]) -> Result<GridLoginResponse, CodecError> {
        let map = parse_llsd_map(body)?;
        Ok(GridLoginResponse {
            login: map.get("login").and_then(llsd_to_bool),
            reason: map.get("reason").and_then(llsd_to_string),
            message: map.get("message").and_then(llsd_to_string),
            message_id: map.get("message_id").and_then(llsd_to_string),
            next_url: map.get("next_url").and_then(llsd_to_string),
            next_method: map.get("next_method").and_then(llsd_to_string),
            agent_id: map.get("agent_id").and_then(llsd_to_string),
            session_id: map.get("session_id").and_then(llsd_to_string),
            secure_session_id: map.get("secure_session_id").and_then(llsd_to_string),
            circuit_code: map.get("circuit_code").and_then(llsd_to_u32),
            sim_ip: map.get("sim_ip").and_then(llsd_to_string),
            sim_port: map.get("sim_port").and_then(llsd_to_u16),
            region_x: map.get("region_x").and_then(llsd_to_u32),
            region_y: map.get("region_y").and_then(llsd_to_u32),
            seed_capability: map.get("seed_capability").and_then(llsd_to_string),
            start_location: map.get("start_location").and_then(llsd_to_string),
            look_at: map.get("look_at").and_then(llsd_to_string),
            home: map.get("home").and_then(llsd_to_string),
            motd: map.get("motd").and_then(llsd_to_string),
            buddy_list: parse_llsd_buddy_list(body).unwrap_or_default(),
        })
    }
}

impl LoginCodec for XmlRpcLoginCodec {
    fn content_type(&self) -> &str {
        "text/xml"
    }

    fn encode_request(&self, request: &GridLoginRequest) -> Result<Vec<u8>, CodecError> {
        let mut xml = String::from("<?xml version=\"1.0\"?><methodCall>");
        xml.push_str(&format!(
            "<methodName>{}</methodName>",
            escape_xml(&request.method)
        ));
        xml.push_str("<params><param><value><struct>");

        let credentials = classify_legacy_login_name(&request.params.username);
        xmlrpc_member_string(&mut xml, "first", &credentials.first);
        xmlrpc_member_string(&mut xml, "last", &credentials.last);
        xmlrpc_member_string(
            &mut xml,
            "passwd",
            &normalize_legacy_passwd(&request.params.password),
        );
        xmlrpc_member_string(&mut xml, "start", &request.params.start);
        xmlrpc_member_bool(&mut xml, "agree_to_tos", request.params.agree_to_tos);
        xmlrpc_member_bool(&mut xml, "read_critical", request.params.read_critical);
        if let Some(token) = &request.params.token {
            xmlrpc_member_string(&mut xml, "token", token);
        }
        xmlrpc_member_string(&mut xml, "channel", &request.params.channel);
        xmlrpc_member_string(&mut xml, "version", &request.params.version);
        xmlrpc_member_string(&mut xml, "platform", &request.params.platform);
        xmlrpc_member_string(
            &mut xml,
            "platform_version",
            &request.params.platform_version,
        );
        xmlrpc_member_string(&mut xml, "host_id", &request.params.host_id);
        xmlrpc_member_string(&mut xml, "machine_hash", &request.params.machine_hash);
        xmlrpc_member_array_of_strings(&mut xml, "options", &request.options);

        xml.push_str("</struct></value></param></params></methodCall>");
        Ok(xml.into_bytes())
    }

    fn decode_response(&self, body: &[u8]) -> Result<GridLoginResponse, CodecError> {
        let text =
            std::str::from_utf8(body).map_err(|err| CodecError::Deserialize(err.to_string()))?;
        let doc = Document::parse(text).map_err(|err| CodecError::Deserialize(err.to_string()))?;

        if let Some(fault_value) = doc
            .descendants()
            .find(|node| node.has_tag_name("fault"))
            .and_then(|fault| first_child_with_tag(fault, "value"))
        {
            let fault = parse_xmlrpc_value(fault_value)?;
            let message = match fault {
                XmlRpcValue::Struct(map) => map
                    .get("faultString")
                    .and_then(xmlrpc_as_string)
                    .or_else(|| map.get("message").and_then(xmlrpc_as_string)),
                _ => None,
            };

            return Ok(GridLoginResponse {
                login: Some(false),
                reason: Some(String::from("fault")),
                message,
                ..Default::default()
            });
        }

        let value_node = doc
            .descendants()
            .find(|node| node.has_tag_name("params"))
            .and_then(|params| first_child_with_tag(params, "param"))
            .and_then(|param| first_child_with_tag(param, "value"))
            .ok_or_else(|| CodecError::Deserialize(String::from("missing xml-rpc params/value")))?;

        let parsed = parse_xmlrpc_value(value_node)?;
        let map = match parsed {
            XmlRpcValue::Struct(map) => map,
            _ => {
                return Err(CodecError::Deserialize(String::from(
                    "xml-rpc response value is not a struct",
                )));
            }
        };

        Ok(GridLoginResponse {
            login: map.get("login").and_then(xmlrpc_as_bool),
            reason: map.get("reason").and_then(xmlrpc_as_string),
            message: map.get("message").and_then(xmlrpc_as_string),
            message_id: map.get("message_id").and_then(xmlrpc_as_string),
            next_url: map.get("next_url").and_then(xmlrpc_as_string),
            next_method: map.get("next_method").and_then(xmlrpc_as_string),
            agent_id: map.get("agent_id").and_then(xmlrpc_as_string),
            session_id: map.get("session_id").and_then(xmlrpc_as_string),
            secure_session_id: map.get("secure_session_id").and_then(xmlrpc_as_string),
            circuit_code: map.get("circuit_code").and_then(xmlrpc_as_u32),
            sim_ip: map.get("sim_ip").and_then(xmlrpc_as_string),
            sim_port: map.get("sim_port").and_then(xmlrpc_as_u16),
            region_x: map.get("region_x").and_then(xmlrpc_as_u32),
            region_y: map.get("region_y").and_then(xmlrpc_as_u32),
            seed_capability: map.get("seed_capability").and_then(xmlrpc_as_string),
            start_location: map.get("start_location").and_then(xmlrpc_as_string),
            look_at: map.get("look_at").and_then(xmlrpc_as_string),
            home: map.get("home").and_then(xmlrpc_as_string),
            motd: map.get("motd").and_then(xmlrpc_as_string),
            buddy_list: map
                .get("buddy-list")
                .and_then(xmlrpc_as_buddy_list)
                .unwrap_or_default(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub first_name: String,
    pub last_name: String,
    pub password: String,
}

#[derive(Debug, Clone)]
pub struct LoginTrace {
    pub initial_request: LoginTraceRequest,
    pub redirect_steps: Vec<LoginTraceRedirect>,
    pub final_response: LoginTraceResponse,
    pub final_result: LoginTraceFinalResult,
}

#[derive(Debug, Clone)]
pub struct LoginTraceRequest {
    pub method: String,
    pub start_location: String,
    pub options: Vec<String>,
    pub agree_to_tos: bool,
    pub read_critical: bool,
    pub had_mfa_token: bool,
}

#[derive(Debug, Clone)]
pub struct LoginTraceRedirect {
    pub url: String,
    pub method: String,
    pub result_type: String,
}

#[derive(Debug, Clone)]
pub struct LoginTraceResponse {
    pub login: Option<bool>,
    pub reason: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LoginTraceFinalResult {
    pub outcome: String,
    pub reason: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoginFallbackClassifiedReason {
    RequestShapeMissingPassword,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginFallbackOutcome {
    pub primary_wire_format: LoginWireFormat,
    pub fallback_used: bool,
    pub final_wire_format: LoginWireFormat,
    pub classified_reason: Option<LoginFallbackClassifiedReason>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub account_name: String,
    pub session_token: String,
    pub seed_capability: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstSimulatorTarget {
    pub sim_ip: String,
    pub sim_port: u16,
    pub region_x: u32,
    pub region_y: u32,
    pub seed_capability: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstSimulatorHandshakePrerequisites {
    pub agent_id: String,
    pub session_id: String,
    pub circuit_code: u32,
    pub target: FirstSimulatorTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FirstSimulatorHandshakeStage {
    BootstrapPrerequisitesReady,
    FirstRegionTargetKnown,
    UseCircuitCode,
    CompleteAgentMovement,
    WaitingForAgentMovementComplete,
    AgentMovementComplete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstSimulatorHandshakeState {
    pub stage: FirstSimulatorHandshakeStage,
    pub prerequisites: FirstSimulatorHandshakePrerequisites,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FirstSimulatorHandshakeAction {
    UseCircuitCode,
    CompleteAgentMovement,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstSimulatorHandshakeSendDiagnostic {
    pub action: FirstSimulatorHandshakeAction,
    pub target: String,
    pub packet_id: u32,
    pub packet_message_number: Option<u32>,
    pub appended_ack_ids: Vec<u32>,
    pub payload_len: usize,
    pub elapsed_ms: u128,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FirstSimulatorSocketDiagnosticKind {
    ProbeBind,
    FreshBind,
    RetainProbeSocket,
    ReuseRetainedProbeSocket,
    Send,
    Receive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstSimulatorSocketDiagnostic {
    pub event_index: usize,
    pub kind: FirstSimulatorSocketDiagnosticKind,
    pub reason: String,
    pub local_addr: Option<String>,
    pub remote_target: Option<String>,
    pub packet_message_number: Option<u32>,
    pub payload_len: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstSimulatorSocketDiagnosticSummary {
    pub events: usize,
    pub unique_local_ports: Vec<u16>,
    pub probe_bind_events: usize,
    pub fresh_bind_events: usize,
    pub retained_probe_events: usize,
    pub reused_retained_probe_events: usize,
    pub send_events: usize,
    pub receive_events: usize,
    pub split_local_port_detected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FirstSimulatorInboundMessageKind {
    TestMessage,
    PacketAck,
    AgentMovementComplete,
    RegionHandshake,
    RegionHandshakeReply,
    HealthMessage,
    LayerData,
    ChatFromSimulator,
    SimulatorViewerTimeMessage,
    EnableSimulator,
    AgentDataUpdate,
    OnlineNotification,
    ViewerEffect,
    CoarseLocationUpdate,
    AttachedSound,
    CrossedRegion,
    ConfirmEnableSimulator,
    CameraConstraint,
    GenericMessage,
    ObjectUpdate,
    ObjectUpdateCompressed,
    ObjectUpdateCached,
    ImprovedTerseObjectUpdate,
    ObjectExtraParams,
    KillObject,
    Irrelevant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FirstSimulatorInboundDecodeSource {
    PacketMessageNumber,
    JsonField,
    TextScan,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FirstSimulatorInboundTrafficScope {
    BootstrapRelevant,
    TransportControl,
    RegionTransitionControl,
    LikelyBroaderTraffic,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstSimulatorInboundClassification {
    pub kind: FirstSimulatorInboundMessageKind,
    pub scope: FirstSimulatorInboundTrafficScope,
    pub signal: String,
    pub decode_source: FirstSimulatorInboundDecodeSource,
    pub packet_message_number: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstSimulatorHandshakeReceiveDiagnostic {
    pub observation_index: usize,
    pub kind: FirstSimulatorInboundMessageKind,
    pub scope: FirstSimulatorInboundTrafficScope,
    pub payload_len: usize,
    pub stage_before: Option<FirstSimulatorHandshakeStage>,
    pub stage_after: Option<FirstSimulatorHandshakeStage>,
    pub advanced_stage: bool,
    pub signal: String,
    pub decode_source: FirstSimulatorInboundDecodeSource,
    pub packet_message_number: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstSimulatorHandshakeProbeObservation {
    pub observation_index: usize,
    pub payload_len: usize,
    pub classification: FirstSimulatorInboundClassification,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstSimulatorPostBoundarySummary {
    pub observations: usize,
    pub bootstrap_relevant: usize,
    pub transport_control: usize,
    pub region_transition_control: usize,
    pub likely_broader_traffic: usize,
    pub unknown: usize,
    pub crossed_region: usize,
    pub confirm_enable_simulator: usize,
    pub watched_region_transition_control_not_seen: bool,
    pub kinds: Vec<FirstSimulatorInboundMessageKind>,
    pub unknown_packet_message_numbers: Vec<u32>,
    pub repeated_unknown_packet_message_numbers: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegionTransitionControlKind {
    CrossedRegion,
    ConfirmEnableSimulator,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionTransitionControlObservation {
    pub observation_index: usize,
    pub kind: RegionTransitionControlKind,
    pub packet_message_number: Option<u32>,
    pub payload_len: usize,
    pub signal: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionTransitionControlSummary {
    pub observations: usize,
    pub crossed_region: usize,
    pub confirm_enable_simulator: usize,
    pub not_seen_in_run: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EarlySimulatorTrafficKind {
    HealthMessage,
    LayerData,
    ChatFromSimulator,
    SimulatorViewerTimeMessage,
    OnlineNotification,
    ViewerEffect,
    CoarseLocationUpdate,
    AttachedSound,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EarlySimulatorTrafficObservation {
    pub observation_index: usize,
    pub kind: EarlySimulatorTrafficKind,
    pub packet_message_number: Option<u32>,
    pub payload_len: usize,
    pub signal: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EarlySimulatorTrafficSummary {
    pub observations: usize,
    pub health_message: usize,
    pub layer_data: usize,
    pub chat_from_simulator: usize,
    pub simulator_viewer_time_message: usize,
    pub online_notification: usize,
    pub viewer_effect: usize,
    pub coarse_location_update: usize,
    pub attached_sound: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecodedCoarseLocationUpdate {
    pub location_count: u8,
    pub first_location: Option<[u8; 3]>,
    pub second_location: Option<[u8; 3]>,
    pub third_location: Option<[u8; 3]>,
    pub avatars: Vec<DecodedCoarseAvatar>,
    pub self_index: Option<u16>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecodedCoarseAvatar {
    pub agent_id: Option<String>,
    pub xyz: [u8; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DecodedHealthMessage {
    pub health: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DecodedAgentMovementComplete {
    pub position: [i32; 3],
    pub region_handle: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecodedRegionHandshake {
    pub region_flags: u32,
    pub sim_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecodedSimulatorViewerTimeMessage {
    pub body_len: u16,
    pub signature: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecodedObjectFaceMaterial {
    pub face_id: u16,
    pub texture_id: Option<String>,
    pub normal_id: Option<String>,
    pub specular_id: Option<String>,
    pub material_id: Option<String>,
    pub rgba: [u8; 4],
    pub offset_s: i16,
    pub offset_t: i16,
    pub scale_s: i16,
    pub scale_t: i16,
    pub rotation: i16,
    pub bump: u8,
    pub fullbright: bool,
    pub shiny: u8,
    pub media_flags: u8,
    pub glow: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecodedObjectFeedObject {
    pub local_id: u32,
    pub scale_centi: Option<[u16; 3]>,
    pub position_centi: Option<[i32; 3]>,
    pub mesh_id: Option<String>,
    pub texture_id: Option<String>,
    pub default_face_material: Option<DecodedObjectFaceMaterial>,
    pub face_material_overrides: Vec<DecodedObjectFaceMaterial>,
    pub object_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulatorPayloadDecodeSummary {
    pub coarse_location_updates: usize,
    pub coarse_location_last_count: Option<u8>,
    pub coarse_location_last_first: Option<[u8; 3]>,
    pub coarse_location_last_second: Option<[u8; 3]>,
    pub coarse_location_last_third: Option<[u8; 3]>,
    pub coarse_location_last_avatars: Vec<DecodedCoarseAvatar>,
    pub coarse_location_last_self_index: Option<u16>,
    pub region_handshake_updates: usize,
    pub region_handshake_last_sim_name: Option<String>,
    pub agent_movement_complete_updates: usize,
    pub agent_movement_complete_last_position: Option<[i32; 3]>,
    pub agent_movement_complete_last_region_handle: Option<u64>,
    pub health_updates: usize,
    pub health_last_basis_points: Option<u16>,
    pub simulator_viewer_time_updates: usize,
    pub simulator_viewer_time_last_body_len: Option<u16>,
    pub simulator_viewer_time_last_signature: Option<u32>,
    pub object_feed_update_messages: usize,
    pub object_feed_object_update_messages: usize,
    pub object_feed_object_update_mesh_hits: usize,
    pub object_feed_object_update_compressed_messages: usize,
    pub object_feed_object_update_compressed_mesh_hits: usize,
    pub object_feed_object_update_cached_messages: usize,
    pub object_feed_improved_terse_messages: usize,
    pub object_feed_object_extra_params_messages: usize,
    pub object_feed_object_extra_params_mesh_hits: usize,
    pub object_feed_kill_messages: usize,
    pub object_feed_decode_dropped: usize,
    pub object_feed_evicted: usize,
    pub object_feed_total_objects: usize,
    pub object_feed_state_mesh_objects: usize,
    pub object_feed_export_objects: usize,
    pub object_feed_export_mesh_objects: usize,
    pub object_feed_export_truncated: bool,
    pub object_feed_objects: Vec<DecodedObjectFeedObject>,
    pub object_feed_recent_kills: Vec<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstSimulatorAckForensicsSummary {
    pub pending_ack_count: usize,
    pub pending_ack_ids_preview: Vec<u32>,
    pub outbound_appended_ack_sends: usize,
    pub outbound_appended_ack_ids_last: Vec<u32>,
    pub explicit_packet_ack_receives: usize,
    pub first_packet_ack_receive_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetFetchAttempt {
    pub url: String,
    pub range_header: Option<String>,
    pub status: Option<u16>,
    pub error: Option<String>,
    pub response_body: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstSimulatorReceiveForensicsSummary {
    pub receive_observations: usize,
    pub typed_kind_counts: Vec<(FirstSimulatorInboundMessageKind, usize)>,
    pub raw_packet_message_numbers: Vec<(u32, usize)>,
    pub unclassified_packet_message_numbers: Vec<(u32, usize)>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstSimulatorStartupTimelineSummary {
    pub first_region_handshake_index: Option<usize>,
    pub first_region_handshake_reply_index: Option<usize>,
    pub first_agent_movement_complete_index: Option<usize>,
    pub first_packet_ack_index: Option<usize>,
    pub first_camera_constraint_index: Option<usize>,
    pub first_generic_message_index: Option<usize>,
    pub first_object_update_index: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstSimulatorStartupTranscriptSummary {
    pub send_events: Vec<String>,
    pub receive_events: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstSimulatorStartupInterestSendEvidence {
    pub message: String,
    pub order_index: usize,
    pub packet_id: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstSimulatorStartupInterestGateSummary {
    pub passed: bool,
    pub required: Vec<FirstSimulatorStartupInterestSendEvidence>,
    pub missing: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstSimulatorHandshakeProbeReport {
    pub observations: Vec<FirstSimulatorHandshakeProbeObservation>,
    pub timed_out: bool,
    pub agent_movement_complete_observation_index: Option<usize>,
    pub post_movement_observations: usize,
    pub post_boundary_summary: Option<FirstSimulatorPostBoundarySummary>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SeedCapabilityMap {
    pub entries: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CapabilityUrlFamily {
    SimulatorHost12043,
    SimulatorHost12046,
    AssetCdn,
    BakeTextureCdn,
    MapCdn,
    PhoenixViewer,
    Analytics,
    GenericWeb,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityUrlClassification {
    pub family: CapabilityUrlFamily,
    pub host: Option<String>,
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeedCapabilityInventoryEntry {
    pub name: String,
    pub classification: CapabilityUrlClassification,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventQueueInspection {
    pub top_level_keys: Vec<String>,
    pub has_events_array: bool,
    pub has_id: bool,
    pub event_count: usize,
    pub event_names: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventQueueMessage {
    pub message: String,
    pub fields: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventQueuePollResult {
    pub id: Option<u64>,
    pub events: Vec<EventQueueMessage>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventQueueSimulatorTarget {
    pub message: String,
    pub handle: Option<String>,
    pub ip: Option<String>,
    pub port: Option<String>,
    pub sim_ip_and_port: Option<String>,
    pub seed_capability: Option<String>,
    pub endpoint_ip: Option<String>,
    pub endpoint_port: Option<u16>,
    pub endpoint_source: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventQueueParcelSummary {
    pub message: String,
    pub local_id: Option<String>,
    pub name: Option<String>,
    pub parcel_id: Option<String>,
    pub owner_id: Option<String>,
    pub area: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NearbyChatMessage {
    pub sender: String,
    pub text: String,
    pub source: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResolvedAvatarName {
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentProfileGroup {
    pub id: String,
    pub name: String,
    pub image_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentProfilePick {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentProfilePickDetails {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub snapshot_id: Option<String>,
    pub parcel_id: Option<String>,
    pub sim_name: Option<String>,
    pub parcel_name: Option<String>,
    pub global_position: Option<[i32; 3]>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentProfileClassified {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentProfileClassifiedDetails {
    pub id: String,
    pub name: Option<String>,
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentProfileData {
    pub id: String,
    pub profile_url: Option<String>,
    pub sl_about_text: String,
    pub fl_about_text: String,
    pub notes: String,
    pub sl_image_id: Option<String>,
    pub fl_image_id: Option<String>,
    pub partner_id: Option<String>,
    pub member_since: Option<String>,
    pub online: Option<bool>,
    pub allow_publish: Option<bool>,
    pub identified: Option<bool>,
    pub transacted: Option<bool>,
    pub display_name: Option<String>,
    pub username: Option<String>,
    pub groups: Vec<AgentProfileGroup>,
    pub picks: Vec<AgentProfilePick>,
    pub pick_details: Vec<AgentProfilePickDetails>,
    pub classifieds: Vec<AgentProfileClassified>,
    pub classified_details: Vec<AgentProfileClassifiedDetails>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectImPayload {
    pub from_id: String,
    pub to_id: String,
    pub session_id: String,
    pub from_name: String,
    pub message: String,
    pub dialog: u8,
    pub timestamp: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocialEvent {
    FriendOnline {
        agent_id: String,
    },
    FriendOffline {
        agent_id: String,
    },
    FriendRights {
        agent_id: String,
        related_id: String,
        rights: i32,
    },
    DirectIm(DirectImPayload),
}

#[derive(Debug)]
pub struct SocialCircuit {
    bind: String,
    target: FirstSimulatorTarget,
    socket: UdpSocket,
}

impl SocialCircuit {
    pub fn target_endpoint(&self) -> (&str, u16) {
        (&self.target.sim_ip, self.target.sim_port)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventQueueAttemptDiagnostic {
    pub attempt: usize,
    pub status: Option<u16>,
    pub elapsed_ms: u128,
    pub retryable: bool,
    pub retry_class: EventQueueRetryClass,
    pub retry_reason: String,
    pub scheduled_backoff_ms: Option<u128>,
    pub terminal_reason: Option<String>,
    pub error_kind: String,
    pub response_headers: Vec<(String, String)>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EventQueueRetryClass {
    #[default]
    Unknown,
    Timeout,
    Transport,
    UpstreamHttp,
    NonRetryableHttp,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SimulatorFeaturesInspection {
    pub top_level_keys: Vec<String>,
    pub scalar_values: BTreeMap<String, String>,
    pub complex_value_types: BTreeMap<String, String>,
}

pub type MapLayerInspection = SimulatorFeaturesInspection;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CapabilityTransportProbe {
    pub status: u16,
    pub content_type: Option<String>,
    pub body_bytes: usize,
    pub selected_url: Option<String>,
    pub method: String,
    pub decode: String,
    pub body_preview_hash: Option<String>,
    pub body_preview: Option<String>,
    pub response_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CapabilityProbeRequest {
    pub method: String,
    pub url_candidates: Vec<String>,
    pub accept: Option<String>,
    pub content_type: Option<String>,
    pub range: Option<String>,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RegionObjectsInspection {
    pub top_level_keys: Vec<String>,
    pub scalar_values: BTreeMap<String, String>,
    pub complex_value_types: BTreeMap<String, String>,
    pub array_lengths: BTreeMap<String, usize>,
    pub first_array_item_keys: BTreeMap<String, Vec<String>>,
    pub child_map_keys: BTreeMap<String, Vec<String>>,
    pub child_map_scalar_values: BTreeMap<String, Vec<String>>,
    pub child_map_profiles: BTreeMap<String, String>,
    pub child_map_semantic_values: BTreeMap<String, Vec<String>>,
    pub child_map_pathfinding_summaries: BTreeMap<String, RegionObjectsPathfindingSummary>,
    pub typed_object_samples: Vec<RegionObjectsTypedObjectSample>,
    pub tuple_description_analysis: Option<RegionObjectsTupleDescriptionAnalysis>,
    pub candidate_mesh_asset_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RegionObjectsPathfindingSummary {
    pub profile: String,
    pub variant_hint: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub description_shape: Option<String>,
    pub description_numeric_tuple: Option<Vec<String>>,
    pub owner: Option<String>,
    pub owner_is_group: Option<bool>,
    pub position_key_present: bool,
    pub position: Option<String>,
    pub position_shape: Option<String>,
    pub landimpact: Option<i32>,
    pub modifiable: Option<bool>,
    pub navmesh_category: Option<i32>,
    pub can_be_volume: Option<bool>,
    pub is_scripted: Option<bool>,
    pub phantom: Option<bool>,
    pub walkability_coefficients: Option<[i32; 4]>,
    pub linkset_use: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RegionObjectsTypedObjectSample {
    pub object_id: String,
    pub profile: String,
    pub name: Option<String>,
    pub owner: Option<String>,
    pub position: Option<String>,
    pub description_shape: Option<String>,
    pub linkset_use: Option<String>,
    pub walkability_coefficients: Option<[i32; 4]>,
    pub landimpact: Option<i32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RegionObjectsTupleDescriptionAnalysis {
    pub sample_count: usize,
    pub slot_count: usize,
    pub slot_distinct_values: Vec<Vec<String>>,
    pub distinct_names: Vec<String>,
    pub sample_pairs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    LoggedIn,
}

#[derive(Debug, Error)]
pub enum ConnectionError {
    #[error("invalid state transition from {0:?}")]
    InvalidState(ConnectionState),
    #[error("grid adapter error: {0}")]
    Adapter(#[from] GridAdapterError),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("http status {status}: {body}")]
    HttpStatus { status: StatusCode, body: String },
    #[error("invalid login response: {0}")]
    InvalidResponse(String),
    #[error("redirect limit of {0} exceeded")]
    RedirectLimitExceeded(usize),
    #[error("unsupported redirect method: {0}")]
    UnsupportedRedirectMethod(String),
    #[error("codec error: {0}")]
    Codec(#[from] CodecError),
    #[error("seed capability URL is unavailable in current session")]
    MissingSeedCapability,
    #[error("required capability is unavailable: {0}")]
    MissingCapability(String),
    #[error("first-simulator handshake prerequisites are unavailable in current session")]
    MissingFirstSimulatorHandshakePrerequisites,
    #[error("first-simulator handshake is not initialized")]
    FirstSimulatorHandshakeNotInitialized,
    #[error("invalid first-simulator handshake transition from {from:?} to {to:?}")]
    InvalidFirstSimulatorHandshakeTransition {
        from: FirstSimulatorHandshakeStage,
        to: FirstSimulatorHandshakeStage,
    },
    #[error("invalid first-simulator transport target '{target}': {reason}")]
    InvalidFirstSimulatorTransportTarget { target: String, reason: String },
    #[error("first-simulator {action:?} send failed to {target}: {reason}")]
    FirstSimulatorHandshakeSendFailed {
        action: FirstSimulatorHandshakeAction,
        target: String,
        reason: String,
    },
    #[error("invalid first-simulator receive bind target '{bind}': {reason}")]
    InvalidFirstSimulatorReceiveBind { bind: String, reason: String },
    #[error("first-simulator receive timed out on {bind} after {timeout_ms} ms")]
    FirstSimulatorReceiveTimedOut { bind: String, timeout_ms: u128 },
    #[error("first-simulator receive failed on {bind}: {reason}")]
    FirstSimulatorReceiveFailed { bind: String, reason: String },
    #[error("capability response decode error: {0}")]
    CapabilityDecode(String),
    #[error(
        "map layer capability currently appears non-HTTP for one-shot probing (status {status}): {body}"
    )]
    MapLayerLikelyLegacyUdp { status: StatusCode, body: String },
    #[error("event queue one-shot failed after {attempts_len} attempts")]
    EventQueueOneShotFailed {
        attempts_len: usize,
        attempts: Vec<EventQueueAttemptDiagnostic>,
    },
}

/// Minimal networking boundary.
/// Real protocol transport and grid-specific behavior will plug in later.
#[derive(Debug)]
pub struct Connection {
    config: ConnectionConfig,
    state: ConnectionState,
    session: Option<Session>,
    first_simulator_handshake_prerequisites: Option<FirstSimulatorHandshakePrerequisites>,
    first_simulator_handshake_state: Option<FirstSimulatorHandshakeState>,
    retained_probe_socket: Option<UdpSocket>,
    first_simulator_handshake_send_diagnostics: Vec<FirstSimulatorHandshakeSendDiagnostic>,
    first_simulator_handshake_receive_diagnostics: Vec<FirstSimulatorHandshakeReceiveDiagnostic>,
    first_simulator_socket_diagnostics: Vec<FirstSimulatorSocketDiagnostic>,
    early_simulator_traffic_observations: Vec<EarlySimulatorTrafficObservation>,
    region_transition_control_observations: Vec<RegionTransitionControlObservation>,
    simulator_payload_decode_summary: SimulatorPayloadDecodeSummary,
    object_feed_objects: BTreeMap<u32, ObjectFeedObjectState>,
    object_feed_recent_kills: Vec<u32>,
    object_feed_tick: u64,
    next_first_simulator_packet_id: u32,
    pending_first_simulator_ack_ids: Vec<u32>,
    pending_object_cache_miss_ids: Vec<u32>,
    pending_region_handshake_reply_flags: Option<u32>,
    region_handshake_reply_sent: bool,
    require_observed_region_handshake_for_reply: bool,
    last_bootstrap_sender_endpoint: Option<SocketAddr>,
    last_region_handshake_sender_endpoint: Option<SocketAddr>,
    continuity_summary: RegionContinuitySummary,
    last_phase_change_at: Instant,
}

#[derive(Debug, Clone, Default)]
struct ObjectFaceMaterialState {
    texture_id_bytes: Option<[u8; 16]>,
    normal_id_bytes: Option<[u8; 16]>,
    specular_id_bytes: Option<[u8; 16]>,
    material_id_bytes: Option<[u8; 16]>,
    rgba: [u8; 4],
    offset_s: i16,
    offset_t: i16,
    scale_s: i16,
    scale_t: i16,
    rotation: i16,
    bump: u8,
    fullbright: bool,
    shiny: u8,
    media_flags: u8,
    glow: u8,
}

impl ObjectFaceMaterialState {
    fn with_defaults(texture_id_bytes: Option<[u8; 16]>) -> Self {
        Self {
            texture_id_bytes,
            normal_id_bytes: None,
            specular_id_bytes: None,
            material_id_bytes: None,
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
        }
    }
}

#[derive(Debug, Clone, Default)]
struct ObjectFeedObjectState {
    last_seen_tick: u64,
    scale_centi: Option<[u16; 3]>,
    position_centi: Option<[i32; 3]>,
    mesh_id_bytes: Option<[u8; 16]>,
    texture_id_bytes: Option<[u8; 16]>,
    default_face_material: Option<ObjectFaceMaterialState>,
    face_material_overrides: BTreeMap<u8, ObjectFaceMaterialState>,
    object_id_bytes: Option<[u8; 16]>,
}

#[derive(Debug, Clone, Default)]
struct DecodedObjectFeedIngressObject {
    local_id: u32,
    scale_centi: Option<[u16; 3]>,
    position_centi: Option<[i32; 3]>,
    mesh_id_bytes: Option<[u8; 16]>,
    texture_id_bytes: Option<[u8; 16]>,
    default_face_material: Option<ObjectFaceMaterialState>,
    face_material_overrides: Vec<(u8, ObjectFaceMaterialState)>,
    object_id_bytes: Option<[u8; 16]>,
}

impl Connection {
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            state: ConnectionState::Disconnected,
            session: None,
            first_simulator_handshake_prerequisites: None,
            first_simulator_handshake_state: None,
            retained_probe_socket: None,
            first_simulator_handshake_send_diagnostics: Vec::new(),
            first_simulator_handshake_receive_diagnostics: Vec::new(),
            first_simulator_socket_diagnostics: Vec::new(),
            early_simulator_traffic_observations: Vec::new(),
            region_transition_control_observations: Vec::new(),
            simulator_payload_decode_summary: SimulatorPayloadDecodeSummary::default(),
            object_feed_objects: BTreeMap::new(),
            object_feed_recent_kills: Vec::new(),
            object_feed_tick: 0,
            next_first_simulator_packet_id: 1,
            pending_first_simulator_ack_ids: Vec::new(),
            pending_object_cache_miss_ids: Vec::new(),
            pending_region_handshake_reply_flags: None,
            region_handshake_reply_sent: false,
            require_observed_region_handshake_for_reply: false,
            last_bootstrap_sender_endpoint: None,
            last_region_handshake_sender_endpoint: None,
            continuity_summary: RegionContinuitySummary::default(),
            last_phase_change_at: Instant::now(),
        }
    }

    pub fn state(&self) -> ConnectionState {
        self.state
    }

    pub fn session(&self) -> Option<&Session> {
        self.session.as_ref()
    }

    pub fn config(&self) -> &ConnectionConfig {
        &self.config
    }

    pub fn first_simulator_handshake_state(&self) -> Option<&FirstSimulatorHandshakeState> {
        self.first_simulator_handshake_state.as_ref()
    }

    pub fn first_simulator_handshake_send_diagnostics(
        &self,
    ) -> &[FirstSimulatorHandshakeSendDiagnostic] {
        &self.first_simulator_handshake_send_diagnostics
    }

    pub fn first_simulator_handshake_receive_diagnostics(
        &self,
    ) -> &[FirstSimulatorHandshakeReceiveDiagnostic] {
        &self.first_simulator_handshake_receive_diagnostics
    }

    pub fn has_observed_region_handshake(&self) -> bool {
        self.first_simulator_handshake_receive_diagnostics
            .iter()
            .any(|diag| {
                diag.kind == FirstSimulatorInboundMessageKind::RegionHandshake
                    && diag.decode_source == FirstSimulatorInboundDecodeSource::PacketMessageNumber
            })
    }

    pub fn set_require_observed_region_handshake_for_reply(&mut self, require: bool) {
        self.require_observed_region_handshake_for_reply = require;
    }

    pub fn first_simulator_socket_diagnostics(&self) -> &[FirstSimulatorSocketDiagnostic] {
        &self.first_simulator_socket_diagnostics
    }

    pub fn summarize_first_simulator_socket_diagnostics(
        &self,
    ) -> FirstSimulatorSocketDiagnosticSummary {
        let mut summary = FirstSimulatorSocketDiagnosticSummary {
            events: self.first_simulator_socket_diagnostics.len(),
            ..FirstSimulatorSocketDiagnosticSummary::default()
        };
        let mut ports = BTreeSet::new();
        for diag in &self.first_simulator_socket_diagnostics {
            if let Some(port) = diag
                .local_addr
                .as_deref()
                .and_then(parse_socket_port_from_text)
            {
                ports.insert(port);
            }
            match diag.kind {
                FirstSimulatorSocketDiagnosticKind::ProbeBind => summary.probe_bind_events += 1,
                FirstSimulatorSocketDiagnosticKind::FreshBind => summary.fresh_bind_events += 1,
                FirstSimulatorSocketDiagnosticKind::RetainProbeSocket => {
                    summary.retained_probe_events += 1
                }
                FirstSimulatorSocketDiagnosticKind::ReuseRetainedProbeSocket => {
                    summary.reused_retained_probe_events += 1
                }
                FirstSimulatorSocketDiagnosticKind::Send => summary.send_events += 1,
                FirstSimulatorSocketDiagnosticKind::Receive => summary.receive_events += 1,
            }
        }
        summary.unique_local_ports = ports.into_iter().collect();
        summary.split_local_port_detected = summary.unique_local_ports.len() > 1;
        summary
    }

    pub fn early_simulator_traffic_observations(&self) -> &[EarlySimulatorTrafficObservation] {
        &self.early_simulator_traffic_observations
    }

    pub fn region_transition_control_observations(&self) -> &[RegionTransitionControlObservation] {
        &self.region_transition_control_observations
    }

    pub fn continuity_summary(&self) -> RegionContinuitySummary {
        let mut summary = self.continuity_summary.clone();
        summary.phase_age_ms = self.last_phase_change_at.elapsed().as_millis() as u64;
        summary
    }

    /// Execute a bounded continuity probe against currently available simulator capabilities.
    pub async fn execute_continuity_probe(&self) -> Result<(), ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }

        let seed_caps = self.fetch_seed_capabilities().await?;
        if let Some(url) = seed_caps.entries.get("SimulatorFeatures")
            && !url.trim().is_empty()
        {
            let _ = self.fetch_simulator_features_once(url).await?;
            return Ok(());
        }
        if let Some(url) = seed_caps.entries.get("MapLayer")
            && !url.trim().is_empty()
        {
            let _ = self.fetch_map_layer_once(url).await?;
            return Ok(());
        }

        Err(ConnectionError::MissingCapability(String::from(
            "SimulatorFeatures/MapLayer",
        )))
    }

    pub fn record_region_continuity_observation(&mut self, summary: RegionContinuitySummary) {
        // Guard: do not move backwards in handoff phase priority unless it's a reset to None
        let current_priority = self.continuity_summary.phase.priority();
        let next_priority = summary.phase.priority();
        if next_priority < current_priority && summary.phase != HandoffPhase::None {
            return;
        }

        if self.continuity_summary.phase != summary.phase {
            self.last_phase_change_at = Instant::now();
        }

        let mut neighbors = summary.neighbors;
        if neighbors.len() > MAX_CONTINUITY_NEIGHBORS {
            neighbors.truncate(MAX_CONTINUITY_NEIGHBORS);
        }

        let phase_age_ms = self.last_phase_change_at.elapsed().as_millis() as u64;

        // Inherit coordinates if not provided in the new summary
        let active_region_coords = summary
            .active_region_coords
            .or(self.continuity_summary.active_region_coords);
        let previous_region_coords = summary
            .previous_region_coords
            .or(self.continuity_summary.previous_region_coords);

        self.continuity_summary = RegionContinuitySummary {
            phase: summary.phase,
            outcome: summary.outcome,
            reason: summary.reason,
            phase_age_ms,
            last_probe_result: summary.last_probe_result,
            last_probe_time_unix_ms: summary.last_probe_time_unix_ms,
            active_region_coords,
            previous_region_coords,
            neighbors,
        };
    }

    fn continuity_summary_with_phase(&self, phase: HandoffPhase) -> RegionContinuitySummary {
        RegionContinuitySummary {
            phase,
            outcome: self.continuity_summary.outcome,
            reason: self.continuity_summary.reason,
            phase_age_ms: 0,
            last_probe_result: self.continuity_summary.last_probe_result,
            last_probe_time_unix_ms: self.continuity_summary.last_probe_time_unix_ms,
            active_region_coords: self.continuity_summary.active_region_coords,
            previous_region_coords: self.continuity_summary.previous_region_coords,
            neighbors: self.continuity_summary.neighbors.clone(),
        }
    }

    pub fn simulator_payload_decode_summary(&self) -> &SimulatorPayloadDecodeSummary {
        &self.simulator_payload_decode_summary
    }

    pub fn summarize_first_simulator_ack_forensics(&self) -> FirstSimulatorAckForensicsSummary {
        let outbound_appended_ack_sends = self
            .first_simulator_handshake_send_diagnostics
            .iter()
            .filter(|diag| !diag.appended_ack_ids.is_empty())
            .count();
        let outbound_appended_ack_ids_last = self
            .first_simulator_handshake_send_diagnostics
            .iter()
            .rev()
            .find(|diag| !diag.appended_ack_ids.is_empty())
            .map(|diag| diag.appended_ack_ids.clone())
            .unwrap_or_default();
        let explicit_packet_ack_receives = self
            .first_simulator_handshake_receive_diagnostics
            .iter()
            .filter(|diag| diag.kind == FirstSimulatorInboundMessageKind::PacketAck)
            .count();
        let first_packet_ack_receive_index = self
            .first_simulator_handshake_receive_diagnostics
            .iter()
            .find(|diag| diag.kind == FirstSimulatorInboundMessageKind::PacketAck)
            .map(|diag| diag.observation_index);

        FirstSimulatorAckForensicsSummary {
            pending_ack_count: self.pending_first_simulator_ack_ids.len(),
            pending_ack_ids_preview: self
                .pending_first_simulator_ack_ids
                .iter()
                .take(8)
                .copied()
                .collect(),
            outbound_appended_ack_sends,
            outbound_appended_ack_ids_last,
            explicit_packet_ack_receives,
            first_packet_ack_receive_index,
        }
    }

    pub fn summarize_first_simulator_receive_forensics(
        &self,
    ) -> FirstSimulatorReceiveForensicsSummary {
        let mut typed_kind_counts: BTreeMap<FirstSimulatorInboundMessageKind, usize> =
            BTreeMap::new();
        let mut raw_packet_message_numbers: BTreeMap<u32, usize> = BTreeMap::new();
        let mut unclassified_packet_message_numbers: BTreeMap<u32, usize> = BTreeMap::new();

        for diag in &self.first_simulator_handshake_receive_diagnostics {
            *typed_kind_counts.entry(diag.kind).or_default() += 1;
            if let Some(message_number) = diag.packet_message_number {
                *raw_packet_message_numbers
                    .entry(message_number)
                    .or_default() += 1;
                if diag.scope == FirstSimulatorInboundTrafficScope::Unknown
                    || diag.kind == FirstSimulatorInboundMessageKind::Irrelevant
                {
                    *unclassified_packet_message_numbers
                        .entry(message_number)
                        .or_default() += 1;
                }
            }
        }

        FirstSimulatorReceiveForensicsSummary {
            receive_observations: self.first_simulator_handshake_receive_diagnostics.len(),
            typed_kind_counts: typed_kind_counts.into_iter().collect(),
            raw_packet_message_numbers: raw_packet_message_numbers.into_iter().collect(),
            unclassified_packet_message_numbers: unclassified_packet_message_numbers
                .into_iter()
                .collect(),
        }
    }

    pub fn summarize_first_simulator_startup_timeline(
        &self,
    ) -> FirstSimulatorStartupTimelineSummary {
        let first_region_handshake_index = self
            .first_simulator_handshake_receive_diagnostics
            .iter()
            .find(|diag| diag.kind == FirstSimulatorInboundMessageKind::RegionHandshake)
            .map(|diag| diag.observation_index);
        let first_region_handshake_reply_index = self
            .first_simulator_handshake_receive_diagnostics
            .iter()
            .find(|diag| diag.kind == FirstSimulatorInboundMessageKind::RegionHandshakeReply)
            .map(|diag| diag.observation_index);
        let first_agent_movement_complete_index = self
            .first_simulator_handshake_receive_diagnostics
            .iter()
            .find(|diag| diag.kind == FirstSimulatorInboundMessageKind::AgentMovementComplete)
            .map(|diag| diag.observation_index);
        let first_packet_ack_index = self
            .first_simulator_handshake_receive_diagnostics
            .iter()
            .find(|diag| diag.kind == FirstSimulatorInboundMessageKind::PacketAck)
            .map(|diag| diag.observation_index);
        let first_camera_constraint_index = self
            .first_simulator_handshake_receive_diagnostics
            .iter()
            .find(|diag| diag.kind == FirstSimulatorInboundMessageKind::CameraConstraint)
            .map(|diag| diag.observation_index);
        let first_generic_message_index = self
            .first_simulator_handshake_receive_diagnostics
            .iter()
            .find(|diag| diag.kind == FirstSimulatorInboundMessageKind::GenericMessage)
            .map(|diag| diag.observation_index);
        let first_object_update_index = self
            .first_simulator_handshake_receive_diagnostics
            .iter()
            .find(|diag| {
                matches!(
                    diag.kind,
                    FirstSimulatorInboundMessageKind::ObjectUpdate
                        | FirstSimulatorInboundMessageKind::ObjectUpdateCompressed
                        | FirstSimulatorInboundMessageKind::ObjectUpdateCached
                        | FirstSimulatorInboundMessageKind::ImprovedTerseObjectUpdate
                )
            })
            .map(|diag| diag.observation_index);

        FirstSimulatorStartupTimelineSummary {
            first_region_handshake_index,
            first_region_handshake_reply_index,
            first_agent_movement_complete_index,
            first_packet_ack_index,
            first_camera_constraint_index,
            first_generic_message_index,
            first_object_update_index,
        }
    }

    pub fn summarize_first_simulator_startup_transcript(
        &self,
        tail_len: usize,
    ) -> FirstSimulatorStartupTranscriptSummary {
        let send_events = self
            .first_simulator_handshake_send_diagnostics
            .iter()
            .rev()
            .take(tail_len)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|diag| {
                let label = diag
                    .packet_message_number
                    .map(first_simulator_message_label)
                    .unwrap_or_else(|| String::from("unknown"));
                let ack_suffix = if diag.appended_ack_ids.is_empty() {
                    String::new()
                } else {
                    format!(" ack={}", format_u32_hex_list(&diag.appended_ack_ids))
                };
                format!(
                    "{}:{}#{}",
                    first_simulator_action_label(diag.action),
                    label,
                    diag.packet_id
                ) + &ack_suffix
            })
            .collect();
        let receive_events = self
            .first_simulator_handshake_receive_diagnostics
            .iter()
            .rev()
            .take(tail_len)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|diag| {
                let label = match diag.packet_message_number {
                    Some(message_number) => first_simulator_message_label(message_number),
                    None => format!("{:?}", diag.kind),
                };
                format!("{}:{}", diag.observation_index, label)
            })
            .collect();

        FirstSimulatorStartupTranscriptSummary {
            send_events,
            receive_events,
        }
    }

    pub fn summarize_first_simulator_startup_interest_gate(
        &self,
    ) -> FirstSimulatorStartupInterestGateSummary {
        let mut throttle = None;
        let mut update = None;
        let mut height_width = None;

        for (order_index, diag) in self
            .first_simulator_handshake_send_diagnostics
            .iter()
            .enumerate()
        {
            let Some(message_number) = diag.packet_message_number else {
                continue;
            };
            if message_number == lludp_low_frequency_message_number(LLUDP_AGENT_THROTTLE_LOW_ID)
                && throttle.is_none()
            {
                throttle = Some(FirstSimulatorStartupInterestSendEvidence {
                    message: String::from("AgentThrottle"),
                    order_index,
                    packet_id: diag.packet_id,
                });
            } else if message_number == u32::from(LLUDP_AGENT_UPDATE_HIGH_ID) && update.is_none() {
                update = Some(FirstSimulatorStartupInterestSendEvidence {
                    message: String::from("AgentUpdate"),
                    order_index,
                    packet_id: diag.packet_id,
                });
            } else if message_number
                == lludp_low_frequency_message_number(LLUDP_AGENT_HEIGHT_WIDTH_LOW_ID)
                && height_width.is_none()
            {
                height_width = Some(FirstSimulatorStartupInterestSendEvidence {
                    message: String::from("AgentHeightWidth"),
                    order_index,
                    packet_id: diag.packet_id,
                });
            }
        }

        let mut required = Vec::new();
        let mut missing = Vec::new();
        if let Some(entry) = throttle {
            required.push(entry);
        } else {
            missing.push(String::from("AgentThrottle"));
        }
        if let Some(entry) = update {
            required.push(entry);
        } else {
            missing.push(String::from("AgentUpdate"));
        }
        if let Some(entry) = height_width {
            required.push(entry);
        } else {
            missing.push(String::from("AgentHeightWidth"));
        }

        FirstSimulatorStartupInterestGateSummary {
            passed: missing.is_empty(),
            required,
            missing,
        }
    }

    pub fn summarize_seed_capability_inventory(
        &self,
        map: &SeedCapabilityMap,
    ) -> Vec<SeedCapabilityInventoryEntry> {
        summarize_seed_capability_inventory(map)
    }

    #[allow(clippy::too_many_arguments)]
    fn object_feed_upsert(
        &mut self,
        local_id: u32,
        scale_centi: Option<[u16; 3]>,
        position_centi: Option<[i32; 3]>,
        mesh_id_bytes: Option<[u8; 16]>,
        texture_id_bytes: Option<[u8; 16]>,
        default_face_material: Option<ObjectFaceMaterialState>,
        face_material_overrides: &[(u8, ObjectFaceMaterialState)],
        object_id_bytes: Option<[u8; 16]>,
    ) {
        if local_id == 0 {
            return;
        }
        self.object_feed_tick = self.object_feed_tick.saturating_add(1);
        let entry = self.object_feed_objects.entry(local_id).or_default();
        entry.last_seen_tick = self.object_feed_tick;
        if let Some(scale) = scale_centi {
            entry.scale_centi = Some(scale);
        }
        if let Some(pos) = position_centi {
            entry.position_centi = Some(pos);
        }
        if let Some(mesh) = mesh_id_bytes {
            entry.mesh_id_bytes = Some(mesh);
        }
        if let Some(tex) = texture_id_bytes {
            entry.texture_id_bytes = Some(tex);
        }
        if let Some(mat) = default_face_material {
            entry.default_face_material = Some(mat);
        }
        if !face_material_overrides.is_empty() {
            for (face_id, material) in face_material_overrides {
                entry
                    .face_material_overrides
                    .insert(*face_id, material.clone());
            }
        }
        if let Some(obj) = object_id_bytes {
            entry.object_id_bytes = Some(obj);
        }

        if self.object_feed_objects.len() > MAX_OBJECT_FEED_OBJECTS
            && let Some((evict_id, _)) = self
                .object_feed_objects
                .iter()
                .map(|(id, state)| (*id, state.last_seen_tick))
                .min_by_key(|(id, tick)| (*tick, *id))
        {
            self.object_feed_objects.remove(&evict_id);
            self.simulator_payload_decode_summary.object_feed_evicted += 1;
        }
    }

    fn object_feed_kill(&mut self, local_id: u32) {
        if local_id == 0 {
            return;
        }
        self.object_feed_objects.remove(&local_id);
        self.object_feed_recent_kills.push(local_id);
        if self.object_feed_recent_kills.len() > MAX_OBJECT_FEED_RECENT_KILLS {
            let trim_from = self.object_feed_recent_kills.len() - MAX_OBJECT_FEED_RECENT_KILLS;
            self.object_feed_recent_kills.drain(0..trim_from);
        }
    }

    fn refresh_object_feed_summary_export(&mut self) {
        self.simulator_payload_decode_summary
            .object_feed_total_objects = self.object_feed_objects.len();
        self.simulator_payload_decode_summary
            .object_feed_state_mesh_objects = self
            .object_feed_objects
            .values()
            .filter(|state| {
                state
                    .mesh_id_bytes
                    .is_some_and(|id| !is_null_uuid_bytes(id))
            })
            .count();
        self.simulator_payload_decode_summary
            .object_feed_export_truncated =
            self.object_feed_objects.len() > MAX_OBJECT_FEED_EXPORT_OBJECTS;
        let export_truncated = self
            .simulator_payload_decode_summary
            .object_feed_export_truncated;

        let mut entries: Vec<(u32, ObjectFeedObjectState)> = self
            .object_feed_objects
            .iter()
            .map(|(id, state)| (*id, state.clone()))
            .collect();
        if export_truncated {
            entries.sort_by(|a, b| {
                b.1.mesh_id_bytes
                    .is_some_and(|id| !is_null_uuid_bytes(id))
                    .cmp(&a.1.mesh_id_bytes.is_some_and(|id| !is_null_uuid_bytes(id)))
                    .then_with(|| b.1.last_seen_tick.cmp(&a.1.last_seen_tick))
                    .then_with(|| a.0.cmp(&b.0))
            });
        } else {
            entries.sort_by(|a, b| a.0.cmp(&b.0));
        }
        entries.truncate(MAX_OBJECT_FEED_EXPORT_OBJECTS);
        let export: Vec<DecodedObjectFeedObject> = entries
            .into_iter()
            .map(|(local_id, state)| DecodedObjectFeedObject {
                local_id,
                scale_centi: state.scale_centi,
                position_centi: state.position_centi,
                mesh_id: state
                    .mesh_id_bytes
                    .map(format_uuid_bytes)
                    .filter(|id| !is_null_uuid(id)),
                texture_id: state
                    .texture_id_bytes
                    .map(format_uuid_bytes)
                    .filter(|id| !is_null_uuid(id)),
                default_face_material: state
                    .default_face_material
                    .as_ref()
                    .map(format_object_face_material),
                face_material_overrides: state
                    .face_material_overrides
                    .iter()
                    .map(|(face_id, mat)| format_object_face_material_with_face(*face_id, mat))
                    .collect(),
                object_id: state
                    .object_id_bytes
                    .map(format_uuid_bytes)
                    .filter(|id| !is_null_uuid(id)),
            })
            .collect();
        self.simulator_payload_decode_summary
            .object_feed_export_objects = export.len();
        self.simulator_payload_decode_summary
            .object_feed_export_mesh_objects =
            export.iter().filter(|obj| obj.mesh_id.is_some()).count();
        self.simulator_payload_decode_summary.object_feed_objects = export;
        self.simulator_payload_decode_summary
            .object_feed_recent_kills = self.object_feed_recent_kills.clone();
    }

    pub fn summarize_early_simulator_traffic(&self) -> EarlySimulatorTrafficSummary {
        let mut summary = EarlySimulatorTrafficSummary::default();
        for observation in &self.early_simulator_traffic_observations {
            summary.observations += 1;
            match observation.kind {
                EarlySimulatorTrafficKind::HealthMessage => summary.health_message += 1,
                EarlySimulatorTrafficKind::LayerData => summary.layer_data += 1,
                EarlySimulatorTrafficKind::ChatFromSimulator => summary.chat_from_simulator += 1,
                EarlySimulatorTrafficKind::SimulatorViewerTimeMessage => {
                    summary.simulator_viewer_time_message += 1
                }
                EarlySimulatorTrafficKind::OnlineNotification => summary.online_notification += 1,
                EarlySimulatorTrafficKind::ViewerEffect => summary.viewer_effect += 1,
                EarlySimulatorTrafficKind::CoarseLocationUpdate => {
                    summary.coarse_location_update += 1
                }
                EarlySimulatorTrafficKind::AttachedSound => summary.attached_sound += 1,
            }
        }
        summary
    }

    pub fn summarize_region_transition_control(&self) -> RegionTransitionControlSummary {
        let mut summary = RegionTransitionControlSummary::default();
        for observation in &self.region_transition_control_observations {
            summary.observations += 1;
            match observation.kind {
                RegionTransitionControlKind::CrossedRegion => summary.crossed_region += 1,
                RegionTransitionControlKind::ConfirmEnableSimulator => {
                    summary.confirm_enable_simulator += 1
                }
            }
        }
        summary.not_seen_in_run = summary.observations == 0;
        summary
    }

    pub async fn connect(&mut self) -> Result<(), ConnectionError> {
        if self.state != ConnectionState::Disconnected {
            return Err(ConnectionError::InvalidState(self.state));
        }

        self.state = ConnectionState::Connecting;

        // Placeholder async hook to keep call sites and lifecycle future-proof.
        tokio::task::yield_now().await;

        self.state = ConnectionState::Connected;
        Ok(())
    }

    pub async fn login(&mut self, request: LoginRequest) -> Result<(), ConnectionError> {
        if self.state != ConnectionState::Connected {
            return Err(ConnectionError::InvalidState(self.state));
        }

        // Placeholder async hook for future grid login flow.
        tokio::task::yield_now().await;

        self.session = Some(Session {
            account_name: format!("{} {}", request.first_name, request.last_name),
            session_token: String::from("placeholder-session-token"),
            seed_capability: None,
        });
        self.first_simulator_handshake_prerequisites = None;
        self.first_simulator_handshake_state = None;
        self.retained_probe_socket = None;
        self.first_simulator_handshake_send_diagnostics.clear();
        self.first_simulator_handshake_receive_diagnostics.clear();
        self.first_simulator_socket_diagnostics.clear();
        self.early_simulator_traffic_observations.clear();
        self.region_transition_control_observations.clear();
        self.simulator_payload_decode_summary = SimulatorPayloadDecodeSummary::default();
        self.object_feed_objects.clear();
        self.object_feed_recent_kills.clear();
        self.object_feed_tick = 0;
        self.next_first_simulator_packet_id = 1;
        self.pending_first_simulator_ack_ids.clear();
        self.pending_object_cache_miss_ids.clear();
        self.pending_region_handshake_reply_flags = None;
        self.region_handshake_reply_sent = false;
        self.last_bootstrap_sender_endpoint = None;
        self.last_region_handshake_sender_endpoint = None;
        self.continuity_summary = RegionContinuitySummary::default();
        self.state = ConnectionState::LoggedIn;
        Ok(())
    }

    pub async fn login_with_adapter<A: GridLoginAdapter>(
        &mut self,
        adapter: &A,
        intent: LoginIntent,
    ) -> Result<GridLoginResult, ConnectionError> {
        if self.state != ConnectionState::Connected {
            return Err(ConnectionError::InvalidState(self.state));
        }

        self.login_with_trace(adapter, intent)
            .await
            .map(|(result, _trace)| result)
    }

    pub async fn login_with_trace<A: GridLoginAdapter>(
        &mut self,
        adapter: &A,
        intent: LoginIntent,
    ) -> Result<(GridLoginResult, LoginTrace), ConnectionError> {
        self.login_with_trace_using_wire(adapter, intent, self.config.wire_format)
            .await
    }

    pub async fn login_with_trace_with_fallback<A: GridLoginAdapter>(
        &mut self,
        adapter: &A,
        intent: LoginIntent,
    ) -> Result<(GridLoginResult, LoginTrace, LoginFallbackOutcome), ConnectionError> {
        let primary_wire_format = self.config.wire_format;
        let (primary_result, primary_trace) = self
            .login_with_trace_using_wire(adapter, intent.clone(), primary_wire_format)
            .await?;
        if let Some(classified_reason) = classify_login_request_shape_failure(
            primary_wire_format,
            &self.config.endpoint,
            &primary_trace.final_response,
        ) {
            let fallback_wire_format = LoginWireFormat::XmlRpc;
            let (fallback_result, fallback_trace) = self
                .login_with_trace_using_wire(adapter, intent, fallback_wire_format)
                .await?;
            return Ok((
                fallback_result,
                fallback_trace,
                LoginFallbackOutcome {
                    primary_wire_format,
                    fallback_used: true,
                    final_wire_format: fallback_wire_format,
                    classified_reason: Some(classified_reason),
                },
            ));
        }

        Ok((
            primary_result,
            primary_trace,
            LoginFallbackOutcome {
                primary_wire_format,
                fallback_used: false,
                final_wire_format: primary_wire_format,
                classified_reason: None,
            },
        ))
    }

    async fn login_with_trace_using_wire<A: GridLoginAdapter>(
        &mut self,
        adapter: &A,
        intent: LoginIntent,
        wire_format: LoginWireFormat,
    ) -> Result<(GridLoginResult, LoginTrace), ConnectionError> {
        if self.state != ConnectionState::Connected {
            return Err(ConnectionError::InvalidState(self.state));
        }

        let request = adapter.shape_login_request(&intent);
        let mut endpoint = self.config.endpoint.clone();
        let mut http_method = Method::POST;
        let mut redirects = 0;
        let codec = wire_format.codec();
        let mut trace = LoginTrace {
            initial_request: trace_request(&request),
            redirect_steps: Vec::new(),
            final_response: LoginTraceResponse {
                login: None,
                reason: None,
                message: None,
            },
            final_result: LoginTraceFinalResult {
                outcome: String::from("unknown"),
                reason: None,
                message: None,
            },
        };

        loop {
            let response = self
                .http_transport_login(&request, &endpoint, http_method.clone(), codec.as_ref())
                .await?;
            trace.final_response = trace_response(&response);
            let result = adapter.interpret_login_response(&response)?;
            let result_trace = trace_result(&result, &trace.final_response);

            match result {
                GridLoginResult::Redirect {
                    next_url,
                    next_method,
                } => {
                    redirects += 1;
                    if redirects >= MAX_LOGIN_REDIRECTS {
                        return Err(ConnectionError::RedirectLimitExceeded(MAX_LOGIN_REDIRECTS));
                    }

                    let parsed_method =
                        Method::from_bytes(next_method.as_bytes()).map_err(|_| {
                            ConnectionError::UnsupportedRedirectMethod(next_method.clone())
                        })?;
                    if parsed_method != Method::POST {
                        return Err(ConnectionError::UnsupportedRedirectMethod(next_method));
                    }

                    trace.redirect_steps.push(LoginTraceRedirect {
                        url: endpoint.clone(),
                        method: http_method.as_str().to_string(),
                        result_type: String::from("redirect"),
                    });

                    endpoint = next_url;
                    http_method = parsed_method;
                }
                GridLoginResult::Success(bootstrap) => {
                    self.apply_bootstrap_session(&bootstrap);
                    self.state = ConnectionState::LoggedIn;
                    trace.final_result = result_trace;
                    return Ok((GridLoginResult::Success(bootstrap), trace));
                }
                terminal => {
                    trace.final_result = result_trace;
                    return Ok((terminal, trace));
                }
            }
        }
    }

    pub async fn disconnect(&mut self) -> Result<(), ConnectionError> {
        if self.state == ConnectionState::Disconnected {
            return Ok(());
        }

        // Placeholder async hook for future transport teardown.
        tokio::task::yield_now().await;

        self.session = None;
        self.first_simulator_handshake_prerequisites = None;
        self.first_simulator_handshake_state = None;
        self.retained_probe_socket = None;
        self.first_simulator_handshake_send_diagnostics.clear();
        self.first_simulator_handshake_receive_diagnostics.clear();
        self.first_simulator_socket_diagnostics.clear();
        self.early_simulator_traffic_observations.clear();
        self.region_transition_control_observations.clear();
        self.simulator_payload_decode_summary = SimulatorPayloadDecodeSummary::default();
        self.object_feed_objects.clear();
        self.object_feed_recent_kills.clear();
        self.object_feed_tick = 0;
        self.next_first_simulator_packet_id = 1;
        self.pending_first_simulator_ack_ids.clear();
        self.pending_object_cache_miss_ids.clear();
        self.pending_region_handshake_reply_flags = None;
        self.region_handshake_reply_sent = false;
        self.last_bootstrap_sender_endpoint = None;
        self.last_region_handshake_sender_endpoint = None;
        self.state = ConnectionState::Disconnected;
        Ok(())
    }

    pub fn begin_first_simulator_handshake_scaffold(
        &mut self,
    ) -> Result<&FirstSimulatorHandshakeState, ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }

        let prerequisites = self
            .first_simulator_handshake_prerequisites
            .clone()
            .ok_or(ConnectionError::MissingFirstSimulatorHandshakePrerequisites)?;
        self.first_simulator_handshake_state = Some(FirstSimulatorHandshakeState {
            stage: FirstSimulatorHandshakeStage::BootstrapPrerequisitesReady,
            prerequisites,
        });
        Ok(self
            .first_simulator_handshake_state
            .as_ref()
            .expect("handshake state just initialized"))
    }

    pub fn advance_first_simulator_handshake_scaffold(
        &mut self,
        to: FirstSimulatorHandshakeStage,
    ) -> Result<&FirstSimulatorHandshakeState, ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }

        let current = self
            .first_simulator_handshake_state
            .as_mut()
            .ok_or(ConnectionError::FirstSimulatorHandshakeNotInitialized)?;

        let expected_next = match current.stage {
            FirstSimulatorHandshakeStage::BootstrapPrerequisitesReady => {
                FirstSimulatorHandshakeStage::FirstRegionTargetKnown
            }
            FirstSimulatorHandshakeStage::FirstRegionTargetKnown => {
                FirstSimulatorHandshakeStage::UseCircuitCode
            }
            FirstSimulatorHandshakeStage::UseCircuitCode => {
                FirstSimulatorHandshakeStage::CompleteAgentMovement
            }
            FirstSimulatorHandshakeStage::CompleteAgentMovement => {
                FirstSimulatorHandshakeStage::WaitingForAgentMovementComplete
            }
            FirstSimulatorHandshakeStage::WaitingForAgentMovementComplete => {
                FirstSimulatorHandshakeStage::AgentMovementComplete
            }
            FirstSimulatorHandshakeStage::AgentMovementComplete => {
                return Err(ConnectionError::InvalidFirstSimulatorHandshakeTransition {
                    from: current.stage,
                    to,
                });
            }
        };

        if to != expected_next {
            return Err(ConnectionError::InvalidFirstSimulatorHandshakeTransition {
                from: current.stage,
                to,
            });
        }

        current.stage = to;
        Ok(self
            .first_simulator_handshake_state
            .as_ref()
            .expect("handshake state should remain initialized"))
    }

    pub async fn send_first_simulator_use_circuit_code(
        &mut self,
    ) -> Result<&FirstSimulatorHandshakeState, ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }

        let current_stage = self
            .first_simulator_handshake_state
            .as_ref()
            .ok_or(ConnectionError::FirstSimulatorHandshakeNotInitialized)?
            .stage;
        if current_stage == FirstSimulatorHandshakeStage::BootstrapPrerequisitesReady {
            self.advance_first_simulator_handshake_scaffold(
                FirstSimulatorHandshakeStage::FirstRegionTargetKnown,
            )?;
        } else if current_stage != FirstSimulatorHandshakeStage::FirstRegionTargetKnown {
            return Err(ConnectionError::InvalidFirstSimulatorHandshakeTransition {
                from: current_stage,
                to: FirstSimulatorHandshakeStage::UseCircuitCode,
            });
        }

        let prerequisites = self
            .first_simulator_handshake_state
            .as_ref()
            .expect("handshake state must be initialized")
            .prerequisites
            .clone();
        let payload = encode_first_simulator_use_circuit_code_payload(
            &prerequisites,
            self.next_first_simulator_packet_id(),
        )?;
        self.send_first_simulator_handshake_datagram(
            FirstSimulatorHandshakeAction::UseCircuitCode,
            &prerequisites.target,
            &payload,
        )
        .await?;

        self.advance_first_simulator_handshake_scaffold(
            FirstSimulatorHandshakeStage::UseCircuitCode,
        )
    }

    pub async fn send_first_simulator_complete_agent_movement(
        &mut self,
    ) -> Result<&FirstSimulatorHandshakeState, ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }

        let current_stage = self
            .first_simulator_handshake_state
            .as_ref()
            .ok_or(ConnectionError::FirstSimulatorHandshakeNotInitialized)?
            .stage;
        if current_stage != FirstSimulatorHandshakeStage::UseCircuitCode {
            return Err(ConnectionError::InvalidFirstSimulatorHandshakeTransition {
                from: current_stage,
                to: FirstSimulatorHandshakeStage::CompleteAgentMovement,
            });
        }

        self.advance_first_simulator_handshake_scaffold(
            FirstSimulatorHandshakeStage::CompleteAgentMovement,
        )?;

        let prerequisites = self
            .first_simulator_handshake_state
            .as_ref()
            .expect("handshake state must be initialized")
            .prerequisites
            .clone();
        let payload = encode_first_simulator_complete_agent_movement_payload(
            &prerequisites,
            self.next_first_simulator_packet_id(),
        )?;
        self.send_first_simulator_handshake_datagram(
            FirstSimulatorHandshakeAction::CompleteAgentMovement,
            &prerequisites.target,
            &payload,
        )
        .await?;

        self.advance_first_simulator_handshake_scaffold(
            FirstSimulatorHandshakeStage::WaitingForAgentMovementComplete,
        )
    }

    pub fn mark_first_simulator_agent_movement_complete_received(
        &mut self,
    ) -> Result<&FirstSimulatorHandshakeState, ConnectionError> {
        self.advance_first_simulator_handshake_scaffold(
            FirstSimulatorHandshakeStage::AgentMovementComplete,
        )
    }

    pub fn observe_first_simulator_inbound_payload(
        &mut self,
        payload: &[u8],
    ) -> Result<FirstSimulatorInboundClassification, ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }

        let classification = classify_first_simulator_inbound_message(payload);
        self.collect_first_simulator_reliable_ack(payload);
        let stage_before = self
            .first_simulator_handshake_state
            .as_ref()
            .map(|s| s.stage);
        let mut advanced_stage = false;

        if classification.kind == FirstSimulatorInboundMessageKind::AgentMovementComplete
            && classification.decode_source
                == FirstSimulatorInboundDecodeSource::PacketMessageNumber
            && stage_before == Some(FirstSimulatorHandshakeStage::WaitingForAgentMovementComplete)
        {
            self.mark_first_simulator_agent_movement_complete_received()?;
            advanced_stage = true;
        }

        let stage_after = self
            .first_simulator_handshake_state
            .as_ref()
            .map(|s| s.stage);
        let observation_index = self.first_simulator_handshake_receive_diagnostics.len() + 1;
        self.first_simulator_handshake_receive_diagnostics.push(
            FirstSimulatorHandshakeReceiveDiagnostic {
                observation_index,
                kind: classification.kind,
                scope: classification.scope,
                payload_len: payload.len(),
                stage_before,
                stage_after,
                advanced_stage,
                signal: classification.signal.clone(),
                decode_source: classification.decode_source,
                packet_message_number: classification.packet_message_number,
            },
        );

        if let Some(kind) = to_early_simulator_traffic_kind(classification.kind) {
            self.early_simulator_traffic_observations
                .push(EarlySimulatorTrafficObservation {
                    observation_index,
                    kind,
                    packet_message_number: classification.packet_message_number,
                    payload_len: payload.len(),
                    signal: classification.signal.clone(),
                });
        }
        if classification.kind == FirstSimulatorInboundMessageKind::CoarseLocationUpdate
            && classification.decode_source
                == FirstSimulatorInboundDecodeSource::PacketMessageNumber
            && let Some(decoded) = decode_coarse_location_update(payload)
        {
            self.simulator_payload_decode_summary
                .coarse_location_updates += 1;
            self.simulator_payload_decode_summary
                .coarse_location_last_count = Some(decoded.location_count);
            self.simulator_payload_decode_summary
                .coarse_location_last_first = decoded.first_location;
            self.simulator_payload_decode_summary
                .coarse_location_last_second = decoded.second_location;
            self.simulator_payload_decode_summary
                .coarse_location_last_third = decoded.third_location;
            self.simulator_payload_decode_summary
                .coarse_location_last_avatars = decoded.avatars;
            self.simulator_payload_decode_summary
                .coarse_location_last_self_index = decoded.self_index;
        }
        if classification.kind == FirstSimulatorInboundMessageKind::RegionHandshake
            && classification.decode_source
                == FirstSimulatorInboundDecodeSource::PacketMessageNumber
            && let Some(decoded) = decode_region_handshake(payload)
        {
            self.simulator_payload_decode_summary
                .region_handshake_updates += 1;
            self.simulator_payload_decode_summary
                .region_handshake_last_sim_name = decoded.sim_name;
            self.pending_region_handshake_reply_flags = Some(decoded.region_flags);
        }
        if classification.kind == FirstSimulatorInboundMessageKind::AgentMovementComplete
            && classification.decode_source
                == FirstSimulatorInboundDecodeSource::PacketMessageNumber
            && let Some(decoded) = decode_agent_movement_complete(payload)
        {
            self.simulator_payload_decode_summary
                .agent_movement_complete_updates += 1;
            self.simulator_payload_decode_summary
                .agent_movement_complete_last_position = Some(decoded.position);
            self.simulator_payload_decode_summary
                .agent_movement_complete_last_region_handle = Some(decoded.region_handle);

            // Continuity: Movement complete signals the completion of the handoff.
            let coords = [
                (decoded.region_handle >> 32) as u32,
                (decoded.region_handle & 0xFFFFFFFF) as u32,
            ];
            let mut continuity_update = self.continuity_summary_with_phase(HandoffPhase::Completed);
            if continuity_update.active_region_coords != Some(coords) {
                continuity_update.previous_region_coords = continuity_update.active_region_coords;
                continuity_update.active_region_coords = Some(coords);
            }
            self.record_region_continuity_observation(continuity_update);
        }
        if classification.kind == FirstSimulatorInboundMessageKind::HealthMessage
            && classification.decode_source
                == FirstSimulatorInboundDecodeSource::PacketMessageNumber
            && let Some(decoded) = decode_health_message(payload)
        {
            self.simulator_payload_decode_summary.health_updates += 1;
            self.simulator_payload_decode_summary
                .health_last_basis_points = Some(health_to_basis_points(decoded.health));
        }
        if classification.kind == FirstSimulatorInboundMessageKind::SimulatorViewerTimeMessage
            && classification.decode_source
                == FirstSimulatorInboundDecodeSource::PacketMessageNumber
            && let Some(decoded) = decode_simulator_viewer_time_message(payload)
        {
            self.simulator_payload_decode_summary
                .simulator_viewer_time_updates += 1;
            self.simulator_payload_decode_summary
                .simulator_viewer_time_last_body_len = Some(decoded.body_len);
            self.simulator_payload_decode_summary
                .simulator_viewer_time_last_signature = decoded.signature;
        }

        if classification.decode_source == FirstSimulatorInboundDecodeSource::PacketMessageNumber {
            match classification.kind {
                FirstSimulatorInboundMessageKind::ObjectUpdate => {
                    self.simulator_payload_decode_summary
                        .object_feed_update_messages += 1;
                    self.simulator_payload_decode_summary
                        .object_feed_object_update_messages += 1;
                    if let Some(objects) = decode_object_update_ids_and_scales(payload) {
                        self.simulator_payload_decode_summary
                            .object_feed_object_update_mesh_hits += objects
                            .iter()
                            .filter(|obj| obj.mesh_id_bytes.is_some())
                            .count();
                        for obj in objects {
                            self.object_feed_upsert(
                                obj.local_id,
                                obj.scale_centi,
                                obj.position_centi,
                                obj.mesh_id_bytes,
                                obj.texture_id_bytes,
                                obj.default_face_material.clone(),
                                &obj.face_material_overrides,
                                obj.object_id_bytes,
                            );
                        }
                    } else {
                        self.simulator_payload_decode_summary
                            .object_feed_decode_dropped += 1;
                    }
                    self.refresh_object_feed_summary_export();
                }
                FirstSimulatorInboundMessageKind::ObjectUpdateCompressed => {
                    self.simulator_payload_decode_summary
                        .object_feed_update_messages += 1;
                    self.simulator_payload_decode_summary
                        .object_feed_object_update_compressed_messages += 1;
                    if let Some(objects) = decode_object_update_compressed_objects(payload) {
                        self.simulator_payload_decode_summary
                            .object_feed_object_update_compressed_mesh_hits += objects
                            .iter()
                            .filter(|obj| obj.mesh_id_bytes.is_some())
                            .count();
                        for obj in objects {
                            self.object_feed_upsert(
                                obj.local_id,
                                obj.scale_centi,
                                obj.position_centi,
                                obj.mesh_id_bytes,
                                obj.texture_id_bytes,
                                obj.default_face_material.clone(),
                                &obj.face_material_overrides,
                                obj.object_id_bytes,
                            );
                        }
                    } else {
                        self.simulator_payload_decode_summary
                            .object_feed_decode_dropped += 1;
                    }
                    self.refresh_object_feed_summary_export();
                }
                FirstSimulatorInboundMessageKind::ObjectUpdateCached => {
                    self.simulator_payload_decode_summary
                        .object_feed_update_messages += 1;
                    self.simulator_payload_decode_summary
                        .object_feed_object_update_cached_messages += 1;
                    if let Some(ids) = decode_object_update_cached_local_ids(payload) {
                        for local_id in ids {
                            self.object_feed_upsert(
                                local_id,
                                None,
                                None,
                                None,
                                None,
                                None,
                                &[],
                                None,
                            );
                            self.queue_object_cache_miss(local_id);
                        }
                    } else {
                        self.simulator_payload_decode_summary
                            .object_feed_decode_dropped += 1;
                    }
                    self.refresh_object_feed_summary_export();
                }
                FirstSimulatorInboundMessageKind::ImprovedTerseObjectUpdate => {
                    self.simulator_payload_decode_summary
                        .object_feed_update_messages += 1;
                    self.simulator_payload_decode_summary
                        .object_feed_improved_terse_messages += 1;
                    if let Some(objects) = decode_improved_terse_object_update_objects(payload) {
                        for obj in objects {
                            self.object_feed_upsert(
                                obj.local_id,
                                obj.scale_centi,
                                obj.position_centi,
                                obj.mesh_id_bytes,
                                obj.texture_id_bytes,
                                obj.default_face_material.clone(),
                                &obj.face_material_overrides,
                                obj.object_id_bytes,
                            );
                        }
                    } else {
                        self.simulator_payload_decode_summary
                            .object_feed_decode_dropped += 1;
                    }
                    self.refresh_object_feed_summary_export();
                }
                FirstSimulatorInboundMessageKind::ObjectExtraParams => {
                    self.simulator_payload_decode_summary
                        .object_feed_update_messages += 1;
                    self.simulator_payload_decode_summary
                        .object_feed_object_extra_params_messages += 1;
                    if let Some(objects) = decode_object_extra_params_mesh_updates(payload) {
                        self.simulator_payload_decode_summary
                            .object_feed_object_extra_params_mesh_hits += objects
                            .iter()
                            .filter(|obj| obj.mesh_id_bytes.is_some())
                            .count();
                        for obj in objects {
                            self.object_feed_upsert(
                                obj.local_id,
                                obj.scale_centi,
                                obj.position_centi,
                                obj.mesh_id_bytes,
                                obj.texture_id_bytes,
                                obj.default_face_material.clone(),
                                &obj.face_material_overrides,
                                obj.object_id_bytes,
                            );
                        }
                    } else {
                        self.simulator_payload_decode_summary
                            .object_feed_decode_dropped += 1;
                    }
                    self.refresh_object_feed_summary_export();
                }
                FirstSimulatorInboundMessageKind::KillObject => {
                    self.simulator_payload_decode_summary
                        .object_feed_kill_messages += 1;
                    if let Some(ids) = decode_kill_object_local_ids(payload) {
                        for local_id in ids {
                            self.object_feed_kill(local_id);
                        }
                    } else {
                        self.simulator_payload_decode_summary
                            .object_feed_decode_dropped += 1;
                    }
                    self.refresh_object_feed_summary_export();
                }
                _ => {}
            }
        }
        if let Some(kind) = to_region_transition_control_kind(classification.kind) {
            self.region_transition_control_observations
                .push(RegionTransitionControlObservation {
                    observation_index,
                    kind,
                    packet_message_number: classification.packet_message_number,
                    payload_len: payload.len(),
                    signal: classification.signal.clone(),
                });
            if self.region_transition_control_observations.len() > MAX_CONTINUITY_OBSERVATIONS {
                let overflow =
                    self.region_transition_control_observations.len() - MAX_CONTINUITY_OBSERVATIONS;
                self.region_transition_control_observations
                    .drain(0..overflow);
            }

            // Continuity: Transition phase progression.
            match kind {
                RegionTransitionControlKind::CrossedRegion => {
                    self.record_region_continuity_observation(
                        self.continuity_summary_with_phase(HandoffPhase::Crossed),
                    );
                }
                RegionTransitionControlKind::ConfirmEnableSimulator => {
                    if self.continuity_summary.phase == HandoffPhase::Crossed {
                        self.record_region_continuity_observation(
                            self.continuity_summary_with_phase(HandoffPhase::Confirming),
                        );
                    }
                }
            }
        }

        Ok(classification)
    }

    fn collect_first_simulator_reliable_ack(&mut self, payload: &[u8]) {
        let Some(header) = decode_first_simulator_packet_header(payload) else {
            return;
        };
        if header.flags & LLUDP_RELIABLE_FLAG == 0 {
            return;
        }
        if header.message_number == lludp_low_frequency_message_number(LLUDP_PACKET_ACK_LOW_ID) {
            return;
        }
        self.queue_first_simulator_ack_id(header.packet_id);
    }

    fn queue_first_simulator_ack_id(&mut self, packet_id: u32) {
        if self.pending_first_simulator_ack_ids.contains(&packet_id) {
            return;
        }
        if self.pending_first_simulator_ack_ids.len() >= MAX_PENDING_FIRST_SIMULATOR_ACK_IDS {
            self.pending_first_simulator_ack_ids.remove(0);
        }
        self.pending_first_simulator_ack_ids.push(packet_id);
    }

    fn queue_object_cache_miss(&mut self, local_id: u32) {
        if local_id == 0 {
            return;
        }
        if self.pending_object_cache_miss_ids.contains(&local_id) {
            return;
        }
        if self.pending_object_cache_miss_ids.len() >= MAX_PENDING_OBJECT_CACHE_MISS_IDS {
            self.pending_object_cache_miss_ids.remove(0);
        }
        self.pending_object_cache_miss_ids.push(local_id);
    }

    fn first_simulator_ack_batch(&self, payload: &[u8]) -> Vec<u32> {
        if self.pending_first_simulator_ack_ids.is_empty() {
            return Vec::new();
        }
        let Some(header) = decode_first_simulator_packet_header(payload) else {
            return Vec::new();
        };
        if header.message_number == lludp_low_frequency_message_number(LLUDP_PACKET_ACK_LOW_ID) {
            return Vec::new();
        }
        if payload.len().saturating_add(1) > LLUDP_MTU_BYTES {
            return Vec::new();
        }
        let available_bytes = LLUDP_MTU_BYTES
            .saturating_sub(payload.len())
            .saturating_sub(1);
        let max_ids_by_size = available_bytes / 4;
        let batch_len = self
            .pending_first_simulator_ack_ids
            .len()
            .min(LLUDP_MAX_APPENDED_ACKS)
            .min(max_ids_by_size);
        self.pending_first_simulator_ack_ids[..batch_len].to_vec()
    }

    fn record_first_simulator_socket_diagnostic(
        &mut self,
        kind: FirstSimulatorSocketDiagnosticKind,
        reason: impl Into<String>,
        local_addr: Option<String>,
        remote_target: Option<String>,
        packet_message_number: Option<u32>,
        payload_len: Option<usize>,
    ) {
        if self.first_simulator_socket_diagnostics.len() >= MAX_FIRST_SIMULATOR_SOCKET_DIAGNOSTICS {
            self.first_simulator_socket_diagnostics.remove(0);
        }
        let next_event_index = self
            .first_simulator_socket_diagnostics
            .last()
            .map(|diag| diag.event_index.saturating_add(1))
            .unwrap_or(1);
        self.first_simulator_socket_diagnostics
            .push(FirstSimulatorSocketDiagnostic {
                event_index: next_event_index,
                kind,
                reason: reason.into(),
                local_addr,
                remote_target,
                packet_message_number,
                payload_len,
            });
    }

    pub async fn receive_first_simulator_handshake_datagram_once(
        &mut self,
        bind: &str,
        wait_timeout: Duration,
    ) -> Result<FirstSimulatorInboundClassification, ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }

        let socket = UdpSocket::bind(bind).await.map_err(|err| {
            ConnectionError::InvalidFirstSimulatorReceiveBind {
                bind: bind.to_string(),
                reason: err.to_string(),
            }
        })?;
        let local_addr = socket_local_addr_text(&socket);
        self.record_first_simulator_socket_diagnostic(
            FirstSimulatorSocketDiagnosticKind::FreshBind,
            "receive_first_simulator_handshake_datagram_once",
            local_addr.clone(),
            None,
            None,
            None,
        );
        let mut buf = vec![0u8; 2048];

        let recv = timeout(wait_timeout, socket.recv_from(&mut buf)).await;
        let (received_len, sender) = match recv {
            Ok(Ok(parts)) => parts,
            Ok(Err(err)) => {
                return Err(ConnectionError::FirstSimulatorReceiveFailed {
                    bind: bind.to_string(),
                    reason: err.to_string(),
                });
            }
            Err(_) => {
                return Err(ConnectionError::FirstSimulatorReceiveTimedOut {
                    bind: bind.to_string(),
                    timeout_ms: wait_timeout.as_millis(),
                });
            }
        };

        self.record_first_simulator_socket_diagnostic(
            FirstSimulatorSocketDiagnosticKind::Receive,
            "receive_first_simulator_handshake_datagram_once",
            local_addr,
            Some(sender.to_string()),
            decode_first_simulator_packet_header(&buf[..received_len])
                .map(|header| header.message_number),
            Some(received_len),
        );

        let classification = self.observe_first_simulator_inbound_payload(&buf[..received_len])?;
        self.note_first_simulator_sender_endpoint(&sender, &classification);
        Ok(classification)
    }

    pub async fn probe_first_simulator_handshake_once(
        &mut self,
        bind: &str,
        wait_timeout: Duration,
    ) -> Result<FirstSimulatorInboundClassification, ConnectionError> {
        let report = self
            .probe_first_simulator_handshake_window_with_tail(bind, wait_timeout, 1, 0)
            .await?;
        if let Some(first) = report.observations.first() {
            return Ok(first.classification.clone());
        }
        Err(ConnectionError::FirstSimulatorReceiveTimedOut {
            bind: bind.to_string(),
            timeout_ms: wait_timeout.as_millis(),
        })
    }

    pub async fn probe_first_simulator_handshake_window(
        &mut self,
        bind: &str,
        wait_timeout: Duration,
        max_packets: usize,
    ) -> Result<FirstSimulatorHandshakeProbeReport, ConnectionError> {
        self.probe_first_simulator_handshake_window_with_tail(bind, wait_timeout, max_packets, 0)
            .await
    }

    pub async fn probe_first_simulator_handshake_window_with_tail(
        &mut self,
        bind: &str,
        wait_timeout: Duration,
        max_packets: usize,
        post_movement_tail_packets: usize,
    ) -> Result<FirstSimulatorHandshakeProbeReport, ConnectionError> {
        self.probe_first_simulator_handshake_window_with_policy(
            bind,
            wait_timeout,
            max_packets,
            post_movement_tail_packets,
            None,
            false,
        )
        .await
    }

    pub async fn probe_first_simulator_handshake_window_with_policy(
        &mut self,
        bind: &str,
        wait_timeout: Duration,
        max_packets: usize,
        post_movement_tail_packets: usize,
        post_movement_wait_timeout: Option<Duration>,
        stop_on_region_transition_control: bool,
    ) -> Result<FirstSimulatorHandshakeProbeReport, ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }
        if max_packets == 0 {
            return Err(ConnectionError::CapabilityDecode(String::from(
                "max_packets must be greater than zero",
            )));
        }
        self.retained_probe_socket = None;

        if self.first_simulator_handshake_state.is_none() {
            self.begin_first_simulator_handshake_scaffold()?;
        }

        let socket = UdpSocket::bind(bind).await.map_err(|err| {
            ConnectionError::InvalidFirstSimulatorReceiveBind {
                bind: bind.to_string(),
                reason: err.to_string(),
            }
        })?;
        let probe_local_addr = socket_local_addr_text(&socket);
        let probe_remote_target =
            self.first_simulator_handshake_prerequisites
                .as_ref()
                .map(|prerequisites| {
                    format!(
                        "{}:{}",
                        prerequisites.target.sim_ip, prerequisites.target.sim_port
                    )
                });
        self.record_first_simulator_socket_diagnostic(
            FirstSimulatorSocketDiagnosticKind::ProbeBind,
            "probe_first_simulator_handshake_window_with_policy",
            probe_local_addr.clone(),
            probe_remote_target.clone(),
            None,
            None,
        );

        let current_stage = self
            .first_simulator_handshake_state
            .as_ref()
            .map(|state| state.stage)
            .ok_or(ConnectionError::MissingFirstSimulatorHandshakePrerequisites)?;

        if current_stage == FirstSimulatorHandshakeStage::BootstrapPrerequisitesReady
            || current_stage == FirstSimulatorHandshakeStage::FirstRegionTargetKnown
        {
            if current_stage == FirstSimulatorHandshakeStage::BootstrapPrerequisitesReady {
                self.advance_first_simulator_handshake_scaffold(
                    FirstSimulatorHandshakeStage::FirstRegionTargetKnown,
                )?;
            }
            let prerequisites = self
                .first_simulator_handshake_state
                .as_ref()
                .expect("handshake state must be initialized")
                .prerequisites
                .clone();
            let payload = encode_first_simulator_use_circuit_code_payload(
                &prerequisites,
                self.next_first_simulator_packet_id(),
            )?;
            self.send_first_simulator_handshake_datagram_with_socket(
                FirstSimulatorHandshakeAction::UseCircuitCode,
                &prerequisites.target,
                &payload,
                &socket,
            )
            .await?;
            self.advance_first_simulator_handshake_scaffold(
                FirstSimulatorHandshakeStage::UseCircuitCode,
            )?;
        }

        let current_stage = self
            .first_simulator_handshake_state
            .as_ref()
            .map(|state| state.stage)
            .ok_or(ConnectionError::MissingFirstSimulatorHandshakePrerequisites)?;

        if current_stage == FirstSimulatorHandshakeStage::CompleteAgentMovement {
            let prerequisites = self
                .first_simulator_handshake_state
                .as_ref()
                .expect("handshake state must be initialized")
                .prerequisites
                .clone();
            let payload = encode_first_simulator_complete_agent_movement_payload(
                &prerequisites,
                self.next_first_simulator_packet_id(),
            )?;
            self.send_first_simulator_handshake_datagram_with_socket(
                FirstSimulatorHandshakeAction::CompleteAgentMovement,
                &prerequisites.target,
                &payload,
                &socket,
            )
            .await?;
            self.advance_first_simulator_handshake_scaffold(
                FirstSimulatorHandshakeStage::WaitingForAgentMovementComplete,
            )?;
        } else if current_stage == FirstSimulatorHandshakeStage::UseCircuitCode {
            self.advance_first_simulator_handshake_scaffold(
                FirstSimulatorHandshakeStage::CompleteAgentMovement,
            )?;
            let prerequisites = self
                .first_simulator_handshake_state
                .as_ref()
                .expect("handshake state must be initialized")
                .prerequisites
                .clone();
            let payload = encode_first_simulator_complete_agent_movement_payload(
                &prerequisites,
                self.next_first_simulator_packet_id(),
            )?;
            self.send_first_simulator_handshake_datagram_with_socket(
                FirstSimulatorHandshakeAction::CompleteAgentMovement,
                &prerequisites.target,
                &payload,
                &socket,
            )
            .await?;
            self.advance_first_simulator_handshake_scaffold(
                FirstSimulatorHandshakeStage::WaitingForAgentMovementComplete,
            )?;
        }

        let current_stage = self
            .first_simulator_handshake_state
            .as_ref()
            .map(|state| state.stage)
            .ok_or(ConnectionError::MissingFirstSimulatorHandshakePrerequisites)?;
        if current_stage != FirstSimulatorHandshakeStage::WaitingForAgentMovementComplete {
            return Err(ConnectionError::InvalidFirstSimulatorHandshakeTransition {
                from: current_stage,
                to: FirstSimulatorHandshakeStage::WaitingForAgentMovementComplete,
            });
        }

        let mut observations = Vec::new();
        let mut timed_out = false;
        let mut agent_movement_complete_observation_index = None;
        let mut post_movement_observations = 0usize;
        let mut post_boundary_bootstrap_relevant = 0usize;
        let mut post_boundary_transport_control = 0usize;
        let mut post_boundary_region_transition_control = 0usize;
        let mut post_boundary_likely_broader_traffic = 0usize;
        let mut post_boundary_unknown = 0usize;
        let mut post_boundary_crossed_region = 0usize;
        let mut post_boundary_confirm_enable_simulator = 0usize;
        let mut post_boundary_kinds = Vec::new();
        let mut post_boundary_unknown_packet_message_numbers = Vec::new();
        for _ in 0..max_packets {
            let mut buf = vec![0u8; 2048];
            let recv_timeout = if agent_movement_complete_observation_index.is_some() {
                post_movement_wait_timeout.unwrap_or(wait_timeout)
            } else {
                wait_timeout
            };
            let recv = timeout(recv_timeout, socket.recv_from(&mut buf)).await;
            let (received_len, sender) = match recv {
                Ok(Ok(parts)) => parts,
                Ok(Err(err)) => {
                    return Err(ConnectionError::FirstSimulatorReceiveFailed {
                        bind: bind.to_string(),
                        reason: err.to_string(),
                    });
                }
                Err(_) => {
                    timed_out = true;
                    break;
                }
            };
            self.record_first_simulator_socket_diagnostic(
                FirstSimulatorSocketDiagnosticKind::Receive,
                "probe_first_simulator_handshake_window_with_policy",
                probe_local_addr.clone(),
                Some(sender.to_string()),
                decode_first_simulator_packet_header(&buf[..received_len])
                    .map(|header| header.message_number),
                Some(received_len),
            );

            let classification =
                self.observe_first_simulator_inbound_payload(&buf[..received_len])?;
            self.note_first_simulator_sender_endpoint(&sender, &classification);
            let observation_index = self.first_simulator_handshake_receive_diagnostics.len();
            observations.push(FirstSimulatorHandshakeProbeObservation {
                observation_index,
                payload_len: received_len,
                classification: classification.clone(),
            });

            let is_agent_movement_complete = self
                .first_simulator_handshake_state
                .as_ref()
                .map(|state| state.stage)
                == Some(FirstSimulatorHandshakeStage::AgentMovementComplete);
            if is_agent_movement_complete {
                if agent_movement_complete_observation_index.is_none() {
                    agent_movement_complete_observation_index = Some(observation_index);
                    if post_movement_tail_packets == 0 {
                        break;
                    }
                    continue;
                }
                post_movement_observations += 1;
                match classification.scope {
                    FirstSimulatorInboundTrafficScope::BootstrapRelevant => {
                        post_boundary_bootstrap_relevant += 1
                    }
                    FirstSimulatorInboundTrafficScope::TransportControl => {
                        post_boundary_transport_control += 1
                    }
                    FirstSimulatorInboundTrafficScope::RegionTransitionControl => {
                        post_boundary_region_transition_control += 1
                    }
                    FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic => {
                        post_boundary_likely_broader_traffic += 1
                    }
                    FirstSimulatorInboundTrafficScope::Unknown => {
                        post_boundary_unknown += 1;
                        if let Some(number) = classification.packet_message_number {
                            post_boundary_unknown_packet_message_numbers.push(number);
                        }
                    }
                }
                match classification.kind {
                    FirstSimulatorInboundMessageKind::CrossedRegion => {
                        post_boundary_crossed_region += 1
                    }
                    FirstSimulatorInboundMessageKind::ConfirmEnableSimulator => {
                        post_boundary_confirm_enable_simulator += 1
                    }
                    _ => {}
                }
                post_boundary_kinds.push(classification.kind);
                if stop_on_region_transition_control
                    && classification.scope
                        == FirstSimulatorInboundTrafficScope::RegionTransitionControl
                {
                    break;
                }
                if post_movement_observations >= post_movement_tail_packets {
                    break;
                }
            } else if agent_movement_complete_observation_index.is_some() {
                post_movement_observations += 1;
                match classification.scope {
                    FirstSimulatorInboundTrafficScope::BootstrapRelevant => {
                        post_boundary_bootstrap_relevant += 1
                    }
                    FirstSimulatorInboundTrafficScope::TransportControl => {
                        post_boundary_transport_control += 1
                    }
                    FirstSimulatorInboundTrafficScope::RegionTransitionControl => {
                        post_boundary_region_transition_control += 1
                    }
                    FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic => {
                        post_boundary_likely_broader_traffic += 1
                    }
                    FirstSimulatorInboundTrafficScope::Unknown => {
                        post_boundary_unknown += 1;
                        if let Some(number) = classification.packet_message_number {
                            post_boundary_unknown_packet_message_numbers.push(number);
                        }
                    }
                }
                match classification.kind {
                    FirstSimulatorInboundMessageKind::CrossedRegion => {
                        post_boundary_crossed_region += 1
                    }
                    FirstSimulatorInboundMessageKind::ConfirmEnableSimulator => {
                        post_boundary_confirm_enable_simulator += 1
                    }
                    _ => {}
                }
                post_boundary_kinds.push(classification.kind);
                if stop_on_region_transition_control
                    && classification.scope
                        == FirstSimulatorInboundTrafficScope::RegionTransitionControl
                {
                    break;
                }
                if post_movement_observations >= post_movement_tail_packets {
                    break;
                }
            }

            if observations.len() >= max_packets {
                break;
            }
        }

        let post_boundary_summary = if agent_movement_complete_observation_index.is_some() {
            let mut unknown_counts = BTreeMap::new();
            for number in &post_boundary_unknown_packet_message_numbers {
                *unknown_counts.entry(*number).or_insert(0usize) += 1;
            }
            let repeated_unknown_packet_message_numbers = unknown_counts
                .into_iter()
                .filter_map(|(number, count)| if count > 1 { Some(number) } else { None })
                .collect::<Vec<_>>();
            Some(FirstSimulatorPostBoundarySummary {
                observations: post_movement_observations,
                bootstrap_relevant: post_boundary_bootstrap_relevant,
                transport_control: post_boundary_transport_control,
                region_transition_control: post_boundary_region_transition_control,
                likely_broader_traffic: post_boundary_likely_broader_traffic,
                unknown: post_boundary_unknown,
                crossed_region: post_boundary_crossed_region,
                confirm_enable_simulator: post_boundary_confirm_enable_simulator,
                watched_region_transition_control_not_seen: post_boundary_region_transition_control
                    == 0,
                kinds: post_boundary_kinds,
                unknown_packet_message_numbers: post_boundary_unknown_packet_message_numbers,
                repeated_unknown_packet_message_numbers,
            })
        } else {
            None
        };

        self.record_first_simulator_socket_diagnostic(
            FirstSimulatorSocketDiagnosticKind::RetainProbeSocket,
            "probe_first_simulator_handshake_window_with_policy",
            probe_local_addr,
            probe_remote_target,
            None,
            None,
        );
        self.retained_probe_socket = Some(socket);

        Ok(FirstSimulatorHandshakeProbeReport {
            observations,
            timed_out,
            agent_movement_complete_observation_index,
            post_movement_observations,
            post_boundary_summary,
        })
    }

    pub async fn fetch_seed_capabilities(&self) -> Result<SeedCapabilityMap, ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }

        let seed_url = self
            .session
            .as_ref()
            .and_then(|session| session.seed_capability.as_deref())
            .ok_or(ConnectionError::MissingSeedCapability)?;

        self.fetch_seed_capabilities_from_url(seed_url).await
    }

    pub async fn fetch_seed_capabilities_from_url(
        &self,
        seed_url: &str,
    ) -> Result<SeedCapabilityMap, ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }

        let client = reqwest::Client::builder()
            .timeout(self.config.connect_timeout)
            .build()?;

        let request_body = llsd_string_array(DEFAULT_SEED_CAPABILITY_REQUEST);
        let response = client
            .post(seed_url)
            .header(CONTENT_TYPE, "application/llsd+xml")
            .body(request_body)
            .send()
            .await?;
        let status = response.status();
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.to_ascii_lowercase());
        let bytes = response.bytes().await?;

        if !status.is_success() {
            return Err(ConnectionError::HttpStatus {
                status,
                body: String::from_utf8_lossy(&bytes).to_string(),
            });
        }

        parse_seed_capability_map(&bytes, content_type.as_deref())
    }

    pub async fn fetch_event_queue_once(
        &self,
        event_queue_url: &str,
    ) -> Result<EventQueueInspection, ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }

        let timeout = self
            .config
            .connect_timeout
            .max(EVENT_QUEUE_ONE_SHOT_MIN_TIMEOUT);
        let client = reqwest::Client::builder().timeout(timeout).build()?;

        let mut attempts = Vec::new();
        for attempt in 1..=MAX_EVENT_QUEUE_ONE_SHOT_ATTEMPTS {
            let request_body = llsd_event_queue_request(0, false);
            let started = Instant::now();
            let response = client
                .post(event_queue_url)
                .header(CONTENT_TYPE, LLSD_XML_CONTENT_TYPE)
                .header(ACCEPT, LLSD_XML_CONTENT_TYPE)
                .body(request_body)
                .send()
                .await;
            let elapsed_ms = started.elapsed().as_millis();

            let response = match response {
                Ok(response) => response,
                Err(err) => {
                    let (retryable, retry_class, retry_reason) =
                        classify_event_queue_transport_retry(&err);
                    let scheduled_backoff_ms =
                        event_queue_retry_backoff_ms(attempt, retryable, None);
                    let terminal_reason = if scheduled_backoff_ms.is_none() {
                        Some(retry_reason.clone())
                    } else {
                        None
                    };
                    attempts.push(EventQueueAttemptDiagnostic {
                        attempt,
                        status: None,
                        elapsed_ms,
                        retryable,
                        retry_class,
                        retry_reason,
                        scheduled_backoff_ms,
                        terminal_reason,
                        error_kind: format!("transport:{err}"),
                        response_headers: Vec::new(),
                    });
                    if let Some(backoff_ms) = scheduled_backoff_ms {
                        tokio::time::sleep(Duration::from_millis(backoff_ms as u64)).await;
                        continue;
                    }
                    return Err(ConnectionError::EventQueueOneShotFailed {
                        attempts_len: attempts.len(),
                        attempts,
                    });
                }
            };

            let status = response.status();
            let headers = response
                .headers()
                .iter()
                .map(|(name, value)| {
                    (
                        name.as_str().to_string(),
                        value.to_str().unwrap_or_default().to_string(),
                    )
                })
                .collect::<Vec<_>>();
            let content_type = response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .map(|value| value.to_ascii_lowercase());
            let bytes = response.bytes().await?;

            if status.is_success() {
                return parse_event_queue_once_response(&bytes, content_type.as_deref());
            }

            let body = String::from_utf8_lossy(&bytes).to_string();
            let retryable = is_retryable_event_queue_http_failure(status, &body);
            let retry_class = classify_event_queue_http_retry(status, retryable);
            let retry_reason = event_queue_http_retry_reason(status, &body, retryable);
            let scheduled_backoff_ms =
                event_queue_retry_backoff_ms(attempt, retryable, Some(status));
            let terminal_reason = if scheduled_backoff_ms.is_none() {
                Some(retry_reason.clone())
            } else {
                None
            };
            attempts.push(EventQueueAttemptDiagnostic {
                attempt,
                status: Some(status.as_u16()),
                elapsed_ms,
                retryable,
                retry_class,
                retry_reason,
                scheduled_backoff_ms,
                terminal_reason,
                error_kind: format!("http:{status}"),
                response_headers: headers,
            });
            if let Some(backoff_ms) = scheduled_backoff_ms {
                tokio::time::sleep(Duration::from_millis(backoff_ms as u64)).await;
                continue;
            }
            return Err(ConnectionError::EventQueueOneShotFailed {
                attempts_len: attempts.len(),
                attempts,
            });
        }

        Err(ConnectionError::EventQueueOneShotFailed {
            attempts_len: attempts.len(),
            attempts,
        })
    }

    pub async fn poll_event_queue_once(
        &self,
        event_queue_url: &str,
        ack: u64,
    ) -> Result<EventQueuePollResult, ConnectionError> {
        let timeout = self
            .config
            .connect_timeout
            .max(EVENT_QUEUE_ONE_SHOT_MIN_TIMEOUT);
        self.poll_event_queue_once_with_timeout(event_queue_url, ack, timeout)
            .await
    }

    pub async fn poll_event_queue_once_with_timeout(
        &self,
        event_queue_url: &str,
        ack: u64,
        timeout: Duration,
    ) -> Result<EventQueuePollResult, ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }

        let client = reqwest::Client::builder()
            .timeout(timeout.max(Duration::from_millis(100)))
            .build()?;

        let request_body = llsd_event_queue_request(ack, false);
        let response = client
            .post(event_queue_url)
            .header(CONTENT_TYPE, LLSD_XML_CONTENT_TYPE)
            .header(ACCEPT, LLSD_XML_CONTENT_TYPE)
            .body(request_body)
            .send()
            .await?;
        let status = response.status();
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.to_ascii_lowercase());
        let bytes = response.bytes().await?;

        if !status.is_success() {
            return Err(ConnectionError::HttpStatus {
                status,
                body: String::from_utf8_lossy(&bytes).to_string(),
            });
        }

        parse_event_queue_poll_response(&bytes, content_type.as_deref())
    }

    pub async fn resolve_avatar_display_names(
        &self,
        capability_url: &str,
        agent_ids: &[String],
    ) -> Result<Vec<ResolvedAvatarName>, ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }
        resolve_avatar_display_names_with_timeout(
            capability_url,
            agent_ids,
            self.config.connect_timeout.max(Duration::from_secs(15)),
        )
        .await
    }

    pub async fn fetch_agent_profile(
        &self,
        capability_url: &str,
        avatar_id: &str,
    ) -> Result<AgentProfileData, ConnectionError> {
        self.fetch_agent_profile_with_timeout(
            capability_url,
            avatar_id,
            self.config.connect_timeout.max(Duration::from_secs(15)),
        )
        .await
    }

    pub async fn fetch_profile_picks(
        &self,
        capability_url: &str,
        avatar_id: &str,
    ) -> Result<Vec<AgentProfilePick>, ConnectionError> {
        Ok(self
            .fetch_agent_profile(capability_url, avatar_id)
            .await?
            .picks)
    }

    pub async fn fetch_pick_info(
        &self,
        capability_url: &str,
        avatar_id: &str,
        pick_id: &str,
    ) -> Result<Option<AgentProfilePickDetails>, ConnectionError> {
        let profile = self.fetch_agent_profile(capability_url, avatar_id).await?;
        Ok(profile
            .picks
            .iter()
            .find(|entry| entry.id == pick_id)
            .map(|entry| AgentProfilePickDetails {
                id: entry.id.clone(),
                name: Some(entry.name.clone()),
                ..AgentProfilePickDetails::default()
            }))
    }

    pub async fn fetch_profile_classifieds(
        &self,
        capability_url: &str,
        avatar_id: &str,
    ) -> Result<Vec<AgentProfileClassified>, ConnectionError> {
        Ok(self
            .fetch_agent_profile(capability_url, avatar_id)
            .await?
            .classifieds)
    }

    pub async fn fetch_classified_info(
        &self,
        capability_url: &str,
        avatar_id: &str,
        classified_id: &str,
    ) -> Result<Option<AgentProfileClassifiedDetails>, ConnectionError> {
        let profile = self.fetch_agent_profile(capability_url, avatar_id).await?;
        Ok(profile
            .classifieds
            .iter()
            .find(|entry| entry.id == classified_id)
            .map(|entry| AgentProfileClassifiedDetails {
                id: entry.id.clone(),
                name: Some(entry.name.clone()),
                ..AgentProfileClassifiedDetails::default()
            }))
    }

    pub async fn fetch_profile_notes(
        &self,
        capability_url: &str,
        avatar_id: &str,
    ) -> Result<String, ConnectionError> {
        Ok(self
            .fetch_agent_profile(capability_url, avatar_id)
            .await?
            .notes)
    }

    pub async fn fetch_profile_groups(
        &self,
        capability_url: &str,
        avatar_id: &str,
    ) -> Result<Vec<AgentProfileGroup>, ConnectionError> {
        Ok(self
            .fetch_agent_profile(capability_url, avatar_id)
            .await?
            .groups)
    }

    pub async fn fetch_profile_image_bytes(
        &self,
        image_cap_url: &str,
        asset_id: &str,
    ) -> Result<Vec<u8>, ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }
        if image_cap_url.trim().is_empty() {
            return Err(ConnectionError::MissingCapability(String::from(
                "GetTexture/ViewerAsset",
            )));
        }
        let asset_id = asset_id.trim();
        if asset_id.is_empty() {
            return Err(ConnectionError::InvalidResponse(String::from(
                "empty asset id for profile image fetch",
            )));
        }
        let candidates = viewer_grid::AssetCapabilityPolicy::texture_url_candidates_from_base(
            image_cap_url,
            asset_id,
        );
        fetch_texture_asset_bytes(
            &candidates,
            self.config.connect_timeout.max(Duration::from_secs(15)),
        )
        .await
    }

    pub fn derive_profile_feed_url(
        &self,
        profile: &AgentProfileData,
        capability_url: Option<&str>,
    ) -> Option<String> {
        if let Some(url) = profile.profile_url.as_ref()
            && !url.trim().is_empty()
        {
            return Some(url.clone());
        }
        let cap_url = capability_url?.trim();
        if cap_url.is_empty() {
            return None;
        }
        let parsed = reqwest::Url::parse(cap_url).ok()?;
        let host = parsed.host_str()?;
        Some(format!("{}://{host}/my/{}", parsed.scheme(), profile.id))
    }

    async fn fetch_agent_profile_with_timeout(
        &self,
        capability_url: &str,
        avatar_id: &str,
        timeout: Duration,
    ) -> Result<AgentProfileData, ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }
        if capability_url.trim().is_empty() {
            return Err(ConnectionError::MissingCapability(String::from(
                "AgentProfile",
            )));
        }
        let avatar_id = avatar_id.trim();
        if avatar_id.is_empty() {
            return Err(ConnectionError::InvalidResponse(String::from(
                "empty avatar id for AgentProfile fetch",
            )));
        }

        let client = reqwest::Client::builder()
            .timeout(timeout.max(Duration::from_millis(100)))
            .build()?;
        let final_url = format!("{}/{}", capability_url.trim_end_matches('/'), avatar_id);
        let response = client
            .get(&final_url)
            .header(ACCEPT, LLSD_XML_CONTENT_TYPE)
            .send()
            .await?;
        let status = response.status();
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.to_ascii_lowercase());
        let bytes = response.bytes().await?;
        if !status.is_success() {
            return Err(ConnectionError::HttpStatus {
                status,
                body: String::from_utf8_lossy(&bytes).to_string(),
            });
        }
        parse_agent_profile_response(&bytes, content_type.as_deref(), avatar_id)
    }

    pub fn extract_nearby_chat_messages(
        &self,
        poll: &EventQueuePollResult,
    ) -> Vec<NearbyChatMessage> {
        poll.events
            .iter()
            .filter_map(|event| {
                if !event.message.to_ascii_lowercase().contains("chat") {
                    return None;
                }
                let sender = event
                    .fields
                    .get("from")
                    .or_else(|| event.fields.get("from_name"))
                    .or_else(|| event.fields.get("owner_name"))
                    .cloned()
                    .unwrap_or_else(|| String::from("unknown"));
                let text = event
                    .fields
                    .get("message")
                    .or_else(|| event.fields.get("text"))
                    .or_else(|| event.fields.get("chat"))
                    .cloned()?;
                Some(NearbyChatMessage {
                    sender,
                    text,
                    source: event.message.clone(),
                })
            })
            .collect()
    }

    pub fn summarize_event_queue_event_fields(
        &self,
        event: &EventQueueMessage,
        limit: usize,
    ) -> String {
        if event.fields.is_empty() || limit == 0 {
            return String::from("none");
        }
        event
            .fields
            .iter()
            .take(limit)
            .map(|(key, value)| format!("{key}={}", summarize_event_queue_value(value, 96)))
            .collect::<Vec<_>>()
            .join(";")
    }

    pub fn extract_event_queue_simulator_targets(
        &self,
        poll: &EventQueuePollResult,
    ) -> Vec<EventQueueSimulatorTarget> {
        poll.events
            .iter()
            .filter_map(extract_event_queue_simulator_target)
            .collect()
    }

    pub fn extract_event_queue_parcel_summaries(
        &self,
        poll: &EventQueuePollResult,
    ) -> Vec<EventQueueParcelSummary> {
        poll.events
            .iter()
            .filter_map(extract_event_queue_parcel_summary)
            .collect()
    }

    pub async fn send_nearby_chat(
        &mut self,
        text: &str,
        bind: &str,
        receive_timeout: Duration,
        receive_max_packets: usize,
    ) -> Result<Vec<NearbyChatMessage>, ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }
        if receive_max_packets == 0 {
            return Ok(Vec::new());
        }
        let (prerequisites, socket) = self.prepare_chat_socket(bind, "send_nearby_chat").await?;

        let chat_payload = encode_chat_from_viewer_payload(
            &prerequisites,
            self.next_first_simulator_packet_id(),
            text,
            1,
            0,
        )?;
        self.send_first_simulator_handshake_datagram_with_socket(
            FirstSimulatorHandshakeAction::CompleteAgentMovement,
            &prerequisites.target,
            &chat_payload,
            &socket,
        )
        .await?;
        self.receive_nearby_chat_on_socket(&socket, bind, receive_timeout, receive_max_packets)
            .await
    }

    pub async fn send_nearby_chat_on_circuit(
        &mut self,
        circuit: &SocialCircuit,
        text: &str,
        receive_timeout: Duration,
        receive_max_packets: usize,
    ) -> Result<Vec<NearbyChatMessage>, ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }
        let prerequisites = self
            .first_simulator_handshake_prerequisites
            .clone()
            .ok_or(ConnectionError::MissingFirstSimulatorHandshakePrerequisites)?;

        let chat_payload = encode_chat_from_viewer_payload(
            &prerequisites,
            self.next_first_simulator_packet_id(),
            text,
            1,
            0,
        )?;
        self.send_first_simulator_handshake_datagram_with_socket(
            FirstSimulatorHandshakeAction::CompleteAgentMovement,
            &circuit.target,
            &chat_payload,
            &circuit.socket,
        )
        .await?;
        self.receive_nearby_chat_on_socket(
            &circuit.socket,
            &circuit.bind,
            receive_timeout,
            receive_max_packets,
        )
        .await
    }

    pub async fn poll_nearby_chat_udp(
        &mut self,
        bind: &str,
        receive_timeout: Duration,
        receive_max_packets: usize,
    ) -> Result<Vec<NearbyChatMessage>, ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }
        if receive_max_packets == 0 {
            return Ok(Vec::new());
        }
        let (_prerequisites, socket) = self
            .prepare_chat_socket(bind, "poll_nearby_chat_udp")
            .await?;
        self.receive_nearby_chat_on_socket(&socket, bind, receive_timeout, receive_max_packets)
            .await
    }

    pub async fn poll_nearby_chat_on_circuit(
        &mut self,
        circuit: &SocialCircuit,
        receive_timeout: Duration,
        receive_max_packets: usize,
    ) -> Result<Vec<NearbyChatMessage>, ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }
        if receive_max_packets == 0 {
            return Ok(Vec::new());
        }
        self.receive_nearby_chat_on_socket(
            &circuit.socket,
            &circuit.bind,
            receive_timeout,
            receive_max_packets,
        )
        .await
    }

    pub async fn open_social_circuit(
        &mut self,
        bind: &str,
    ) -> Result<SocialCircuit, ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }
        let prerequisites = self
            .first_simulator_handshake_prerequisites
            .clone()
            .ok_or(ConnectionError::MissingFirstSimulatorHandshakePrerequisites)?;
        let socket = if let Some(socket) = self.retained_probe_socket.take() {
            self.record_first_simulator_socket_diagnostic(
                FirstSimulatorSocketDiagnosticKind::ReuseRetainedProbeSocket,
                "open_social_circuit",
                socket_local_addr_text(&socket),
                Some(format!(
                    "{}:{}",
                    prerequisites.target.sim_ip, prerequisites.target.sim_port
                )),
                None,
                None,
            );
            socket
        } else {
            self.prepare_chat_socket(bind, "open_social_circuit")
                .await?
                .1
        };
        Ok(SocialCircuit {
            bind: bind.to_string(),
            target: prerequisites.target,
            socket,
        })
    }

    pub async fn send_use_circuit_code_on_circuit_to_port(
        &mut self,
        circuit: &SocialCircuit,
        port: u16,
    ) -> Result<(), ConnectionError> {
        let sim_ip = circuit.target.sim_ip.as_str();
        self.send_use_circuit_code_on_circuit_to_target(circuit, sim_ip, port)
            .await
    }

    pub async fn send_use_circuit_code_on_circuit_to_target(
        &mut self,
        circuit: &SocialCircuit,
        sim_ip: &str,
        port: u16,
    ) -> Result<(), ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }
        let prerequisites = self
            .first_simulator_handshake_prerequisites
            .clone()
            .ok_or(ConnectionError::MissingFirstSimulatorHandshakePrerequisites)?;
        let mut target = circuit.target.clone();
        target.sim_ip = sim_ip.to_string();
        target.sim_port = port;
        let payload = encode_first_simulator_use_circuit_code_payload(
            &prerequisites,
            self.next_first_simulator_packet_id(),
        )?;
        self.send_first_simulator_handshake_datagram_with_socket(
            FirstSimulatorHandshakeAction::UseCircuitCode,
            &target,
            &payload,
            &circuit.socket,
        )
        .await?;
        Ok(())
    }

    pub async fn send_complete_agent_movement_on_circuit_to_port(
        &mut self,
        circuit: &SocialCircuit,
        port: u16,
    ) -> Result<(), ConnectionError> {
        let sim_ip = circuit.target.sim_ip.as_str();
        self.send_complete_agent_movement_on_circuit_to_target(circuit, sim_ip, port)
            .await
    }

    pub async fn send_complete_agent_movement_on_circuit_to_target(
        &mut self,
        circuit: &SocialCircuit,
        sim_ip: &str,
        port: u16,
    ) -> Result<(), ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }
        let prerequisites = self
            .first_simulator_handshake_prerequisites
            .clone()
            .ok_or(ConnectionError::MissingFirstSimulatorHandshakePrerequisites)?;
        let mut target = circuit.target.clone();
        target.sim_ip = sim_ip.to_string();
        target.sim_port = port;
        let payload = encode_first_simulator_complete_agent_movement_payload(
            &prerequisites,
            self.next_first_simulator_packet_id(),
        )?;
        self.send_first_simulator_handshake_datagram_with_socket(
            FirstSimulatorHandshakeAction::CompleteAgentMovement,
            &target,
            &payload,
            &circuit.socket,
        )
        .await?;
        Ok(())
    }

    pub async fn send_handshake_reprime_bundle(
        &mut self,
        circuit: &SocialCircuit,
    ) -> Result<usize, ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }

        let endpoints = self.handshake_reprime_endpoints(circuit);
        let mut datagrams_sent = 0usize;
        for (sim_ip, sim_port) in endpoints {
            self.send_use_circuit_code_on_circuit_to_target(circuit, &sim_ip, sim_port)
                .await?;
            datagrams_sent = datagrams_sent.saturating_add(1);
            self.send_complete_agent_movement_on_circuit_to_target(circuit, &sim_ip, sim_port)
                .await?;
            datagrams_sent = datagrams_sent.saturating_add(1);
        }
        Ok(datagrams_sent)
    }

    pub async fn send_retrieve_instant_messages(
        &mut self,
        circuit: &SocialCircuit,
    ) -> Result<(), ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }
        let prerequisites = self
            .first_simulator_handshake_prerequisites
            .clone()
            .ok_or(ConnectionError::MissingFirstSimulatorHandshakePrerequisites)?;
        let payload = encode_retrieve_instant_messages_payload(
            &prerequisites,
            self.next_first_simulator_packet_id(),
        )?;
        self.send_first_simulator_handshake_datagram_with_socket(
            FirstSimulatorHandshakeAction::CompleteAgentMovement,
            &circuit.target,
            &payload,
            &circuit.socket,
        )
        .await
    }

    pub async fn send_pending_region_handshake_reply(
        &mut self,
        circuit: &SocialCircuit,
    ) -> Result<bool, ConnectionError> {
        if self.region_handshake_reply_sent {
            return Ok(false);
        }
        if self.pending_region_handshake_reply_flags.is_none() {
            if self.require_observed_region_handshake_for_reply {
                return Ok(false);
            }
            let stage_allows_fallback = matches!(
                self.first_simulator_handshake_state
                    .as_ref()
                    .map(|state| state.stage),
                Some(FirstSimulatorHandshakeStage::WaitingForAgentMovementComplete)
                    | Some(FirstSimulatorHandshakeStage::AgentMovementComplete)
            );
            let observed_agent_movement_complete = self
                .first_simulator_handshake_receive_diagnostics
                .iter()
                .any(|diag| {
                    diag.kind == FirstSimulatorInboundMessageKind::AgentMovementComplete
                        && diag.decode_source
                            == FirstSimulatorInboundDecodeSource::PacketMessageNumber
                });
            if !(stage_allows_fallback || observed_agent_movement_complete) {
                return Ok(false);
            }
        }
        let target = self.resolve_region_handshake_reply_target(circuit);
        self.send_region_handshake_reply_to_target(
            circuit,
            &target,
            viewer_region_handshake_reply_flags(),
        )
        .await?;
        self.region_handshake_reply_sent = true;
        Ok(true)
    }

    pub async fn send_startup_interest_messages(
        &mut self,
        circuit: &SocialCircuit,
    ) -> Result<(), ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }
        self.send_agent_throttle_on_circuit(circuit).await?;
        self.send_agent_height_width_on_circuit(circuit).await?;
        self.send_agent_update_on_circuit(circuit, true).await?;
        self.send_agent_animation_on_circuit(circuit).await?;
        self.send_set_always_run_on_circuit(circuit, false).await
    }

    pub async fn send_startup_request_parity_messages(
        &mut self,
        circuit: &SocialCircuit,
    ) -> Result<(), ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }
        self.send_mute_list_request_on_circuit(circuit).await?;
        self.send_money_balance_request_on_circuit(circuit).await?;
        self.send_agent_data_update_request_on_circuit(circuit)
            .await
    }

    pub async fn flush_pending_ack_ids_on_circuit(
        &mut self,
        circuit: &SocialCircuit,
    ) -> Result<usize, ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }
        let mut flushed = 0usize;
        while !self.pending_first_simulator_ack_ids.is_empty() {
            let batch_len = self
                .pending_first_simulator_ack_ids
                .len()
                .min(LLUDP_MAX_APPENDED_ACKS);
            let ack_batch = self.pending_first_simulator_ack_ids[..batch_len].to_vec();
            let payload =
                encode_packet_ack_payload(self.next_first_simulator_packet_id(), &ack_batch)?;
            self.send_first_simulator_handshake_datagram_with_socket(
                FirstSimulatorHandshakeAction::CompleteAgentMovement,
                &circuit.target,
                &payload,
                &circuit.socket,
            )
            .await?;
            self.pending_first_simulator_ack_ids
                .drain(0..ack_batch.len());
            flushed = flushed.saturating_add(ack_batch.len());
        }
        Ok(flushed)
    }

    async fn flush_pending_object_cache_miss_ids_on_circuit(
        &mut self,
        circuit: &SocialCircuit,
    ) -> Result<usize, ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }
        if self.pending_object_cache_miss_ids.is_empty() {
            return Ok(0);
        }
        let prerequisites = self
            .first_simulator_handshake_prerequisites
            .clone()
            .ok_or(ConnectionError::MissingFirstSimulatorHandshakePrerequisites)?;
        let mut flushed = 0usize;
        while !self.pending_object_cache_miss_ids.is_empty() {
            let batch_len = self
                .pending_object_cache_miss_ids
                .len()
                .min(LLUDP_MAX_REQUEST_MULTIPLE_OBJECTS_BLOCKS);
            let ids = self.pending_object_cache_miss_ids[..batch_len].to_vec();
            let payload = encode_request_multiple_objects_payload(
                &prerequisites,
                self.next_first_simulator_packet_id(),
                &ids,
            )?;
            self.send_first_simulator_handshake_datagram_with_socket(
                FirstSimulatorHandshakeAction::CompleteAgentMovement,
                &circuit.target,
                &payload,
                &circuit.socket,
            )
            .await?;
            self.pending_object_cache_miss_ids.drain(0..batch_len);
            flushed = flushed.saturating_add(batch_len);
        }
        Ok(flushed)
    }

    pub async fn send_agent_update_on_circuit(
        &mut self,
        circuit: &SocialCircuit,
        reliable: bool,
    ) -> Result<(), ConnectionError> {
        self.send_agent_update_custom_on_circuit(circuit, reliable, STARTUP_AGENT_UPDATE_FAR, 0)
            .await
    }

    pub async fn send_agent_update_custom_on_circuit(
        &mut self,
        circuit: &SocialCircuit,
        reliable: bool,
        far: f32,
        control_flags: u32,
    ) -> Result<(), ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }
        let prerequisites = self
            .first_simulator_handshake_prerequisites
            .clone()
            .ok_or(ConnectionError::MissingFirstSimulatorHandshakePrerequisites)?;
        let agent_update_payload = encode_agent_update_payload(
            &prerequisites,
            self.next_first_simulator_packet_id(),
            self.agent_update_camera_center(),
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            far,
            control_flags,
            0,
            reliable,
        )?;
        self.send_first_simulator_handshake_datagram_with_socket(
            FirstSimulatorHandshakeAction::CompleteAgentMovement,
            &circuit.target,
            &agent_update_payload,
            &circuit.socket,
        )
        .await
    }

    pub fn current_agent_update_camera_center(&self) -> [f32; 3] {
        self.agent_update_camera_center()
    }

    async fn send_region_handshake_reply_to_target(
        &mut self,
        circuit: &SocialCircuit,
        target: &FirstSimulatorTarget,
        region_flags: u32,
    ) -> Result<(), ConnectionError> {
        let prerequisites = self
            .first_simulator_handshake_prerequisites
            .clone()
            .ok_or(ConnectionError::MissingFirstSimulatorHandshakePrerequisites)?;
        let payload = encode_region_handshake_reply_payload(
            &prerequisites,
            self.next_first_simulator_packet_id(),
            region_flags,
        )?;
        self.send_first_simulator_handshake_datagram_with_socket(
            FirstSimulatorHandshakeAction::CompleteAgentMovement,
            target,
            &payload,
            &circuit.socket,
        )
        .await
    }

    fn note_first_simulator_sender_endpoint(
        &mut self,
        sender: &SocketAddr,
        classification: &FirstSimulatorInboundClassification,
    ) {
        if matches!(
            classification.scope,
            FirstSimulatorInboundTrafficScope::BootstrapRelevant
                | FirstSimulatorInboundTrafficScope::RegionTransitionControl
        ) {
            self.last_bootstrap_sender_endpoint = Some(*sender);
        }
        if classification.kind == FirstSimulatorInboundMessageKind::RegionHandshake {
            self.last_region_handshake_sender_endpoint = Some(*sender);
        }
    }

    fn resolve_region_handshake_reply_target(
        &self,
        circuit: &SocialCircuit,
    ) -> FirstSimulatorTarget {
        if let Some(sender) = self.last_region_handshake_sender_endpoint {
            return self.with_target_endpoint(circuit, sender);
        }
        if let Some(sender) = self.last_bootstrap_sender_endpoint {
            return self.with_target_endpoint(circuit, sender);
        }
        circuit.target.clone()
    }

    fn handshake_reprime_endpoints(&self, circuit: &SocialCircuit) -> Vec<(String, u16)> {
        let mut endpoints = Vec::new();
        let mut seen = BTreeSet::<String>::new();
        let mut push_endpoint = |ip: String, port: u16| {
            let key = format!("{ip}:{port}");
            if seen.insert(key) {
                endpoints.push((ip, port));
            }
        };

        if let Some(sender) = self.last_region_handshake_sender_endpoint {
            push_endpoint(sender.ip().to_string(), sender.port());
        }
        if let Some(sender) = self.last_bootstrap_sender_endpoint {
            push_endpoint(sender.ip().to_string(), sender.port());
        }
        push_endpoint(circuit.target.sim_ip.clone(), circuit.target.sim_port);

        endpoints
    }

    fn with_target_endpoint(
        &self,
        circuit: &SocialCircuit,
        sender: SocketAddr,
    ) -> FirstSimulatorTarget {
        let mut target = circuit.target.clone();
        target.sim_ip = sender.ip().to_string();
        target.sim_port = sender.port();
        target
    }

    pub async fn send_direct_im(
        &mut self,
        circuit: &SocialCircuit,
        to_agent_id: &str,
        session_id: &str,
        from_name: &str,
        message: &str,
    ) -> Result<(), ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }
        let prerequisites = self
            .first_simulator_handshake_prerequisites
            .clone()
            .ok_or(ConnectionError::MissingFirstSimulatorHandshakePrerequisites)?;
        let payload = encode_improved_instant_message_payload(
            &prerequisites,
            self.next_first_simulator_packet_id(),
            to_agent_id,
            session_id,
            from_name,
            message,
        )?;
        self.send_first_simulator_handshake_datagram_with_socket(
            FirstSimulatorHandshakeAction::CompleteAgentMovement,
            &circuit.target,
            &payload,
            &circuit.socket,
        )
        .await
    }

    async fn send_agent_throttle_on_circuit(
        &mut self,
        circuit: &SocialCircuit,
    ) -> Result<(), ConnectionError> {
        let prerequisites = self
            .first_simulator_handshake_prerequisites
            .clone()
            .ok_or(ConnectionError::MissingFirstSimulatorHandshakePrerequisites)?;
        let throttle_payload = encode_agent_throttle_payload(
            &prerequisites,
            self.next_first_simulator_packet_id(),
            0,
            &STARTUP_AGENT_THROTTLES,
        )?;
        self.send_first_simulator_handshake_datagram_with_socket(
            FirstSimulatorHandshakeAction::CompleteAgentMovement,
            &circuit.target,
            &throttle_payload,
            &circuit.socket,
        )
        .await
    }

    async fn send_agent_height_width_on_circuit(
        &mut self,
        circuit: &SocialCircuit,
    ) -> Result<(), ConnectionError> {
        let prerequisites = self
            .first_simulator_handshake_prerequisites
            .clone()
            .ok_or(ConnectionError::MissingFirstSimulatorHandshakePrerequisites)?;
        let payload = encode_agent_height_width_payload(
            &prerequisites,
            self.next_first_simulator_packet_id(),
            0,
            STARTUP_AGENT_HEIGHT,
            STARTUP_AGENT_WIDTH,
        )?;
        self.send_first_simulator_handshake_datagram_with_socket(
            FirstSimulatorHandshakeAction::CompleteAgentMovement,
            &circuit.target,
            &payload,
            &circuit.socket,
        )
        .await
    }

    async fn send_set_always_run_on_circuit(
        &mut self,
        circuit: &SocialCircuit,
        always_run: bool,
    ) -> Result<(), ConnectionError> {
        let prerequisites = self
            .first_simulator_handshake_prerequisites
            .clone()
            .ok_or(ConnectionError::MissingFirstSimulatorHandshakePrerequisites)?;
        let payload = encode_set_always_run_payload(
            &prerequisites,
            self.next_first_simulator_packet_id(),
            always_run,
        )?;
        self.send_first_simulator_handshake_datagram_with_socket(
            FirstSimulatorHandshakeAction::CompleteAgentMovement,
            &circuit.target,
            &payload,
            &circuit.socket,
        )
        .await
    }

    async fn send_agent_animation_on_circuit(
        &mut self,
        circuit: &SocialCircuit,
    ) -> Result<(), ConnectionError> {
        let prerequisites = self
            .first_simulator_handshake_prerequisites
            .clone()
            .ok_or(ConnectionError::MissingFirstSimulatorHandshakePrerequisites)?;
        let payload = encode_agent_animation_payload(
            &prerequisites,
            self.next_first_simulator_packet_id(),
            STARTUP_AGENT_ANIMATION_ID,
            false,
        )?;
        self.send_first_simulator_handshake_datagram_with_socket(
            FirstSimulatorHandshakeAction::CompleteAgentMovement,
            &circuit.target,
            &payload,
            &circuit.socket,
        )
        .await
    }

    async fn send_mute_list_request_on_circuit(
        &mut self,
        circuit: &SocialCircuit,
    ) -> Result<(), ConnectionError> {
        let prerequisites = self
            .first_simulator_handshake_prerequisites
            .clone()
            .ok_or(ConnectionError::MissingFirstSimulatorHandshakePrerequisites)?;
        let payload = encode_mute_list_request_payload(
            &prerequisites,
            self.next_first_simulator_packet_id(),
        )?;
        self.send_first_simulator_handshake_datagram_with_socket(
            FirstSimulatorHandshakeAction::CompleteAgentMovement,
            &circuit.target,
            &payload,
            &circuit.socket,
        )
        .await
    }

    async fn send_money_balance_request_on_circuit(
        &mut self,
        circuit: &SocialCircuit,
    ) -> Result<(), ConnectionError> {
        let prerequisites = self
            .first_simulator_handshake_prerequisites
            .clone()
            .ok_or(ConnectionError::MissingFirstSimulatorHandshakePrerequisites)?;
        let payload = encode_money_balance_request_payload(
            &prerequisites,
            self.next_first_simulator_packet_id(),
        )?;
        self.send_first_simulator_handshake_datagram_with_socket(
            FirstSimulatorHandshakeAction::CompleteAgentMovement,
            &circuit.target,
            &payload,
            &circuit.socket,
        )
        .await
    }

    async fn send_agent_data_update_request_on_circuit(
        &mut self,
        circuit: &SocialCircuit,
    ) -> Result<(), ConnectionError> {
        let prerequisites = self
            .first_simulator_handshake_prerequisites
            .clone()
            .ok_or(ConnectionError::MissingFirstSimulatorHandshakePrerequisites)?;
        let payload = encode_agent_data_update_request_payload(
            &prerequisites,
            self.next_first_simulator_packet_id(),
        )?;
        self.send_first_simulator_handshake_datagram_with_socket(
            FirstSimulatorHandshakeAction::CompleteAgentMovement,
            &circuit.target,
            &payload,
            &circuit.socket,
        )
        .await
    }

    fn agent_update_camera_center(&self) -> [f32; 3] {
        self.simulator_payload_decode_summary
            .agent_movement_complete_last_position
            .map(|pos| [pos[0] as f32, pos[1] as f32, pos[2] as f32])
            .unwrap_or([128.0, 128.0, 25.0])
    }

    pub async fn fetch_agent_profile_legacy(
        &mut self,
        circuit: &SocialCircuit,
        avatar_id: &str,
        receive_timeout: Duration,
        receive_max_packets: usize,
    ) -> Result<AgentProfileData, ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }
        let prerequisites = self
            .first_simulator_handshake_prerequisites
            .clone()
            .ok_or(ConnectionError::MissingFirstSimulatorHandshakePrerequisites)?;
        let payload = encode_avatar_properties_request_payload(
            &prerequisites,
            self.next_first_simulator_packet_id(),
            avatar_id,
        )?;
        self.send_first_simulator_handshake_datagram_with_socket(
            FirstSimulatorHandshakeAction::CompleteAgentMovement,
            &circuit.target,
            &payload,
            &circuit.socket,
        )
        .await?;
        let notes_request = encode_generic_message_payload(
            &prerequisites,
            self.next_first_simulator_packet_id(),
            "avatarnotesrequest",
            &[avatar_id],
        )?;
        self.send_first_simulator_handshake_datagram_with_socket(
            FirstSimulatorHandshakeAction::CompleteAgentMovement,
            &circuit.target,
            &notes_request,
            &circuit.socket,
        )
        .await?;
        let picks_request = encode_generic_message_payload(
            &prerequisites,
            self.next_first_simulator_packet_id(),
            "avatarpicksrequest",
            &[avatar_id],
        )?;
        self.send_first_simulator_handshake_datagram_with_socket(
            FirstSimulatorHandshakeAction::CompleteAgentMovement,
            &circuit.target,
            &picks_request,
            &circuit.socket,
        )
        .await?;
        let classifieds_request = encode_generic_message_payload(
            &prerequisites,
            self.next_first_simulator_packet_id(),
            "avatarclassifiedsrequest",
            &[avatar_id],
        )?;
        self.send_first_simulator_handshake_datagram_with_socket(
            FirstSimulatorHandshakeAction::CompleteAgentMovement,
            &circuit.target,
            &classifieds_request,
            &circuit.socket,
        )
        .await?;
        let groups_request = encode_generic_message_payload(
            &prerequisites,
            self.next_first_simulator_packet_id(),
            "avatargroupsrequest",
            &[avatar_id],
        )?;
        let _ = self
            .send_first_simulator_handshake_datagram_with_socket(
                FirstSimulatorHandshakeAction::CompleteAgentMovement,
                &circuit.target,
                &groups_request,
                &circuit.socket,
            )
            .await;

        let mut out = AgentProfileData {
            id: avatar_id.to_string(),
            ..AgentProfileData::default()
        };
        let mut got_properties = false;
        let mut requested_pick_details = BTreeSet::new();
        let mut requested_classified_details = BTreeSet::new();
        let mut buf = vec![0u8; 4096];
        let started = Instant::now();
        let local_addr = socket_local_addr_text(&circuit.socket);
        for _ in 0..receive_max_packets {
            if started.elapsed() > Duration::from_secs(8) {
                break;
            }
            let recv = timeout(receive_timeout, circuit.socket.recv_from(&mut buf)).await;
            let (received_len, sender) = match recv {
                Ok(Ok(parts)) => parts,
                Ok(Err(err)) => {
                    return Err(ConnectionError::FirstSimulatorReceiveFailed {
                        bind: circuit.bind.clone(),
                        reason: err.to_string(),
                    });
                }
                Err(_) => break,
            };
            let packet = &buf[..received_len];
            self.record_first_simulator_socket_diagnostic(
                FirstSimulatorSocketDiagnosticKind::Receive,
                "fetch_agent_profile_legacy",
                local_addr.clone(),
                Some(sender.to_string()),
                decode_first_simulator_packet_header(packet).map(|header| header.message_number),
                Some(received_len),
            );
            if let Ok(classification) = self.observe_first_simulator_inbound_payload(packet) {
                self.note_first_simulator_sender_endpoint(&sender, &classification);
            }
            if let Some(profile) = decode_legacy_avatar_properties_reply(packet, avatar_id) {
                out.id = profile.id;
                out.sl_about_text = profile.sl_about_text;
                out.fl_about_text = profile.fl_about_text;
                out.profile_url = profile.profile_url;
                out.sl_image_id = profile.sl_image_id;
                out.fl_image_id = profile.fl_image_id;
                out.partner_id = profile.partner_id;
                out.member_since = profile.member_since;
                out.allow_publish = profile.allow_publish;
                out.identified = profile.identified;
                out.transacted = profile.transacted;
                out.online = profile.online;
                got_properties = true;
            }
            if let Some(groups) = decode_legacy_avatar_groups_reply(packet, avatar_id) {
                out.groups = groups;
            }
            if let Some(notes) = decode_legacy_avatar_notes_reply(packet, avatar_id) {
                out.notes = notes;
            }
            if let Some(picks) = decode_legacy_avatar_picks_reply(packet, avatar_id) {
                out.picks = picks;
                for pick in &out.picks {
                    if requested_pick_details.insert(pick.id.clone()) {
                        let pick_request = encode_generic_message_payload(
                            &prerequisites,
                            self.next_first_simulator_packet_id(),
                            "pickinforequest",
                            &[pick.id.as_str()],
                        )?;
                        let _ = self
                            .send_first_simulator_handshake_datagram_with_socket(
                                FirstSimulatorHandshakeAction::CompleteAgentMovement,
                                &circuit.target,
                                &pick_request,
                                &circuit.socket,
                            )
                            .await;
                    }
                }
            }
            if let Some(pick_details) = decode_legacy_pick_info_reply(packet, avatar_id) {
                if let Some(existing) = out
                    .pick_details
                    .iter_mut()
                    .find(|d| d.id == pick_details.id)
                {
                    *existing = pick_details;
                } else {
                    out.pick_details.push(pick_details);
                }
            }
            if let Some(classifieds) = decode_legacy_avatar_classifieds_reply(packet, avatar_id) {
                out.classifieds = classifieds;
                for classified in &out.classifieds {
                    if requested_classified_details.insert(classified.id.clone()) {
                        let details_request = encode_classified_info_request_payload(
                            &prerequisites,
                            self.next_first_simulator_packet_id(),
                            &classified.id,
                        )?;
                        let _ = self
                            .send_first_simulator_handshake_datagram_with_socket(
                                FirstSimulatorHandshakeAction::CompleteAgentMovement,
                                &circuit.target,
                                &details_request,
                                &circuit.socket,
                            )
                            .await;
                    }
                }
            }
            if let Some(classified_details) = decode_legacy_classified_info_reply(packet, avatar_id)
            {
                if let Some(existing) = out
                    .classified_details
                    .iter_mut()
                    .find(|d| d.id == classified_details.id)
                {
                    *existing = classified_details;
                } else {
                    out.classified_details.push(classified_details);
                }
            }
            if got_properties
                && started.elapsed() > Duration::from_millis(1500)
                && out.pick_details.len() >= out.picks.len()
                && out.classified_details.len() >= out.classifieds.len()
            {
                break;
            }
        }

        if got_properties {
            Ok(out)
        } else {
            Err(ConnectionError::CapabilityDecode(String::from(
                "legacy AvatarPropertiesReply not received (timeout)",
            )))
        }
    }

    pub async fn poll_social_events(
        &mut self,
        circuit: &SocialCircuit,
        receive_timeout: Duration,
        receive_max_packets: usize,
    ) -> Result<Vec<SocialEvent>, ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }
        let _ = self
            .flush_pending_object_cache_miss_ids_on_circuit(circuit)
            .await?;
        let mut events = Vec::new();
        let mut buf = vec![0u8; 4096];
        let local_addr = socket_local_addr_text(&circuit.socket);
        for _ in 0..receive_max_packets {
            let recv = timeout(receive_timeout, circuit.socket.recv_from(&mut buf)).await;
            let (received_len, sender) = match recv {
                Ok(Ok(parts)) => parts,
                Ok(Err(err)) => {
                    return Err(ConnectionError::FirstSimulatorReceiveFailed {
                        bind: circuit.bind.clone(),
                        reason: err.to_string(),
                    });
                }
                Err(_) => break,
            };
            let payload = &buf[..received_len];
            self.record_first_simulator_socket_diagnostic(
                FirstSimulatorSocketDiagnosticKind::Receive,
                "poll_social_events",
                local_addr.clone(),
                Some(sender.to_string()),
                decode_first_simulator_packet_header(payload).map(|header| header.message_number),
                Some(received_len),
            );
            if let Ok(classification) = self.observe_first_simulator_inbound_payload(payload) {
                self.note_first_simulator_sender_endpoint(&sender, &classification);
            }
            let _ = self.send_pending_region_handshake_reply(circuit).await?;
            let _ = self.flush_pending_ack_ids_on_circuit(circuit).await?;
            let _ = self
                .flush_pending_object_cache_miss_ids_on_circuit(circuit)
                .await?;
            events.extend(decode_social_events(payload));
        }
        Ok(events)
    }

    pub async fn fetch_simulator_features_once(
        &self,
        simulator_features_url: &str,
    ) -> Result<SimulatorFeaturesInspection, ConnectionError> {
        self.fetch_simulator_capability_shape_once(simulator_features_url)
            .await
    }

    pub async fn fetch_map_layer_once(
        &self,
        map_layer_url: &str,
    ) -> Result<MapLayerInspection, ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }

        let (status, content_type, bytes) =
            self.fetch_simulator_capability_bytes(map_layer_url).await?;

        if !status.is_success() {
            if status == StatusCode::METHOD_NOT_ALLOWED {
                return Err(ConnectionError::MapLayerLikelyLegacyUdp {
                    status,
                    body: String::from_utf8_lossy(&bytes).to_string(),
                });
            }
            return Err(ConnectionError::HttpStatus {
                status,
                body: String::from_utf8_lossy(&bytes).to_string(),
            });
        }

        parse_simulator_features_response(&bytes, content_type.as_deref())
    }

    pub async fn fetch_interest_list_once(
        &self,
        interest_list_url: &str,
    ) -> Result<SimulatorFeaturesInspection, ConnectionError> {
        self.fetch_simulator_capability_shape_once(interest_list_url)
            .await
    }

    pub async fn fetch_untrusted_simulator_message_once(
        &self,
        untrusted_simulator_message_url: &str,
    ) -> Result<SimulatorFeaturesInspection, ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }

        let (get_probe, get_bytes) = self
            .execute_capability_probe_once_with_body(&CapabilityProbeRequest {
                method: String::from("GET"),
                url_candidates: vec![untrusted_simulator_message_url.to_string()],
                accept: Some(String::from(LLSD_XML_CONTENT_TYPE)),
                ..Default::default()
            })
            .await?;
        let get_status = StatusCode::from_u16(get_probe.status).map_err(|err| {
            ConnectionError::CapabilityDecode(format!("invalid probe status code: {err}"))
        })?;
        if get_status.is_success() {
            return Ok(parse_untrusted_probe_inspection_or_transport_fallback(
                &get_bytes,
                get_probe.content_type.as_deref(),
                get_status,
            ));
        }
        if get_status != StatusCode::METHOD_NOT_ALLOWED {
            return Err(ConnectionError::HttpStatus {
                status: get_status,
                body: String::from_utf8_lossy(&get_bytes).to_string(),
            });
        }

        let probe_body = self.build_untrusted_simulator_message_probe_llsd_body();
        let (post_probe, post_bytes) = self
            .execute_capability_probe_once_with_body(&CapabilityProbeRequest {
                method: String::from("POST"),
                url_candidates: vec![untrusted_simulator_message_url.to_string()],
                accept: Some(String::from(LLSD_XML_CONTENT_TYPE)),
                content_type: Some(String::from(LLSD_XML_CONTENT_TYPE)),
                body: Some(probe_body.clone()),
                ..Default::default()
            })
            .await?;
        let post_status = StatusCode::from_u16(post_probe.status).map_err(|err| {
            ConnectionError::CapabilityDecode(format!("invalid probe status code: {err}"))
        })?;
        if !post_status.is_success() {
            return Err(ConnectionError::HttpStatus {
                status: post_status,
                body: String::from_utf8_lossy(&post_bytes).to_string(),
            });
        }
        Ok(parse_untrusted_probe_inspection_or_transport_fallback(
            &post_bytes,
            post_probe.content_type.as_deref(),
            post_status,
        ))
    }

    pub async fn fetch_region_objects_once(
        &self,
        region_objects_url: &str,
    ) -> Result<RegionObjectsInspection, ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }

        let (status, content_type, bytes) = self
            .fetch_simulator_capability_bytes(region_objects_url)
            .await?;

        if !status.is_success() {
            return Err(ConnectionError::HttpStatus {
                status,
                body: String::from_utf8_lossy(&bytes).to_string(),
            });
        }

        parse_region_objects_response(&bytes, content_type.as_deref())
    }

    pub async fn probe_capability_transport_once(
        &self,
        capability_url: &str,
    ) -> Result<CapabilityTransportProbe, ConnectionError> {
        let request = CapabilityProbeRequest {
            method: String::from("GET"),
            url_candidates: vec![capability_url.to_string()],
            accept: Some(String::from(LLSD_XML_CONTENT_TYPE)),
            ..Default::default()
        };
        self.execute_capability_probe_once(&request).await
    }

    pub async fn execute_capability_probe_once(
        &self,
        request: &CapabilityProbeRequest,
    ) -> Result<CapabilityTransportProbe, ConnectionError> {
        let (probe, _) = self
            .execute_capability_probe_once_with_body(request)
            .await?;
        Ok(probe)
    }

    async fn execute_capability_probe_once_with_body(
        &self,
        request: &CapabilityProbeRequest,
    ) -> Result<(CapabilityTransportProbe, Vec<u8>), ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }
        if request.url_candidates.is_empty() {
            return Err(ConnectionError::MissingCapability(String::from(
                "capability probe url candidates",
            )));
        }
        let method = parse_probe_method(&request.method)?;
        let client = reqwest::Client::builder()
            .timeout(self.config.connect_timeout)
            .build()?;
        let mut last_error: Option<ConnectionError> = None;
        for url in &request.url_candidates {
            let mut req = client.request(method.clone(), url);
            if let Some(accept) = request.accept.as_deref() {
                req = req.header(ACCEPT, accept);
            }
            if let Some(content_type) = request.content_type.as_deref() {
                req = req.header(CONTENT_TYPE, content_type);
            }
            if let Some(range) = request.range.as_deref() {
                req = req.header("Range", range);
            }
            if let Some(body) = request.body.as_ref() {
                req = req.body(body.clone());
            }
            let response = match req.send().await {
                Ok(response) => response,
                Err(err) => {
                    last_error = Some(ConnectionError::Http(err));
                    continue;
                }
            };
            let status = response.status();
            let content_type = response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .map(|value| value.to_ascii_lowercase());
            let bytes = response.bytes().await?.to_vec();
            let decode = classify_probe_decode(content_type.as_deref(), &bytes);
            let body_preview = bounded_probe_body_preview(&bytes);
            let body_preview_hash = body_preview
                .as_deref()
                .map(|preview| stable_probe_hash_hex(preview.as_bytes()));
            let probe = CapabilityTransportProbe {
                status: status.as_u16(),
                content_type,
                body_bytes: bytes.len(),
                selected_url: Some(url.clone()),
                method: request.method.to_ascii_uppercase(),
                decode,
                body_preview_hash,
                body_preview,
                response_class: classify_probe_response_class(status),
            };
            return Ok((probe, bytes));
        }
        Err(last_error.unwrap_or_else(|| {
            ConnectionError::CapabilityDecode(String::from("capability probe transport failed"))
        }))
    }

    async fn fetch_simulator_capability_shape_once(
        &self,
        capability_url: &str,
    ) -> Result<SimulatorFeaturesInspection, ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }
        let (status, content_type, bytes) = self
            .fetch_simulator_capability_bytes(capability_url)
            .await?;
        if !status.is_success() {
            return Err(ConnectionError::HttpStatus {
                status,
                body: String::from_utf8_lossy(&bytes).to_string(),
            });
        }
        parse_simulator_features_response(&bytes, content_type.as_deref())
    }

    async fn fetch_simulator_capability_bytes(
        &self,
        capability_url: &str,
    ) -> Result<(StatusCode, Option<String>, Vec<u8>), ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }
        let client = reqwest::Client::builder()
            .timeout(self.config.connect_timeout)
            .build()?;

        let response = client
            .get(capability_url)
            .header(ACCEPT, LLSD_XML_CONTENT_TYPE)
            .send()
            .await?;
        let status = response.status();
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.to_ascii_lowercase());
        let bytes = response.bytes().await?.to_vec();
        Ok((status, content_type, bytes))
    }

    fn build_untrusted_simulator_message_probe_llsd_body(&self) -> String {
        let (agent_id, session_id) = self
            .first_simulator_handshake_prerequisites
            .as_ref()
            .map(|p| (p.agent_id.as_str(), p.session_id.as_str()))
            .unwrap_or(("", ""));
        format!(
            "<llsd><map>\
<key>message</key><string>AgentDataUpdateRequest</string>\
<key>body</key><map>\
<key>AgentData</key><array><map>\
<key>AgentID</key><uuid>{agent_id}</uuid>\
<key>SessionID</key><uuid>{session_id}</uuid>\
</map></array>\
</map>\
</map></llsd>"
        )
    }

    async fn http_transport_login(
        &self,
        request: &GridLoginRequest,
        endpoint: &str,
        method: Method,
        codec: &dyn LoginCodec,
    ) -> Result<GridLoginResponse, ConnectionError> {
        let client = reqwest::Client::builder()
            .timeout(self.config.connect_timeout)
            .build()?;

        let body = codec.encode_request(request)?;
        let response = client
            .request(method, endpoint)
            .header(CONTENT_TYPE, codec.content_type())
            .body(body)
            .send()
            .await?;
        let status = response.status();
        let bytes = response.bytes().await?;

        if !status.is_success() {
            return Err(ConnectionError::HttpStatus {
                status,
                body: String::from_utf8_lossy(&bytes).to_string(),
            });
        }

        let decoded = codec.decode_response(&bytes)?;
        Ok(decoded)
    }

    fn apply_bootstrap_session(&mut self, bootstrap: &SessionBootstrap) {
        self.session = Some(Session {
            account_name: bootstrap.agent_id.clone(),
            session_token: bootstrap.session_id.clone(),
            seed_capability: Some(bootstrap.seed_capability.clone()),
        });
        self.first_simulator_handshake_prerequisites = Some(FirstSimulatorHandshakePrerequisites {
            agent_id: bootstrap.agent_id.clone(),
            session_id: bootstrap.session_id.clone(),
            circuit_code: bootstrap.circuit_code,
            target: FirstSimulatorTarget {
                sim_ip: bootstrap.first_sim.sim_ip.clone(),
                sim_port: bootstrap.first_sim.sim_port,
                region_x: bootstrap.first_sim.region_x,
                region_y: bootstrap.first_sim.region_y,
                seed_capability: bootstrap.seed_capability.clone(),
            },
        });
        self.first_simulator_handshake_state = None;
        self.retained_probe_socket = None;
        self.first_simulator_handshake_send_diagnostics.clear();
        self.first_simulator_handshake_receive_diagnostics.clear();
        self.first_simulator_socket_diagnostics.clear();
        self.early_simulator_traffic_observations.clear();
        self.region_transition_control_observations.clear();
        self.simulator_payload_decode_summary = SimulatorPayloadDecodeSummary::default();
        self.object_feed_objects.clear();
        self.object_feed_recent_kills.clear();
        self.object_feed_tick = 0;
        self.next_first_simulator_packet_id = 1;
        self.pending_first_simulator_ack_ids.clear();
        self.pending_object_cache_miss_ids.clear();
        self.pending_region_handshake_reply_flags = None;
        self.region_handshake_reply_sent = false;
        self.last_bootstrap_sender_endpoint = None;
        self.last_region_handshake_sender_endpoint = None;
        self.continuity_summary = RegionContinuitySummary {
            phase: HandoffPhase::None,
            outcome: viewer_core::HandoffOutcome::Normal,
            reason: viewer_core::HandoffReason::None,
            phase_age_ms: 0,
            last_probe_result: None,
            last_probe_time_unix_ms: None,
            active_region_coords: Some([
                bootstrap.first_sim.region_x,
                bootstrap.first_sim.region_y,
            ]),
            previous_region_coords: None,
            neighbors: Vec::new(),
        };
    }

    fn next_first_simulator_packet_id(&mut self) -> u32 {
        let packet_id = self.next_first_simulator_packet_id;
        self.next_first_simulator_packet_id =
            self.next_first_simulator_packet_id.wrapping_add(1).max(1);
        packet_id
    }

    async fn send_first_simulator_handshake_datagram(
        &mut self,
        action: FirstSimulatorHandshakeAction,
        target: &FirstSimulatorTarget,
        payload: &[u8],
    ) -> Result<(), ConnectionError> {
        let socket = UdpSocket::bind("0.0.0.0:0").await.map_err(|err| {
            ConnectionError::FirstSimulatorHandshakeSendFailed {
                action,
                target: format!("{}:{}", target.sim_ip, target.sim_port),
                reason: err.to_string(),
            }
        })?;
        self.record_first_simulator_socket_diagnostic(
            FirstSimulatorSocketDiagnosticKind::FreshBind,
            format!("send_first_simulator_handshake_datagram:{action:?}"),
            socket_local_addr_text(&socket),
            Some(format!("{}:{}", target.sim_ip, target.sim_port)),
            decode_first_simulator_packet_header(payload).map(|header| header.message_number),
            Some(payload.len()),
        );
        self.send_first_simulator_handshake_datagram_with_socket(action, target, payload, &socket)
            .await
    }

    async fn send_first_simulator_handshake_datagram_with_socket(
        &mut self,
        action: FirstSimulatorHandshakeAction,
        target: &FirstSimulatorTarget,
        payload: &[u8],
        socket: &UdpSocket,
    ) -> Result<(), ConnectionError> {
        let packet_id = decode_lludp_packet_id(payload).unwrap_or_default();
        let packet_message_number =
            decode_first_simulator_packet_header(payload).map(|header| header.message_number);
        let ack_batch = self.first_simulator_ack_batch(payload);
        let outbound_payload = if ack_batch.is_empty() {
            payload.to_vec()
        } else {
            append_lludp_ack_trailer(payload, &ack_batch)
        };
        let target_text = format!("{}:{}", target.sim_ip, target.sim_port);
        let socket_addr = match target_text.parse::<SocketAddr>() {
            Ok(addr) => addr,
            Err(err) => {
                self.first_simulator_handshake_send_diagnostics.push(
                    FirstSimulatorHandshakeSendDiagnostic {
                        action,
                        target: target_text.clone(),
                        packet_id,
                        packet_message_number,
                        appended_ack_ids: ack_batch.clone(),
                        payload_len: outbound_payload.len(),
                        elapsed_ms: 0,
                        success: false,
                        error: Some(err.to_string()),
                    },
                );
                return Err(ConnectionError::InvalidFirstSimulatorTransportTarget {
                    target: target_text,
                    reason: err.to_string(),
                });
            }
        };

        let started = Instant::now();
        let send_result = socket.send_to(&outbound_payload, socket_addr).await;
        let elapsed_ms = started.elapsed().as_millis();

        match send_result {
            Ok(sent_len) if sent_len == outbound_payload.len() => {
                if !ack_batch.is_empty() {
                    self.pending_first_simulator_ack_ids
                        .drain(0..ack_batch.len());
                }
                self.record_first_simulator_socket_diagnostic(
                    FirstSimulatorSocketDiagnosticKind::Send,
                    format!("send_first_simulator_handshake_datagram_with_socket:{action:?}"),
                    socket_local_addr_text(socket),
                    Some(target_text.clone()),
                    packet_message_number,
                    Some(outbound_payload.len()),
                );
                self.first_simulator_handshake_send_diagnostics.push(
                    FirstSimulatorHandshakeSendDiagnostic {
                        action,
                        target: target_text,
                        packet_id,
                        packet_message_number,
                        appended_ack_ids: ack_batch,
                        payload_len: outbound_payload.len(),
                        elapsed_ms,
                        success: true,
                        error: None,
                    },
                );
                Ok(())
            }
            Ok(sent_len) => {
                let reason = format!(
                    "partial datagram send ({sent_len}/{})",
                    outbound_payload.len()
                );
                self.first_simulator_handshake_send_diagnostics.push(
                    FirstSimulatorHandshakeSendDiagnostic {
                        action,
                        target: target_text.clone(),
                        packet_id,
                        packet_message_number,
                        appended_ack_ids: ack_batch,
                        payload_len: outbound_payload.len(),
                        elapsed_ms,
                        success: false,
                        error: Some(reason.clone()),
                    },
                );
                Err(ConnectionError::FirstSimulatorHandshakeSendFailed {
                    action,
                    target: target_text,
                    reason,
                })
            }
            Err(err) => {
                self.first_simulator_handshake_send_diagnostics.push(
                    FirstSimulatorHandshakeSendDiagnostic {
                        action,
                        target: target_text.clone(),
                        packet_id,
                        packet_message_number,
                        appended_ack_ids: ack_batch,
                        payload_len: outbound_payload.len(),
                        elapsed_ms,
                        success: false,
                        error: Some(err.to_string()),
                    },
                );
                Err(ConnectionError::FirstSimulatorHandshakeSendFailed {
                    action,
                    target: target_text,
                    reason: err.to_string(),
                })
            }
        }
    }

    async fn prepare_chat_socket(
        &mut self,
        bind: &str,
        reason: &str,
    ) -> Result<(FirstSimulatorHandshakePrerequisites, UdpSocket), ConnectionError> {
        let prerequisites = self
            .first_simulator_handshake_prerequisites
            .clone()
            .ok_or(ConnectionError::MissingFirstSimulatorHandshakePrerequisites)?;

        let socket = UdpSocket::bind(bind).await.map_err(|err| {
            ConnectionError::InvalidFirstSimulatorReceiveBind {
                bind: bind.to_string(),
                reason: err.to_string(),
            }
        })?;
        self.record_first_simulator_socket_diagnostic(
            FirstSimulatorSocketDiagnosticKind::FreshBind,
            format!("prepare_chat_socket:{reason}"),
            socket_local_addr_text(&socket),
            Some(format!(
                "{}:{}",
                prerequisites.target.sim_ip, prerequisites.target.sim_port
            )),
            None,
            None,
        );

        let use_circuit = encode_first_simulator_use_circuit_code_payload(
            &prerequisites,
            self.next_first_simulator_packet_id(),
        )?;
        self.send_first_simulator_handshake_datagram_with_socket(
            FirstSimulatorHandshakeAction::UseCircuitCode,
            &prerequisites.target,
            &use_circuit,
            &socket,
        )
        .await?;

        let complete_movement = encode_first_simulator_complete_agent_movement_payload(
            &prerequisites,
            self.next_first_simulator_packet_id(),
        )?;
        self.send_first_simulator_handshake_datagram_with_socket(
            FirstSimulatorHandshakeAction::CompleteAgentMovement,
            &prerequisites.target,
            &complete_movement,
            &socket,
        )
        .await?;

        Ok((prerequisites, socket))
    }

    async fn receive_nearby_chat_on_socket(
        &mut self,
        socket: &UdpSocket,
        bind: &str,
        receive_timeout: Duration,
        receive_max_packets: usize,
    ) -> Result<Vec<NearbyChatMessage>, ConnectionError> {
        let mut nearby = Vec::new();
        let mut buf = vec![0u8; 2048];
        let local_addr = socket_local_addr_text(socket);
        for _ in 0..receive_max_packets {
            let recv = timeout(receive_timeout, socket.recv_from(&mut buf)).await;
            let (received_len, sender) = match recv {
                Ok(Ok(parts)) => parts,
                Ok(Err(err)) => {
                    return Err(ConnectionError::FirstSimulatorReceiveFailed {
                        bind: bind.to_string(),
                        reason: err.to_string(),
                    });
                }
                Err(_) => break,
            };
            let payload = &buf[..received_len];
            self.record_first_simulator_socket_diagnostic(
                FirstSimulatorSocketDiagnosticKind::Receive,
                "receive_nearby_chat_on_socket",
                local_addr.clone(),
                Some(sender.to_string()),
                decode_first_simulator_packet_header(payload).map(|header| header.message_number),
                Some(received_len),
            );
            if let Ok(classification) = self.observe_first_simulator_inbound_payload(payload) {
                self.note_first_simulator_sender_endpoint(&sender, &classification);
            }
            if let Some(chat) = decode_chat_from_simulator(payload) {
                nearby.push(chat);
            }
        }
        Ok(nearby)
    }
}

pub async fn poll_event_queue_url_once(
    event_queue_url: &str,
    ack: u64,
    timeout: Duration,
) -> Result<EventQueuePollResult, ConnectionError> {
    let client = reqwest::Client::builder()
        .timeout(timeout.max(Duration::from_secs(5)))
        .build()?;
    let request_body = llsd_event_queue_request(ack, false);
    let response = client
        .post(event_queue_url)
        .header(CONTENT_TYPE, LLSD_XML_CONTENT_TYPE)
        .header(ACCEPT, LLSD_XML_CONTENT_TYPE)
        .body(request_body)
        .send()
        .await;
    let response = match response {
        Ok(response) => response,
        Err(err) => {
            if err.is_timeout() {
                return Ok(EventQueuePollResult {
                    id: Some(ack),
                    events: Vec::new(),
                });
            }
            return Err(ConnectionError::Http(err));
        }
    };

    let status = response.status();
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_ascii_lowercase());
    let bytes = response.bytes().await?;
    if !status.is_success() {
        let body = String::from_utf8_lossy(&bytes).to_string();
        if matches!(status.as_u16(), 499 | 502 | 503 | 504)
            || is_retryable_event_queue_http_failure(status, &body)
        {
            return Ok(EventQueuePollResult {
                id: Some(ack),
                events: Vec::new(),
            });
        }
        return Err(ConnectionError::HttpStatus { status, body });
    }
    parse_event_queue_poll_response(&bytes, content_type.as_deref())
}

pub async fn resolve_avatar_display_names_with_timeout(
    capability_url: &str,
    agent_ids: &[String],
    timeout: Duration,
) -> Result<Vec<ResolvedAvatarName>, ConnectionError> {
    let ids: Vec<&str> = agent_ids
        .iter()
        .map(String::as_str)
        .filter(|id| !id.trim().is_empty())
        .collect();
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut url = capability_url.to_string();
    if !url.contains('?') {
        url.push('?');
    } else if !url.ends_with('?') && !url.ends_with('&') {
        url.push('&');
    }
    for (idx, id) in ids.iter().enumerate() {
        if idx > 0 {
            url.push('&');
        }
        url.push_str("ids=");
        url.push_str(id);
    }
    let client = reqwest::Client::builder()
        .timeout(timeout.max(Duration::from_secs(5)))
        .build()?;
    let response = client
        .get(url)
        .header(ACCEPT, "application/llsd+xml, application/json")
        .send()
        .await?;
    let status = response.status();
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_ascii_lowercase());
    let bytes = response.bytes().await?;
    if !status.is_success() {
        return Err(ConnectionError::HttpStatus {
            status,
            body: String::from_utf8_lossy(&bytes).to_string(),
        });
    }
    parse_resolved_avatar_names(&bytes, content_type.as_deref())
}

fn llsd_string_array(items: &[&str]) -> String {
    let mut xml = String::from("<llsd><array>");
    for item in items {
        xml.push_str(&format!("<string>{}</string>", escape_xml(item)));
    }
    xml.push_str("</array></llsd>");
    xml
}

fn llsd_event_queue_request(ack: u64, done: bool) -> String {
    let done_str = if done { "true" } else { "false" };
    format!(
        "<llsd><map><key>ack</key><integer>{ack}</integer><key>done</key><boolean>{done_str}</boolean></map></llsd>"
    )
}

fn encode_chat_from_viewer_payload(
    prerequisites: &FirstSimulatorHandshakePrerequisites,
    packet_id: u32,
    text: &str,
    chat_type: u8,
    channel: i32,
) -> Result<Vec<u8>, ConnectionError> {
    let agent_id = parse_uuid_bytes(&prerequisites.agent_id)?;
    let session_id = parse_uuid_bytes(&prerequisites.session_id)?;
    let mut message = text.as_bytes().to_vec();
    // Firestorm packs ChatData.Message as a variable string including a trailing NUL.
    message.push(0);
    let msg_len = u16::try_from(message.len())
        .map_err(|_| ConnectionError::CapabilityDecode(String::from("chat message too long")))?;
    let mut body = Vec::with_capacity(16 + 16 + 2 + message.len() + 1 + 4);
    body.extend_from_slice(&agent_id);
    body.extend_from_slice(&session_id);
    body.extend_from_slice(&msg_len.to_le_bytes());
    body.extend_from_slice(&message);
    body.push(chat_type);
    body.extend_from_slice(&channel.to_le_bytes());
    Ok(encode_lludp_low_frequency_packet(
        packet_id,
        LLUDP_CHAT_FROM_VIEWER_LOW_ID,
        &body,
    ))
}

fn encode_retrieve_instant_messages_payload(
    prerequisites: &FirstSimulatorHandshakePrerequisites,
    packet_id: u32,
) -> Result<Vec<u8>, ConnectionError> {
    let agent_id = parse_uuid_bytes(&prerequisites.agent_id)?;
    let session_id = parse_uuid_bytes(&prerequisites.session_id)?;
    let mut body = Vec::with_capacity(32);
    body.extend_from_slice(&agent_id);
    body.extend_from_slice(&session_id);
    Ok(encode_lludp_low_frequency_packet(
        packet_id,
        LLUDP_RETRIEVE_INSTANT_MESSAGES_LOW_ID,
        &body,
    ))
}

fn encode_mute_list_request_payload(
    prerequisites: &FirstSimulatorHandshakePrerequisites,
    packet_id: u32,
) -> Result<Vec<u8>, ConnectionError> {
    let agent_id = parse_uuid_bytes(&prerequisites.agent_id)?;
    let session_id = parse_uuid_bytes(&prerequisites.session_id)?;
    let mut body = Vec::with_capacity(16 + 16 + 4);
    body.extend_from_slice(&agent_id);
    body.extend_from_slice(&session_id);
    body.extend_from_slice(&0u32.to_le_bytes());
    Ok(encode_lludp_low_frequency_packet(
        packet_id,
        LLUDP_MUTE_LIST_REQUEST_LOW_ID,
        &body,
    ))
}

fn encode_money_balance_request_payload(
    prerequisites: &FirstSimulatorHandshakePrerequisites,
    packet_id: u32,
) -> Result<Vec<u8>, ConnectionError> {
    let agent_id = parse_uuid_bytes(&prerequisites.agent_id)?;
    let session_id = parse_uuid_bytes(&prerequisites.session_id)?;
    let mut body = Vec::with_capacity(16 + 16 + 16);
    body.extend_from_slice(&agent_id);
    body.extend_from_slice(&session_id);
    body.extend_from_slice(&[0u8; 16]);
    Ok(encode_lludp_low_frequency_packet(
        packet_id,
        LLUDP_MONEY_BALANCE_REQUEST_LOW_ID,
        &body,
    ))
}

fn encode_agent_data_update_request_payload(
    prerequisites: &FirstSimulatorHandshakePrerequisites,
    packet_id: u32,
) -> Result<Vec<u8>, ConnectionError> {
    let agent_id = parse_uuid_bytes(&prerequisites.agent_id)?;
    let session_id = parse_uuid_bytes(&prerequisites.session_id)?;
    let mut body = Vec::with_capacity(32);
    body.extend_from_slice(&agent_id);
    body.extend_from_slice(&session_id);
    Ok(encode_lludp_low_frequency_packet(
        packet_id,
        LLUDP_AGENT_DATA_UPDATE_REQUEST_LOW_ID,
        &body,
    ))
}

fn encode_packet_ack_payload(packet_id: u32, ack_ids: &[u32]) -> Result<Vec<u8>, ConnectionError> {
    let ack_count = u8::try_from(ack_ids.len()).map_err(|_| {
        ConnectionError::CapabilityDecode(String::from("too many packet ack ids in one datagram"))
    })?;
    let mut body = Vec::with_capacity(1 + ack_ids.len() * 4);
    body.push(ack_count);
    for ack_id in ack_ids {
        body.extend_from_slice(&ack_id.to_le_bytes());
    }
    Ok(encode_lludp_low_frequency_packet(
        packet_id,
        LLUDP_PACKET_ACK_LOW_ID,
        &body,
    ))
}

fn encode_request_multiple_objects_payload(
    prerequisites: &FirstSimulatorHandshakePrerequisites,
    packet_id: u32,
    local_ids: &[u32],
) -> Result<Vec<u8>, ConnectionError> {
    let block_count = u8::try_from(local_ids.len()).map_err(|_| {
        ConnectionError::CapabilityDecode(String::from(
            "too many RequestMultipleObjects blocks in one datagram",
        ))
    })?;
    let agent_id = parse_uuid_bytes(&prerequisites.agent_id)?;
    let session_id = parse_uuid_bytes(&prerequisites.session_id)?;
    let mut body = Vec::with_capacity(16 + 16 + 1 + local_ids.len() * 5);
    body.extend_from_slice(&agent_id);
    body.extend_from_slice(&session_id);
    body.push(block_count);
    for local_id in local_ids {
        body.push(LLUDP_CACHE_MISS_TYPE_TOTAL);
        body.extend_from_slice(&local_id.to_le_bytes());
    }
    Ok(encode_lludp_medium_frequency_packet(
        packet_id,
        LLUDP_REQUEST_MULTIPLE_OBJECTS_MEDIUM_ID,
        &body,
        true,
    ))
}

fn encode_avatar_properties_request_payload(
    prerequisites: &FirstSimulatorHandshakePrerequisites,
    packet_id: u32,
    avatar_id: &str,
) -> Result<Vec<u8>, ConnectionError> {
    let agent_id = parse_uuid_bytes(&prerequisites.agent_id)?;
    let session_id = parse_uuid_bytes(&prerequisites.session_id)?;
    let avatar_id = parse_uuid_bytes(avatar_id)?;
    let mut body = Vec::with_capacity(48);
    body.extend_from_slice(&agent_id);
    body.extend_from_slice(&session_id);
    body.extend_from_slice(&avatar_id);
    Ok(encode_lludp_low_frequency_packet(
        packet_id,
        LLUDP_AVATAR_PROPERTIES_REQUEST_LOW_ID,
        &body,
    ))
}

fn encode_classified_info_request_payload(
    prerequisites: &FirstSimulatorHandshakePrerequisites,
    packet_id: u32,
    classified_id: &str,
) -> Result<Vec<u8>, ConnectionError> {
    let agent_id = parse_uuid_bytes(&prerequisites.agent_id)?;
    let session_id = parse_uuid_bytes(&prerequisites.session_id)?;
    let classified_id = parse_uuid_bytes(classified_id)?;
    let mut body = Vec::with_capacity(48);
    body.extend_from_slice(&agent_id);
    body.extend_from_slice(&session_id);
    body.extend_from_slice(&classified_id);
    Ok(encode_lludp_low_frequency_packet(
        packet_id,
        LLUDP_CLASSIFIED_INFO_REQUEST_LOW_ID,
        &body,
    ))
}

fn encode_generic_message_payload(
    prerequisites: &FirstSimulatorHandshakePrerequisites,
    packet_id: u32,
    method: &str,
    params: &[&str],
) -> Result<Vec<u8>, ConnectionError> {
    let agent_id = parse_uuid_bytes(&prerequisites.agent_id)?;
    let session_id = parse_uuid_bytes(&prerequisites.session_id)?;
    let mut body = Vec::new();
    body.extend_from_slice(&agent_id);
    body.extend_from_slice(&session_id);
    body.extend_from_slice(&[0u8; 16]); // TransactionID
    let method = method.as_bytes();
    let method_len = u8::try_from(method.len())
        .map_err(|_| ConnectionError::CapabilityDecode(String::from("generic method too long")))?;
    body.push(method_len);
    body.extend_from_slice(method);
    body.extend_from_slice(&[0u8; 16]); // Invoice
    let block_count = u8::try_from(params.len()).map_err(|_| {
        ConnectionError::CapabilityDecode(String::from("generic parameter count overflow"))
    })?;
    body.push(block_count);
    for param in params {
        let bytes = param.as_bytes();
        let len = u8::try_from(bytes.len()).map_err(|_| {
            ConnectionError::CapabilityDecode(String::from("generic parameter too long"))
        })?;
        body.push(len);
        body.extend_from_slice(bytes);
    }
    Ok(encode_lludp_low_frequency_packet(
        packet_id,
        LLUDP_GENERIC_MESSAGE_LOW_ID,
        &body,
    ))
}

fn encode_improved_instant_message_payload(
    prerequisites: &FirstSimulatorHandshakePrerequisites,
    packet_id: u32,
    to_agent_id: &str,
    im_session_id: &str,
    from_name: &str,
    message: &str,
) -> Result<Vec<u8>, ConnectionError> {
    let agent_id = parse_uuid_bytes(&prerequisites.agent_id)?;
    let session_id = parse_uuid_bytes(&prerequisites.session_id)?;
    let to_id = parse_uuid_bytes(to_agent_id)?;
    let im_id = parse_uuid_bytes(im_session_id)?;

    let mut from_name_buf = from_name.as_bytes().to_vec();
    from_name_buf.push(0);
    let from_name_len = u8::try_from(from_name_buf.len())
        .map_err(|_| ConnectionError::CapabilityDecode(String::from("from_name too long")))?;

    let mut message_buf = message.as_bytes().to_vec();
    message_buf.push(0);
    let message_len = u16::try_from(message_buf.len())
        .map_err(|_| ConnectionError::CapabilityDecode(String::from("im message too long")))?;

    let binary_bucket = [0u8; 1];
    let binary_bucket_len = u16::try_from(binary_bucket.len())
        .map_err(|_| ConnectionError::CapabilityDecode(String::from("invalid binary bucket")))?;

    let mut body = Vec::with_capacity(128 + from_name_buf.len() + message_buf.len());
    body.extend_from_slice(&agent_id);
    body.extend_from_slice(&session_id);
    body.push(0); // FromGroup
    body.extend_from_slice(&to_id);
    body.extend_from_slice(&0u32.to_le_bytes()); // ParentEstateID
    body.extend_from_slice(&[0u8; 16]); // RegionID
    body.extend_from_slice(&0f32.to_le_bytes());
    body.extend_from_slice(&0f32.to_le_bytes());
    body.extend_from_slice(&0f32.to_le_bytes());
    body.push(0); // Offline
    body.push(0); // Dialog: IM_NOTHING_SPECIAL
    body.extend_from_slice(&im_id);
    body.extend_from_slice(&0u32.to_le_bytes()); // Timestamp
    body.push(from_name_len);
    body.extend_from_slice(&from_name_buf);
    body.extend_from_slice(&message_len.to_le_bytes());
    body.extend_from_slice(&message_buf);
    body.extend_from_slice(&binary_bucket_len.to_le_bytes());
    body.extend_from_slice(&binary_bucket);
    Ok(encode_lludp_low_frequency_packet(
        packet_id,
        LLUDP_IMPROVED_INSTANT_MESSAGE_LOW_ID,
        &body,
    ))
}

fn encode_first_simulator_use_circuit_code_payload(
    prerequisites: &FirstSimulatorHandshakePrerequisites,
    packet_id: u32,
) -> Result<Vec<u8>, ConnectionError> {
    let session_id = parse_uuid_bytes(&prerequisites.session_id)?;
    let agent_id = parse_uuid_bytes(&prerequisites.agent_id)?;
    let mut body = Vec::with_capacity(36);
    body.extend_from_slice(&prerequisites.circuit_code.to_le_bytes());
    body.extend_from_slice(&session_id);
    body.extend_from_slice(&agent_id);
    Ok(encode_lludp_low_frequency_packet(
        packet_id,
        LLUDP_USE_CIRCUIT_CODE_LOW_ID,
        &body,
    ))
}

fn encode_first_simulator_complete_agent_movement_payload(
    prerequisites: &FirstSimulatorHandshakePrerequisites,
    packet_id: u32,
) -> Result<Vec<u8>, ConnectionError> {
    let session_id = parse_uuid_bytes(&prerequisites.session_id)?;
    let agent_id = parse_uuid_bytes(&prerequisites.agent_id)?;
    let mut body = Vec::with_capacity(36);
    body.extend_from_slice(&agent_id);
    body.extend_from_slice(&session_id);
    body.extend_from_slice(&prerequisites.circuit_code.to_le_bytes());
    Ok(encode_lludp_low_frequency_packet(
        packet_id,
        LLUDP_COMPLETE_AGENT_MOVEMENT_LOW_ID,
        &body,
    ))
}

fn encode_region_handshake_reply_payload(
    prerequisites: &FirstSimulatorHandshakePrerequisites,
    packet_id: u32,
    region_flags: u32,
) -> Result<Vec<u8>, ConnectionError> {
    let agent_id = parse_uuid_bytes(&prerequisites.agent_id)?;
    let session_id = parse_uuid_bytes(&prerequisites.session_id)?;
    let mut body = Vec::with_capacity(16 + 16 + 4);
    body.extend_from_slice(&agent_id);
    body.extend_from_slice(&session_id);
    body.extend_from_slice(&region_flags.to_le_bytes());
    Ok(encode_lludp_low_frequency_packet(
        packet_id,
        LLUDP_REGION_HANDSHAKE_REPLY_LOW_ID,
        &body,
    ))
}

fn viewer_region_handshake_reply_flags() -> u32 {
    REGION_HANDSHAKE_REPLY_FLAG_SUPPORTS_SELF_APPEARANCE
}

fn encode_agent_throttle_payload(
    prerequisites: &FirstSimulatorHandshakePrerequisites,
    packet_id: u32,
    gen_counter: u32,
    throttles: &[f32; 7],
) -> Result<Vec<u8>, ConnectionError> {
    let agent_id = parse_uuid_bytes(&prerequisites.agent_id)?;
    let session_id = parse_uuid_bytes(&prerequisites.session_id)?;
    let mut throttle_bytes = Vec::with_capacity(throttles.len() * 4);
    for throttle in throttles {
        throttle_bytes.extend_from_slice(&(throttle * 1024.0).to_le_bytes());
    }
    let throttle_len = u8::try_from(throttle_bytes.len())
        .map_err(|_| ConnectionError::CapabilityDecode(String::from("agent throttle overflow")))?;
    let mut body = Vec::with_capacity(16 + 16 + 4 + 4 + 1 + throttle_bytes.len());
    body.extend_from_slice(&agent_id);
    body.extend_from_slice(&session_id);
    body.extend_from_slice(&prerequisites.circuit_code.to_le_bytes());
    body.extend_from_slice(&gen_counter.to_le_bytes());
    body.push(throttle_len);
    body.extend_from_slice(&throttle_bytes);
    Ok(encode_lludp_low_frequency_packet(
        packet_id,
        LLUDP_AGENT_THROTTLE_LOW_ID,
        &body,
    ))
}

fn encode_agent_height_width_payload(
    prerequisites: &FirstSimulatorHandshakePrerequisites,
    packet_id: u32,
    gen_counter: u32,
    height: u16,
    width: u16,
) -> Result<Vec<u8>, ConnectionError> {
    let agent_id = parse_uuid_bytes(&prerequisites.agent_id)?;
    let session_id = parse_uuid_bytes(&prerequisites.session_id)?;
    let mut body = Vec::with_capacity(16 + 16 + 4 + 4 + 2 + 2);
    body.extend_from_slice(&agent_id);
    body.extend_from_slice(&session_id);
    body.extend_from_slice(&prerequisites.circuit_code.to_le_bytes());
    body.extend_from_slice(&gen_counter.to_le_bytes());
    body.extend_from_slice(&height.to_le_bytes());
    body.extend_from_slice(&width.to_le_bytes());
    Ok(encode_lludp_low_frequency_packet(
        packet_id,
        LLUDP_AGENT_HEIGHT_WIDTH_LOW_ID,
        &body,
    ))
}

fn encode_set_always_run_payload(
    prerequisites: &FirstSimulatorHandshakePrerequisites,
    packet_id: u32,
    always_run: bool,
) -> Result<Vec<u8>, ConnectionError> {
    let agent_id = parse_uuid_bytes(&prerequisites.agent_id)?;
    let session_id = parse_uuid_bytes(&prerequisites.session_id)?;
    let mut body = Vec::with_capacity(16 + 16 + 1);
    body.extend_from_slice(&agent_id);
    body.extend_from_slice(&session_id);
    body.push(u8::from(always_run));
    Ok(encode_lludp_low_frequency_packet(
        packet_id,
        LLUDP_SET_ALWAYS_RUN_LOW_ID,
        &body,
    ))
}

fn encode_agent_animation_payload(
    prerequisites: &FirstSimulatorHandshakePrerequisites,
    packet_id: u32,
    anim_id: &str,
    start_anim: bool,
) -> Result<Vec<u8>, ConnectionError> {
    let agent_id = parse_uuid_bytes(&prerequisites.agent_id)?;
    let session_id = parse_uuid_bytes(&prerequisites.session_id)?;
    let anim_id = parse_uuid_bytes(anim_id)?;
    let mut body = Vec::with_capacity(16 + 16 + 1 + 16 + 1 + 1 + 1);
    body.extend_from_slice(&agent_id);
    body.extend_from_slice(&session_id);
    body.push(1); // AnimationList count
    body.extend_from_slice(&anim_id);
    body.push(u8::from(start_anim));
    body.push(1); // PhysicalAvatarEventList count
    body.push(0); // TypeData length
    Ok(encode_lludp_high_frequency_packet(
        packet_id,
        LLUDP_AGENT_ANIMATION_HIGH_ID,
        &body,
        true,
    ))
}

#[allow(clippy::too_many_arguments)]
fn encode_agent_update_payload(
    prerequisites: &FirstSimulatorHandshakePrerequisites,
    packet_id: u32,
    camera_center: [f32; 3],
    camera_at_axis: [f32; 3],
    camera_left_axis: [f32; 3],
    camera_up_axis: [f32; 3],
    far: f32,
    control_flags: u32,
    flags: u8,
    reliable: bool,
) -> Result<Vec<u8>, ConnectionError> {
    let agent_id = parse_uuid_bytes(&prerequisites.agent_id)?;
    let session_id = parse_uuid_bytes(&prerequisites.session_id)?;
    let mut body = Vec::with_capacity(16 + 16 + 12 + 12 + 1 + 12 + 12 + 12 + 12 + 4 + 4 + 1);
    body.extend_from_slice(&agent_id);
    body.extend_from_slice(&session_id);
    body.extend_from_slice(&[0f32.to_le_bytes(), 0f32.to_le_bytes(), 0f32.to_le_bytes()].concat());
    body.extend_from_slice(&[0f32.to_le_bytes(), 0f32.to_le_bytes(), 0f32.to_le_bytes()].concat());
    body.push(0);
    for component in camera_center {
        body.extend_from_slice(&component.to_le_bytes());
    }
    for component in camera_at_axis {
        body.extend_from_slice(&component.to_le_bytes());
    }
    for component in camera_left_axis {
        body.extend_from_slice(&component.to_le_bytes());
    }
    for component in camera_up_axis {
        body.extend_from_slice(&component.to_le_bytes());
    }
    body.extend_from_slice(&far.to_le_bytes());
    body.extend_from_slice(&control_flags.to_le_bytes());
    body.push(flags);
    Ok(encode_lludp_high_frequency_packet(
        packet_id,
        LLUDP_AGENT_UPDATE_HIGH_ID,
        &body,
        reliable,
    ))
}

fn parse_uuid_bytes(raw: &str) -> Result<[u8; 16], ConnectionError> {
    let compact = raw.replace('-', "");
    if compact.len() != 32 {
        return Err(ConnectionError::CapabilityDecode(format!(
            "invalid UUID length for '{raw}'"
        )));
    }

    let mut bytes = [0u8; 16];
    for (idx, slot) in bytes.iter_mut().enumerate() {
        let start = idx * 2;
        let end = start + 2;
        *slot = u8::from_str_radix(&compact[start..end], 16).map_err(|err| {
            ConnectionError::CapabilityDecode(format!("invalid UUID hex for '{raw}': {err}"))
        })?;
    }
    Ok(bytes)
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

fn encode_lludp_low_frequency_packet(packet_id: u32, message_id: u16, body: &[u8]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(LLUDP_PACKET_ID_SIZE + 4 + body.len());
    payload.push(LLUDP_RELIABLE_FLAG);
    payload.extend_from_slice(&packet_id.to_be_bytes());
    payload.push(0);
    payload.push(LLUDP_MESSAGE_PREFIX);
    payload.push(LLUDP_MESSAGE_PREFIX);
    payload.extend_from_slice(&message_id.to_be_bytes());
    payload.extend_from_slice(body);
    payload
}

fn encode_lludp_medium_frequency_packet(
    packet_id: u32,
    message_id: u8,
    body: &[u8],
    reliable: bool,
) -> Vec<u8> {
    let mut payload = Vec::with_capacity(LLUDP_PACKET_ID_SIZE + 2 + body.len());
    payload.push(if reliable { LLUDP_RELIABLE_FLAG } else { 0 });
    payload.extend_from_slice(&packet_id.to_be_bytes());
    payload.push(0);
    payload.push(LLUDP_MESSAGE_PREFIX);
    payload.push(message_id);
    payload.extend_from_slice(body);
    payload
}

fn encode_lludp_high_frequency_packet(
    packet_id: u32,
    message_id: u8,
    body: &[u8],
    reliable: bool,
) -> Vec<u8> {
    let mut payload = Vec::with_capacity(LLUDP_PACKET_ID_SIZE + 1 + body.len());
    payload.push(if reliable { LLUDP_RELIABLE_FLAG } else { 0 });
    payload.extend_from_slice(&packet_id.to_be_bytes());
    payload.push(0);
    payload.push(message_id);
    payload.extend_from_slice(body);
    payload
}

fn decode_lludp_packet_id(payload: &[u8]) -> Option<u32> {
    if payload.len() < LLUDP_PACKET_ID_SIZE {
        return None;
    }
    let id_bytes: [u8; 4] = payload[1..5].try_into().ok()?;
    Some(u32::from_be_bytes(id_bytes))
}

fn socket_local_addr_text(socket: &UdpSocket) -> Option<String> {
    socket.local_addr().ok().map(|addr| addr.to_string())
}

fn parse_socket_port_from_text(text: &str) -> Option<u16> {
    text.parse::<SocketAddr>().ok().map(|addr| addr.port())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FirstSimulatorPacketHeader {
    flags: u8,
    packet_id: u32,
    message_number: u32,
    body_offset: usize,
    body_end: usize,
}

impl FirstSimulatorPacketHeader {
    fn body<'a>(&self, payload: &'a [u8]) -> Option<&'a [u8]> {
        payload.get(self.body_offset..self.body_end)
    }
}

fn lludp_low_frequency_message_number(message_id: u16) -> u32 {
    LLUDP_LOW_FREQUENCY_PREFIX | u32::from(message_id)
}

fn lludp_medium_frequency_message_number(message_id: u8) -> u32 {
    (u32::from(LLUDP_MESSAGE_PREFIX) << 8) | u32::from(message_id)
}

fn decode_first_simulator_packet_header(payload: &[u8]) -> Option<FirstSimulatorPacketHeader> {
    if payload.len() < LLUDP_MINIMUM_VALID_PACKET_SIZE {
        return None;
    }

    let flags = payload[0];
    let packet_id = decode_lludp_packet_id(payload)?;
    let extra_header_len = usize::from(payload[LLUDP_PACKET_ID_SIZE - 1]);
    let header_start = LLUDP_PACKET_ID_SIZE.checked_add(extra_header_len)?;
    if payload.len() <= header_start {
        return None;
    }

    let header = &payload[header_start..];
    if header.is_empty() {
        return None;
    }

    let (message_number, consumed_header_bytes) = if header[0] != LLUDP_MESSAGE_PREFIX {
        (u32::from(header[0]), 1usize)
    } else if header.len() >= 2 && header[1] != LLUDP_MESSAGE_PREFIX {
        (
            (u32::from(LLUDP_MESSAGE_PREFIX) << 8) | u32::from(header[1]),
            2usize,
        )
    } else if header.len() >= 4 && header[1] == LLUDP_MESSAGE_PREFIX {
        let low = u16::from_be_bytes([header[2], header[3]]);
        (lludp_low_frequency_message_number(low), 4usize)
    } else {
        return None;
    };

    let body_offset = header_start + consumed_header_bytes;
    let ack_trailer_len = decode_lludp_ack_trailer_len(payload, flags, body_offset)?;
    let body_end = payload.len().checked_sub(ack_trailer_len)?;
    if body_end < body_offset {
        return None;
    }

    Some(FirstSimulatorPacketHeader {
        flags,
        packet_id,
        message_number,
        body_offset,
        body_end,
    })
}

fn decode_lludp_ack_trailer_len(payload: &[u8], flags: u8, body_offset: usize) -> Option<usize> {
    if flags & LLUDP_ACK_FLAG == 0 {
        return Some(0);
    }
    let ack_count = usize::from(*payload.last()?);
    let ack_trailer_len = ack_count.checked_mul(4)?.checked_add(1)?;
    if payload.len() < body_offset.saturating_add(ack_trailer_len) {
        return None;
    }
    Some(ack_trailer_len)
}

fn append_lludp_ack_trailer(payload: &[u8], ack_ids: &[u32]) -> Vec<u8> {
    if ack_ids.is_empty() {
        return payload.to_vec();
    }
    let mut out = Vec::with_capacity(payload.len() + ack_ids.len() * 4 + 1);
    out.extend_from_slice(payload);
    out[0] |= LLUDP_ACK_FLAG;
    for ack_id in ack_ids {
        out.extend_from_slice(&ack_id.to_be_bytes());
    }
    out.push(u8::try_from(ack_ids.len()).expect("ack count should fit in u8"));
    out
}

fn classify_first_simulator_inbound_from_packet(
    payload: &[u8],
) -> Option<FirstSimulatorInboundClassification> {
    let header = decode_first_simulator_packet_header(payload)?;
    let signal = format!("packet:0x{:08x}", header.message_number);

    match header.message_number {
        num if num == u32::from(LLUDP_LAYER_DATA_HIGH_ID) => {
            Some(FirstSimulatorInboundClassification {
                kind: FirstSimulatorInboundMessageKind::LayerData,
                scope: FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic,
                signal,
                decode_source: FirstSimulatorInboundDecodeSource::PacketMessageNumber,
                packet_message_number: Some(header.message_number),
            })
        }
        num if num == u32::from(LLUDP_OBJECT_UPDATE_HIGH_ID) => {
            Some(FirstSimulatorInboundClassification {
                kind: FirstSimulatorInboundMessageKind::ObjectUpdate,
                scope: FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic,
                signal,
                decode_source: FirstSimulatorInboundDecodeSource::PacketMessageNumber,
                packet_message_number: Some(header.message_number),
            })
        }
        num if num == u32::from(LLUDP_OBJECT_UPDATE_COMPRESSED_HIGH_ID) => {
            Some(FirstSimulatorInboundClassification {
                kind: FirstSimulatorInboundMessageKind::ObjectUpdateCompressed,
                scope: FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic,
                signal,
                decode_source: FirstSimulatorInboundDecodeSource::PacketMessageNumber,
                packet_message_number: Some(header.message_number),
            })
        }
        num if num == u32::from(LLUDP_OBJECT_UPDATE_CACHED_HIGH_ID) => {
            Some(FirstSimulatorInboundClassification {
                kind: FirstSimulatorInboundMessageKind::ObjectUpdateCached,
                scope: FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic,
                signal,
                decode_source: FirstSimulatorInboundDecodeSource::PacketMessageNumber,
                packet_message_number: Some(header.message_number),
            })
        }
        num if num == u32::from(LLUDP_IMPROVED_TERSE_OBJECT_UPDATE_HIGH_ID) => {
            Some(FirstSimulatorInboundClassification {
                kind: FirstSimulatorInboundMessageKind::ImprovedTerseObjectUpdate,
                scope: FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic,
                signal,
                decode_source: FirstSimulatorInboundDecodeSource::PacketMessageNumber,
                packet_message_number: Some(header.message_number),
            })
        }
        num if num == u32::from(LLUDP_KILL_OBJECT_HIGH_ID) => {
            Some(FirstSimulatorInboundClassification {
                kind: FirstSimulatorInboundMessageKind::KillObject,
                scope: FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic,
                signal,
                decode_source: FirstSimulatorInboundDecodeSource::PacketMessageNumber,
                packet_message_number: Some(header.message_number),
            })
        }
        num if num == lludp_low_frequency_message_number(LLUDP_AGENT_MOVEMENT_COMPLETE_LOW_ID) => {
            Some(FirstSimulatorInboundClassification {
                kind: FirstSimulatorInboundMessageKind::AgentMovementComplete,
                scope: FirstSimulatorInboundTrafficScope::BootstrapRelevant,
                signal,
                decode_source: FirstSimulatorInboundDecodeSource::PacketMessageNumber,
                packet_message_number: Some(header.message_number),
            })
        }
        num if num == lludp_low_frequency_message_number(LLUDP_TEST_MESSAGE_LOW_ID) => {
            Some(FirstSimulatorInboundClassification {
                kind: FirstSimulatorInboundMessageKind::TestMessage,
                scope: FirstSimulatorInboundTrafficScope::TransportControl,
                signal,
                decode_source: FirstSimulatorInboundDecodeSource::PacketMessageNumber,
                packet_message_number: Some(header.message_number),
            })
        }
        num if num == lludp_low_frequency_message_number(LLUDP_OBJECT_EXTRA_PARAMS_LOW_ID) => {
            Some(FirstSimulatorInboundClassification {
                kind: FirstSimulatorInboundMessageKind::ObjectExtraParams,
                scope: FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic,
                signal,
                decode_source: FirstSimulatorInboundDecodeSource::PacketMessageNumber,
                packet_message_number: Some(header.message_number),
            })
        }
        num if num == lludp_low_frequency_message_number(LLUDP_PACKET_ACK_LOW_ID) => {
            Some(FirstSimulatorInboundClassification {
                kind: FirstSimulatorInboundMessageKind::PacketAck,
                scope: FirstSimulatorInboundTrafficScope::TransportControl,
                signal,
                decode_source: FirstSimulatorInboundDecodeSource::PacketMessageNumber,
                packet_message_number: Some(header.message_number),
            })
        }
        num if num == lludp_low_frequency_message_number(LLUDP_REGION_HANDSHAKE_LOW_ID) => {
            Some(FirstSimulatorInboundClassification {
                kind: FirstSimulatorInboundMessageKind::RegionHandshake,
                scope: FirstSimulatorInboundTrafficScope::BootstrapRelevant,
                signal,
                decode_source: FirstSimulatorInboundDecodeSource::PacketMessageNumber,
                packet_message_number: Some(header.message_number),
            })
        }
        num if num == lludp_low_frequency_message_number(LLUDP_REGION_HANDSHAKE_REPLY_LOW_ID) => {
            Some(FirstSimulatorInboundClassification {
                kind: FirstSimulatorInboundMessageKind::RegionHandshakeReply,
                scope: FirstSimulatorInboundTrafficScope::BootstrapRelevant,
                signal,
                decode_source: FirstSimulatorInboundDecodeSource::PacketMessageNumber,
                packet_message_number: Some(header.message_number),
            })
        }
        num if num == lludp_low_frequency_message_number(LLUDP_HEALTH_MESSAGE_LOW_ID) => {
            Some(FirstSimulatorInboundClassification {
                kind: FirstSimulatorInboundMessageKind::HealthMessage,
                scope: FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic,
                signal,
                decode_source: FirstSimulatorInboundDecodeSource::PacketMessageNumber,
                packet_message_number: Some(header.message_number),
            })
        }
        num if num == lludp_low_frequency_message_number(LLUDP_CHAT_FROM_SIMULATOR_LOW_ID) => {
            Some(FirstSimulatorInboundClassification {
                kind: FirstSimulatorInboundMessageKind::ChatFromSimulator,
                scope: FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic,
                signal: format!("packet:0x{num:08x}"),
                decode_source: FirstSimulatorInboundDecodeSource::PacketMessageNumber,
                packet_message_number: Some(header.message_number),
            })
        }
        num if num == lludp_low_frequency_message_number(LLUDP_SIMULATOR_VIEWER_TIME_LOW_ID) => {
            Some(FirstSimulatorInboundClassification {
                kind: FirstSimulatorInboundMessageKind::SimulatorViewerTimeMessage,
                scope: FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic,
                signal,
                decode_source: FirstSimulatorInboundDecodeSource::PacketMessageNumber,
                packet_message_number: Some(header.message_number),
            })
        }
        num if num == lludp_low_frequency_message_number(LLUDP_ENABLE_SIMULATOR_LOW_ID) => {
            Some(FirstSimulatorInboundClassification {
                kind: FirstSimulatorInboundMessageKind::EnableSimulator,
                scope: FirstSimulatorInboundTrafficScope::BootstrapRelevant,
                signal,
                decode_source: FirstSimulatorInboundDecodeSource::PacketMessageNumber,
                packet_message_number: Some(header.message_number),
            })
        }
        num if num == lludp_low_frequency_message_number(LLUDP_AGENT_DATA_UPDATE_LOW_ID) => {
            Some(FirstSimulatorInboundClassification {
                kind: FirstSimulatorInboundMessageKind::AgentDataUpdate,
                scope: FirstSimulatorInboundTrafficScope::BootstrapRelevant,
                signal,
                decode_source: FirstSimulatorInboundDecodeSource::PacketMessageNumber,
                packet_message_number: Some(header.message_number),
            })
        }
        num if num == lludp_low_frequency_message_number(LLUDP_ONLINE_NOTIFICATION_LOW_ID) => {
            Some(FirstSimulatorInboundClassification {
                kind: FirstSimulatorInboundMessageKind::OnlineNotification,
                scope: FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic,
                signal,
                decode_source: FirstSimulatorInboundDecodeSource::PacketMessageNumber,
                packet_message_number: Some(header.message_number),
            })
        }
        num if num == lludp_medium_frequency_message_number(LLUDP_VIEWER_EFFECT_MEDIUM_ID) => {
            Some(FirstSimulatorInboundClassification {
                kind: FirstSimulatorInboundMessageKind::ViewerEffect,
                scope: FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic,
                signal,
                decode_source: FirstSimulatorInboundDecodeSource::PacketMessageNumber,
                packet_message_number: Some(header.message_number),
            })
        }
        num if num
            == lludp_medium_frequency_message_number(LLUDP_COARSE_LOCATION_UPDATE_MEDIUM_ID) =>
        {
            Some(FirstSimulatorInboundClassification {
                kind: FirstSimulatorInboundMessageKind::CoarseLocationUpdate,
                scope: FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic,
                signal,
                decode_source: FirstSimulatorInboundDecodeSource::PacketMessageNumber,
                packet_message_number: Some(header.message_number),
            })
        }
        num if num == lludp_medium_frequency_message_number(LLUDP_ATTACHED_SOUND_MEDIUM_ID) => {
            Some(FirstSimulatorInboundClassification {
                kind: FirstSimulatorInboundMessageKind::AttachedSound,
                scope: FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic,
                signal,
                decode_source: FirstSimulatorInboundDecodeSource::PacketMessageNumber,
                packet_message_number: Some(header.message_number),
            })
        }
        num if num == lludp_medium_frequency_message_number(LLUDP_CROSSED_REGION_MEDIUM_ID) => {
            Some(FirstSimulatorInboundClassification {
                kind: FirstSimulatorInboundMessageKind::CrossedRegion,
                scope: FirstSimulatorInboundTrafficScope::RegionTransitionControl,
                signal,
                decode_source: FirstSimulatorInboundDecodeSource::PacketMessageNumber,
                packet_message_number: Some(header.message_number),
            })
        }
        num if num
            == lludp_medium_frequency_message_number(LLUDP_CONFIRM_ENABLE_SIMULATOR_MEDIUM_ID) =>
        {
            Some(FirstSimulatorInboundClassification {
                kind: FirstSimulatorInboundMessageKind::ConfirmEnableSimulator,
                scope: FirstSimulatorInboundTrafficScope::RegionTransitionControl,
                signal,
                decode_source: FirstSimulatorInboundDecodeSource::PacketMessageNumber,
                packet_message_number: Some(header.message_number),
            })
        }
        num if num == u32::from(LLUDP_CAMERA_CONSTRAINT_HIGH_ID) => {
            Some(FirstSimulatorInboundClassification {
                kind: FirstSimulatorInboundMessageKind::CameraConstraint,
                scope: FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic,
                signal,
                decode_source: FirstSimulatorInboundDecodeSource::PacketMessageNumber,
                packet_message_number: Some(header.message_number),
            })
        }
        num if num == lludp_low_frequency_message_number(LLUDP_GENERIC_MESSAGE_LOW_ID) => {
            Some(FirstSimulatorInboundClassification {
                kind: FirstSimulatorInboundMessageKind::GenericMessage,
                scope: FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic,
                signal,
                decode_source: FirstSimulatorInboundDecodeSource::PacketMessageNumber,
                packet_message_number: Some(header.message_number),
            })
        }
        _ => None,
    }
}

fn to_early_simulator_traffic_kind(
    kind: FirstSimulatorInboundMessageKind,
) -> Option<EarlySimulatorTrafficKind> {
    match kind {
        FirstSimulatorInboundMessageKind::HealthMessage => {
            Some(EarlySimulatorTrafficKind::HealthMessage)
        }
        FirstSimulatorInboundMessageKind::LayerData => Some(EarlySimulatorTrafficKind::LayerData),
        FirstSimulatorInboundMessageKind::ChatFromSimulator => {
            Some(EarlySimulatorTrafficKind::ChatFromSimulator)
        }
        FirstSimulatorInboundMessageKind::SimulatorViewerTimeMessage => {
            Some(EarlySimulatorTrafficKind::SimulatorViewerTimeMessage)
        }
        FirstSimulatorInboundMessageKind::OnlineNotification => {
            Some(EarlySimulatorTrafficKind::OnlineNotification)
        }
        FirstSimulatorInboundMessageKind::ViewerEffect => {
            Some(EarlySimulatorTrafficKind::ViewerEffect)
        }
        FirstSimulatorInboundMessageKind::CoarseLocationUpdate => {
            Some(EarlySimulatorTrafficKind::CoarseLocationUpdate)
        }
        FirstSimulatorInboundMessageKind::AttachedSound => {
            Some(EarlySimulatorTrafficKind::AttachedSound)
        }
        _ => None,
    }
}

fn to_region_transition_control_kind(
    kind: FirstSimulatorInboundMessageKind,
) -> Option<RegionTransitionControlKind> {
    match kind {
        FirstSimulatorInboundMessageKind::CrossedRegion => {
            Some(RegionTransitionControlKind::CrossedRegion)
        }
        FirstSimulatorInboundMessageKind::ConfirmEnableSimulator => {
            Some(RegionTransitionControlKind::ConfirmEnableSimulator)
        }
        _ => None,
    }
}

fn first_simulator_action_label(action: FirstSimulatorHandshakeAction) -> &'static str {
    match action {
        FirstSimulatorHandshakeAction::UseCircuitCode => "send",
        FirstSimulatorHandshakeAction::CompleteAgentMovement => "send",
    }
}

fn first_simulator_message_label(message_number: u32) -> String {
    let name = match message_number {
        num if num == lludp_low_frequency_message_number(LLUDP_USE_CIRCUIT_CODE_LOW_ID) => {
            "UseCircuitCode"
        }
        num if num == lludp_low_frequency_message_number(LLUDP_COMPLETE_AGENT_MOVEMENT_LOW_ID) => {
            "CompleteAgentMovement"
        }
        num if num == u32::from(LLUDP_OBJECT_UPDATE_HIGH_ID) => "ObjectUpdate",
        num if num == u32::from(LLUDP_OBJECT_UPDATE_COMPRESSED_HIGH_ID) => "ObjectUpdateCompressed",
        num if num == u32::from(LLUDP_OBJECT_UPDATE_CACHED_HIGH_ID) => "ObjectUpdateCached",
        num if num == u32::from(LLUDP_IMPROVED_TERSE_OBJECT_UPDATE_HIGH_ID) => {
            "ImprovedTerseObjectUpdate"
        }
        num if num == u32::from(LLUDP_KILL_OBJECT_HIGH_ID) => "KillObject",
        num if num == lludp_low_frequency_message_number(LLUDP_AGENT_MOVEMENT_COMPLETE_LOW_ID) => {
            "AgentMovementComplete"
        }
        num if num == lludp_low_frequency_message_number(LLUDP_TEST_MESSAGE_LOW_ID) => {
            "TestMessage"
        }
        num if num == lludp_low_frequency_message_number(LLUDP_PACKET_ACK_LOW_ID) => "PacketAck",
        num if num == lludp_low_frequency_message_number(LLUDP_OBJECT_EXTRA_PARAMS_LOW_ID) => {
            "ObjectExtraParams"
        }
        num if num == lludp_low_frequency_message_number(LLUDP_REGION_HANDSHAKE_LOW_ID) => {
            "RegionHandshake"
        }
        num if num == lludp_low_frequency_message_number(LLUDP_REGION_HANDSHAKE_REPLY_LOW_ID) => {
            "RegionHandshakeReply"
        }
        num if num == lludp_low_frequency_message_number(LLUDP_GENERIC_MESSAGE_LOW_ID) => {
            "GenericMessage"
        }
        num if num == lludp_low_frequency_message_number(LLUDP_HEALTH_MESSAGE_LOW_ID) => {
            "HealthMessage"
        }
        num if num == lludp_low_frequency_message_number(LLUDP_CHAT_FROM_SIMULATOR_LOW_ID) => {
            "ChatFromSimulator"
        }
        num if num == lludp_low_frequency_message_number(LLUDP_SIMULATOR_VIEWER_TIME_LOW_ID) => {
            "SimulatorViewerTimeMessage"
        }
        num if num == lludp_low_frequency_message_number(LLUDP_ENABLE_SIMULATOR_LOW_ID) => {
            "EnableSimulator"
        }
        num if num == lludp_low_frequency_message_number(LLUDP_AGENT_DATA_UPDATE_LOW_ID) => {
            "AgentDataUpdate"
        }
        num if num == lludp_low_frequency_message_number(LLUDP_AGENT_THROTTLE_LOW_ID) => {
            "AgentThrottle"
        }
        num if num == lludp_low_frequency_message_number(LLUDP_AGENT_HEIGHT_WIDTH_LOW_ID) => {
            "AgentHeightWidth"
        }
        num if num == u32::from(LLUDP_AGENT_UPDATE_HIGH_ID) => "AgentUpdate",
        num if num == u32::from(LLUDP_AGENT_ANIMATION_HIGH_ID) => "AgentAnimation",
        num if num == lludp_low_frequency_message_number(LLUDP_SET_ALWAYS_RUN_LOW_ID) => {
            "SetAlwaysRun"
        }
        num if num == lludp_low_frequency_message_number(LLUDP_MUTE_LIST_REQUEST_LOW_ID) => {
            "MuteListRequest"
        }
        num if num == lludp_low_frequency_message_number(LLUDP_MONEY_BALANCE_REQUEST_LOW_ID) => {
            "MoneyBalanceRequest"
        }
        num if num
            == lludp_low_frequency_message_number(LLUDP_AGENT_DATA_UPDATE_REQUEST_LOW_ID) =>
        {
            "AgentDataUpdateRequest"
        }
        num if num == lludp_low_frequency_message_number(LLUDP_ONLINE_NOTIFICATION_LOW_ID) => {
            "OnlineNotification"
        }
        num if num == lludp_medium_frequency_message_number(LLUDP_VIEWER_EFFECT_MEDIUM_ID) => {
            "ViewerEffect"
        }
        num if num
            == lludp_medium_frequency_message_number(LLUDP_REQUEST_MULTIPLE_OBJECTS_MEDIUM_ID) =>
        {
            "RequestMultipleObjects"
        }
        num if num
            == lludp_medium_frequency_message_number(LLUDP_COARSE_LOCATION_UPDATE_MEDIUM_ID) =>
        {
            "CoarseLocationUpdate"
        }
        num if num == lludp_medium_frequency_message_number(LLUDP_ATTACHED_SOUND_MEDIUM_ID) => {
            "AttachedSound"
        }
        num if num == lludp_medium_frequency_message_number(LLUDP_CROSSED_REGION_MEDIUM_ID) => {
            "CrossedRegion"
        }
        num if num
            == lludp_medium_frequency_message_number(LLUDP_CONFIRM_ENABLE_SIMULATOR_MEDIUM_ID) =>
        {
            "ConfirmEnableSimulator"
        }
        num if num == u32::from(LLUDP_CAMERA_CONSTRAINT_HIGH_ID) => "CameraConstraint",
        num if num == u32::from(LLUDP_LAYER_DATA_HIGH_ID) => "LayerData",
        _ => "Unknown",
    };
    format!("{name}(0x{message_number:08x})")
}

fn format_u32_hex_list(values: &[u32]) -> String {
    values
        .iter()
        .map(|value| format!("0x{value:08x}"))
        .collect::<Vec<_>>()
        .join(",")
}

pub fn classify_capability_url(url: &str) -> CapabilityUrlClassification {
    let Ok(parsed) = reqwest::Url::parse(url) else {
        return CapabilityUrlClassification::default();
    };
    let host = parsed.host_str().map(|value| value.to_ascii_lowercase());
    let port = parsed.port_or_known_default();
    let family = match (host.as_deref(), port) {
        (Some(host), Some(12043))
            if host.starts_with("simhost-") || host.ends_with(".agni.secondlife.io") =>
        {
            CapabilityUrlFamily::SimulatorHost12043
        }
        (Some(host), Some(12046))
            if host.starts_with("simhost-") || host.ends_with(".agni.secondlife.io") =>
        {
            CapabilityUrlFamily::SimulatorHost12046
        }
        (Some(host), _) if host.contains("asset-cdn.") => CapabilityUrlFamily::AssetCdn,
        (Some(host), _) if host.contains("bake-texture.") => CapabilityUrlFamily::BakeTextureCdn,
        (Some(host), _) if host.contains("maps-cdn") => CapabilityUrlFamily::MapCdn,
        (Some(host), _) if host.contains("phoenixviewer.com") => CapabilityUrlFamily::PhoenixViewer,
        (Some(host), _) if host.contains("google-analytics.com") => CapabilityUrlFamily::Analytics,
        (Some(_), Some(80 | 443)) => CapabilityUrlFamily::GenericWeb,
        (Some(_), _) => CapabilityUrlFamily::GenericWeb,
        _ => CapabilityUrlFamily::Unknown,
    };
    CapabilityUrlClassification { family, host, port }
}

pub fn summarize_seed_capability_inventory(
    map: &SeedCapabilityMap,
) -> Vec<SeedCapabilityInventoryEntry> {
    map.entries
        .iter()
        .map(|(name, url)| SeedCapabilityInventoryEntry {
            name: name.clone(),
            classification: classify_capability_url(url),
        })
        .collect()
}

fn classify_first_simulator_inbound_message(payload: &[u8]) -> FirstSimulatorInboundClassification {
    if let Some(classification) = classify_first_simulator_inbound_from_packet(payload) {
        return classification;
    }

    let text = String::from_utf8_lossy(payload);
    let lowered = text.to_ascii_lowercase();

    if let Ok(json) = serde_json::from_slice::<Value>(payload)
        && let Some(kind) = classify_first_simulator_inbound_from_json(&json)
    {
        return kind;
    }

    if lowered.contains("agentmovementcomplete") {
        return FirstSimulatorInboundClassification {
            kind: FirstSimulatorInboundMessageKind::AgentMovementComplete,
            scope: FirstSimulatorInboundTrafficScope::BootstrapRelevant,
            signal: String::from("text:agentmovementcomplete"),
            decode_source: FirstSimulatorInboundDecodeSource::TextScan,
            packet_message_number: None,
        };
    }
    if lowered.contains("regionhandshake") {
        return FirstSimulatorInboundClassification {
            kind: FirstSimulatorInboundMessageKind::RegionHandshake,
            scope: FirstSimulatorInboundTrafficScope::BootstrapRelevant,
            signal: String::from("text:regionhandshake"),
            decode_source: FirstSimulatorInboundDecodeSource::TextScan,
            packet_message_number: None,
        };
    }
    if lowered.contains("enablesimulator") {
        return FirstSimulatorInboundClassification {
            kind: FirstSimulatorInboundMessageKind::EnableSimulator,
            scope: FirstSimulatorInboundTrafficScope::BootstrapRelevant,
            signal: String::from("text:enablesimulator"),
            decode_source: FirstSimulatorInboundDecodeSource::TextScan,
            packet_message_number: None,
        };
    }

    let packet_message_number =
        decode_first_simulator_packet_header(payload).map(|header| header.message_number);
    let signal = packet_message_number
        .map(|message_number| format!("packet:0x{message_number:08x}:unmapped"))
        .unwrap_or_else(|| String::from("text:none"));

    FirstSimulatorInboundClassification {
        kind: FirstSimulatorInboundMessageKind::Irrelevant,
        scope: FirstSimulatorInboundTrafficScope::Unknown,
        signal,
        decode_source: FirstSimulatorInboundDecodeSource::Unknown,
        packet_message_number,
    }
}

fn classify_first_simulator_inbound_from_json(
    json: &Value,
) -> Option<FirstSimulatorInboundClassification> {
    let mut candidates = Vec::new();
    if let Some(obj) = json.as_object() {
        for key in ["message", "type", "packet", "event", "name"] {
            if let Some(value) = obj.get(key).and_then(Value::as_str) {
                candidates.push(format!("{key}:{value}"));
            }
        }
    }

    for candidate in candidates {
        let lowered = candidate.to_ascii_lowercase();
        if lowered.contains("agentmovementcomplete") {
            return Some(FirstSimulatorInboundClassification {
                kind: FirstSimulatorInboundMessageKind::AgentMovementComplete,
                scope: FirstSimulatorInboundTrafficScope::BootstrapRelevant,
                signal: format!("json:{candidate}"),
                decode_source: FirstSimulatorInboundDecodeSource::JsonField,
                packet_message_number: None,
            });
        }
        if lowered.contains("regionhandshake") {
            return Some(FirstSimulatorInboundClassification {
                kind: FirstSimulatorInboundMessageKind::RegionHandshake,
                scope: FirstSimulatorInboundTrafficScope::BootstrapRelevant,
                signal: format!("json:{candidate}"),
                decode_source: FirstSimulatorInboundDecodeSource::JsonField,
                packet_message_number: None,
            });
        }
        if lowered.contains("enablesimulator") {
            return Some(FirstSimulatorInboundClassification {
                kind: FirstSimulatorInboundMessageKind::EnableSimulator,
                scope: FirstSimulatorInboundTrafficScope::BootstrapRelevant,
                signal: format!("json:{candidate}"),
                decode_source: FirstSimulatorInboundDecodeSource::JsonField,
                packet_message_number: None,
            });
        }
    }

    None
}

fn classify_login_request_shape_failure(
    wire_format: LoginWireFormat,
    endpoint: &str,
    response: &LoginTraceResponse,
) -> Option<LoginFallbackClassifiedReason> {
    if wire_format != LoginWireFormat::Llsd {
        return None;
    }
    if !looks_like_second_life_login_endpoint(endpoint) {
        return None;
    }

    let reason = response.reason.as_deref().unwrap_or_default();
    let message = response
        .message
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if reason.eq_ignore_ascii_case("viewer-data") && message.contains("missing password") {
        return Some(LoginFallbackClassifiedReason::RequestShapeMissingPassword);
    }

    None
}

fn looks_like_second_life_login_endpoint(endpoint: &str) -> bool {
    let lowered = endpoint.to_ascii_lowercase();
    lowered.contains("login.agni.lindenlab.com") || lowered.contains("secondlife.com")
}

fn classify_event_queue_transport_retry(
    err: &reqwest::Error,
) -> (bool, EventQueueRetryClass, String) {
    if err.is_timeout() {
        return (
            true,
            EventQueueRetryClass::Timeout,
            String::from("transport timeout"),
        );
    }
    if err.is_connect() || err.is_request() {
        return (
            true,
            EventQueueRetryClass::Transport,
            String::from("transport request/connect failure"),
        );
    }
    (
        false,
        EventQueueRetryClass::Unknown,
        String::from("non-retryable transport error"),
    )
}

fn classify_event_queue_http_retry(status: StatusCode, retryable: bool) -> EventQueueRetryClass {
    if retryable {
        return EventQueueRetryClass::UpstreamHttp;
    }
    if status.is_client_error() || status.is_server_error() {
        return EventQueueRetryClass::NonRetryableHttp;
    }
    EventQueueRetryClass::Unknown
}

fn event_queue_http_retry_reason(status: StatusCode, body: &str, retryable: bool) -> String {
    if retryable {
        return format!("retryable upstream http failure ({status})");
    }
    let compact_body = body
        .split_whitespace()
        .take(8)
        .collect::<Vec<_>>()
        .join(" ");
    if compact_body.is_empty() {
        return format!("non-retryable http failure ({status})");
    }
    format!("non-retryable http failure ({status}): {compact_body}")
}

fn event_queue_retry_backoff_ms(
    attempt: usize,
    retryable: bool,
    status: Option<StatusCode>,
) -> Option<u128> {
    if !retryable || attempt >= MAX_EVENT_QUEUE_ONE_SHOT_ATTEMPTS {
        return None;
    }
    if let Some(status) = status
        && !status.is_server_error()
        && !matches!(status.as_u16(), 499 | 502 | 503 | 504)
    {
        return None;
    }
    let exp = attempt.saturating_sub(1) as u32;
    let multiplier = 1u128 << exp.min(8);
    let base_ms = EVENT_QUEUE_ONE_SHOT_RETRY_BASE_DELAY.as_millis();
    let max_ms = EVENT_QUEUE_ONE_SHOT_RETRY_MAX_DELAY.as_millis();
    Some((base_ms * multiplier).min(max_ms))
}

fn is_retryable_event_queue_http_failure(status: StatusCode, body: &str) -> bool {
    if status.is_server_error() {
        return true;
    }

    let body_lower = body.to_ascii_lowercase();
    body_lower.contains("proxy error")
        || body_lower.contains("upstream")
        || body_lower.contains("error reading from remote server")
}

fn parse_seed_capability_map(
    body: &[u8],
    content_type: Option<&str>,
) -> Result<SeedCapabilityMap, ConnectionError> {
    let looks_json = content_type
        .map(|value| value.contains("json"))
        .unwrap_or(false);
    if looks_json {
        let value: Value = serde_json::from_slice(body)
            .map_err(|err| ConnectionError::CapabilityDecode(err.to_string()))?;
        if let Some(obj) = value.as_object() {
            let mut entries = BTreeMap::new();
            for (key, raw_value) in obj {
                if let Some(url) = raw_value.as_str() {
                    entries.insert(key.clone(), url.to_string());
                }
            }
            return Ok(SeedCapabilityMap { entries });
        }
        return Err(ConnectionError::CapabilityDecode(String::from(
            "json capability response is not an object",
        )));
    }

    let map = parse_llsd_map(body).map_err(ConnectionError::Codec)?;
    let mut entries = BTreeMap::new();
    for (key, raw_value) in map {
        if let Some(url) = llsd_to_string(&raw_value) {
            entries.insert(key, url);
        }
    }
    Ok(SeedCapabilityMap { entries })
}

fn parse_event_queue_once_response(
    body: &[u8],
    content_type: Option<&str>,
) -> Result<EventQueueInspection, ConnectionError> {
    let looks_json = content_type
        .map(|value| value.contains("json"))
        .unwrap_or(false);
    if looks_json {
        let value: Value = serde_json::from_slice(body)
            .map_err(|err| ConnectionError::CapabilityDecode(err.to_string()))?;
        return parse_event_queue_from_json(&value);
    }

    parse_event_queue_from_llsd_xml(body)
}

fn parse_event_queue_poll_response(
    body: &[u8],
    content_type: Option<&str>,
) -> Result<EventQueuePollResult, ConnectionError> {
    let looks_json = content_type
        .map(|value| value.contains("json"))
        .unwrap_or(false);
    if looks_json {
        let value: Value = serde_json::from_slice(body)
            .map_err(|err| ConnectionError::CapabilityDecode(err.to_string()))?;
        return parse_event_queue_poll_from_json(&value);
    }

    parse_event_queue_poll_from_llsd_xml(body)
}

fn parse_event_queue_from_json(value: &Value) -> Result<EventQueueInspection, ConnectionError> {
    let Some(map) = value.as_object() else {
        return Err(ConnectionError::CapabilityDecode(String::from(
            "event queue json response is not an object",
        )));
    };

    let mut inspection = EventQueueInspection {
        top_level_keys: map.keys().cloned().collect(),
        has_events_array: false,
        has_id: map.contains_key("id"),
        event_count: 0,
        event_names: Vec::new(),
    };
    inspection.top_level_keys.sort();

    if let Some(events) = map.get("events").and_then(Value::as_array) {
        inspection.has_events_array = true;
        inspection.event_count = events.len();
        for item in events {
            if let Some(name) = item.get("message").and_then(Value::as_str) {
                inspection.event_names.push(name.to_string());
            }
        }
    }

    Ok(inspection)
}

fn parse_event_queue_poll_from_json(
    value: &Value,
) -> Result<EventQueuePollResult, ConnectionError> {
    let Some(map) = value.as_object() else {
        return Err(ConnectionError::CapabilityDecode(String::from(
            "event queue json response is not an object",
        )));
    };
    let id = map.get("id").and_then(Value::as_u64);
    let mut events = Vec::new();
    if let Some(raw_events) = map.get("events").and_then(Value::as_array) {
        for raw_event in raw_events {
            let Some(event_map) = raw_event.as_object() else {
                continue;
            };
            let message = event_map
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let mut fields = BTreeMap::new();
            if let Some(body) = event_map.get("body") {
                flatten_json_event_queue_fields(None, body, &mut fields);
            }
            events.push(EventQueueMessage { message, fields });
        }
    }
    Ok(EventQueuePollResult { id, events })
}

fn parse_event_queue_from_llsd_xml(body: &[u8]) -> Result<EventQueueInspection, ConnectionError> {
    let text = std::str::from_utf8(body)
        .map_err(|err| ConnectionError::CapabilityDecode(err.to_string()))?;
    let doc =
        Document::parse(text).map_err(|err| ConnectionError::CapabilityDecode(err.to_string()))?;
    let Some(map) = find_llsd_root_map(&doc)? else {
        return Ok(EventQueueInspection::default());
    };

    let mut keys = Vec::new();
    let mut inspection = EventQueueInspection::default();
    let children: Vec<Node<'_, '_>> = map.children().filter(|node| node.is_element()).collect();
    let mut idx = 0usize;
    while idx + 1 < children.len() {
        let key_node = children[idx];
        if key_node.has_tag_name("key") {
            let key_name = key_node.text().unwrap_or_default().to_string();
            keys.push(key_name.clone());
            let value_node = children[idx + 1];
            if key_name == "id" {
                inspection.has_id = true;
            } else if key_name == "events" && value_node.has_tag_name("array") {
                inspection.has_events_array = true;
                inspection.event_count = extract_llsd_event_messages(value_node, &mut inspection);
            }
        }
        idx += 2;
    }

    keys.sort();
    inspection.top_level_keys = keys;
    Ok(inspection)
}

fn parse_event_queue_poll_from_llsd_xml(
    body: &[u8],
) -> Result<EventQueuePollResult, ConnectionError> {
    let text = std::str::from_utf8(body)
        .map_err(|err| ConnectionError::CapabilityDecode(err.to_string()))?;
    let doc =
        Document::parse(text).map_err(|err| ConnectionError::CapabilityDecode(err.to_string()))?;
    let Some(map) = find_llsd_root_map(&doc)? else {
        return Ok(EventQueuePollResult {
            id: None,
            events: Vec::new(),
        });
    };

    let mut id = None;
    let mut events = Vec::new();
    let children: Vec<Node<'_, '_>> = map.children().filter(|node| node.is_element()).collect();
    let mut idx = 0usize;
    while idx + 1 < children.len() {
        let key_node = children[idx];
        let value_node = children[idx + 1];
        if key_node.has_tag_name("key") {
            let key_name = key_node.text().unwrap_or_default();
            if key_name == "id" {
                id = value_node
                    .text()
                    .and_then(|value| value.trim().parse::<u64>().ok());
            } else if key_name == "events" && value_node.has_tag_name("array") {
                events.extend(parse_llsd_event_messages(value_node));
            }
        }
        idx += 2;
    }
    Ok(EventQueuePollResult { id, events })
}

fn parse_resolved_avatar_names(
    body: &[u8],
    content_type: Option<&str>,
) -> Result<Vec<ResolvedAvatarName>, ConnectionError> {
    let looks_json = content_type
        .map(|value| value.contains("json"))
        .unwrap_or(false);
    if looks_json {
        let value: Value = serde_json::from_slice(body)
            .map_err(|err| ConnectionError::CapabilityDecode(err.to_string()))?;
        return Ok(parse_resolved_avatar_names_from_json(&value));
    }
    if let Ok(value) = serde_json::from_slice::<Value>(body) {
        return Ok(parse_resolved_avatar_names_from_json(&value));
    }
    parse_resolved_avatar_names_from_llsd_xml(body)
}

fn parse_resolved_avatar_names_from_json(value: &Value) -> Vec<ResolvedAvatarName> {
    let mut resolved = Vec::new();
    let mut seen = BTreeSet::new();
    let Some(map) = value.as_object() else {
        return resolved;
    };
    if let Some(agents) = map.get("agents").and_then(Value::as_array) {
        for row in agents {
            let Some(row_map) = row.as_object() else {
                continue;
            };
            let id = row_map
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_string();
            if id.is_empty() || seen.contains(&id) {
                continue;
            }
            let display_name = row_map
                .get("display_name")
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| {
                    row_map
                        .get("username")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                })
                .or_else(|| {
                    let first = row_map.get("legacy_first_name").and_then(Value::as_str)?;
                    let last = row_map.get("legacy_last_name").and_then(Value::as_str)?;
                    Some(format!("{first} {last}"))
                })
                .unwrap_or_default();
            if display_name.trim().is_empty() {
                continue;
            }
            seen.insert(id.clone());
            resolved.push(ResolvedAvatarName { id, display_name });
        }
    }
    if let Some(agents) = map.get("agents").and_then(Value::as_object) {
        for (id, row) in agents {
            if id.trim().is_empty() || seen.contains(id) {
                continue;
            }
            let Some(row_map) = row.as_object() else {
                continue;
            };
            let display_name = row_map
                .get("display_name")
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| {
                    row_map
                        .get("username")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                })
                .or_else(|| {
                    let first = row_map.get("legacy_first_name").and_then(Value::as_str)?;
                    let last = row_map.get("legacy_last_name").and_then(Value::as_str)?;
                    Some(format!("{first} {last}"))
                })
                .unwrap_or_default();
            if display_name.trim().is_empty() {
                continue;
            }
            seen.insert(id.clone());
            resolved.push(ResolvedAvatarName {
                id: id.to_string(),
                display_name,
            });
        }
    }
    resolved
}

fn parse_resolved_avatar_names_from_llsd_xml(
    body: &[u8],
) -> Result<Vec<ResolvedAvatarName>, ConnectionError> {
    let text = std::str::from_utf8(body)
        .map_err(|err| ConnectionError::CapabilityDecode(err.to_string()))?;
    let doc =
        Document::parse(text).map_err(|err| ConnectionError::CapabilityDecode(err.to_string()))?;
    let Some(map) = find_llsd_root_map(&doc)? else {
        return Ok(Vec::new());
    };

    let children: Vec<Node<'_, '_>> = map.children().filter(|node| node.is_element()).collect();
    let mut idx = 0usize;
    let mut resolved = Vec::new();
    let mut seen = BTreeSet::new();
    while idx + 1 < children.len() {
        let key_node = children[idx];
        let value_node = children[idx + 1];
        if key_node.has_tag_name("key")
            && key_node.text().unwrap_or_default() == "agents"
            && value_node.has_tag_name("array")
        {
            for item in value_node
                .children()
                .filter(|node| node.has_tag_name("map"))
            {
                let fields = parse_llsd_scalar_map(item);
                let id = fields.get("id").cloned().unwrap_or_default();
                if id.trim().is_empty() || seen.contains(&id) {
                    continue;
                }
                let display_name = fields
                    .get("display_name")
                    .cloned()
                    .or_else(|| fields.get("username").cloned())
                    .or_else(|| {
                        let first = fields.get("legacy_first_name")?;
                        let last = fields.get("legacy_last_name")?;
                        Some(format!("{first} {last}"))
                    })
                    .unwrap_or_default();
                if display_name.trim().is_empty() {
                    continue;
                }
                seen.insert(id.clone());
                resolved.push(ResolvedAvatarName { id, display_name });
            }
            break;
        }
        idx += 2;
    }
    Ok(resolved)
}

fn extract_llsd_event_messages(
    array_node: Node<'_, '_>,
    inspection: &mut EventQueueInspection,
) -> usize {
    let mut count = 0usize;
    for event_map in array_node
        .children()
        .filter(|node| node.has_tag_name("map"))
    {
        count += 1;
        let children: Vec<Node<'_, '_>> = event_map
            .children()
            .filter(|node| node.is_element())
            .collect();
        let mut idx = 0usize;
        while idx + 1 < children.len() {
            let key_node = children[idx];
            let value_node = children[idx + 1];
            if key_node.has_tag_name("key")
                && key_node.text().unwrap_or_default() == "message"
                && value_node.has_tag_name("string")
            {
                inspection
                    .event_names
                    .push(value_node.text().unwrap_or_default().to_string());
                break;
            }
            idx += 2;
        }
    }
    count
}

fn parse_llsd_event_messages(array_node: Node<'_, '_>) -> Vec<EventQueueMessage> {
    let mut events = Vec::new();
    for event_map in array_node
        .children()
        .filter(|node| node.has_tag_name("map"))
    {
        let children: Vec<Node<'_, '_>> = event_map
            .children()
            .filter(|node| node.is_element())
            .collect();
        let mut idx = 0usize;
        let mut message = String::new();
        let mut fields = BTreeMap::new();
        while idx + 1 < children.len() {
            let key_node = children[idx];
            let value_node = children[idx + 1];
            if key_node.has_tag_name("key") {
                let key_name = key_node.text().unwrap_or_default();
                if key_name == "message" && value_node.has_tag_name("string") {
                    message = value_node.text().unwrap_or_default().to_string();
                } else if key_name == "body" && value_node.has_tag_name("map") {
                    flatten_llsd_event_queue_fields(None, value_node, &mut fields);
                }
            }
            idx += 2;
        }
        events.push(EventQueueMessage { message, fields });
    }
    events
}

fn parse_llsd_scalar_map(map_node: Node<'_, '_>) -> BTreeMap<String, String> {
    let mut fields = BTreeMap::new();
    let children: Vec<Node<'_, '_>> = map_node
        .children()
        .filter(|node| node.is_element())
        .collect();
    let mut idx = 0usize;
    while idx + 1 < children.len() {
        let key_node = children[idx];
        let value_node = children[idx + 1];
        if key_node.has_tag_name("key") {
            let key = key_node.text().unwrap_or_default().to_string();
            if let Some(value) = llsd_node_scalar_to_string(value_node) {
                fields.insert(key, value);
            }
        }
        idx += 2;
    }
    fields
}

fn flatten_json_event_queue_fields(
    prefix: Option<&str>,
    value: &Value,
    fields: &mut BTreeMap<String, String>,
) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let next = join_event_queue_field_path(prefix, key);
                flatten_json_event_queue_fields(Some(&next), child, fields);
            }
        }
        Value::Array(items) => {
            for (idx, child) in items.iter().enumerate() {
                let key = match prefix {
                    Some(prefix) => format!("{prefix}[{idx}]"),
                    None => format!("[{idx}]"),
                };
                flatten_json_event_queue_fields(Some(&key), child, fields);
            }
        }
        _ => {
            if let Some(path) = prefix
                && let Some(text) = json_scalar_to_string(value)
            {
                fields.insert(path.to_string(), text);
            }
        }
    }
}

fn flatten_llsd_event_queue_fields(
    prefix: Option<&str>,
    node: Node<'_, '_>,
    fields: &mut BTreeMap<String, String>,
) {
    if node.has_tag_name("map") {
        let children: Vec<Node<'_, '_>> =
            node.children().filter(|child| child.is_element()).collect();
        let mut idx = 0usize;
        while idx + 1 < children.len() {
            let key_node = children[idx];
            let value_node = children[idx + 1];
            if key_node.has_tag_name("key") {
                let key = key_node.text().unwrap_or_default();
                let next = join_event_queue_field_path(prefix, key);
                flatten_llsd_event_queue_fields(Some(&next), value_node, fields);
            }
            idx += 2;
        }
        return;
    }

    if node.has_tag_name("array") {
        for (idx, child) in node
            .children()
            .filter(|child| child.is_element())
            .enumerate()
        {
            let key = match prefix {
                Some(prefix) => format!("{prefix}[{idx}]"),
                None => format!("[{idx}]"),
            };
            flatten_llsd_event_queue_fields(Some(&key), child, fields);
        }
        return;
    }

    if let Some(path) = prefix
        && let Some(value) = llsd_node_scalar_to_string(node)
    {
        fields.insert(path.to_string(), value);
    }
}

fn join_event_queue_field_path(prefix: Option<&str>, key: &str) -> String {
    match prefix {
        Some(prefix) if !prefix.is_empty() => format!("{prefix}.{key}"),
        _ => key.to_string(),
    }
}

fn summarize_event_queue_value(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        return value.to_string();
    }
    value.chars().take(limit).collect::<String>() + "..."
}

fn event_queue_field_value<'a>(
    event: &'a EventQueueMessage,
    candidates: &[&str],
) -> Option<&'a str> {
    candidates
        .iter()
        .find_map(|key| event.fields.get(*key).map(String::as_str))
        .filter(|value| !value.trim().is_empty())
}

fn extract_event_queue_simulator_target(
    event: &EventQueueMessage,
) -> Option<EventQueueSimulatorTarget> {
    let message = event.message.as_str();
    if message != "EnableSimulator" && message != "EstablishAgentCommunication" {
        return None;
    }

    let ip = event_queue_field_value(event, &["SimulatorInfo.IP", "SimulatorInfo[0].IP", "ip"])
        .map(str::to_string);
    let port = event_queue_field_value(
        event,
        &["SimulatorInfo.Port", "SimulatorInfo[0].Port", "port"],
    )
    .map(str::to_string);
    let sim_ip_and_port =
        event_queue_field_value(event, &["sim-ip-and-port", "sim_ip_and_port", "sim"])
            .map(str::to_string);
    let (endpoint_ip, endpoint_port, endpoint_source) = extract_event_queue_target_endpoint(
        ip.as_deref(),
        port.as_deref(),
        sim_ip_and_port.as_deref(),
    );

    Some(EventQueueSimulatorTarget {
        message: event.message.clone(),
        handle: event_queue_field_value(
            event,
            &[
                "SimulatorInfo.Handle",
                "SimulatorInfo[0].Handle",
                "handle",
                "region_handle",
            ],
        )
        .map(str::to_string),
        ip,
        port,
        sim_ip_and_port,
        seed_capability: event_queue_field_value(event, &["seed-capability", "seed_capability"])
            .map(str::to_string),
        endpoint_ip,
        endpoint_port,
        endpoint_source,
    })
}

fn extract_event_queue_target_endpoint(
    raw_ip: Option<&str>,
    raw_port: Option<&str>,
    raw_sim_ip_and_port: Option<&str>,
) -> (Option<String>, Option<u16>, Option<String>) {
    if let Some(sim_ip_and_port) = raw_sim_ip_and_port
        && let Ok(parsed) = sim_ip_and_port.parse::<SocketAddr>()
    {
        return (
            Some(parsed.ip().to_string()),
            Some(parsed.port()),
            Some(String::from("sim_ip_and_port")),
        );
    }

    let parsed_port = raw_port.and_then(|value| value.parse::<u16>().ok());
    #[allow(clippy::collapsible_if)]
    if let (Some(raw_ip), Some(port)) = (raw_ip, parsed_port) {
        if let Some(ip) = parse_event_queue_endpoint_ip(raw_ip) {
            let source = if raw_ip.parse::<std::net::IpAddr>().is_ok() {
                "ip_port_fields"
            } else {
                "simulatorinfo_binary_ip_port"
            };
            return (Some(ip), Some(port), Some(source.to_string()));
        }
    }

    (None, parsed_port, None)
}

fn parse_event_queue_endpoint_ip(raw_ip: &str) -> Option<String> {
    let raw_ip = raw_ip.trim();
    if raw_ip.is_empty() {
        return None;
    }
    if let Ok(parsed) = raw_ip.parse::<std::net::IpAddr>() {
        return Some(parsed.to_string());
    }
    let decoded = BASE64_STANDARD.decode(raw_ip).ok()?;
    match decoded.len() {
        4 => {
            let bytes: [u8; 4] = decoded.try_into().ok()?;
            Some(std::net::Ipv4Addr::from(bytes).to_string())
        }
        16 => {
            let bytes: [u8; 16] = decoded.try_into().ok()?;
            Some(std::net::Ipv6Addr::from(bytes).to_string())
        }
        _ => None,
    }
}

fn extract_event_queue_parcel_summary(
    event: &EventQueueMessage,
) -> Option<EventQueueParcelSummary> {
    if event.message != "ParcelProperties" {
        return None;
    }

    Some(EventQueueParcelSummary {
        message: event.message.clone(),
        local_id: event_queue_field_value(event, &["local_id", "LocalID"]).map(str::to_string),
        name: event_queue_field_value(event, &["name", "Name", "parcel_name"]).map(str::to_string),
        parcel_id: event_queue_field_value(event, &["parcel_id", "ParcelID"]).map(str::to_string),
        owner_id: event_queue_field_value(event, &["owner_id", "OwnerID"]).map(str::to_string),
        area: event_queue_field_value(event, &["area", "Area"]).map(str::to_string),
    })
}

fn llsd_node_scalar_to_string(node: Node<'_, '_>) -> Option<String> {
    if node.has_tag_name("string") || node.has_tag_name("uri") || node.has_tag_name("uuid") {
        return Some(node.text().unwrap_or_default().to_string());
    }
    if node.has_tag_name("integer")
        || node.has_tag_name("real")
        || node.has_tag_name("boolean")
        || node.has_tag_name("date")
    {
        return Some(node.text().unwrap_or_default().to_string());
    }
    if node.has_tag_name("true") {
        return Some(String::from("true"));
    }
    if node.has_tag_name("false") {
        return Some(String::from("false"));
    }
    if node.has_tag_name("binary") {
        return Some(node.text().unwrap_or_default().trim().to_string());
    }
    None
}

fn json_scalar_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Bool(flag) => Some(flag.to_string()),
        Value::Number(num) => Some(num.to_string()),
        _ => None,
    }
}

fn parse_json_i32_triplet(value: Option<&Value>) -> Option<[i32; 3]> {
    let value = value?;
    let arr = value.as_array()?;
    if arr.len() < 3 {
        return None;
    }
    Some([
        arr.first()?.as_i64()? as i32,
        arr.get(1)?.as_i64()? as i32,
        arr.get(2)?.as_i64()? as i32,
    ])
}

fn parse_agent_profile_response(
    body: &[u8],
    content_type: Option<&str>,
    fallback_avatar_id: &str,
) -> Result<AgentProfileData, ConnectionError> {
    let looks_json = content_type
        .map(|value| value.contains("json"))
        .unwrap_or(false);
    if looks_json {
        let value: Value = serde_json::from_slice(body)
            .map_err(|err| ConnectionError::CapabilityDecode(err.to_string()))?;
        return parse_agent_profile_from_json(&value, fallback_avatar_id);
    }
    parse_agent_profile_from_llsd_xml(body, fallback_avatar_id)
}

fn parse_agent_profile_from_json(
    value: &Value,
    fallback_avatar_id: &str,
) -> Result<AgentProfileData, ConnectionError> {
    let Some(map) = value.as_object() else {
        return Err(ConnectionError::CapabilityDecode(String::from(
            "agent profile json response is not an object",
        )));
    };

    let mut profile = AgentProfileData {
        id: map
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or(fallback_avatar_id)
            .to_string(),
        profile_url: map
            .get("profile_url")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        sl_about_text: map
            .get("sl_about_text")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        fl_about_text: map
            .get("fl_about_text")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        notes: map
            .get("notes")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        sl_image_id: map
            .get("sl_image_id")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        fl_image_id: map
            .get("fl_image_id")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        partner_id: map
            .get("partner_id")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        member_since: map
            .get("member_since")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        online: map.get("online").and_then(Value::as_bool),
        allow_publish: map.get("allow_publish").and_then(Value::as_bool),
        identified: map.get("identified").and_then(Value::as_bool),
        transacted: map.get("transacted").and_then(Value::as_bool),
        display_name: map
            .get("display_name")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        username: map
            .get("username")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        groups: Vec::new(),
        picks: Vec::new(),
        pick_details: Vec::new(),
        classifieds: Vec::new(),
        classified_details: Vec::new(),
    };

    if let Some(groups) = map.get("groups").and_then(Value::as_array) {
        for item in groups {
            let Some(group) = item.as_object() else {
                continue;
            };
            let id = group
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            if id.is_empty() {
                continue;
            }
            profile.groups.push(AgentProfileGroup {
                id,
                name: group
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                image_id: group
                    .get("image_id")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
            });
        }
    }

    if let Some(picks) = map.get("picks").and_then(Value::as_array) {
        for item in picks {
            let Some(pick) = item.as_object() else {
                continue;
            };
            let id = pick
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            if id.is_empty() {
                continue;
            }
            profile.picks.push(AgentProfilePick {
                id: id.clone(),
                name: pick
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            });
            profile.pick_details.push(AgentProfilePickDetails {
                id,
                name: pick
                    .get("name")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                description: pick
                    .get("description")
                    .or_else(|| pick.get("desc"))
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                snapshot_id: pick
                    .get("snapshot_id")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                parcel_id: pick
                    .get("parcel_id")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                sim_name: pick
                    .get("sim_name")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                parcel_name: pick
                    .get("parcel_name")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                global_position: parse_json_i32_triplet(
                    pick.get("global_position")
                        .or_else(|| pick.get("pos_global")),
                ),
            });
        }
    }

    if let Some(classifieds) = map.get("classifieds").and_then(Value::as_array) {
        for item in classifieds {
            let Some(classified) = item.as_object() else {
                continue;
            };
            let id = classified
                .get("id")
                .or_else(|| classified.get("classified_id"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            if id.is_empty() {
                continue;
            }
            profile.classifieds.push(AgentProfileClassified {
                id: id.clone(),
                name: classified
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            });
            profile
                .classified_details
                .push(AgentProfileClassifiedDetails {
                    id,
                    name: classified
                        .get("name")
                        .and_then(Value::as_str)
                        .map(ToString::to_string),
                    description: classified
                        .get("description")
                        .or_else(|| classified.get("desc"))
                        .and_then(Value::as_str)
                        .map(ToString::to_string),
                    snapshot_id: classified
                        .get("snapshot_id")
                        .and_then(Value::as_str)
                        .map(ToString::to_string),
                    parcel_id: classified
                        .get("parcel_id")
                        .and_then(Value::as_str)
                        .map(ToString::to_string),
                    sim_name: classified
                        .get("sim_name")
                        .and_then(Value::as_str)
                        .map(ToString::to_string),
                    parcel_name: classified
                        .get("parcel_name")
                        .and_then(Value::as_str)
                        .map(ToString::to_string),
                    global_position: parse_json_i32_triplet(
                        classified
                            .get("global_position")
                            .or_else(|| classified.get("pos_global")),
                    ),
                    category: classified
                        .get("category")
                        .and_then(Value::as_u64)
                        .map(|v| v as u32),
                    flags: classified
                        .get("flags")
                        .or_else(|| classified.get("classified_flags"))
                        .and_then(Value::as_u64)
                        .map(|v| v as u8),
                    price_for_listing: classified
                        .get("price_for_listing")
                        .and_then(Value::as_i64)
                        .map(|v| v as i32),
                });
        }
    }

    Ok(profile)
}

fn parse_agent_profile_from_llsd_xml(
    body: &[u8],
    fallback_avatar_id: &str,
) -> Result<AgentProfileData, ConnectionError> {
    let text = std::str::from_utf8(body)
        .map_err(|err| ConnectionError::Codec(CodecError::Deserialize(err.to_string())))?;
    let doc = Document::parse(text)
        .map_err(|err| ConnectionError::Codec(CodecError::Deserialize(err.to_string())))?;
    let top_map = doc
        .descendants()
        .find(|node| node.has_tag_name("map"))
        .ok_or_else(|| {
            ConnectionError::CapabilityDecode(String::from(
                "agent profile llsd response missing map",
            ))
        })?;

    let scalar = parse_llsd_scalar_map(top_map);
    let mut profile = AgentProfileData {
        id: scalar
            .get("id")
            .cloned()
            .unwrap_or_else(|| fallback_avatar_id.to_string()),
        profile_url: scalar.get("profile_url").cloned(),
        sl_about_text: scalar.get("sl_about_text").cloned().unwrap_or_default(),
        fl_about_text: scalar.get("fl_about_text").cloned().unwrap_or_default(),
        notes: scalar.get("notes").cloned().unwrap_or_default(),
        sl_image_id: scalar.get("sl_image_id").cloned(),
        fl_image_id: scalar.get("fl_image_id").cloned(),
        partner_id: scalar.get("partner_id").cloned(),
        member_since: scalar.get("member_since").cloned(),
        online: scalar
            .get("online")
            .and_then(|value| parse_bool_text(value)),
        allow_publish: scalar
            .get("allow_publish")
            .and_then(|value| parse_bool_text(value)),
        identified: scalar
            .get("identified")
            .and_then(|value| parse_bool_text(value)),
        transacted: scalar
            .get("transacted")
            .and_then(|value| parse_bool_text(value)),
        display_name: scalar.get("display_name").cloned(),
        username: scalar.get("username").cloned(),
        groups: Vec::new(),
        picks: Vec::new(),
        pick_details: Vec::new(),
        classifieds: Vec::new(),
        classified_details: Vec::new(),
    };

    let children: Vec<Node<'_, '_>> = top_map
        .children()
        .filter(|node| node.is_element())
        .collect();
    let mut idx = 0usize;
    while idx + 1 < children.len() {
        let key_node = children[idx];
        let value_node = children[idx + 1];
        if key_node.has_tag_name("key") {
            match key_node.text().unwrap_or_default() {
                "groups" if value_node.has_tag_name("array") => {
                    for item in value_node
                        .children()
                        .filter(|node| node.has_tag_name("map"))
                    {
                        let map = parse_llsd_scalar_map(item);
                        let id = map.get("id").cloned().unwrap_or_default();
                        if id.is_empty() {
                            continue;
                        }
                        profile.groups.push(AgentProfileGroup {
                            id,
                            name: map.get("name").cloned().unwrap_or_default(),
                            image_id: map.get("image_id").cloned(),
                        });
                    }
                }
                "picks" if value_node.has_tag_name("array") => {
                    for item in value_node
                        .children()
                        .filter(|node| node.has_tag_name("map"))
                    {
                        let map = parse_llsd_scalar_map(item);
                        let id = map.get("id").cloned().unwrap_or_default();
                        if id.is_empty() {
                            continue;
                        }
                        profile.picks.push(AgentProfilePick {
                            id: id.clone(),
                            name: map.get("name").cloned().unwrap_or_default(),
                        });
                        profile.pick_details.push(AgentProfilePickDetails {
                            id,
                            name: map.get("name").cloned(),
                            description: map
                                .get("description")
                                .or_else(|| map.get("desc"))
                                .cloned(),
                            snapshot_id: map.get("snapshot_id").cloned(),
                            parcel_id: map.get("parcel_id").cloned(),
                            sim_name: map.get("sim_name").cloned(),
                            parcel_name: map.get("parcel_name").cloned(),
                            global_position: None,
                        });
                    }
                }
                "classifieds" if value_node.has_tag_name("array") => {
                    for item in value_node
                        .children()
                        .filter(|node| node.has_tag_name("map"))
                    {
                        let map = parse_llsd_scalar_map(item);
                        let id = map
                            .get("id")
                            .cloned()
                            .or_else(|| map.get("classified_id").cloned())
                            .unwrap_or_default();
                        if id.is_empty() {
                            continue;
                        }
                        profile.classifieds.push(AgentProfileClassified {
                            id: id.clone(),
                            name: map.get("name").cloned().unwrap_or_default(),
                        });
                        profile
                            .classified_details
                            .push(AgentProfileClassifiedDetails {
                                id,
                                name: map.get("name").cloned(),
                                description: map
                                    .get("description")
                                    .or_else(|| map.get("desc"))
                                    .cloned(),
                                snapshot_id: map.get("snapshot_id").cloned(),
                                parcel_id: map.get("parcel_id").cloned(),
                                sim_name: map.get("sim_name").cloned(),
                                parcel_name: map.get("parcel_name").cloned(),
                                global_position: None,
                                category: map
                                    .get("category")
                                    .and_then(|value| value.parse::<u32>().ok()),
                                flags: map.get("flags").and_then(|value| value.parse::<u8>().ok()),
                                price_for_listing: map
                                    .get("price_for_listing")
                                    .and_then(|value| value.parse::<i32>().ok()),
                            });
                    }
                }
                _ => {}
            }
        }
        idx += 2;
    }
    Ok(profile)
}

fn parse_bool_text(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" => Some(true),
        "0" | "false" => Some(false),
        _ => None,
    }
}

fn parse_probe_method(method: &str) -> Result<Method, ConnectionError> {
    match method.trim().to_ascii_uppercase().as_str() {
        "GET" => Ok(Method::GET),
        "POST" => Ok(Method::POST),
        "DEL" | "DELETE" => Ok(Method::DELETE),
        other => Err(ConnectionError::InvalidResponse(format!(
            "unsupported capability probe method: {other}"
        ))),
    }
}

fn classify_probe_response_class(status: StatusCode) -> String {
    if status.is_success() {
        return String::from("success");
    }
    if status.is_redirection() {
        return String::from("redirect");
    }
    if status.is_client_error() {
        return String::from("client_error");
    }
    if status.is_server_error() {
        return String::from("server_error");
    }
    String::from("other")
}

fn classify_probe_decode(content_type: Option<&str>, body: &[u8]) -> String {
    if body.is_empty() {
        return String::from("unparsed");
    }
    if let Some(ct) = content_type {
        if ct.contains("json")
            && serde_json::from_slice::<Value>(body)
                .ok()
                .and_then(|v| v.as_object().map(|_| ()))
                .is_some()
        {
            return String::from("json_obj");
        }
        if ct.contains("xml") || ct.contains("llsd") {
            let text = String::from_utf8_lossy(body);
            if let Ok(doc) = Document::parse(&text) {
                if doc.descendants().any(|node| node.has_tag_name("map")) {
                    return String::from("llsd_map");
                }
                if text.to_ascii_lowercase().contains("<error") {
                    return String::from("xml_error");
                }
            } else {
                return String::from("xml_error");
            }
        }
        if ct.contains("text/plain") || ct.contains("text/html") {
            return String::from("plain_text");
        }
    }
    if serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|v| v.as_object().map(|_| ()))
        .is_some()
    {
        return String::from("json_obj");
    }
    let text = String::from_utf8_lossy(body);
    if text.trim_start().starts_with('<')
        && let Ok(doc) = Document::parse(&text)
    {
        if doc.descendants().any(|node| node.has_tag_name("map")) {
            return String::from("llsd_map");
        }
        if text.to_ascii_lowercase().contains("<error") {
            return String::from("xml_error");
        }
    }
    if text
        .chars()
        .take(128)
        .all(|c| c.is_ascii_graphic() || c.is_ascii_whitespace())
    {
        return String::from("plain_text");
    }
    String::from("unparsed")
}

fn bounded_probe_body_preview(body: &[u8]) -> Option<String> {
    if body.is_empty() {
        return None;
    }
    let text = String::from_utf8_lossy(body);
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.is_empty() {
        return None;
    }
    Some(compact.chars().take(96).collect())
}

fn stable_probe_hash_hex(bytes: &[u8]) -> String {
    const OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = OFFSET_BASIS;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{hash:016x}")
}

fn parse_simulator_features_response(
    body: &[u8],
    content_type: Option<&str>,
) -> Result<SimulatorFeaturesInspection, ConnectionError> {
    let looks_json = content_type
        .map(|value| value.contains("json"))
        .unwrap_or(false);
    if looks_json {
        let value: Value = serde_json::from_slice(body)
            .map_err(|err| ConnectionError::CapabilityDecode(err.to_string()))?;
        return parse_simulator_features_from_json(&value);
    }

    parse_simulator_features_from_llsd_xml(body)
}

fn parse_untrusted_probe_inspection_or_transport_fallback(
    body: &[u8],
    content_type: Option<&str>,
    status: StatusCode,
) -> SimulatorFeaturesInspection {
    match parse_simulator_features_response(body, content_type) {
        Ok(inspection) => inspection,
        Err(_) => {
            let mut inspection = SimulatorFeaturesInspection::default();
            inspection
                .top_level_keys
                .push(String::from("probe_transport_status"));
            inspection
                .top_level_keys
                .push(String::from("probe_body_bytes"));
            inspection.scalar_values.insert(
                String::from("probe_transport_status"),
                status.as_u16().to_string(),
            );
            inspection
                .scalar_values
                .insert(String::from("probe_body_bytes"), body.len().to_string());
            inspection.scalar_values.insert(
                String::from("probe_content_type"),
                content_type.unwrap_or("unknown").to_string(),
            );
            inspection
                .scalar_values
                .insert(String::from("probe_decode"), String::from("unparsed_body"));
            inspection
        }
    }
}

fn parse_simulator_features_from_json(
    value: &Value,
) -> Result<SimulatorFeaturesInspection, ConnectionError> {
    let Some(map) = value.as_object() else {
        return Err(ConnectionError::CapabilityDecode(String::from(
            "simulator features json response is not an object",
        )));
    };

    let mut inspection = SimulatorFeaturesInspection::default();
    for (key, raw) in map {
        inspection.top_level_keys.push(key.clone());
        match raw {
            Value::String(text) => {
                inspection.scalar_values.insert(key.clone(), text.clone());
            }
            Value::Number(num) => {
                inspection
                    .scalar_values
                    .insert(key.clone(), num.to_string());
            }
            Value::Bool(flag) => {
                inspection
                    .scalar_values
                    .insert(key.clone(), flag.to_string());
            }
            Value::Object(_) => {
                inspection
                    .complex_value_types
                    .insert(key.clone(), String::from("map"));
            }
            Value::Array(_) => {
                inspection
                    .complex_value_types
                    .insert(key.clone(), String::from("array"));
            }
            Value::Null => {
                inspection
                    .complex_value_types
                    .insert(key.clone(), String::from("null"));
            }
        }
    }
    inspection.top_level_keys.sort();
    Ok(inspection)
}

fn parse_simulator_features_from_llsd_xml(
    body: &[u8],
) -> Result<SimulatorFeaturesInspection, ConnectionError> {
    let text = std::str::from_utf8(body)
        .map_err(|err| ConnectionError::CapabilityDecode(err.to_string()))?;
    let doc =
        Document::parse(text).map_err(|err| ConnectionError::CapabilityDecode(err.to_string()))?;
    let Some(map) = find_llsd_root_map(&doc)? else {
        return Ok(SimulatorFeaturesInspection::default());
    };

    let mut inspection = SimulatorFeaturesInspection::default();
    let children: Vec<Node<'_, '_>> = map.children().filter(|node| node.is_element()).collect();
    let mut idx = 0usize;
    while idx + 1 < children.len() {
        let key_node = children[idx];
        let value_node = children[idx + 1];
        if key_node.has_tag_name("key") {
            let key_name = key_node.text().unwrap_or_default().to_string();
            inspection.top_level_keys.push(key_name.clone());
            if value_node.has_tag_name("string")
                || value_node.has_tag_name("integer")
                || value_node.has_tag_name("real")
                || value_node.has_tag_name("boolean")
                || value_node.has_tag_name("uri")
                || value_node.has_tag_name("uuid")
                || value_node.has_tag_name("date")
            {
                inspection
                    .scalar_values
                    .insert(key_name, value_node.text().unwrap_or_default().to_string());
            } else if value_node.has_tag_name("map") {
                inspection
                    .complex_value_types
                    .insert(key_name, String::from("map"));
            } else if value_node.has_tag_name("array") {
                inspection
                    .complex_value_types
                    .insert(key_name, String::from("array"));
            } else {
                inspection
                    .complex_value_types
                    .insert(key_name, value_node.tag_name().name().to_string());
            }
        }
        idx += 2;
    }

    inspection.top_level_keys.sort();
    Ok(inspection)
}

fn parse_region_objects_response(
    body: &[u8],
    content_type: Option<&str>,
) -> Result<RegionObjectsInspection, ConnectionError> {
    let looks_json = content_type
        .map(|value| value.contains("json"))
        .unwrap_or(false);
    if looks_json {
        let value: Value = serde_json::from_slice(body)
            .map_err(|err| ConnectionError::CapabilityDecode(err.to_string()))?;
        return parse_region_objects_from_json(&value);
    }

    parse_region_objects_from_llsd_xml(body)
}

fn parse_region_objects_from_json(
    value: &Value,
) -> Result<RegionObjectsInspection, ConnectionError> {
    let mut inspection = RegionObjectsInspection::default();
    match value {
        Value::Object(map) => {
            for (key, raw) in map {
                inspection.top_level_keys.push(key.clone());
                populate_region_objects_json_entry(&mut inspection, key, raw);
            }
        }
        Value::Array(items) => {
            inspection.top_level_keys.push(String::from("<root-array>"));
            inspection
                .complex_value_types
                .insert(String::from("<root-array>"), String::from("array"));
            inspection
                .array_lengths
                .insert(String::from("<root-array>"), items.len());
            if let Some(Value::Object(first_map)) = items.first() {
                let mut keys = first_map.keys().cloned().collect::<Vec<_>>();
                keys.sort();
                inspection
                    .first_array_item_keys
                    .insert(String::from("<root-array>"), keys);
            }
        }
        other => {
            inspection.top_level_keys.push(String::from("<root>"));
            inspection
                .scalar_values
                .insert(String::from("<root>"), other.to_string());
        }
    }
    populate_region_objects_typed_samples(&mut inspection);
    analyze_region_objects_tuple_descriptions(&mut inspection);
    inspection.top_level_keys.sort();
    Ok(inspection)
}

fn populate_region_objects_json_entry(
    inspection: &mut RegionObjectsInspection,
    key: &str,
    raw: &Value,
) {
    match raw {
        Value::String(text) => {
            inspection
                .scalar_values
                .insert(key.to_string(), text.clone());
        }
        Value::Number(num) => {
            inspection
                .scalar_values
                .insert(key.to_string(), num.to_string());
        }
        Value::Bool(flag) => {
            inspection
                .scalar_values
                .insert(key.to_string(), flag.to_string());
        }
        Value::Object(child_map) => {
            inspection
                .complex_value_types
                .insert(key.to_string(), String::from("map"));
            record_region_objects_child_json_map(inspection, key, child_map);
        }
        Value::Array(items) => {
            inspection
                .complex_value_types
                .insert(key.to_string(), String::from("array"));
            inspection
                .array_lengths
                .insert(key.to_string(), items.len());
            if let Some(Value::Object(first_map)) = items.first() {
                let mut keys = first_map.keys().cloned().collect::<Vec<_>>();
                keys.sort();
                inspection
                    .first_array_item_keys
                    .insert(key.to_string(), keys);
            }
        }
        Value::Null => {
            inspection
                .complex_value_types
                .insert(key.to_string(), String::from("null"));
        }
    }
}

fn parse_region_objects_from_llsd_xml(
    body: &[u8],
) -> Result<RegionObjectsInspection, ConnectionError> {
    let text = std::str::from_utf8(body)
        .map_err(|err| ConnectionError::CapabilityDecode(err.to_string()))?;
    let doc =
        Document::parse(text).map_err(|err| ConnectionError::CapabilityDecode(err.to_string()))?;
    let llsd = doc
        .descendants()
        .find(|node| node.has_tag_name("llsd"))
        .ok_or_else(|| ConnectionError::CapabilityDecode(String::from("missing llsd root")))?;
    let root = llsd
        .children()
        .find(|node| node.is_element())
        .ok_or_else(|| ConnectionError::CapabilityDecode(String::from("missing llsd value")))?;

    let mut inspection = RegionObjectsInspection::default();
    if root.has_tag_name("map") {
        let children: Vec<Node<'_, '_>> =
            root.children().filter(|node| node.is_element()).collect();
        let mut idx = 0usize;
        while idx + 1 < children.len() {
            let key_node = children[idx];
            let value_node = children[idx + 1];
            if key_node.has_tag_name("key") {
                let key_name = key_node.text().unwrap_or_default().to_string();
                inspection.top_level_keys.push(key_name.clone());
                populate_region_objects_llsd_entry(&mut inspection, &key_name, value_node);
            }
            idx += 2;
        }
    } else if root.has_tag_name("array") {
        inspection.top_level_keys.push(String::from("<root-array>"));
        inspection
            .complex_value_types
            .insert(String::from("<root-array>"), String::from("array"));
        let items: Vec<Node<'_, '_>> = root.children().filter(|node| node.is_element()).collect();
        inspection
            .array_lengths
            .insert(String::from("<root-array>"), items.len());
        if let Some(first) = items.first()
            && first.has_tag_name("map")
        {
            let mut keys = extract_llsd_map_keys(*first);
            keys.sort();
            inspection
                .first_array_item_keys
                .insert(String::from("<root-array>"), keys);
        }
    } else {
        inspection.top_level_keys.push(String::from("<root>"));
        inspection.scalar_values.insert(
            String::from("<root>"),
            root.text().unwrap_or_default().to_string(),
        );
    }

    populate_region_objects_typed_samples(&mut inspection);
    analyze_region_objects_tuple_descriptions(&mut inspection);
    inspection.top_level_keys.sort();
    Ok(inspection)
}

fn find_llsd_root_map<'a>(doc: &'a Document<'a>) -> Result<Option<Node<'a, 'a>>, ConnectionError> {
    let root = if let Some(llsd) = doc.descendants().find(|node| node.has_tag_name("llsd")) {
        llsd.children().find(|node| node.is_element())
    } else {
        Some(doc.root_element())
    }
    .ok_or_else(|| ConnectionError::CapabilityDecode(String::from("missing llsd value")))?;

    if root.has_tag_name("map") {
        return Ok(Some(root));
    }
    if root.has_tag_name("array") {
        return Ok(root.children().find(|node| node.has_tag_name("map")));
    }
    if root.has_tag_name("undef") {
        return Ok(None);
    }
    Err(ConnectionError::CapabilityDecode(format!(
        "unsupported llsd root value: {}",
        root.tag_name().name()
    )))
}

fn populate_region_objects_typed_samples(inspection: &mut RegionObjectsInspection) {
    inspection.typed_object_samples = inspection
        .child_map_pathfinding_summaries
        .iter()
        .take(6)
        .map(|(object_id, summary)| RegionObjectsTypedObjectSample {
            object_id: object_id.clone(),
            profile: summary.profile.clone(),
            name: summary.name.clone(),
            owner: summary.owner.clone(),
            position: summary.position.clone(),
            description_shape: summary.description_shape.clone(),
            linkset_use: summary.linkset_use.clone(),
            walkability_coefficients: summary.walkability_coefficients,
            landimpact: summary.landimpact,
        })
        .collect();
}

fn populate_region_objects_llsd_entry(
    inspection: &mut RegionObjectsInspection,
    key_name: &str,
    value_node: Node<'_, '_>,
) {
    if value_node.has_tag_name("string")
        || value_node.has_tag_name("integer")
        || value_node.has_tag_name("real")
        || value_node.has_tag_name("boolean")
        || value_node.has_tag_name("uri")
        || value_node.has_tag_name("uuid")
        || value_node.has_tag_name("date")
    {
        inspection.scalar_values.insert(
            key_name.to_string(),
            value_node.text().unwrap_or_default().to_string(),
        );
    } else if value_node.has_tag_name("array") {
        inspection
            .complex_value_types
            .insert(key_name.to_string(), String::from("array"));
        let items: Vec<Node<'_, '_>> = value_node
            .children()
            .filter(|node| node.is_element())
            .collect();
        inspection
            .array_lengths
            .insert(key_name.to_string(), items.len());
        if let Some(first) = items.first()
            && first.has_tag_name("map")
        {
            let mut keys = extract_llsd_map_keys(*first);
            keys.sort();
            inspection
                .first_array_item_keys
                .insert(key_name.to_string(), keys);
        }
    } else if value_node.has_tag_name("map") {
        inspection
            .complex_value_types
            .insert(key_name.to_string(), String::from("map"));
        record_region_objects_child_llsd_map(inspection, key_name, value_node);
    } else {
        inspection.complex_value_types.insert(
            key_name.to_string(),
            value_node.tag_name().name().to_string(),
        );
    }
}

fn extract_llsd_map_keys(map: Node<'_, '_>) -> Vec<String> {
    map.children()
        .filter(|node| node.is_element() && node.has_tag_name("key"))
        .filter_map(|node| node.text())
        .map(str::to_string)
        .collect()
}

fn record_region_objects_child_json_map(
    inspection: &mut RegionObjectsInspection,
    key: &str,
    child_map: &serde_json::Map<String, Value>,
) {
    const MAX_CHILD_MAP_SUMMARIES: usize = 8;
    if inspection.child_map_keys.len() >= MAX_CHILD_MAP_SUMMARIES {
        return;
    }

    let mut keys = child_map.keys().cloned().collect::<Vec<_>>();
    keys.sort();
    inspection
        .child_map_keys
        .insert(key.to_string(), keys.into_iter().take(8).collect());

    let mut scalar_entries = Vec::new();
    let mut scalar_fields = BTreeMap::new();
    for (child_key, child_value) in child_map {
        match child_value {
            Value::String(text) => {
                if scalar_entries.len() < 6 {
                    scalar_entries.push(format!("{child_key}={text}"));
                }
                scalar_fields.insert(child_key.clone(), text.clone());
                maybe_record_region_objects_mesh_candidate(inspection, child_key, text);
            }
            Value::Number(num) => {
                let value = num.to_string();
                if scalar_entries.len() < 6 {
                    scalar_entries.push(format!("{child_key}={value}"));
                }
                scalar_fields.insert(child_key.clone(), value);
            }
            Value::Bool(flag) => {
                let value = flag.to_string();
                if scalar_entries.len() < 6 {
                    scalar_entries.push(format!("{child_key}={value}"));
                }
                scalar_fields.insert(child_key.clone(), value);
            }
            _ => {}
        }
    }
    if !scalar_entries.is_empty() {
        inspection
            .child_map_scalar_values
            .insert(key.to_string(), scalar_entries);
    }
    record_region_objects_child_semantics(inspection, key, child_map.keys(), &scalar_fields, {
        inspect_region_objects_json_position(child_map.get("position"))
    });
}

fn record_region_objects_child_llsd_map(
    inspection: &mut RegionObjectsInspection,
    key: &str,
    value_node: Node<'_, '_>,
) {
    const MAX_CHILD_MAP_SUMMARIES: usize = 8;
    if inspection.child_map_keys.len() >= MAX_CHILD_MAP_SUMMARIES {
        return;
    }

    let mut keys = extract_llsd_map_keys(value_node);
    keys.sort();
    inspection
        .child_map_keys
        .insert(key.to_string(), keys.into_iter().take(8).collect());

    let children: Vec<Node<'_, '_>> = value_node
        .children()
        .filter(|node| node.is_element())
        .collect();
    let mut idx = 0usize;
    let mut scalar_entries = Vec::new();
    let mut scalar_fields = BTreeMap::new();
    while idx + 1 < children.len() {
        let key_node = children[idx];
        let child_value_node = children[idx + 1];
        if key_node.has_tag_name("key") {
            let child_key = key_node.text().unwrap_or_default();
            if child_value_node.has_tag_name("string")
                || child_value_node.has_tag_name("integer")
                || child_value_node.has_tag_name("real")
                || child_value_node.has_tag_name("boolean")
                || child_value_node.has_tag_name("uri")
                || child_value_node.has_tag_name("uuid")
                || child_value_node.has_tag_name("date")
            {
                let value = child_value_node.text().unwrap_or_default().to_string();
                if scalar_entries.len() < 6 {
                    scalar_entries.push(format!("{child_key}={value}"));
                }
                scalar_fields.insert(child_key.to_string(), value);
                if let Some(value) = scalar_fields.get(child_key) {
                    maybe_record_region_objects_mesh_candidate(inspection, child_key, value);
                }
            }
        }
        idx += 2;
    }
    if !scalar_entries.is_empty() {
        inspection
            .child_map_scalar_values
            .insert(key.to_string(), scalar_entries);
    }
    record_region_objects_child_semantics(
        inspection,
        key,
        extract_llsd_map_keys(value_node).iter(),
        &scalar_fields,
        inspect_region_objects_llsd_position(value_node),
    );
}

const MAX_REGION_OBJECTS_MESH_CANDIDATES: usize = 12;

fn maybe_record_region_objects_mesh_candidate(
    inspection: &mut RegionObjectsInspection,
    field_key: &str,
    field_value: &str,
) {
    if inspection.candidate_mesh_asset_ids.len() >= MAX_REGION_OBJECTS_MESH_CANDIDATES {
        return;
    }

    let key = field_key.trim().to_ascii_lowercase();
    if key.is_empty() {
        return;
    }
    let looks_mesh_related = key.contains("mesh")
        || key.contains("sculpt")
        || key.contains("model")
        || key.contains("shape")
        || key.contains("asset");
    if !looks_mesh_related {
        return;
    }

    let value = field_value.trim().to_ascii_lowercase();
    if !looks_like_uuid(&value) {
        return;
    }

    if !inspection.candidate_mesh_asset_ids.contains(&value) {
        inspection.candidate_mesh_asset_ids.push(value);
    }
}

fn looks_like_uuid(value: &str) -> bool {
    let compact = value.replace('-', "");
    if compact.len() != 32 {
        return false;
    }
    compact.chars().all(|c| c.is_ascii_hexdigit())
}

fn record_region_objects_child_semantics<'a, I>(
    inspection: &mut RegionObjectsInspection,
    key: &str,
    keys: I,
    scalar_fields: &BTreeMap<String, String>,
    position_inspection: RegionObjectsPositionInspection,
) where
    I: IntoIterator<Item = &'a String>,
{
    let key_set = keys
        .into_iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if let Some(pathfinding_summary) =
        build_region_objects_pathfinding_summary(&key_set, scalar_fields, position_inspection)
    {
        let profile = pathfinding_summary.profile.clone();
        let semantic_values = summarize_region_objects_pathfinding_summary(&pathfinding_summary);
        inspection
            .child_map_profiles
            .insert(key.to_string(), profile);
        if !semantic_values.is_empty() {
            inspection
                .child_map_semantic_values
                .insert(key.to_string(), semantic_values);
        }
        inspection
            .child_map_pathfinding_summaries
            .insert(key.to_string(), pathfinding_summary);
    }
}

fn build_region_objects_pathfinding_summary(
    key_set: &BTreeSet<&str>,
    scalar_fields: &BTreeMap<String, String>,
    position_inspection: RegionObjectsPositionInspection,
) -> Option<RegionObjectsPathfindingSummary> {
    let looks_like_pathfinding_object =
        key_set.contains("description") && key_set.contains("name") && key_set.contains("position");
    let looks_like_pathfinding_linkset = key_set.contains("description")
        && key_set.contains("can_be_volume")
        && key_set.contains("A")
        && key_set.contains("B")
        && key_set.contains("C")
        && key_set.contains("D");
    if !looks_like_pathfinding_linkset && !looks_like_pathfinding_object {
        return None;
    }

    let mut summary = RegionObjectsPathfindingSummary {
        profile: if looks_like_pathfinding_linkset {
            String::from("pathfinding_linkset")
        } else {
            String::from("pathfinding_object")
        },
        name: scalar_fields.get("name").cloned(),
        description: scalar_fields.get("description").cloned(),
        owner: scalar_fields.get("owner").cloned(),
        owner_is_group: scalar_fields
            .get("owner_is_group")
            .and_then(|value| parse_region_objects_boolish(value)),
        position_key_present: position_inspection.present,
        position: position_inspection.summary,
        position_shape: position_inspection.shape,
        ..Default::default()
    };
    apply_region_objects_description_variant_hints(&mut summary);
    if looks_like_pathfinding_linkset {
        summary.linkset_use = derive_pathfinding_linkset_use(scalar_fields).map(str::to_string);
        summary.walkability_coefficients =
            derive_pathfinding_walkability_coefficients(scalar_fields);
        summary.can_be_volume = scalar_fields
            .get("can_be_volume")
            .and_then(|value| parse_region_objects_boolish(value));
        summary.phantom = scalar_fields
            .get("phantom")
            .and_then(|value| parse_region_objects_boolish(value));
        summary.navmesh_category = scalar_fields
            .get("navmesh_category")
            .and_then(|value| parse_region_objects_i32(value));
        summary.landimpact = scalar_fields
            .get("landimpact")
            .and_then(|value| parse_region_objects_i32(value));
        summary.modifiable = scalar_fields
            .get("modifiable")
            .and_then(|value| parse_region_objects_boolish(value));
        summary.is_scripted = scalar_fields
            .get("is_scripted")
            .and_then(|value| parse_region_objects_boolish(value));
    }

    Some(summary)
}

fn summarize_region_objects_pathfinding_summary(
    summary: &RegionObjectsPathfindingSummary,
) -> Vec<String> {
    let mut semantic_values = Vec::new();
    if let Some(variant_hint) = &summary.variant_hint {
        semantic_values.push(format!("variant={variant_hint}"));
    }
    if let Some(linkset_use) = &summary.linkset_use {
        semantic_values.push(format!("linkset_use={linkset_use}"));
    }
    if let Some([a, b, c, d]) = summary.walkability_coefficients {
        semantic_values.push(format!("walkability=A:{a}|B:{b}|C:{c}|D:{d}"));
    }
    if let Some(can_be_volume) = summary.can_be_volume {
        semantic_values.push(format!("can_be_volume={can_be_volume}"));
    }
    if let Some(phantom) = summary.phantom {
        semantic_values.push(format!("phantom={phantom}"));
    }
    if let Some(name) = &summary.name {
        semantic_values.push(format!("name={}", summarize_region_objects_text(name)));
    }
    if let Some(description) = &summary.description {
        semantic_values.push(format!(
            "description={}",
            summarize_region_objects_text(description)
        ));
    }
    if let Some(description_shape) = &summary.description_shape {
        semantic_values.push(format!("description_shape={description_shape}"));
    }
    if let Some(description_numeric_tuple) = &summary.description_numeric_tuple {
        semantic_values.push(format!(
            "description_tuple={}",
            description_numeric_tuple.join("|")
        ));
    }
    if let Some(position) = &summary.position {
        semantic_values.push(format!("position={position}"));
    }
    if let Some(position_shape) = &summary.position_shape {
        semantic_values.push(format!("position_shape={position_shape}"));
    }
    if let Some(navmesh_category) = summary.navmesh_category {
        semantic_values.push(format!("navmesh_category={navmesh_category}"));
    }
    if let Some(landimpact) = summary.landimpact {
        semantic_values.push(format!("landimpact={landimpact}"));
    }
    if let Some(modifiable) = summary.modifiable {
        semantic_values.push(format!("modifiable={modifiable}"));
    }
    if let Some(is_scripted) = summary.is_scripted {
        semantic_values.push(format!("is_scripted={is_scripted}"));
    }
    if let Some(owner) = &summary.owner {
        semantic_values.push(format!("owner={owner}"));
    }
    if let Some(owner_is_group) = summary.owner_is_group {
        semantic_values.push(format!("owner_is_group={owner_is_group}"));
    }
    semantic_values.truncate(8);
    semantic_values
}

fn apply_region_objects_description_variant_hints(summary: &mut RegionObjectsPathfindingSummary) {
    let Some(description) = summary.description.as_deref() else {
        if summary.position.is_none() {
            summary.variant_hint = Some(match summary.position_key_present {
                true => String::from("pathfinding_position_present_unparsed"),
                false => String::from("pathfinding_no_position_key"),
            });
        }
        return;
    };

    if let Some(tuple_values) = parse_region_objects_numericish_description_tuple(description) {
        summary.description_shape = Some(String::from("comma_numeric_tuple6"));
        summary.description_numeric_tuple = Some(tuple_values);
        summary.variant_hint = Some(pathfinding_description_variant_label(
            "pathfinding_tuple_description",
            summary,
        ));
        return;
    }

    if description == "(No Description)" {
        summary.description_shape = Some(String::from("placeholder_text"));
        summary.variant_hint = Some(pathfinding_description_variant_label(
            "pathfinding_placeholder_description",
            summary,
        ));
        return;
    }

    summary.description_shape = Some(String::from("free_text"));
    if summary.position.is_none() {
        summary.variant_hint = Some(pathfinding_description_variant_label(
            "pathfinding_text_description",
            summary,
        ));
    }
}

fn pathfinding_description_variant_label(
    prefix: &str,
    summary: &RegionObjectsPathfindingSummary,
) -> String {
    if summary.position.is_some() {
        format!("{prefix}_with_position")
    } else if summary.position_key_present {
        format!("{prefix}_position_unparsed")
    } else {
        format!("{prefix}_no_position_key")
    }
}

fn analyze_region_objects_tuple_descriptions(inspection: &mut RegionObjectsInspection) {
    const MAX_SLOT_DISTINCT_VALUES: usize = 4;
    const MAX_DISTINCT_NAMES: usize = 6;
    const MAX_SAMPLE_PAIRS: usize = 6;

    let tuple_summaries = inspection
        .child_map_pathfinding_summaries
        .iter()
        .filter_map(|(key, summary)| {
            summary.description_numeric_tuple.as_ref().map(|tuple| {
                let mut pair = String::new();
                if let Some(name) = &summary.name {
                    pair.push_str(name);
                } else {
                    pair.push_str(key);
                }
                if let Some(position) = &summary.position {
                    pair.push('@');
                    pair.push_str(position);
                }
                pair.push('=');
                pair.push_str(&tuple.join("|"));
                (tuple.clone(), pair)
            })
        })
        .collect::<Vec<_>>();

    if tuple_summaries.is_empty() {
        inspection.tuple_description_analysis = None;
        return;
    }

    let slot_count = tuple_summaries
        .iter()
        .map(|(tuple, _)| tuple.len())
        .max()
        .unwrap_or(0);
    let mut distinct_names = Vec::new();
    for (_, pair) in &tuple_summaries {
        let name = pair
            .split('@')
            .next()
            .unwrap_or_default()
            .split('=')
            .next()
            .unwrap_or_default()
            .to_string();
        if !name.is_empty() && !distinct_names.iter().any(|existing| existing == &name) {
            distinct_names.push(name);
            if distinct_names.len() >= MAX_DISTINCT_NAMES {
                break;
            }
        }
    }
    let mut slot_distinct_values = Vec::with_capacity(slot_count);
    for idx in 0..slot_count {
        let mut distinct = Vec::new();
        for (tuple, _) in &tuple_summaries {
            let Some(value) = tuple.get(idx) else {
                continue;
            };
            if !distinct.iter().any(|existing| existing == value) {
                distinct.push(value.clone());
                if distinct.len() >= MAX_SLOT_DISTINCT_VALUES {
                    break;
                }
            }
        }
        slot_distinct_values.push(distinct);
    }

    inspection.tuple_description_analysis = Some(RegionObjectsTupleDescriptionAnalysis {
        sample_count: tuple_summaries.len(),
        slot_count,
        slot_distinct_values,
        distinct_names,
        sample_pairs: tuple_summaries
            .into_iter()
            .map(|(_, pair)| pair)
            .take(MAX_SAMPLE_PAIRS)
            .collect(),
    });
}

fn parse_region_objects_numericish_description_tuple(text: &str) -> Option<Vec<String>> {
    let parts = text
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if parts.len() != 6 {
        return None;
    }
    if parts.iter().all(|part| {
        part.chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == '-')
    }) {
        Some(parts)
    } else {
        None
    }
}

#[derive(Clone, Debug, Default)]
struct RegionObjectsPositionInspection {
    present: bool,
    summary: Option<String>,
    shape: Option<String>,
}

fn inspect_region_objects_json_position(raw: Option<&Value>) -> RegionObjectsPositionInspection {
    let Some(value) = raw else {
        return RegionObjectsPositionInspection::default();
    };
    let mut inspection = RegionObjectsPositionInspection {
        present: true,
        ..Default::default()
    };
    match value {
        Value::Array(items) => {
            inspection.shape = Some(format!("json_array_len{}", items.len()));
            if items.len() >= 3 {
                inspection.summary = items
                    .iter()
                    .take(3)
                    .map(region_objects_json_value_to_text)
                    .collect::<Option<Vec<_>>>()
                    .map(|values| values.join("|"));
            }
        }
        Value::Object(map) => {
            let x = map.get("x").and_then(region_objects_json_value_to_text);
            let y = map.get("y").and_then(region_objects_json_value_to_text);
            let z = map.get("z").and_then(region_objects_json_value_to_text);
            if x.is_some() || y.is_some() || z.is_some() {
                inspection.shape = Some(String::from("json_object_xyz"));
                inspection.summary = Some(format!(
                    "{}|{}|{}",
                    x.unwrap_or_else(|| String::from("?")),
                    y.unwrap_or_else(|| String::from("?")),
                    z.unwrap_or_else(|| String::from("?"))
                ));
            } else {
                inspection.shape = Some(format!("json_object_keys{}", map.len()));
            }
        }
        Value::String(_) => {
            inspection.shape = Some(String::from("json_string"));
            inspection.summary = region_objects_json_value_to_text(value);
        }
        Value::Number(_) => {
            inspection.shape = Some(String::from("json_number"));
            inspection.summary = region_objects_json_value_to_text(value);
        }
        Value::Bool(_) => {
            inspection.shape = Some(String::from("json_bool"));
            inspection.summary = region_objects_json_value_to_text(value);
        }
        Value::Null => {
            inspection.shape = Some(String::from("json_null"));
        }
    }
    inspection
}

fn region_objects_json_value_to_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(num) => Some(num.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }
}

fn inspect_region_objects_llsd_position(
    value_node: Node<'_, '_>,
) -> RegionObjectsPositionInspection {
    let mut inspection = RegionObjectsPositionInspection::default();
    let children: Vec<Node<'_, '_>> = value_node
        .children()
        .filter(|node| node.is_element())
        .collect();
    let mut idx = 0usize;
    while idx + 1 < children.len() {
        let key_node = children[idx];
        let child_value_node = children[idx + 1];
        if key_node.has_tag_name("key") && key_node.text().unwrap_or_default() == "position" {
            inspection.present = true;
            if child_value_node.has_tag_name("array") {
                let values = child_value_node
                    .children()
                    .filter(|node| node.is_element())
                    .take(3)
                    .map(region_objects_llsd_value_to_text)
                    .collect::<Option<Vec<_>>>();
                let child_len = child_value_node
                    .children()
                    .filter(|node| node.is_element())
                    .count();
                inspection.shape = Some(format!("llsd_array_len{child_len}"));
                if let Some(values) = values
                    && values.len() == 3
                {
                    inspection.summary = Some(values.join("|"));
                }
            } else if child_value_node.has_tag_name("map") {
                inspection.shape = Some(format!(
                    "llsd_map_keys{}",
                    extract_llsd_map_keys(child_value_node).len()
                ));
            } else {
                inspection.shape = Some(format!("llsd_{}", child_value_node.tag_name().name()));
            }
            return inspection;
        }
        idx += 2;
    }
    inspection
}

fn region_objects_llsd_value_to_text(node: Node<'_, '_>) -> Option<String> {
    if node.has_tag_name("string")
        || node.has_tag_name("integer")
        || node.has_tag_name("real")
        || node.has_tag_name("boolean")
        || node.has_tag_name("uri")
        || node.has_tag_name("uuid")
        || node.has_tag_name("date")
    {
        Some(node.text().unwrap_or_default().to_string())
    } else {
        None
    }
}

fn derive_pathfinding_walkability_coefficients(
    scalar_fields: &BTreeMap<String, String>,
) -> Option<[i32; 4]> {
    let a = parse_region_objects_i32(scalar_fields.get("A")?)?;
    let b = parse_region_objects_i32(scalar_fields.get("B")?)?;
    let c = parse_region_objects_i32(scalar_fields.get("C")?)?;
    let d = parse_region_objects_i32(scalar_fields.get("D")?)?;
    Some([a, b, c, d])
}

fn derive_pathfinding_linkset_use(
    scalar_fields: &BTreeMap<String, String>,
) -> Option<&'static str> {
    let navmesh_category = scalar_fields.get("navmesh_category")?.trim();
    let phantom = parse_region_objects_boolish(scalar_fields.get("phantom")?)?;
    match (phantom, navmesh_category) {
        (false, "0") => Some("walkable"),
        (false, "1") => Some("static_obstacle"),
        (false, "2") => Some("dynamic_obstacle"),
        (true, "0") => Some("material_volume"),
        (true, "1") => Some("exclusion_volume"),
        (true, "2") => Some("dynamic_phantom"),
        _ => None,
    }
}

fn summarize_region_objects_text(text: &str) -> String {
    const MAX_LEN: usize = 32;
    if text.chars().count() <= MAX_LEN {
        return text.to_string();
    }
    let truncated = text.chars().take(MAX_LEN).collect::<String>();
    format!("{truncated}...")
}

fn parse_region_objects_boolish(text: &str) -> Option<bool> {
    match text.trim() {
        "1" => Some(true),
        "0" => Some(false),
        value if value.eq_ignore_ascii_case("true") => Some(true),
        value if value.eq_ignore_ascii_case("false") => Some(false),
        _ => None,
    }
}

fn parse_region_objects_i32(text: &str) -> Option<i32> {
    text.trim().parse::<i32>().ok()
}

fn normalize_login_response(raw: Value) -> Value {
    let mut payload = match raw {
        Value::Object(mut root) => {
            if let Some(responses) = root.remove("responses") {
                responses
            } else {
                Value::Object(root)
            }
        }
        other => other,
    };

    if let Some(obj) = payload.as_object_mut() {
        if let Some(login_str) = obj.get("login").and_then(Value::as_str) {
            match login_str {
                "true" => {
                    obj.insert(String::from("login"), Value::Bool(true));
                }
                "false" => {
                    obj.insert(String::from("login"), Value::Bool(false));
                }
                _ => {}
            }
        }

        let data_reason = obj
            .get("data")
            .and_then(Value::as_object)
            .and_then(|data| data.get("reason"))
            .cloned();
        if obj.get("reason").is_none()
            && let Some(reason) = data_reason
        {
            obj.insert(String::from("reason"), reason);
        }

        let data_message = obj
            .get("data")
            .and_then(Value::as_object)
            .and_then(|data| data.get("message"))
            .cloned();
        if obj.get("message").is_none()
            && let Some(message) = data_message
        {
            obj.insert(String::from("message"), message);
        }

        let data_message_id = obj
            .get("data")
            .and_then(Value::as_object)
            .and_then(|data| data.get("message_id"))
            .cloned();
        if obj.get("message_id").is_none()
            && let Some(message_id) = data_message_id
        {
            obj.insert(String::from("message_id"), message_id);
        }
    }

    payload
}

fn trace_request(request: &GridLoginRequest) -> LoginTraceRequest {
    LoginTraceRequest {
        method: request.method.clone(),
        start_location: request.params.start.clone(),
        options: request.options.clone(),
        agree_to_tos: request.params.agree_to_tos,
        read_critical: request.params.read_critical,
        had_mfa_token: request.params.token.is_some(),
    }
}

fn trace_response(response: &GridLoginResponse) -> LoginTraceResponse {
    LoginTraceResponse {
        login: response.login,
        reason: response.reason.clone(),
        message: response.message.clone(),
    }
}

fn trace_result(result: &GridLoginResult, response: &LoginTraceResponse) -> LoginTraceFinalResult {
    match result {
        GridLoginResult::Success(_) => LoginTraceFinalResult {
            outcome: String::from("success"),
            reason: response.reason.clone(),
            message: response.message.clone(),
        },
        GridLoginResult::Redirect { .. } => LoginTraceFinalResult {
            outcome: String::from("redirect"),
            reason: response.reason.clone(),
            message: response.message.clone(),
        },
        GridLoginResult::RequiresTos { message } => LoginTraceFinalResult {
            outcome: String::from("requires_tos"),
            reason: response.reason.clone(),
            message: message.clone(),
        },
        GridLoginResult::RequiresMfa { message } => LoginTraceFinalResult {
            outcome: String::from("requires_mfa"),
            reason: response.reason.clone(),
            message: message.clone(),
        },
        GridLoginResult::UpdateRequired { message } => LoginTraceFinalResult {
            outcome: String::from("update_required"),
            reason: response.reason.clone(),
            message: message.clone(),
        },
        GridLoginResult::Failed(error) => LoginTraceFinalResult {
            outcome: String::from("failed"),
            reason: error.reason.clone(),
            message: error.message.clone(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::Duration as StdDuration;
    use tokio::net::UdpSocket;
    use tokio::time::timeout;
    use viewer_grid::{GridLoginResult, SecondLifeAdapter, StartLocation, StartLocationIntent};
    use wiremock::matchers::{
        body_partial_json, body_string_contains, header, method, path, query_param,
    };
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_intent(agree_to_tos: bool) -> LoginIntent {
        LoginIntent {
            username: String::from("test.user"),
            password: String::from("secret"),
            start_location: StartLocationIntent::Saved(StartLocation::Last),
            agree_to_tos,
            read_critical: true,
            mfa_token: None,
        }
    }

    fn make_low_frequency_packet(low_id: u16) -> Vec<u8> {
        let [high, low] = low_id.to_be_bytes();
        vec![
            0x00, // flags
            0x00, 0x00, 0x00, 0x01, // packet sequence
            0x00, // extra header offset
            0xFF, 0xFF, high, low, // low-frequency message number
        ]
    }

    fn make_low_frequency_packet_with_body(low_id: u16, body: &[u8]) -> Vec<u8> {
        let mut payload = make_low_frequency_packet(low_id);
        payload.extend_from_slice(body);
        payload
    }

    fn mark_packet_reliable(mut payload: Vec<u8>) -> Vec<u8> {
        payload[0] |= LLUDP_RELIABLE_FLAG;
        payload
    }

    fn make_medium_frequency_packet(medium_id: u8) -> Vec<u8> {
        vec![
            0x00, // flags
            0x00, 0x00, 0x00, 0x01, // packet sequence
            0x00, // extra header offset
            0xFF, medium_id, // medium-frequency message number
        ]
    }

    fn make_medium_frequency_packet_with_body(medium_id: u8, body: &[u8]) -> Vec<u8> {
        let mut payload = make_medium_frequency_packet(medium_id);
        payload.extend_from_slice(body);
        payload
    }

    fn make_high_frequency_packet(high_id: u8) -> Vec<u8> {
        vec![
            0x00, // flags
            0x00, 0x00, 0x00, 0x01, // packet sequence
            0x00, // extra header offset
            high_id,
        ]
    }

    fn make_high_frequency_packet_with_body(high_id: u8, body: &[u8]) -> Vec<u8> {
        let mut payload = make_high_frequency_packet(high_id);
        payload.extend_from_slice(body);
        payload
    }

    fn decode_test_hex(raw: &str) -> Vec<u8> {
        assert_eq!(raw.len() % 2, 0, "hex payload should be even length");
        raw.as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let pair = std::str::from_utf8(pair).expect("hex should be utf8");
                u8::from_str_radix(pair, 16).expect("hex byte should parse")
            })
            .collect()
    }

    fn make_chat_from_simulator_packet(sender: &str, message: &str) -> Vec<u8> {
        let mut payload = make_low_frequency_packet(LLUDP_CHAT_FROM_SIMULATOR_LOW_ID);
        let mut sender_bytes = sender.as_bytes().to_vec();
        sender_bytes.push(0);
        payload.push(u8::try_from(sender_bytes.len()).expect("sender length should fit in u8"));
        payload.extend_from_slice(&sender_bytes);
        payload.extend_from_slice(&[0u8; 16]); // SourceID
        payload.extend_from_slice(&[0u8; 16]); // OwnerID
        payload.push(1); // SourceType
        payload.push(1); // ChatType (normal)
        payload.push(1); // Audible
        payload.extend_from_slice(&[0u8; 12]); // Position
        let mut message_bytes = message.as_bytes().to_vec();
        message_bytes.push(0);
        let message_len =
            u16::try_from(message_bytes.len()).expect("message length should fit in u16");
        payload.extend_from_slice(&message_len.to_le_bytes());
        payload.extend_from_slice(&message_bytes);
        payload
    }

    #[tokio::test]
    async fn fetch_texture_asset_bytes_uses_firestorm_style_candidate_first_with_accept_header() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/cap/"))
            .and(query_param("texture_id", "test-id"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"ok".to_vec()))
            .expect(1)
            .mount(&server)
            .await;

        let urls = viewer_grid::AssetCapabilityPolicy::texture_url_candidates_from_base(
            &format!("{}/cap", server.uri()),
            "test-id",
        );
        let bytes = fetch_texture_asset_bytes(&urls, Duration::from_secs(1))
            .await
            .expect("texture fetch should succeed");

        assert_eq!(bytes, b"ok");
    }

    #[tokio::test]
    async fn fetch_texture_asset_bytes_falls_back_to_second_candidate_after_404() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/cap/"))
            .and(query_param("texture_id", "test-id"))
            .respond_with(ResponseTemplate::new(404).set_body_string("missing first"))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/cap"))
            .and(query_param("texture_id", "test-id"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"fallback".to_vec()))
            .expect(1)
            .mount(&server)
            .await;

        let urls = viewer_grid::AssetCapabilityPolicy::texture_url_candidates_from_base(
            &format!("{}/cap", server.uri()),
            "test-id",
        );
        let bytes = fetch_texture_asset_bytes(&urls, Duration::from_secs(1))
            .await
            .expect("texture fetch should fall back");

        assert_eq!(bytes, b"fallback");
    }

    #[tokio::test]
    async fn fetch_mesh_asset_bytes_requires_candidate_urls() {
        let err = fetch_mesh_asset_bytes(&[], Duration::from_secs(1))
            .await
            .expect_err("mesh fetch should fail without urls");
        assert!(matches!(
            err,
            ConnectionError::MissingCapability(ref cap) if cap == "ViewerAsset(mesh_id)"
        ));
    }

    #[tokio::test]
    async fn fetch_mesh_asset_bytes_reuses_set_cookie_between_probe_and_followup() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/cap/"))
            .and(query_param("mesh_id", "test-mesh"))
            .and(header("accept", MESH_FETCH_ACCEPT_HEADER))
            .and(header("range", MESH_FETCH_RANGE_HEADER_FIRESTORM))
            .and(header("user-agent", FIRESTORM_USER_AGENT_HEADER))
            .respond_with(
                ResponseTemplate::new(206)
                    .insert_header("set-cookie", "MeshAuth=abc123; Path=/")
                    .set_body_bytes(b"partial".to_vec()),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/cap/"))
            .and(query_param("mesh_id", "test-mesh"))
            .and(header("cookie", "MeshAuth=abc123"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"mesh-full".to_vec()))
            .expect(1)
            .mount(&server)
            .await;

        let urls = viewer_grid::AssetCapabilityPolicy::mesh_url_candidates(
            &BTreeMap::from([(String::from("ViewerAsset"), format!("{}/cap", server.uri()))]),
            "test-mesh",
        );
        let bytes = fetch_mesh_asset_bytes(&urls, Duration::from_secs(1))
            .await
            .expect("mesh fetch should reuse mesh auth cookie");

        assert_eq!(bytes, b"mesh-full");
    }

    #[tokio::test]
    async fn fetch_mesh_asset_bytes_falls_through_to_later_mesh_caps_after_403() {
        let viewer_asset = MockServer::start().await;
        let get_mesh2 = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/viewerasset/"))
            .and(query_param("mesh_id", "test-mesh"))
            .respond_with(
                ResponseTemplate::new(403)
                    .insert_header("content-type", "application/xml")
                    .set_body_string("<Error><Code>AccessDenied</Code></Error>"),
            )
            .mount(&viewer_asset)
            .await;

        Mock::given(method("GET"))
            .and(path("/getmesh2/"))
            .and(query_param("mesh_id", "test-mesh"))
            .and(header("range", MESH_FETCH_RANGE_HEADER_FIRESTORM))
            .respond_with(ResponseTemplate::new(206).set_body_bytes(b"partial".to_vec()))
            .expect(1)
            .mount(&get_mesh2)
            .await;
        Mock::given(method("GET"))
            .and(path("/getmesh2/"))
            .and(query_param("mesh_id", "test-mesh"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"mesh-full".to_vec()))
            .expect(1)
            .mount(&get_mesh2)
            .await;

        let urls = viewer_grid::AssetCapabilityPolicy::mesh_url_candidates(
            &BTreeMap::from([
                (
                    String::from("ViewerAsset"),
                    format!("{}/viewerasset", viewer_asset.uri()),
                ),
                (
                    String::from("GetMesh2"),
                    format!("{}/getmesh2", get_mesh2.uri()),
                ),
            ]),
            "test-mesh",
        );
        let (result, attempts) =
            fetch_mesh_asset_bytes_with_attempts(&urls, Duration::from_secs(1)).await;
        let bytes = result.expect("mesh fetch should fall through to GetMesh2");

        assert_eq!(bytes, b"mesh-full");
        assert!(attempts.iter().any(|attempt| {
            attempt.status == Some(403)
                && attempt
                    .response_body
                    .as_deref()
                    .unwrap_or_default()
                    .contains("AccessDenied")
        }));
        assert!(
            attempts
                .iter()
                .any(|attempt| attempt.status == Some(206) && attempt.url.contains("/getmesh2/"))
        );
        assert!(
            attempts
                .iter()
                .any(|attempt| attempt.status == Some(200) && attempt.url.contains("/getmesh2/"))
        );
    }

    #[test]
    fn merge_cookie_header_prefers_new_values_and_preserves_existing() {
        let merged = merge_cookie_header(
            Some("MeshAuth=abc123; Session=old"),
            Some("Session=new; Scope=mesh"),
        )
        .expect("merged cookie header should exist");
        assert_eq!(merged, "MeshAuth=abc123; Scope=mesh; Session=new");
    }

    #[tokio::test]
    async fn fetch_profile_image_bytes_reuses_shared_texture_candidate_fetch() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/profile-image/"))
            .and(query_param("texture_id", "test-id"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"profile".to_vec()))
            .expect(1)
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig::default());
        connection.state = ConnectionState::LoggedIn;

        let bytes = connection
            .fetch_profile_image_bytes(&format!("{}/profile-image", server.uri()), "test-id")
            .await
            .expect("profile image fetch should succeed");

        assert_eq!(bytes, b"profile");
    }

    fn decode_outbound_handshake_payload(payload: &[u8]) -> (u8, u32, u32, &[u8]) {
        assert!(payload.len() >= LLUDP_PACKET_ID_SIZE + 4);
        let flags = payload[0];
        let packet_id = u32::from_be_bytes(payload[1..5].try_into().expect("packet id bytes"));
        let message_number = {
            assert_eq!(payload[6], 0xFF);
            assert_eq!(payload[7], 0xFF);
            let low = u16::from_be_bytes(payload[8..10].try_into().expect("low id bytes"));
            lludp_low_frequency_message_number(low)
        };
        (flags, packet_id, message_number, &payload[10..])
    }

    #[tokio::test]
    async fn adapter_login_success_sets_logged_in_state_and_session() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .and(body_partial_json(json!({
                "method": "login_to_simulator",
                "params": {
                    "agree_to_tos": true
                }
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "message": "http login success",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": 13000,
                "region_x": 1000,
                "region_y": 1000,
                "seed_capability": "https://seed-cap.example.invalid",
                "start_location": "last"
            })))
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        let adapter = SecondLifeAdapter;

        connection.connect().await.expect("connect should succeed");
        let result = connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login with adapter should succeed");

        match result {
            GridLoginResult::Success(bootstrap) => {
                assert_eq!(bootstrap.circuit_code, 424242);
                assert_eq!(bootstrap.first_sim.sim_ip, "127.0.0.1");
            }
            other => panic!("expected success result, got {other:?}"),
        }

        assert_eq!(connection.state(), ConnectionState::LoggedIn);
        let session = connection.session().expect("session must be set");
        assert_eq!(
            session.session_token,
            "22222222-2222-2222-2222-222222222222"
        );
    }

    #[tokio::test]
    async fn adapter_login_tos_required_does_not_log_in() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "responses": {
                    "login": "false",
                    "data": {
                        "reason": "tos",
                        "message": "Terms of service acceptance required"
                    }
                }
            })))
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        let adapter = SecondLifeAdapter;

        connection.connect().await.expect("connect should succeed");
        let result = connection
            .login_with_adapter(&adapter, make_intent(false))
            .await
            .expect("adapter interpretation should succeed");

        match result {
            GridLoginResult::RequiresTos { .. } => {}
            other => panic!("expected RequiresTos result, got {other:?}"),
        }

        assert_eq!(connection.state(), ConnectionState::Connected);
        assert!(connection.session().is_none());
    }

    #[tokio::test]
    async fn login_with_redirect_then_success() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": "false",
                "reason": "indeterminate",
                "next_url": format!("{}/continue", server.uri()),
                "next_method": "POST"
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/continue"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "message": "redirect success",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": 13000,
                "region_x": 1000,
                "region_y": 1000,
                "seed_capability": "https://seed-cap.example.invalid",
                "start_location": "last"
            })))
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        let adapter = SecondLifeAdapter;

        connection.connect().await.expect("connect should succeed");
        let result = connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login with adapter should succeed");

        match result {
            GridLoginResult::Success(bootstrap) => {
                assert_eq!(bootstrap.circuit_code, 424242);
            }
            other => panic!("expected success result, got {other:?}"),
        }

        assert_eq!(connection.state(), ConnectionState::LoggedIn);
    }

    #[tokio::test]
    async fn login_redirect_limit_exceeded() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": "false",
                "reason": "indeterminate",
                "next_url": format!("{}/login", server.uri()),
                "next_method": "POST"
            })))
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        let adapter = SecondLifeAdapter;

        connection.connect().await.expect("connect should succeed");
        let err = connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect_err("should fail after too many redirects");

        match err {
            ConnectionError::RedirectLimitExceeded(limit) => {
                assert_eq!(limit, MAX_LOGIN_REDIRECTS);
            }
            other => panic!("unexpected error {other:?}"),
        }

        assert_eq!(connection.state(), ConnectionState::Connected);
    }

    #[tokio::test]
    async fn login_redirect_then_tos() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": "false",
                "reason": "indeterminate",
                "next_url": format!("{}/tos", server.uri()),
                "next_method": "POST"
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/tos"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": "false",
                "reason": "tos",
                "message": "needs tos"
            })))
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        let adapter = SecondLifeAdapter;

        connection.connect().await.expect("connect should succeed");
        let result = connection
            .login_with_adapter(&adapter, make_intent(false))
            .await
            .expect("adapter interpretation should succeed");

        match result {
            GridLoginResult::RequiresTos { .. } => {}
            other => panic!("expected RequiresTos, got {other:?}"),
        }

        assert_eq!(connection.state(), ConnectionState::Connected);
        assert!(connection.session().is_none());
    }

    #[tokio::test]
    async fn login_with_trace_records_redirect_chain() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": "false",
                "reason": "indeterminate",
                "next_url": format!("{}/continue", server.uri()),
                "next_method": "POST"
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/continue"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "message": "redirect success",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": 13000,
                "region_x": 1000,
                "region_y": 1000,
                "seed_capability": "https://seed-cap.example.invalid",
                "start_location": "last"
            })))
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        let adapter = SecondLifeAdapter;

        connection.connect().await.expect("connect should succeed");
        let (result, trace) = connection
            .login_with_trace(&adapter, make_intent(true))
            .await
            .expect("login with trace should succeed");

        match result {
            GridLoginResult::Success(_) => {}
            other => panic!("expected success result, got {other:?}"),
        }

        assert_eq!(trace.redirect_steps.len(), 1);
        assert_eq!(
            trace.redirect_steps[0].url,
            format!("{}/login", server.uri())
        );
        assert_eq!(trace.final_result.outcome, "success");
        assert_eq!(trace.final_response.reason.as_deref(), Some("connect"));
    }

    #[tokio::test]
    async fn login_with_trace_sanitizes_sensitive_fields() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": 13000,
                "region_x": 1000,
                "region_y": 1000,
                "seed_capability": "https://seed-cap.example.invalid",
                "start_location": "last"
            })))
            .mount(&server)
            .await;

        let mut intent = make_intent(true);
        intent.mfa_token = Some(String::from("sensitive-token"));
        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        let adapter = SecondLifeAdapter;

        connection.connect().await.expect("connect should succeed");
        let (_result, trace) = connection
            .login_with_trace(&adapter, intent)
            .await
            .expect("login with trace should succeed");

        assert!(trace.initial_request.had_mfa_token);
        assert_eq!(trace.initial_request.method, "login_to_simulator");
        assert_eq!(trace.initial_request.start_location, "last");
    }

    #[tokio::test]
    async fn login_with_trace_with_fallback_retries_llsd_request_shape_failure() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/secondlife.com/login"))
            .and(header("content-type", "application/llsd+xml"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"<llsd><map>
                    <key>login</key><boolean>false</boolean>
                    <key>reason</key><string>viewer-data</string>
                    <key>message</key><string>Missing password</string>
                </map></llsd>"#,
            ))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/secondlife.com/login"))
            .and(header("content-type", "text/xml"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"<?xml version="1.0"?>
<methodResponse><params><param><value><struct>
<member><name>login</name><value><boolean>0</boolean></value></member>
<member><name>reason</name><value><string>key</string></value></member>
<member><name>message</name><value><string>invalid credentials</string></value></member>
</struct></value></param></params></methodResponse>"#,
            ))
            .expect(1)
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/secondlife.com/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            wire_format: LoginWireFormat::Llsd,
        });
        let adapter = SecondLifeAdapter;
        connection.connect().await.expect("connect should succeed");

        let (result, _trace, fallback) = connection
            .login_with_trace_with_fallback(&adapter, make_intent(true))
            .await
            .expect("fallback login should return terminal result");

        assert!(matches!(result, GridLoginResult::Failed(_)));
        assert!(fallback.fallback_used);
        assert_eq!(fallback.primary_wire_format, LoginWireFormat::Llsd);
        assert_eq!(fallback.final_wire_format, LoginWireFormat::XmlRpc);
        assert_eq!(
            fallback.classified_reason,
            Some(LoginFallbackClassifiedReason::RequestShapeMissingPassword)
        );
    }

    #[tokio::test]
    async fn login_with_trace_with_fallback_does_not_retry_auth_failure() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/secondlife.com/login"))
            .and(header("content-type", "application/llsd+xml"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"<llsd><map>
                    <key>login</key><boolean>false</boolean>
                    <key>reason</key><string>key</string>
                    <key>message</key><string>invalid credentials</string>
                </map></llsd>"#,
            ))
            .expect(1)
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/secondlife.com/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            wire_format: LoginWireFormat::Llsd,
        });
        let adapter = SecondLifeAdapter;
        connection.connect().await.expect("connect should succeed");

        let (result, _trace, fallback) = connection
            .login_with_trace_with_fallback(&adapter, make_intent(true))
            .await
            .expect("login should return terminal result");

        assert!(matches!(result, GridLoginResult::Failed(_)));
        assert!(!fallback.fallback_used);
        assert_eq!(fallback.final_wire_format, LoginWireFormat::Llsd);
        assert_eq!(fallback.classified_reason, None);
    }

    #[test]
    fn json_codec_decodes_wrapped_login_response() {
        let codec = JsonLoginCodec;
        let body = br#"{
            "responses": {
                "login": "false",
                "data": {
                    "reason": "tos",
                    "message": "Terms required"
                }
            }
        }"#;

        let decoded = codec
            .decode_response(body)
            .expect("codec should decode normalized response");
        assert_eq!(decoded.login, Some(false));
        assert_eq!(decoded.reason.as_deref(), Some("tos"));
        assert_eq!(decoded.message.as_deref(), Some("Terms required"));
    }

    #[test]
    fn llsd_codec_encodes_login_request() {
        let codec = LlsdLoginCodec;
        let request = SecondLifeAdapter.shape_login_request(&make_intent(true));
        let encoded = String::from_utf8(
            codec
                .encode_request(&request)
                .expect("LLSD encode should succeed"),
        )
        .expect("valid UTF-8");
        assert!(encoded.contains("<key>method</key><string>login_to_simulator</string>"));
        assert!(encoded.contains("<key>username</key><string>test.user</string>"));
        assert!(
            encoded
                .contains("<key>passwd</key><string>$1$5ebe2294ecd0e0f08eab7690d2a6ee69</string>")
        );
        assert!(!encoded.contains("<key>password</key>"));
        assert!(encoded.contains("<string>inventory-root</string>"));
        assert!(encoded.contains("<key>options</key><array>"));
    }

    #[test]
    fn llsd_codec_encodes_legacy_first_last_fields() {
        let codec = LlsdLoginCodec;
        let mut intent = make_intent(true);
        intent.username = String::from("first.last");
        let request = SecondLifeAdapter.shape_login_request(&intent);

        let encoded = String::from_utf8(
            codec
                .encode_request(&request)
                .expect("LLSD encode should succeed"),
        )
        .expect("valid UTF-8");

        assert!(encoded.contains("<key>first</key><string>first</string>"));
        assert!(encoded.contains("<key>last</key><string>last</string>"));
        assert!(
            encoded
                .contains("<key>passwd</key><string>$1$5ebe2294ecd0e0f08eab7690d2a6ee69</string>")
        );
    }

    #[test]
    fn llsd_codec_preserves_existing_legacy_passwd_prefix() {
        let codec = LlsdLoginCodec;
        let mut intent = make_intent(true);
        intent.password = String::from("$1$alreadyhashed");
        let request = SecondLifeAdapter.shape_login_request(&intent);

        let encoded = String::from_utf8(
            codec
                .encode_request(&request)
                .expect("LLSD encode should succeed"),
        )
        .expect("valid UTF-8");

        assert!(encoded.contains("<key>passwd</key><string>$1$alreadyhashed</string>"));
    }

    #[test]
    fn llsd_codec_decodes_simple_login_response() {
        let codec = LlsdLoginCodec;
        let body = br#"<llsd><map>
            <key>login</key><boolean>true</boolean>
            <key>reason</key><string>connect</string>
            <key>agent_id</key><string>abcdef</string>
            <key>session_id</key><string>12345</string>
            <key>circuit_code</key><integer>4242</integer>
        </map></llsd>"#;

        let decoded = codec
            .decode_response(body)
            .expect("LLSD decode should succeed");
        assert_eq!(decoded.login, Some(true));
        assert_eq!(decoded.reason.as_deref(), Some("connect"));
        assert_eq!(decoded.agent_id.as_deref(), Some("abcdef"));
        assert_eq!(decoded.session_id.as_deref(), Some("12345"));
        assert_eq!(decoded.circuit_code, Some(4242));
    }

    #[test]
    fn xmlrpc_codec_encodes_method_call_request() {
        let codec = XmlRpcLoginCodec;
        let request = SecondLifeAdapter.shape_login_request(&make_intent(true));
        let encoded = String::from_utf8(
            codec
                .encode_request(&request)
                .expect("xml-rpc encode should succeed"),
        )
        .expect("valid UTF-8");

        assert!(encoded.contains("<methodCall>"));
        assert!(encoded.contains("<methodName>login_to_simulator</methodName>"));
        assert!(encoded.contains("<name>first</name><value><string>test</string></value>"));
        assert!(encoded.contains("<name>last</name><value><string>user</string></value>"));
        assert!(!encoded.contains("<name>username</name>"));
        assert!(encoded.contains("<name>passwd</name>"));
        assert!(encoded.contains("$1$5ebe2294ecd0e0f08eab7690d2a6ee69"));
        assert!(encoded.contains("<name>options</name>"));
    }

    #[test]
    fn xmlrpc_codec_encodes_legacy_first_last_for_space_separated_names() {
        let codec = XmlRpcLoginCodec;
        let mut intent = make_intent(true);
        intent.username = String::from("legacy resident");
        let request = SecondLifeAdapter.shape_login_request(&intent);
        let encoded = String::from_utf8(
            codec
                .encode_request(&request)
                .expect("xml-rpc encode should succeed"),
        )
        .expect("valid UTF-8");

        assert!(encoded.contains("<name>first</name><value><string>legacy</string></value>"));
        assert!(encoded.contains("<name>last</name><value><string>resident</string></value>"));
        assert!(!encoded.contains("<name>username</name>"));
    }

    #[test]
    fn xmlrpc_codec_encodes_resident_last_name_for_single_identifier() {
        let codec = XmlRpcLoginCodec;
        let mut intent = make_intent(true);
        intent.username = String::from("bobsmith12");
        let request = SecondLifeAdapter.shape_login_request(&intent);
        let encoded = String::from_utf8(
            codec
                .encode_request(&request)
                .expect("xml-rpc encode should succeed"),
        )
        .expect("valid UTF-8");

        assert!(encoded.contains("<name>first</name><value><string>bobsmith12</string></value>"));
        assert!(encoded.contains("<name>last</name><value><string>Resident</string></value>"));
        assert!(!encoded.contains("<name>username</name>"));
    }

    #[test]
    fn xmlrpc_codec_decodes_login_response() {
        let codec = XmlRpcLoginCodec;
        let body = br#"<?xml version="1.0"?>
<methodResponse>
  <params>
    <param>
      <value>
        <struct>
          <member><name>login</name><value><boolean>1</boolean></value></member>
          <member><name>reason</name><value><string>connect</string></value></member>
          <member><name>agent_id</name><value><string>abc</string></value></member>
          <member><name>session_id</name><value><string>def</string></value></member>
          <member><name>secure_session_id</name><value><string>ghi</string></value></member>
          <member><name>circuit_code</name><value><int>1</int></value></member>
          <member><name>sim_ip</name><value><string>127.0.0.1</string></value></member>
          <member><name>sim_port</name><value><int>13000</int></value></member>
          <member><name>region_x</name><value><int>1000</int></value></member>
          <member><name>region_y</name><value><int>1000</int></value></member>
          <member><name>seed_capability</name><value><string>https://seed</string></value></member>
        </struct>
      </value>
    </param>
  </params>
</methodResponse>"#;

        let decoded = codec
            .decode_response(body)
            .expect("xml-rpc decode should succeed");
        assert_eq!(decoded.login, Some(true));
        assert_eq!(decoded.reason.as_deref(), Some("connect"));
        assert_eq!(decoded.agent_id.as_deref(), Some("abc"));
        assert_eq!(decoded.circuit_code, Some(1));
    }

    #[tokio::test]
    async fn login_with_json_format_uses_json_codec() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .and(header("content-type", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "0000",
                "session_id": "1111",
                "secure_session_id": "2222",
                "circuit_code": 1,
                "sim_ip": "127.0.0.1",
                "sim_port": 1234,
                "region_x": 10,
                "region_y": 10,
                "seed_capability": "https://seed"
            })))
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        let adapter = SecondLifeAdapter;

        connection.connect().await.expect("connect should succeed");
        let result = connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login succeeds");

        match result {
            GridLoginResult::Success(_) => {}
            other => panic!("expected success, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn login_with_llsd_format_uses_llsd_codec() {
        let server = MockServer::start().await;
        let response = r#"<llsd><map>
            <key>login</key><boolean>true</boolean>
            <key>reason</key><string>connect</string>
            <key>agent_id</key><string>abc</string>
            <key>session_id</key><string>def</string>
            <key>secure_session_id</key><string>ghi</string>
            <key>circuit_code</key><integer>1</integer>
            <key>sim_ip</key><string>127.0.0.1</string>
            <key>sim_port</key><integer>123</integer>
            <key>region_x</key><integer>1</integer>
            <key>region_y</key><integer>1</integer>
            <key>seed_capability</key><string>https://seed</string>
        </map></llsd>"#;

        Mock::given(method("POST"))
            .and(path("/login"))
            .and(header("content-type", "application/llsd+xml"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response))
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            wire_format: LoginWireFormat::Llsd,
        });
        let adapter = SecondLifeAdapter;

        connection.connect().await.expect("connect should succeed");
        let result = connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login succeeds");

        match result {
            GridLoginResult::Success(_) => {}
            other => panic!("expected success, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn login_with_xmlrpc_format_uses_xmlrpc_codec() {
        let server = MockServer::start().await;
        let response = r#"<?xml version="1.0"?>
<methodResponse>
  <params>
    <param>
      <value>
        <struct>
          <member><name>login</name><value><boolean>1</boolean></value></member>
          <member><name>reason</name><value><string>connect</string></value></member>
          <member><name>agent_id</name><value><string>abc</string></value></member>
          <member><name>session_id</name><value><string>def</string></value></member>
          <member><name>secure_session_id</name><value><string>ghi</string></value></member>
          <member><name>circuit_code</name><value><int>1</int></value></member>
          <member><name>sim_ip</name><value><string>127.0.0.1</string></value></member>
          <member><name>sim_port</name><value><int>123</int></value></member>
          <member><name>region_x</name><value><int>1</int></value></member>
          <member><name>region_y</name><value><int>1</int></value></member>
          <member><name>seed_capability</name><value><string>https://seed</string></value></member>
        </struct>
      </value>
    </param>
  </params>
</methodResponse>"#;

        Mock::given(method("POST"))
            .and(path("/login"))
            .and(header("content-type", "text/xml"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response))
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            wire_format: LoginWireFormat::XmlRpc,
        });
        let adapter = SecondLifeAdapter;

        connection.connect().await.expect("connect should succeed");
        let result = connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login succeeds");

        match result {
            GridLoginResult::Success(_) => {}
            other => panic!("expected success, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn fetch_seed_capabilities_after_login_returns_capability_map() {
        let server = MockServer::start().await;
        let seed_url = format!("{}/seed", server.uri());

        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "0000",
                "session_id": "1111",
                "secure_session_id": "2222",
                "circuit_code": 1,
                "sim_ip": "127.0.0.1",
                "sim_port": 1234,
                "region_x": 10,
                "region_y": 10,
                "seed_capability": seed_url
            })))
            .mount(&server)
            .await;

        let capability_map = r#"<llsd><map>
            <key>EventQueueGet</key><string>https://cap.example/event</string>
            <key>AgentProfile</key><string>https://cap.example/agent-profile</string>
            <key>MapLayer</key><string>https://cap.example/map</string>
        </map></llsd>"#;

        Mock::given(method("POST"))
            .and(path("/seed"))
            .and(header("content-type", "application/llsd+xml"))
            .and(body_string_contains("<llsd><array>"))
            .and(body_string_contains("<string>EventQueueGet</string>"))
            .and(body_string_contains(
                "<string>UntrustedSimulatorMessage</string>",
            ))
            .and(body_string_contains("<string>InterestList</string>"))
            .and(body_string_contains("<string>RegionObjects</string>"))
            .and(body_string_contains("<string>AgentProfile</string>"))
            .and(body_string_contains("<string>GetMesh2</string>"))
            .and(body_string_contains("<string>ReadOfflineMsgs</string>"))
            .and(body_string_contains("<string>ViewerStats</string>"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/llsd+xml")
                    .set_body_string(capability_map),
            )
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        let adapter = SecondLifeAdapter;
        connection.connect().await.expect("connect should succeed");
        let login_result = connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");
        match login_result {
            GridLoginResult::Success(_) => {}
            other => panic!("expected success, got {other:?}"),
        }

        let caps = connection
            .fetch_seed_capabilities()
            .await
            .expect("seed capability fetch should succeed");

        assert_eq!(
            caps.entries.get("EventQueueGet").map(String::as_str),
            Some("https://cap.example/event")
        );
        assert_eq!(
            caps.entries.get("MapLayer").map(String::as_str),
            Some("https://cap.example/map")
        );
        assert_eq!(
            caps.entries.get("AgentProfile").map(String::as_str),
            Some("https://cap.example/agent-profile")
        );
    }

    #[tokio::test]
    async fn fetch_event_queue_once_reports_event_names() {
        let server = MockServer::start().await;

        let event_response = r#"<llsd><map>
            <key>events</key><array>
                <map>
                    <key>message</key><string>EnableSimulator</string>
                    <key>body</key><map></map>
                </map>
                <map>
                    <key>message</key><string>ParcelProperties</string>
                    <key>body</key><map></map>
                </map>
            </array>
            <key>id</key><integer>41</integer>
        </map></llsd>"#;

        Mock::given(method("POST"))
            .and(path("/eventqueue"))
            .and(header("content-type", "application/llsd+xml"))
            .and(body_string_contains("<key>ack</key><integer>0</integer>"))
            .and(body_string_contains(
                "<key>done</key><boolean>false</boolean>",
            ))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/llsd+xml")
                    .set_body_string(event_response),
            )
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        connection.state = ConnectionState::LoggedIn;

        let inspection = connection
            .fetch_event_queue_once(&format!("{}/eventqueue", server.uri()))
            .await
            .expect("event queue fetch should succeed");

        assert!(inspection.has_events_array);
        assert!(inspection.has_id);
        assert_eq!(inspection.event_count, 2);
        assert_eq!(
            inspection.event_names,
            vec![
                "EnableSimulator".to_string(),
                "ParcelProperties".to_string()
            ]
        );
        assert!(inspection.top_level_keys.iter().any(|key| key == "events"));
        assert!(inspection.top_level_keys.iter().any(|key| key == "id"));
    }

    #[tokio::test]
    async fn fetch_event_queue_once_uses_extended_timeout_window() {
        let server = MockServer::start().await;

        let event_response = r#"<llsd><map>
            <key>events</key><array></array>
            <key>id</key><integer>42</integer>
        </map></llsd>"#;

        Mock::given(method("POST"))
            .and(path("/eventqueue"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(StdDuration::from_secs(2))
                    .insert_header("content-type", "application/llsd+xml")
                    .set_body_string(event_response),
            )
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(1),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        connection.state = ConnectionState::LoggedIn;

        let inspection = connection
            .fetch_event_queue_once(&format!("{}/eventqueue", server.uri()))
            .await
            .expect("event queue one-shot should succeed despite short login timeout");

        assert!(inspection.has_events_array);
        assert!(inspection.has_id);
        assert_eq!(inspection.event_count, 0);
    }

    #[tokio::test]
    async fn fetch_event_queue_once_does_not_retry_non_retryable_http_failure() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/eventqueue"))
            .respond_with(ResponseTemplate::new(400).set_body_string("bad request"))
            .expect(1)
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(1),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        connection.state = ConnectionState::LoggedIn;

        let err = connection
            .fetch_event_queue_once(&format!("{}/eventqueue", server.uri()))
            .await
            .expect_err("event queue should fail");
        match err {
            ConnectionError::EventQueueOneShotFailed {
                attempts_len,
                attempts,
            } => {
                assert_eq!(attempts_len, 1);
                assert_eq!(attempts.len(), 1);
                assert_eq!(attempts[0].status, Some(400));
                assert!(!attempts[0].retryable);
                assert_eq!(
                    attempts[0].retry_class,
                    EventQueueRetryClass::NonRetryableHttp
                );
                assert!(attempts[0].scheduled_backoff_ms.is_none());
                assert!(attempts[0].terminal_reason.is_some());
            }
            other => panic!("expected EventQueueOneShotFailed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn fetch_event_queue_once_retries_up_to_bounded_limit() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/eventqueue"))
            .respond_with(
                ResponseTemplate::new(500)
                    .set_body_string("Proxy Error: Error reading from remote server"),
            )
            .expect(MAX_EVENT_QUEUE_ONE_SHOT_ATTEMPTS as u64)
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(1),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        connection.state = ConnectionState::LoggedIn;

        let err = connection
            .fetch_event_queue_once(&format!("{}/eventqueue", server.uri()))
            .await
            .expect_err("event queue should fail after retries");
        match err {
            ConnectionError::EventQueueOneShotFailed {
                attempts_len,
                attempts,
            } => {
                assert_eq!(attempts_len, MAX_EVENT_QUEUE_ONE_SHOT_ATTEMPTS);
                assert_eq!(attempts.len(), MAX_EVENT_QUEUE_ONE_SHOT_ATTEMPTS);
                assert!(attempts.iter().all(|attempt| attempt.retryable));
                assert!(attempts.iter().all(|attempt| attempt.status == Some(500)));
                assert_eq!(attempts[0].scheduled_backoff_ms, Some(250));
                assert_eq!(attempts[1].scheduled_backoff_ms, Some(500));
                assert!(attempts[2].scheduled_backoff_ms.is_none());
                assert!(attempts[2].terminal_reason.is_some());
            }
            other => panic!("expected EventQueueOneShotFailed, got {other:?}"),
        }
    }

    #[test]
    fn event_queue_retry_backoff_is_bounded_exponential() {
        assert_eq!(
            event_queue_retry_backoff_ms(1, true, Some(StatusCode::BAD_GATEWAY)),
            Some(250)
        );
        assert_eq!(
            event_queue_retry_backoff_ms(2, true, Some(StatusCode::BAD_GATEWAY)),
            Some(500)
        );
        assert_eq!(
            event_queue_retry_backoff_ms(3, true, Some(StatusCode::BAD_GATEWAY)),
            None
        );
        assert_eq!(
            event_queue_retry_backoff_ms(1, true, Some(StatusCode::BAD_REQUEST)),
            None
        );
        assert_eq!(
            event_queue_retry_backoff_ms(1, false, Some(StatusCode::BAD_GATEWAY)),
            None
        );
    }

    #[tokio::test]
    async fn fetch_simulator_features_once_reports_top_level_shape() {
        let server = MockServer::start().await;
        let simulator_features = r#"<llsd><map>
            <key>MeshRezEnabled</key><boolean>true</boolean>
            <key>PhysicsMaterialsEnabled</key><boolean>false</boolean>
            <key>OpenSimExtras</key><map>
                <key>foo</key><string>bar</string>
            </map>
            <key>Channel</key><string>Second Life Server</string>
        </map></llsd>"#;

        Mock::given(method("GET"))
            .and(path("/sim-features"))
            .and(header("accept", LLSD_XML_CONTENT_TYPE))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/llsd+xml")
                    .set_body_string(simulator_features),
            )
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        connection.state = ConnectionState::LoggedIn;

        let inspection = connection
            .fetch_simulator_features_once(&format!("{}/sim-features", server.uri()))
            .await
            .expect("simulator features fetch should succeed");

        assert!(
            inspection
                .top_level_keys
                .iter()
                .any(|k| k == "MeshRezEnabled")
        );
        assert!(
            inspection
                .top_level_keys
                .iter()
                .any(|k| k == "OpenSimExtras")
        );
        assert_eq!(
            inspection.scalar_values.get("Channel").map(String::as_str),
            Some("Second Life Server")
        );
        assert_eq!(
            inspection
                .complex_value_types
                .get("OpenSimExtras")
                .map(String::as_str),
            Some("map")
        );
    }

    #[tokio::test]
    async fn fetch_map_layer_once_reports_top_level_shape() {
        let server = MockServer::start().await;
        let map_layer_body = r#"<llsd><map>
            <key>MapBlocks</key><array></array>
            <key>MapServerVersion</key><string>1.0</string>
            <key>Enabled</key><boolean>true</boolean>
        </map></llsd>"#;

        Mock::given(method("GET"))
            .and(path("/map-layer"))
            .and(header("accept", LLSD_XML_CONTENT_TYPE))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/llsd+xml")
                    .set_body_string(map_layer_body),
            )
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        connection.state = ConnectionState::LoggedIn;

        let inspection = connection
            .fetch_map_layer_once(&format!("{}/map-layer", server.uri()))
            .await
            .expect("map layer fetch should succeed");

        assert!(inspection.top_level_keys.iter().any(|k| k == "MapBlocks"));
        assert_eq!(
            inspection
                .scalar_values
                .get("MapServerVersion")
                .map(String::as_str),
            Some("1.0")
        );
        assert_eq!(
            inspection
                .complex_value_types
                .get("MapBlocks")
                .map(String::as_str),
            Some("array")
        );
    }

    #[tokio::test]
    async fn fetch_map_layer_once_classifies_405_as_likely_legacy_udp() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/map-layer"))
            .and(header("accept", LLSD_XML_CONTENT_TYPE))
            .respond_with(ResponseTemplate::new(405).set_body_string("Method Not Allowed"))
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        connection.state = ConnectionState::LoggedIn;

        let err = connection
            .fetch_map_layer_once(&format!("{}/map-layer", server.uri()))
            .await
            .expect_err("map layer 405 should be classified");
        match err {
            ConnectionError::MapLayerLikelyLegacyUdp { status, body } => {
                assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
                assert!(body.contains("Method Not Allowed"));
            }
            other => panic!("expected MapLayerLikelyLegacyUdp, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn fetch_untrusted_simulator_message_once_falls_back_to_post_after_405() {
        let server = MockServer::start().await;
        let untrusted_ok = r#"<llsd><map>
            <key>ok</key><boolean>true</boolean>
            <key>status</key><string>accepted</string>
        </map></llsd>"#;

        Mock::given(method("GET"))
            .and(path("/untrusted"))
            .and(header("accept", LLSD_XML_CONTENT_TYPE))
            .respond_with(ResponseTemplate::new(405).set_body_string("Method Not Allowed"))
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/untrusted"))
            .and(header("accept", LLSD_XML_CONTENT_TYPE))
            .and(header("content-type", LLSD_XML_CONTENT_TYPE))
            .and(body_string_contains(
                "<key>message</key><string>AgentDataUpdateRequest</string>",
            ))
            .and(body_string_contains("<key>AgentID</key><uuid>"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/llsd+xml")
                    .set_body_string(untrusted_ok),
            )
            .expect(1)
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        connection.state = ConnectionState::LoggedIn;
        connection.first_simulator_handshake_prerequisites =
            Some(FirstSimulatorHandshakePrerequisites {
                agent_id: String::from("11111111-1111-1111-1111-111111111111"),
                session_id: String::from("22222222-2222-2222-2222-222222222222"),
                circuit_code: 7,
                target: FirstSimulatorTarget {
                    sim_ip: String::from("127.0.0.1"),
                    sim_port: 13001,
                    region_x: 0,
                    region_y: 0,
                    seed_capability: format!("{}/seed", server.uri()),
                },
            });

        let inspection = connection
            .fetch_untrusted_simulator_message_once(&format!("{}/untrusted", server.uri()))
            .await
            .expect("untrusted probe should succeed via POST fallback");
        assert_eq!(
            inspection.scalar_values.get("status").map(String::as_str),
            Some("accepted")
        );
    }

    #[tokio::test]
    async fn fetch_untrusted_simulator_message_once_treats_non_llsd_success_as_transport_ok() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/untrusted-plain"))
            .and(header("accept", LLSD_XML_CONTENT_TYPE))
            .respond_with(ResponseTemplate::new(405).set_body_string("Method Not Allowed"))
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/untrusted-plain"))
            .and(header("accept", LLSD_XML_CONTENT_TYPE))
            .and(header("content-type", LLSD_XML_CONTENT_TYPE))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/plain")
                    .set_body_string("ok"),
            )
            .expect(1)
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        connection.state = ConnectionState::LoggedIn;

        let inspection = connection
            .fetch_untrusted_simulator_message_once(&format!("{}/untrusted-plain", server.uri()))
            .await
            .expect("untrusted probe should surface transport-ok inspection");
        assert_eq!(
            inspection
                .scalar_values
                .get("probe_transport_status")
                .map(String::as_str),
            Some("200")
        );
        assert_eq!(
            inspection
                .scalar_values
                .get("probe_decode")
                .map(String::as_str),
            Some("unparsed_body")
        );
    }

    #[tokio::test]
    async fn probe_capability_transport_once_reports_status_content_type_and_body_size() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/transport-probe"))
            .and(header("accept", LLSD_XML_CONTENT_TYPE))
            .respond_with(
                ResponseTemplate::new(404)
                    .insert_header("content-type", "text/plain")
                    .set_body_string("missing"),
            )
            .expect(1)
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        connection.state = ConnectionState::LoggedIn;

        let probe = connection
            .probe_capability_transport_once(&format!("{}/transport-probe", server.uri()))
            .await
            .expect("transport probe should return bounded metadata");
        assert_eq!(probe.status, 404);
        assert_eq!(probe.content_type.as_deref(), Some("text/plain"));
        assert_eq!(probe.body_bytes, 7);
        assert_eq!(probe.method, "GET");
        assert_eq!(probe.decode, "plain_text");
        assert_eq!(probe.response_class, "client_error");
        assert!(probe.body_preview.is_some());
        assert!(probe.selected_url.is_some());
    }

    #[tokio::test]
    async fn execute_capability_probe_once_supports_post_with_body_and_classifies_json() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/shape-probe"))
            .and(header("content-type", "application/llsd+xml"))
            .and(body_string_contains("<key>mode</key>"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/json")
                    .set_body_json(json!({"ok": true})),
            )
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        connection.state = ConnectionState::LoggedIn;

        let probe = connection
            .execute_capability_probe_once(&CapabilityProbeRequest {
                method: String::from("POST"),
                url_candidates: vec![format!("{}/shape-probe", server.uri())],
                content_type: Some(String::from("application/llsd+xml")),
                body: Some(String::from(
                    "<llsd><map><key>mode</key><string>default</string></map></llsd>",
                )),
                ..Default::default()
            })
            .await
            .expect("probe should succeed");

        assert_eq!(probe.status, 200);
        assert_eq!(probe.method, "POST");
        assert_eq!(probe.decode, "json_obj");
        assert_eq!(probe.response_class, "success");
    }

    #[tokio::test]
    async fn fetch_region_objects_once_reports_array_shape_and_first_item_keys() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/region-objects"))
            .and(header("accept", LLSD_XML_CONTENT_TYPE))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/json")
                    .set_body_json(json!({
                        "objects": [
                            {
                                "id": "object-1",
                                "name": "Cube",
                                "owner_id": "owner-1"
                            },
                            {
                                "id": "object-2"
                            }
                        ],
                        "version": 3,
                        "region": "Sandbox"
                    })),
            )
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        connection.state = ConnectionState::LoggedIn;

        let inspection = connection
            .fetch_region_objects_once(&format!("{}/region-objects", server.uri()))
            .await
            .expect("region objects fetch should succeed");

        assert!(inspection.top_level_keys.iter().any(|key| key == "objects"));
        assert_eq!(inspection.array_lengths.get("objects"), Some(&2));
        assert_eq!(
            inspection.first_array_item_keys.get("objects"),
            Some(&vec![
                String::from("id"),
                String::from("name"),
                String::from("owner_id")
            ])
        );
        assert_eq!(
            inspection.scalar_values.get("version").map(String::as_str),
            Some("3")
        );
        assert_eq!(
            inspection.scalar_values.get("region").map(String::as_str),
            Some("Sandbox")
        );
    }

    #[tokio::test]
    async fn fetch_region_objects_once_reports_child_map_keys_and_scalars() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/region-objects-map"))
            .and(header("accept", LLSD_XML_CONTENT_TYPE))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/json")
                    .set_body_json(json!({
                        "11111111-1111-1111-1111-111111111111": {
                            "name": "Cube",
                            "owner_id": "owner-1",
                            "local_id": 77,
                            "phantom": false
                        },
                        "22222222-2222-2222-2222-222222222222": {
                            "name": "Sphere"
                        }
                    })),
            )
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        connection.state = ConnectionState::LoggedIn;

        let inspection = connection
            .fetch_region_objects_once(&format!("{}/region-objects-map", server.uri()))
            .await
            .expect("region objects fetch should succeed");

        let first_key = "11111111-1111-1111-1111-111111111111";
        assert_eq!(
            inspection.child_map_keys.get(first_key),
            Some(&vec![
                String::from("local_id"),
                String::from("name"),
                String::from("owner_id"),
                String::from("phantom")
            ])
        );
        assert_eq!(
            inspection.child_map_scalar_values.get(first_key),
            Some(&vec![
                String::from("local_id=77"),
                String::from("name=Cube"),
                String::from("owner_id=owner-1"),
                String::from("phantom=false")
            ])
        );
        assert_eq!(inspection.child_map_profiles.get(first_key), None);
        assert_eq!(inspection.child_map_semantic_values.get(first_key), None);
        assert_eq!(
            inspection.child_map_pathfinding_summaries.get(first_key),
            None
        );
    }

    #[tokio::test]
    async fn fetch_region_objects_once_reports_typed_pathfinding_summary() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/region-objects-pathfinding"))
            .and(header("accept", LLSD_XML_CONTENT_TYPE))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/json")
                    .set_body_json(json!({
                        "11111111-1111-1111-1111-111111111111": {
                            "name": "Cube",
                            "description": "Walkable cube",
                            "owner": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
                            "owner_is_group": 1,
                            "position": [128, 64, 32],
                            "landimpact": 2,
                            "modifiable": 1,
                            "navmesh_category": 2,
                            "can_be_volume": 0,
                            "is_scripted": 0,
                            "phantom": 1,
                            "A": 100,
                            "B": 90,
                            "C": 80,
                            "D": 70
                        }
                    })),
            )
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        connection.state = ConnectionState::LoggedIn;

        let inspection = connection
            .fetch_region_objects_once(&format!("{}/region-objects-pathfinding", server.uri()))
            .await
            .expect("region objects fetch should succeed");

        let first_key = "11111111-1111-1111-1111-111111111111";
        let summary = inspection
            .child_map_pathfinding_summaries
            .get(first_key)
            .expect("pathfinding summary should be present");
        assert_eq!(summary.profile, "pathfinding_linkset");
        assert_eq!(summary.name.as_deref(), Some("Cube"));
        assert_eq!(summary.description.as_deref(), Some("Walkable cube"));
        assert_eq!(
            summary.owner.as_deref(),
            Some("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee")
        );
        assert_eq!(summary.owner_is_group, Some(true));
        assert!(summary.position_key_present);
        assert_eq!(summary.position.as_deref(), Some("128|64|32"));
        assert_eq!(summary.position_shape.as_deref(), Some("json_array_len3"));
        assert_eq!(summary.landimpact, Some(2));
        assert_eq!(summary.modifiable, Some(true));
        assert_eq!(summary.navmesh_category, Some(2));
        assert_eq!(summary.can_be_volume, Some(false));
        assert_eq!(summary.is_scripted, Some(false));
        assert_eq!(summary.phantom, Some(true));
        assert_eq!(summary.walkability_coefficients, Some([100, 90, 80, 70]));
        assert_eq!(summary.linkset_use.as_deref(), Some("dynamic_phantom"));
        assert_eq!(summary.variant_hint, None);
        assert_eq!(summary.description_shape.as_deref(), Some("free_text"));
        assert_eq!(summary.description_numeric_tuple, None);
        assert_eq!(inspection.typed_object_samples.len(), 1);
        let typed = inspection
            .typed_object_samples
            .first()
            .expect("typed object sample should be present");
        assert_eq!(typed.object_id, first_key);
        assert_eq!(typed.profile, "pathfinding_linkset");
        assert_eq!(typed.name.as_deref(), Some("Cube"));
        assert_eq!(
            typed.owner.as_deref(),
            Some("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee")
        );
        assert_eq!(typed.position.as_deref(), Some("128|64|32"));
        assert_eq!(typed.description_shape.as_deref(), Some("free_text"));
        assert_eq!(typed.linkset_use.as_deref(), Some("dynamic_phantom"));
        assert_eq!(typed.walkability_coefficients, Some([100, 90, 80, 70]));
        assert_eq!(typed.landimpact, Some(2));
        assert_eq!(inspection.tuple_description_analysis, None);
    }

    #[test]
    fn region_objects_mesh_candidate_extraction_filters_by_key_hint_and_uuid() {
        let mut inspection = RegionObjectsInspection::default();
        maybe_record_region_objects_mesh_candidate(
            &mut inspection,
            "mesh_id",
            "947d4505-eb76-2ef5-c049-e7882881d689",
        );
        maybe_record_region_objects_mesh_candidate(
            &mut inspection,
            "owner",
            "11111111-1111-1111-1111-111111111111",
        );
        maybe_record_region_objects_mesh_candidate(&mut inspection, "mesh_asset", "not-a-uuid");
        maybe_record_region_objects_mesh_candidate(
            &mut inspection,
            "sculpt_id",
            "947D4505-EB76-2EF5-C049-E7882881D689",
        );

        assert_eq!(
            inspection.candidate_mesh_asset_ids,
            vec![String::from("947d4505-eb76-2ef5-c049-e7882881d689")]
        );
    }

    #[test]
    fn classify_region_objects_child_semantics_recognizes_pathfinding_linkset() {
        let key_set = BTreeSet::from([
            "A",
            "B",
            "C",
            "D",
            "can_be_volume",
            "description",
            "landimpact",
            "modifiable",
            "name",
            "navmesh_category",
            "owner",
            "phantom",
            "position",
        ]);
        let scalar_fields = BTreeMap::from([
            (String::from("A"), String::from("100")),
            (String::from("B"), String::from("100")),
            (String::from("C"), String::from("100")),
            (String::from("D"), String::from("100")),
            (String::from("can_be_volume"), String::from("true")),
            (
                String::from("description"),
                String::from("Pathfinding test cube"),
            ),
            (String::from("landimpact"), String::from("1")),
            (String::from("modifiable"), String::from("true")),
            (String::from("name"), String::from("Cube")),
            (String::from("navmesh_category"), String::from("2")),
            (
                String::from("owner"),
                String::from("11111111-1111-1111-1111-111111111111"),
            ),
            (String::from("phantom"), String::from("1")),
        ]);

        let summary = build_region_objects_pathfinding_summary(
            &key_set,
            &scalar_fields,
            RegionObjectsPositionInspection {
                present: true,
                summary: Some(String::from("1|2|3")),
                shape: Some(String::from("json_array_len3")),
            },
        )
        .expect("pathfinding linkset should classify");
        let semantic_values = summarize_region_objects_pathfinding_summary(&summary);

        assert_eq!(summary.profile, "pathfinding_linkset");
        assert!(
            semantic_values
                .iter()
                .any(|value| value == "linkset_use=dynamic_phantom")
        );
        assert!(
            semantic_values
                .iter()
                .any(|value| value == "walkability=A:100|B:100|C:100|D:100")
        );
        assert!(
            semantic_values
                .iter()
                .any(|value| value == "description=Pathfinding test cube")
        );
        assert!(summary.position_key_present);
        assert_eq!(summary.position.as_deref(), Some("1|2|3"));
        assert_eq!(summary.position_shape.as_deref(), Some("json_array_len3"));
        assert_eq!(summary.variant_hint, None);
    }

    #[test]
    fn typed_pathfinding_summary_detects_numeric_tuple_description_variant() {
        let key_set = BTreeSet::from([
            "A",
            "B",
            "C",
            "D",
            "can_be_volume",
            "description",
            "name",
            "owner",
            "phantom",
        ]);
        let scalar_fields = BTreeMap::from([
            (String::from("A"), String::from("100")),
            (String::from("B"), String::from("100")),
            (String::from("C"), String::from("100")),
            (String::from("D"), String::from("100")),
            (String::from("can_be_volume"), String::from("0")),
            (
                String::from("description"),
                String::from("0,10.000000,30,0,2,0"),
            ),
            (String::from("name"), String::from("DSS Candlier Frame")),
            (
                String::from("owner"),
                String::from("10b483ef-e1b9-4e8d-8247-2324b3e4b48c"),
            ),
            (String::from("phantom"), String::from("1")),
        ]);

        let summary = build_region_objects_pathfinding_summary(
            &key_set,
            &scalar_fields,
            RegionObjectsPositionInspection::default(),
        )
        .expect("tuple variant should classify");

        assert_eq!(
            summary.variant_hint.as_deref(),
            Some("pathfinding_tuple_description_no_position_key")
        );
        assert_eq!(
            summary.description_shape.as_deref(),
            Some("comma_numeric_tuple6")
        );
        assert!(!summary.position_key_present);
        assert_eq!(summary.position_shape, None);
        assert_eq!(
            summary.description_numeric_tuple,
            Some(vec![
                String::from("0"),
                String::from("10.000000"),
                String::from("30"),
                String::from("0"),
                String::from("2"),
                String::from("0"),
            ])
        );
    }

    #[test]
    fn typed_pathfinding_summary_detects_placeholder_description_variant() {
        let key_set = BTreeSet::from([
            "A",
            "B",
            "C",
            "D",
            "can_be_volume",
            "description",
            "name",
            "owner",
            "phantom",
        ]);
        let scalar_fields = BTreeMap::from([
            (String::from("A"), String::from("100")),
            (String::from("B"), String::from("100")),
            (String::from("C"), String::from("100")),
            (String::from("D"), String::from("100")),
            (String::from("can_be_volume"), String::from("1")),
            (
                String::from("description"),
                String::from("(No Description)"),
            ),
            (
                String::from("name"),
                String::from("Trance  Chair: Rope Bondage"),
            ),
            (
                String::from("owner"),
                String::from("10b483ef-e1b9-4e8d-8247-2324b3e4b48c"),
            ),
            (String::from("phantom"), String::from("1")),
        ]);

        let summary = build_region_objects_pathfinding_summary(
            &key_set,
            &scalar_fields,
            RegionObjectsPositionInspection::default(),
        )
        .expect("placeholder variant should classify");

        assert_eq!(
            summary.variant_hint.as_deref(),
            Some("pathfinding_placeholder_description_no_position_key")
        );
        assert_eq!(
            summary.description_shape.as_deref(),
            Some("placeholder_text")
        );
        assert!(!summary.position_key_present);
        assert_eq!(summary.description_numeric_tuple, None);
    }

    #[test]
    fn typed_pathfinding_summary_detects_present_but_unparsed_position() {
        let key_set = BTreeSet::from([
            "A",
            "B",
            "C",
            "D",
            "can_be_volume",
            "description",
            "name",
            "owner",
            "phantom",
            "position",
        ]);
        let scalar_fields = BTreeMap::from([
            (String::from("A"), String::from("100")),
            (String::from("B"), String::from("100")),
            (String::from("C"), String::from("100")),
            (String::from("D"), String::from("100")),
            (String::from("can_be_volume"), String::from("0")),
            (
                String::from("description"),
                String::from("0,10.000000,30,0,2,0"),
            ),
            (String::from("name"), String::from("DSS Candlier Frame")),
            (
                String::from("owner"),
                String::from("10b483ef-e1b9-4e8d-8247-2324b3e4b48c"),
            ),
            (String::from("phantom"), String::from("1")),
        ]);

        let summary = build_region_objects_pathfinding_summary(
            &key_set,
            &scalar_fields,
            RegionObjectsPositionInspection {
                present: true,
                summary: None,
                shape: Some(String::from("json_object_keys2")),
            },
        )
        .expect("present-but-unparsed position variant should classify");

        assert!(summary.position_key_present);
        assert_eq!(summary.position, None);
        assert_eq!(summary.position_shape.as_deref(), Some("json_object_keys2"));
        assert_eq!(
            summary.variant_hint.as_deref(),
            Some("pathfinding_tuple_description_position_unparsed")
        );
    }

    #[test]
    fn region_objects_tuple_analysis_detects_constant_and_varying_slots() {
        let mut inspection = RegionObjectsInspection::default();
        inspection.child_map_pathfinding_summaries.insert(
            String::from("a"),
            RegionObjectsPathfindingSummary {
                name: Some(String::from("DSS Candlier Frame")),
                position: Some(String::from("39|68|2999")),
                description_numeric_tuple: Some(vec![
                    String::from("0"),
                    String::from("10.000000"),
                    String::from("30"),
                    String::from("0"),
                    String::from("2"),
                    String::from("0"),
                ]),
                ..Default::default()
            },
        );
        inspection.child_map_pathfinding_summaries.insert(
            String::from("b"),
            RegionObjectsPathfindingSummary {
                name: Some(String::from("DSS Candlier Frame")),
                position: Some(String::from("69|38|2999")),
                description_numeric_tuple: Some(vec![
                    String::from("0"),
                    String::from("10.000000"),
                    String::from("30"),
                    String::from("0"),
                    String::from("3"),
                    String::from("0"),
                ]),
                ..Default::default()
            },
        );

        analyze_region_objects_tuple_descriptions(&mut inspection);

        let analysis = inspection
            .tuple_description_analysis
            .as_ref()
            .expect("tuple analysis should be present");
        assert_eq!(analysis.sample_count, 2);
        assert_eq!(analysis.slot_count, 6);
        assert_eq!(
            analysis.slot_distinct_values,
            vec![
                vec![String::from("0")],
                vec![String::from("10.000000")],
                vec![String::from("30")],
                vec![String::from("0")],
                vec![String::from("2"), String::from("3")],
                vec![String::from("0")],
            ]
        );
        assert_eq!(
            analysis.distinct_names,
            vec![String::from("DSS Candlier Frame")]
        );
        assert_eq!(
            analysis.sample_pairs,
            vec![
                String::from("DSS Candlier Frame@39|68|2999=0|10.000000|30|0|2|0"),
                String::from("DSS Candlier Frame@69|38|2999=0|10.000000|30|0|3|0"),
            ]
        );
    }

    #[tokio::test]
    async fn first_simulator_handshake_scaffold_advances_in_order() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": 13000,
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })))
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        let adapter = SecondLifeAdapter;
        let result = connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");
        assert!(matches!(result, GridLoginResult::Success(_)));

        let state = connection
            .begin_first_simulator_handshake_scaffold()
            .expect("handshake scaffold should initialize");
        assert_eq!(
            state.stage,
            FirstSimulatorHandshakeStage::BootstrapPrerequisitesReady
        );
        assert_eq!(state.prerequisites.target.sim_ip, "127.0.0.1");
        assert_eq!(state.prerequisites.target.region_x, 1000);

        let state = connection
            .advance_first_simulator_handshake_scaffold(
                FirstSimulatorHandshakeStage::FirstRegionTargetKnown,
            )
            .expect("stage should advance to first region target known");
        assert_eq!(
            state.stage,
            FirstSimulatorHandshakeStage::FirstRegionTargetKnown
        );

        let state = connection
            .advance_first_simulator_handshake_scaffold(
                FirstSimulatorHandshakeStage::UseCircuitCode,
            )
            .expect("stage should advance to use circuit code");
        assert_eq!(state.stage, FirstSimulatorHandshakeStage::UseCircuitCode);

        let state = connection
            .advance_first_simulator_handshake_scaffold(
                FirstSimulatorHandshakeStage::CompleteAgentMovement,
            )
            .expect("stage should advance to complete agent movement");
        assert_eq!(
            state.stage,
            FirstSimulatorHandshakeStage::CompleteAgentMovement
        );

        let state = connection
            .advance_first_simulator_handshake_scaffold(
                FirstSimulatorHandshakeStage::WaitingForAgentMovementComplete,
            )
            .expect("stage should advance to waiting for movement complete");
        assert_eq!(
            state.stage,
            FirstSimulatorHandshakeStage::WaitingForAgentMovementComplete
        );
    }

    #[tokio::test]
    async fn first_simulator_handshake_scaffold_rejects_out_of_order_transition() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": 13000,
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })))
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        let adapter = SecondLifeAdapter;
        let result = connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");
        assert!(matches!(result, GridLoginResult::Success(_)));

        connection
            .begin_first_simulator_handshake_scaffold()
            .expect("handshake scaffold should initialize");
        let err = connection
            .advance_first_simulator_handshake_scaffold(
                FirstSimulatorHandshakeStage::CompleteAgentMovement,
            )
            .expect_err("out-of-order transition should fail");
        match err {
            ConnectionError::InvalidFirstSimulatorHandshakeTransition { from, to } => {
                assert_eq!(
                    from,
                    FirstSimulatorHandshakeStage::BootstrapPrerequisitesReady
                );
                assert_eq!(to, FirstSimulatorHandshakeStage::CompleteAgentMovement);
            }
            other => panic!("expected InvalidFirstSimulatorHandshakeTransition, got {other:?}"),
        }
    }

    #[test]
    fn first_simulator_handshake_scaffold_requires_logged_in_state() {
        let mut connection = Connection::new(ConnectionConfig::default());
        let err = connection
            .begin_first_simulator_handshake_scaffold()
            .expect_err("non-logged-in handshake init should fail");
        match err {
            ConnectionError::InvalidState(ConnectionState::Disconnected) => {}
            other => panic!("expected InvalidState(Disconnected), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn send_first_simulator_use_circuit_code_sends_datagram_and_advances_stage() {
        let listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("listener bind should succeed");
        let listener_addr = listener
            .local_addr()
            .expect("listener address should exist");

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": listener_addr.port(),
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })))
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        let adapter = SecondLifeAdapter;
        let result = connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");
        assert!(matches!(result, GridLoginResult::Success(_)));
        connection
            .begin_first_simulator_handshake_scaffold()
            .expect("scaffold should initialize");

        let state = connection
            .send_first_simulator_use_circuit_code()
            .await
            .expect("use circuit code send should succeed");
        assert_eq!(state.stage, FirstSimulatorHandshakeStage::UseCircuitCode);

        let mut buf = [0u8; 1024];
        let (received_len, _) = timeout(Duration::from_secs(1), listener.recv_from(&mut buf))
            .await
            .expect("datagram receive should not timeout")
            .expect("datagram receive should succeed");
        let (flags, packet_id, message_number, body) =
            decode_outbound_handshake_payload(&buf[..received_len]);
        assert_eq!(flags & LLUDP_RELIABLE_FLAG, LLUDP_RELIABLE_FLAG);
        assert_eq!(
            message_number,
            lludp_low_frequency_message_number(LLUDP_USE_CIRCUIT_CODE_LOW_ID)
        );
        assert_eq!(body.len(), 36);
        assert_eq!(
            u32::from_le_bytes(body[0..4].try_into().expect("code bytes")),
            424242
        );

        let diagnostics = connection.first_simulator_handshake_send_diagnostics();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].action,
            FirstSimulatorHandshakeAction::UseCircuitCode
        );
        assert_eq!(diagnostics[0].packet_id, packet_id);
        assert_eq!(diagnostics[0].packet_message_number, Some(message_number));
        assert!(diagnostics[0].success);
    }

    #[tokio::test]
    async fn send_first_simulator_complete_agent_movement_sends_datagram_and_waits_for_movement_complete()
     {
        let listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("listener bind should succeed");
        let listener_addr = listener
            .local_addr()
            .expect("listener address should exist");

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": listener_addr.port(),
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })))
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        let adapter = SecondLifeAdapter;
        connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");
        connection
            .begin_first_simulator_handshake_scaffold()
            .expect("scaffold should initialize");
        connection
            .send_first_simulator_use_circuit_code()
            .await
            .expect("use circuit code send should succeed");

        let mut buf = [0u8; 1024];
        timeout(Duration::from_secs(1), listener.recv_from(&mut buf))
            .await
            .expect("first datagram should not timeout")
            .expect("first datagram receive should succeed");

        let state = connection
            .send_first_simulator_complete_agent_movement()
            .await
            .expect("complete agent movement send should succeed");
        assert_eq!(
            state.stage,
            FirstSimulatorHandshakeStage::WaitingForAgentMovementComplete
        );

        let (received_len, _) = timeout(Duration::from_secs(1), listener.recv_from(&mut buf))
            .await
            .expect("second datagram should not timeout")
            .expect("second datagram receive should succeed");
        let (flags, packet_id, message_number, body) =
            decode_outbound_handshake_payload(&buf[..received_len]);
        assert_eq!(flags & LLUDP_RELIABLE_FLAG, LLUDP_RELIABLE_FLAG);
        assert_eq!(
            message_number,
            lludp_low_frequency_message_number(LLUDP_COMPLETE_AGENT_MOVEMENT_LOW_ID)
        );
        assert_eq!(body.len(), 36);
        assert_eq!(
            u32::from_le_bytes(body[32..36].try_into().expect("circuit bytes")),
            424242
        );

        let diagnostics = connection.first_simulator_handshake_send_diagnostics();
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(
            diagnostics[1].action,
            FirstSimulatorHandshakeAction::CompleteAgentMovement
        );
        assert_eq!(diagnostics[1].packet_id, packet_id);
        assert_eq!(diagnostics[1].packet_message_number, Some(message_number));
        assert!(diagnostics[1].success);
    }

    #[tokio::test]
    async fn send_actions_enforce_ordered_handshake_flow() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": 15000,
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })))
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        let adapter = SecondLifeAdapter;
        connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");
        connection
            .begin_first_simulator_handshake_scaffold()
            .expect("scaffold should initialize");

        let err = connection
            .send_first_simulator_complete_agent_movement()
            .await
            .expect_err("complete agent movement should fail before use circuit code");
        match err {
            ConnectionError::InvalidFirstSimulatorHandshakeTransition { from, to } => {
                assert_eq!(
                    from,
                    FirstSimulatorHandshakeStage::BootstrapPrerequisitesReady
                );
                assert_eq!(to, FirstSimulatorHandshakeStage::CompleteAgentMovement);
            }
            other => panic!("expected InvalidFirstSimulatorHandshakeTransition, got {other:?}"),
        }
    }

    #[test]
    fn observe_first_simulator_inbound_payload_classifies_message_kinds() {
        let movement = classify_first_simulator_inbound_message(&make_low_frequency_packet(250));
        assert_eq!(
            movement.kind,
            FirstSimulatorInboundMessageKind::AgentMovementComplete
        );
        assert_eq!(
            movement.scope,
            FirstSimulatorInboundTrafficScope::BootstrapRelevant
        );
        assert_eq!(movement.signal, "packet:0xffff00fa");
        assert_eq!(
            movement.decode_source,
            FirstSimulatorInboundDecodeSource::PacketMessageNumber
        );
        assert_eq!(movement.packet_message_number, Some(0xffff00fa));

        let test_message = classify_first_simulator_inbound_message(&make_low_frequency_packet(1));
        assert_eq!(
            test_message.kind,
            FirstSimulatorInboundMessageKind::TestMessage
        );
        assert_eq!(
            test_message.scope,
            FirstSimulatorInboundTrafficScope::TransportControl
        );
        assert_eq!(test_message.signal, "packet:0xffff0001");
        assert_eq!(
            test_message.decode_source,
            FirstSimulatorInboundDecodeSource::PacketMessageNumber
        );
        assert_eq!(test_message.packet_message_number, Some(0xffff0001));

        let region = classify_first_simulator_inbound_message(&make_low_frequency_packet(148));
        assert_eq!(
            region.kind,
            FirstSimulatorInboundMessageKind::RegionHandshake
        );
        assert_eq!(
            region.scope,
            FirstSimulatorInboundTrafficScope::BootstrapRelevant
        );
        assert_eq!(region.signal, "packet:0xffff0094");
        assert_eq!(
            region.decode_source,
            FirstSimulatorInboundDecodeSource::PacketMessageNumber
        );
        assert_eq!(region.packet_message_number, Some(0xffff0094));

        let region_reply =
            classify_first_simulator_inbound_message(&make_low_frequency_packet(149));
        assert_eq!(
            region_reply.kind,
            FirstSimulatorInboundMessageKind::RegionHandshakeReply
        );
        assert_eq!(
            region_reply.scope,
            FirstSimulatorInboundTrafficScope::BootstrapRelevant
        );
        assert_eq!(region_reply.signal, "packet:0xffff0095");
        assert_eq!(region_reply.packet_message_number, Some(0xffff0095));

        let health = classify_first_simulator_inbound_message(&make_low_frequency_packet(138));
        assert_eq!(health.kind, FirstSimulatorInboundMessageKind::HealthMessage);
        assert_eq!(
            health.scope,
            FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic
        );
        assert_eq!(health.signal, "packet:0xffff008a");
        assert_eq!(
            health.decode_source,
            FirstSimulatorInboundDecodeSource::PacketMessageNumber
        );
        assert_eq!(health.packet_message_number, Some(0xffff008a));

        let sim_time = classify_first_simulator_inbound_message(&make_low_frequency_packet(150));
        assert_eq!(
            sim_time.kind,
            FirstSimulatorInboundMessageKind::SimulatorViewerTimeMessage
        );
        assert_eq!(
            sim_time.scope,
            FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic
        );
        assert_eq!(sim_time.signal, "packet:0xffff0096");
        assert_eq!(
            sim_time.decode_source,
            FirstSimulatorInboundDecodeSource::PacketMessageNumber
        );
        assert_eq!(sim_time.packet_message_number, Some(0xffff0096));
        assert_eq!(
            to_early_simulator_traffic_kind(sim_time.kind),
            Some(EarlySimulatorTrafficKind::SimulatorViewerTimeMessage)
        );

        let enable = classify_first_simulator_inbound_message(&make_low_frequency_packet(151));
        assert_eq!(
            enable.kind,
            FirstSimulatorInboundMessageKind::EnableSimulator
        );
        assert_eq!(
            enable.scope,
            FirstSimulatorInboundTrafficScope::BootstrapRelevant
        );
        assert_eq!(enable.signal, "packet:0xffff0097");
        assert_eq!(
            enable.decode_source,
            FirstSimulatorInboundDecodeSource::PacketMessageNumber
        );
        assert_eq!(enable.packet_message_number, Some(0xffff0097));

        let agent_data = classify_first_simulator_inbound_message(&make_low_frequency_packet(387));
        assert_eq!(
            agent_data.kind,
            FirstSimulatorInboundMessageKind::AgentDataUpdate
        );
        assert_eq!(
            agent_data.scope,
            FirstSimulatorInboundTrafficScope::BootstrapRelevant
        );
        assert_eq!(agent_data.signal, "packet:0xffff0183");
        assert_eq!(
            agent_data.decode_source,
            FirstSimulatorInboundDecodeSource::PacketMessageNumber
        );
        assert_eq!(agent_data.packet_message_number, Some(0xffff0183));

        let packet_ack =
            classify_first_simulator_inbound_message(&make_low_frequency_packet(0xFFFB));
        assert_eq!(packet_ack.kind, FirstSimulatorInboundMessageKind::PacketAck);
        assert_eq!(
            packet_ack.scope,
            FirstSimulatorInboundTrafficScope::TransportControl
        );
        assert_eq!(packet_ack.signal, "packet:0xfffffffb");
        assert_eq!(packet_ack.packet_message_number, Some(0xfffffffb));
        assert_eq!(to_early_simulator_traffic_kind(packet_ack.kind), None);

        let online_notification =
            classify_first_simulator_inbound_message(&make_low_frequency_packet(322));
        assert_eq!(
            online_notification.kind,
            FirstSimulatorInboundMessageKind::OnlineNotification
        );
        assert_eq!(
            online_notification.scope,
            FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic
        );
        assert_eq!(online_notification.signal, "packet:0xffff0142");
        assert_eq!(online_notification.packet_message_number, Some(0xffff0142));

        let viewer_effect =
            classify_first_simulator_inbound_message(&make_medium_frequency_packet(17));
        assert_eq!(
            viewer_effect.kind,
            FirstSimulatorInboundMessageKind::ViewerEffect
        );
        assert_eq!(
            viewer_effect.scope,
            FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic
        );
        assert_eq!(viewer_effect.signal, "packet:0x0000ff11");
        assert_eq!(viewer_effect.packet_message_number, Some(0x0000ff11));

        let layer_data = classify_first_simulator_inbound_message(&make_high_frequency_packet(11));
        assert_eq!(layer_data.kind, FirstSimulatorInboundMessageKind::LayerData);
        assert_eq!(
            layer_data.scope,
            FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic
        );
        assert_eq!(layer_data.signal, "packet:0x0000000b");
        assert_eq!(layer_data.packet_message_number, Some(0x0000000b));
        assert_eq!(
            to_early_simulator_traffic_kind(layer_data.kind),
            Some(EarlySimulatorTrafficKind::LayerData)
        );

        let coarse_location_update =
            classify_first_simulator_inbound_message(&make_medium_frequency_packet(6));
        assert_eq!(
            coarse_location_update.kind,
            FirstSimulatorInboundMessageKind::CoarseLocationUpdate
        );
        assert_eq!(
            coarse_location_update.scope,
            FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic
        );
        assert_eq!(coarse_location_update.signal, "packet:0x0000ff06");
        assert_eq!(
            coarse_location_update.packet_message_number,
            Some(0x0000ff06)
        );

        let attached_sound =
            classify_first_simulator_inbound_message(&make_medium_frequency_packet(13));
        assert_eq!(
            attached_sound.kind,
            FirstSimulatorInboundMessageKind::AttachedSound
        );
        assert_eq!(
            attached_sound.scope,
            FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic
        );
        assert_eq!(attached_sound.signal, "packet:0x0000ff0d");
        assert_eq!(attached_sound.packet_message_number, Some(0x0000ff0d));
        assert_eq!(
            to_early_simulator_traffic_kind(attached_sound.kind),
            Some(EarlySimulatorTrafficKind::AttachedSound)
        );
        assert_eq!(to_region_transition_control_kind(attached_sound.kind), None);

        let crossed_region =
            classify_first_simulator_inbound_message(&make_medium_frequency_packet(7));
        assert_eq!(
            crossed_region.kind,
            FirstSimulatorInboundMessageKind::CrossedRegion
        );
        assert_eq!(
            crossed_region.scope,
            FirstSimulatorInboundTrafficScope::RegionTransitionControl
        );
        assert_eq!(crossed_region.signal, "packet:0x0000ff07");
        assert_eq!(crossed_region.packet_message_number, Some(0x0000ff07));
        assert_eq!(to_early_simulator_traffic_kind(crossed_region.kind), None);
        assert_eq!(
            to_region_transition_control_kind(crossed_region.kind),
            Some(RegionTransitionControlKind::CrossedRegion)
        );

        let confirm_enable =
            classify_first_simulator_inbound_message(&make_medium_frequency_packet(8));
        assert_eq!(
            confirm_enable.kind,
            FirstSimulatorInboundMessageKind::ConfirmEnableSimulator
        );
        assert_eq!(
            confirm_enable.scope,
            FirstSimulatorInboundTrafficScope::RegionTransitionControl
        );
        assert_eq!(confirm_enable.signal, "packet:0x0000ff08");
        assert_eq!(confirm_enable.packet_message_number, Some(0x0000ff08));
        assert_eq!(to_early_simulator_traffic_kind(confirm_enable.kind), None);
        assert_eq!(
            to_region_transition_control_kind(confirm_enable.kind),
            Some(RegionTransitionControlKind::ConfirmEnableSimulator)
        );

        let camera_constraint =
            classify_first_simulator_inbound_message(&make_high_frequency_packet(22));
        assert_eq!(
            camera_constraint.kind,
            FirstSimulatorInboundMessageKind::CameraConstraint
        );
        assert_eq!(
            camera_constraint.scope,
            FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic
        );
        assert_eq!(camera_constraint.signal, "packet:0x00000016");
        assert_eq!(camera_constraint.packet_message_number, Some(0x00000016));

        let generic_message =
            classify_first_simulator_inbound_message(&make_low_frequency_packet(261));
        assert_eq!(
            generic_message.kind,
            FirstSimulatorInboundMessageKind::GenericMessage
        );
        assert_eq!(
            generic_message.scope,
            FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic
        );
        assert_eq!(generic_message.signal, "packet:0xffff0105");
        assert_eq!(generic_message.packet_message_number, Some(0xffff0105));

        let json_fallback =
            classify_first_simulator_inbound_message(br#"{"message":"AgentMovementComplete"}"#);
        assert_eq!(
            json_fallback.kind,
            FirstSimulatorInboundMessageKind::AgentMovementComplete
        );
        assert_eq!(
            json_fallback.scope,
            FirstSimulatorInboundTrafficScope::BootstrapRelevant
        );
        assert_eq!(json_fallback.signal, "json:message:AgentMovementComplete");
        assert_eq!(
            json_fallback.decode_source,
            FirstSimulatorInboundDecodeSource::JsonField
        );
        assert_eq!(json_fallback.packet_message_number, None);

        let irrelevant = classify_first_simulator_inbound_message(b"totally unrelated");
        assert_eq!(
            irrelevant.kind,
            FirstSimulatorInboundMessageKind::Irrelevant
        );
        assert_eq!(irrelevant.scope, FirstSimulatorInboundTrafficScope::Unknown);
        assert_eq!(
            irrelevant.decode_source,
            FirstSimulatorInboundDecodeSource::Unknown
        );
        assert_eq!(irrelevant.packet_message_number, None);
        assert_eq!(irrelevant.signal, "text:none");

        let unknown_packet =
            classify_first_simulator_inbound_message(&make_low_frequency_packet(42));
        assert_eq!(
            unknown_packet.kind,
            FirstSimulatorInboundMessageKind::Irrelevant
        );
        assert_eq!(
            unknown_packet.scope,
            FirstSimulatorInboundTrafficScope::Unknown
        );
        assert_eq!(
            unknown_packet.decode_source,
            FirstSimulatorInboundDecodeSource::Unknown
        );
        assert_eq!(unknown_packet.signal, "packet:0xffff002a:unmapped");
        assert_eq!(unknown_packet.packet_message_number, Some(0xffff002a));
    }

    #[test]
    fn first_simulator_message_label_maps_use_circuit_code() {
        let label = first_simulator_message_label(lludp_low_frequency_message_number(
            LLUDP_USE_CIRCUIT_CODE_LOW_ID,
        ));
        assert_eq!(label, "UseCircuitCode(0xffff0003)");
    }

    #[test]
    fn decode_first_simulator_packet_header_strips_ack_trailer_from_body() {
        let payload = append_lludp_ack_trailer(
            &mark_packet_reliable(make_low_frequency_packet_with_body(138, &[1, 2, 3, 4])),
            &[0x01020304, 0x05060708],
        );

        let header =
            decode_first_simulator_packet_header(&payload).expect("packet header should decode");

        assert_eq!(header.flags & LLUDP_ACK_FLAG, LLUDP_ACK_FLAG);
        assert_eq!(header.packet_id, 1);
        assert_eq!(
            header.message_number,
            lludp_low_frequency_message_number(LLUDP_HEALTH_MESSAGE_LOW_ID)
        );
        assert_eq!(header.body(&payload), Some(&[1, 2, 3, 4][..]));
    }

    #[test]
    fn observe_first_simulator_inbound_payload_queues_reliable_ack_ids() {
        let mut connection = Connection::new(ConnectionConfig::default());
        connection.state = ConnectionState::LoggedIn;

        let reliable_packet = mark_packet_reliable(make_low_frequency_packet(250));
        connection
            .observe_first_simulator_inbound_payload(&reliable_packet)
            .expect("reliable packet should be observed");

        assert_eq!(connection.pending_first_simulator_ack_ids, vec![1]);

        let packet_ack = mark_packet_reliable(make_low_frequency_packet(LLUDP_PACKET_ACK_LOW_ID));
        connection
            .observe_first_simulator_inbound_payload(&packet_ack)
            .expect("packet ack should be observed");

        assert_eq!(connection.pending_first_simulator_ack_ids, vec![1]);
    }

    #[test]
    fn decode_coarse_location_update_extracts_count_and_first_location() {
        let payload =
            make_medium_frequency_packet_with_body(6, &[3, 10, 20, 8, 30, 40, 9, 60, 70, 11]);
        let decoded = decode_coarse_location_update(&payload)
            .expect("coarse location update should decode from medium packet body");
        assert_eq!(decoded.location_count, 3);
        assert_eq!(decoded.first_location, Some([10, 20, 8]));
        assert_eq!(decoded.second_location, Some([30, 40, 9]));
        assert_eq!(decoded.third_location, Some([60, 70, 11]));
    }

    #[test]
    fn decode_health_message_extracts_health_scalar() {
        let mut payload = make_low_frequency_packet(138);
        payload.extend_from_slice(&0.73f32.to_le_bytes());
        let decoded =
            decode_health_message(&payload).expect("health message should decode from packet body");
        assert!((decoded.health - 0.73).abs() < f32::EPSILON);
        assert_eq!(health_to_basis_points(decoded.health), 7300);
    }

    #[test]
    fn decode_simulator_viewer_time_message_extracts_body_signature() {
        let mut payload = make_low_frequency_packet(150);
        payload.extend_from_slice(&[0x11, 0x22, 0x33, 0x44, 0x99, 0x88]);
        let decoded = decode_simulator_viewer_time_message(&payload)
            .expect("simulator viewer time message should decode from packet body");
        assert_eq!(decoded.body_len, 6);
        assert_eq!(decoded.signature, Some(0x44332211));
    }

    #[test]
    fn decode_kill_object_local_ids_extracts_local_ids() {
        let mut body = Vec::new();
        body.push(2); // block count
        body.extend_from_slice(&123u32.to_le_bytes());
        body.extend_from_slice(&456u32.to_le_bytes());
        let payload = make_high_frequency_packet_with_body(LLUDP_KILL_OBJECT_HIGH_ID, &body);
        let ids = decode_kill_object_local_ids(&payload).expect("kill object should decode");
        assert_eq!(ids, vec![123, 456]);
    }

    #[test]
    fn decode_improved_terse_object_update_extracts_local_ids() {
        let mut body = Vec::new();
        body.extend_from_slice(&1u64.to_le_bytes()); // RegionHandle
        body.extend_from_slice(&0u16.to_le_bytes()); // TimeDilation
        body.push(2); // object count
        for (id, pos_x) in [(42u32, 12.0f32), (99u32, 18.0f32)] {
            body.push(18); // Data len (var 1): local + state/attach + avatar flag + position
            body.extend_from_slice(&id.to_le_bytes());
            body.push(0); // state/attachment
            body.push(0); // avatar flag
            body.extend_from_slice(&pos_x.to_le_bytes()); // position x
            body.extend_from_slice(&8.0f32.to_le_bytes()); // position y
            body.extend_from_slice(&2.0f32.to_le_bytes()); // position z
            body.extend_from_slice(&0u16.to_le_bytes()); // TextureEntry len (var 2)
        }
        let payload =
            make_high_frequency_packet_with_body(LLUDP_IMPROVED_TERSE_OBJECT_UPDATE_HIGH_ID, &body);
        let objects =
            decode_improved_terse_object_update_objects(&payload).expect("terse should decode");
        assert_eq!(
            objects.iter().map(|obj| obj.local_id).collect::<Vec<_>>(),
            vec![42, 99]
        );
        assert_eq!(objects[0].position_centi, Some([1200, 800, 200]));
    }

    #[test]
    fn decode_object_update_cached_extracts_local_ids() {
        let mut body = Vec::new();
        body.extend_from_slice(&1u64.to_le_bytes()); // RegionHandle
        body.extend_from_slice(&0u16.to_le_bytes()); // TimeDilation
        body.push(2); // object count
        for id in [7u32, 8u32] {
            body.extend_from_slice(&id.to_le_bytes());
            body.extend_from_slice(&0u32.to_le_bytes()); // CRC
            body.extend_from_slice(&0u32.to_le_bytes()); // UpdateFlags
        }
        let payload =
            make_high_frequency_packet_with_body(LLUDP_OBJECT_UPDATE_CACHED_HIGH_ID, &body);
        let ids =
            decode_object_update_cached_local_ids(&payload).expect("cached update should decode");
        assert_eq!(ids, vec![7, 8]);
    }

    #[test]
    fn observe_object_update_cached_queues_cache_miss_ids() {
        let mut body = Vec::new();
        body.extend_from_slice(&1u64.to_le_bytes()); // RegionHandle
        body.extend_from_slice(&0u16.to_le_bytes()); // TimeDilation
        body.push(1); // object count
        body.extend_from_slice(&0x11223344u32.to_le_bytes()); // ID
        body.extend_from_slice(&0u32.to_le_bytes()); // CRC
        body.extend_from_slice(&0u32.to_le_bytes()); // UpdateFlags
        let payload =
            make_high_frequency_packet_with_body(LLUDP_OBJECT_UPDATE_CACHED_HIGH_ID, &body);

        let mut connection = Connection::new(ConnectionConfig::default());
        connection.state = ConnectionState::LoggedIn;
        connection
            .observe_first_simulator_inbound_payload(&payload)
            .expect("cached update should be observed");

        assert_eq!(connection.pending_object_cache_miss_ids, vec![0x11223344]);
        assert!(connection.object_feed_objects.contains_key(&0x11223344));
    }

    #[test]
    fn decode_object_update_compressed_extracts_local_ids() {
        let mut body = Vec::new();
        body.extend_from_slice(&1u64.to_le_bytes()); // RegionHandle
        body.extend_from_slice(&0u16.to_le_bytes()); // TimeDilation
        body.push(1); // object count
        body.extend_from_slice(&0u32.to_le_bytes()); // UpdateFlags
        let mut data = vec![0u8; 16]; // UUID
        data.extend_from_slice(&55u32.to_le_bytes()); // LocalID
        data.push(9); // PCode
        data.push(0); // State
        data.extend_from_slice(&0u32.to_le_bytes()); // CRC
        data.push(0); // Material
        data.push(0); // ClickAction
        data.extend_from_slice(&[0, 0, 128, 63]); // Scale X = 1.0
        data.extend_from_slice(&[0, 0, 128, 63]); // Scale Y = 1.0
        data.extend_from_slice(&[0, 0, 128, 63]); // Scale Z = 1.0
        data.extend_from_slice(&[0, 0, 0, 65]); // Pos X = 8.0
        data.extend_from_slice(&[0, 0, 128, 65]); // Pos Y = 16.0
        data.extend_from_slice(&[0, 0, 192, 64]); // Pos Z = 6.0
        data.extend_from_slice(&[0, 0, 0, 0]); // Rot X
        data.extend_from_slice(&[0, 0, 0, 0]); // Rot Y
        data.extend_from_slice(&[0, 0, 0, 0]); // Rot Z
        data.extend_from_slice(&0u32.to_le_bytes()); // CompressedFlags
        data.extend_from_slice(&[0u8; 16]); // OwnerID
        data.push(0); // ExtraParams count
        let data_len = u16::try_from(data.len()).expect("len fits u16");
        body.extend_from_slice(&data_len.to_le_bytes());
        body.extend_from_slice(&data);
        let payload =
            make_high_frequency_packet_with_body(LLUDP_OBJECT_UPDATE_COMPRESSED_HIGH_ID, &body);
        let objects = decode_object_update_compressed_objects(&payload)
            .expect("compressed update should decode");
        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].local_id, 55);
        assert_eq!(objects[0].position_centi, Some([800, 1600, 600]));
        assert_eq!(objects[0].mesh_id_bytes, None);
    }

    #[test]
    fn decode_object_update_compressed_extracts_mesh_id_from_extra_params() {
        let mesh_id = parse_uuid_bytes("10930d3b-1821-c584-a0c7-28a34999800d").expect("uuid bytes");

        let mut body = Vec::new();
        body.extend_from_slice(&1u64.to_le_bytes()); // RegionHandle
        body.extend_from_slice(&0u16.to_le_bytes()); // TimeDilation
        body.push(1); // object count
        body.extend_from_slice(&0u32.to_le_bytes()); // UpdateFlags
        let mut data = vec![0u8; 16]; // UUID
        data.extend_from_slice(&55u32.to_le_bytes()); // LocalID
        data.push(9); // PCode
        data.push(0); // State
        data.extend_from_slice(&0u32.to_le_bytes()); // CRC
        data.push(0); // Material
        data.push(0); // ClickAction
        data.extend_from_slice(&[0, 0, 128, 63]); // Scale X = 1.0
        data.extend_from_slice(&[0, 0, 128, 63]); // Scale Y = 1.0
        data.extend_from_slice(&[0, 0, 128, 63]); // Scale Z = 1.0
        data.extend_from_slice(&[0, 0, 0, 65]); // Pos X = 8.0
        data.extend_from_slice(&[0, 0, 128, 65]); // Pos Y = 16.0
        data.extend_from_slice(&[0, 0, 192, 64]); // Pos Z = 6.0
        data.extend_from_slice(&[0, 0, 0, 0]); // Rot X
        data.extend_from_slice(&[0, 0, 0, 0]); // Rot Y
        data.extend_from_slice(&[0, 0, 0, 0]); // Rot Z
        data.extend_from_slice(&0u32.to_le_bytes()); // CompressedFlags
        data.extend_from_slice(&[0u8; 16]); // OwnerID

        // ExtraParams: one sculpt entry carrying a mesh sculpt UUID.
        data.push(1); // count
        data.extend_from_slice(&EXTRA_PARAM_SCULPT_EP.to_le_bytes());
        data.extend_from_slice(&17u32.to_le_bytes());
        data.extend_from_slice(&mesh_id);
        data.push(SCULPT_TYPE_MESH);

        let data_len = u16::try_from(data.len()).expect("len fits u16");
        body.extend_from_slice(&data_len.to_le_bytes());
        body.extend_from_slice(&data);
        let payload =
            make_high_frequency_packet_with_body(LLUDP_OBJECT_UPDATE_COMPRESSED_HIGH_ID, &body);
        let objects = decode_object_update_compressed_objects(&payload)
            .expect("compressed update should decode");
        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].mesh_id_bytes, Some(mesh_id));
    }

    #[test]
    fn decode_object_extra_params_extracts_mesh_id_updates() {
        let mesh_id = parse_uuid_bytes("10930d3b-1821-c584-a0c7-28a34999800d").expect("uuid bytes");
        let local_id = 77u32;

        let mut payload = make_low_frequency_packet(LLUDP_OBJECT_EXTRA_PARAMS_LOW_ID);
        payload.extend_from_slice(&[0u8; 16]); // AgentID
        payload.extend_from_slice(&[0u8; 16]); // SessionID
        payload.push(1); // ObjectData count
        payload.extend_from_slice(&local_id.to_le_bytes());
        payload.extend_from_slice(&EXTRA_PARAM_SCULPT_EP.to_le_bytes());
        payload.push(1); // ParamInUse
        payload.extend_from_slice(&17u32.to_le_bytes()); // ParamSize
        payload.push(17); // ParamData len
        payload.extend_from_slice(&mesh_id);
        payload.push(SCULPT_TYPE_MESH);

        let decoded = decode_object_extra_params_mesh_updates(&payload)
            .expect("object extra params should decode");
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].local_id, local_id);
        assert_eq!(decoded[0].mesh_id_bytes, Some(mesh_id));
    }

    #[test]
    fn observe_object_extra_params_updates_object_feed_mesh_id() {
        let mesh_id = parse_uuid_bytes("10930d3b-1821-c584-a0c7-28a34999800d").expect("uuid bytes");
        let local_id = 77u32;

        let mut connection = Connection::new(ConnectionConfig::default());
        connection.state = ConnectionState::LoggedIn;

        let mut payload = make_low_frequency_packet(LLUDP_OBJECT_EXTRA_PARAMS_LOW_ID);
        payload.extend_from_slice(&[0u8; 16]); // AgentID
        payload.extend_from_slice(&[0u8; 16]); // SessionID
        payload.push(1); // ObjectData count
        payload.extend_from_slice(&local_id.to_le_bytes());
        payload.extend_from_slice(&EXTRA_PARAM_SCULPT_EP.to_le_bytes());
        payload.push(1); // ParamInUse
        payload.extend_from_slice(&17u32.to_le_bytes()); // ParamSize
        payload.push(17); // ParamData len
        payload.extend_from_slice(&mesh_id);
        payload.push(SCULPT_TYPE_MESH);

        connection
            .observe_first_simulator_inbound_payload(&payload)
            .expect("object extra params should be observed");

        let summary = connection.simulator_payload_decode_summary();
        assert_eq!(summary.object_feed_total_objects, 1);
        assert_eq!(summary.object_feed_update_messages, 1);
        assert_eq!(summary.object_feed_objects[0].local_id, local_id);
        assert_eq!(
            summary.object_feed_objects[0].mesh_id.as_deref(),
            Some("10930d3b-1821-c584-a0c7-28a34999800d")
        );
    }

    #[test]
    fn decode_object_update_allows_empty_object_list() {
        // ObjectUpdate is zerocoded on the wire; this body is the zerocoded encoding of:
        // RegionHandle=1 (LE), TimeDilation=0 (LE), ObjectData count=0.
        let zerocoded_body = vec![1, 0, 7, 0, 2, 0, 1];
        let payload =
            make_high_frequency_packet_with_body(LLUDP_OBJECT_UPDATE_HIGH_ID, &zerocoded_body);
        let decoded = decode_object_update_ids_and_scales(&payload)
            .expect("object update should decode with empty list");
        assert!(decoded.is_empty());
    }

    #[test]
    fn object_feed_export_includes_object_id_when_known() {
        let mut connection = Connection::new(ConnectionConfig::default());
        let object_id_bytes =
            parse_uuid_bytes("10930d3b-1821-c584-a0c7-28a34999800d").expect("uuid bytes");
        connection.object_feed_upsert(
            77,
            Some([100, 120, 80]),
            None,
            None,
            None,
            None,
            &[],
            Some(object_id_bytes),
        );
        connection.refresh_object_feed_summary_export();

        let summary = connection.simulator_payload_decode_summary();
        assert_eq!(summary.object_feed_total_objects, 1);
        assert_eq!(summary.object_feed_objects.len(), 1);
        assert_eq!(
            summary.object_feed_objects[0].object_id.as_deref(),
            Some("10930d3b-1821-c584-a0c7-28a34999800d")
        );
    }

    #[test]
    fn decode_mesh_asset_id_from_extra_params_extracts_sculpt_mesh_uuid() {
        let mesh_id = parse_uuid_bytes("10930d3b-1821-c584-a0c7-28a34999800d").expect("uuid bytes");
        let mut extra = Vec::new();
        extra.push(1); // count
        extra.extend_from_slice(&EXTRA_PARAM_SCULPT_EP.to_le_bytes());
        extra.extend_from_slice(&17u32.to_le_bytes());
        extra.extend_from_slice(&mesh_id);
        extra.push(SCULPT_TYPE_MESH);

        let decoded =
            decode_mesh_asset_id_from_extra_params(&extra).expect("mesh id should decode");
        assert_eq!(decoded, mesh_id);
    }

    #[test]
    fn decode_mesh_asset_id_from_extra_params_tolerates_trailing_bytes() {
        let mesh_id = parse_uuid_bytes("10930d3b-1821-c584-a0c7-28a34999800d").expect("uuid bytes");
        let mut extra = Vec::new();
        extra.push(1); // count
        extra.extend_from_slice(&EXTRA_PARAM_SCULPT_EP.to_le_bytes());
        extra.extend_from_slice(&17u32.to_le_bytes());
        extra.extend_from_slice(&mesh_id);
        extra.push(SCULPT_TYPE_MESH);
        extra.extend_from_slice(&[0xaa, 0xbb]); // trailing garbage should not invalidate decode

        let decoded = decode_mesh_asset_id_from_extra_params(&extra)
            .expect("mesh id should decode with trailing bytes");
        assert_eq!(decoded, mesh_id);
    }

    #[test]
    fn decode_object_update_compressed_tolerates_mixed_valid_and_malformed_blocks() {
        let mut body = Vec::new();
        body.extend_from_slice(&1u64.to_le_bytes()); // RegionHandle
        body.extend_from_slice(&0u16.to_le_bytes()); // TimeDilation
        body.push(2); // object count

        // Block 1: valid compressed object with mesh extra params.
        body.extend_from_slice(&0u32.to_le_bytes()); // UpdateFlags
        let mesh_id = parse_uuid_bytes("10930d3b-1821-c584-a0c7-28a34999800d").expect("uuid bytes");
        let mut valid = vec![0u8; 16]; // FullID
        valid.extend_from_slice(&55u32.to_le_bytes()); // LocalID
        valid.push(9); // PCode
        valid.push(0); // State
        valid.extend_from_slice(&0u32.to_le_bytes()); // CRC
        valid.push(0); // Material
        valid.push(0); // ClickAction
        valid.extend_from_slice(&[0, 0, 128, 63]); // Scale X
        valid.extend_from_slice(&[0, 0, 128, 63]); // Scale Y
        valid.extend_from_slice(&[0, 0, 128, 63]); // Scale Z
        valid.extend_from_slice(&[0, 0, 0, 65]); // Pos X
        valid.extend_from_slice(&[0, 0, 128, 65]); // Pos Y
        valid.extend_from_slice(&[0, 0, 192, 64]); // Pos Z
        valid.extend_from_slice(&[0, 0, 0, 0]); // Rot X
        valid.extend_from_slice(&[0, 0, 0, 0]); // Rot Y
        valid.extend_from_slice(&[0, 0, 0, 0]); // Rot Z
        valid.extend_from_slice(&0u32.to_le_bytes()); // CompressedFlags
        valid.extend_from_slice(&[0u8; 16]); // OwnerID
        valid.push(1); // ExtraParams count
        valid.extend_from_slice(&EXTRA_PARAM_SCULPT_EP.to_le_bytes());
        valid.extend_from_slice(&17u32.to_le_bytes());
        valid.extend_from_slice(&mesh_id);
        valid.push(SCULPT_TYPE_MESH);
        let valid_len = u16::try_from(valid.len()).expect("valid block len fits u16");
        body.extend_from_slice(&valid_len.to_le_bytes());
        body.extend_from_slice(&valid);

        // Block 2: malformed declared length that truncates at body end.
        body.extend_from_slice(&0u32.to_le_bytes()); // UpdateFlags
        body.extend_from_slice(&500u16.to_le_bytes()); // impossible data len for remaining body
        body.extend_from_slice(&[0u8; 8]); // short payload

        let payload =
            make_high_frequency_packet_with_body(LLUDP_OBJECT_UPDATE_COMPRESSED_HIGH_ID, &body);
        let objects = decode_object_update_compressed_objects(&payload)
            .expect("decoder should preserve valid blocks");
        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].local_id, 55);
        assert_eq!(objects[0].mesh_id_bytes, Some(mesh_id));
    }

    #[test]
    fn decode_object_extra_params_tolerates_invalid_entry_and_keeps_valid_following_entry() {
        let mesh_id = parse_uuid_bytes("10930d3b-1821-c584-a0c7-28a34999800d").expect("uuid bytes");
        let mut payload = make_low_frequency_packet(LLUDP_OBJECT_EXTRA_PARAMS_LOW_ID);
        payload.extend_from_slice(&[0u8; 16]); // AgentID
        payload.extend_from_slice(&[0u8; 16]); // SessionID
        payload.push(2); // ObjectData count

        // Entry 1: invalid param_size larger than param_data_len.
        payload.extend_from_slice(&10u32.to_le_bytes()); // LocalID
        payload.extend_from_slice(&EXTRA_PARAM_SCULPT_EP.to_le_bytes()); // ParamType
        payload.push(1); // ParamInUse
        payload.extend_from_slice(&64u32.to_le_bytes()); // ParamSize (invalid)
        payload.push(17); // ParamData len
        payload.extend_from_slice(&[0u8; 17]);

        // Entry 2: valid mesh sculpt data.
        payload.extend_from_slice(&11u32.to_le_bytes()); // LocalID
        payload.extend_from_slice(&EXTRA_PARAM_SCULPT_EP.to_le_bytes()); // ParamType
        payload.push(1); // ParamInUse
        payload.extend_from_slice(&17u32.to_le_bytes()); // ParamSize
        payload.push(17); // ParamData len
        payload.extend_from_slice(&mesh_id);
        payload.push(SCULPT_TYPE_MESH);

        let decoded = decode_object_extra_params_mesh_updates(&payload)
            .expect("decoder should keep valid entries");
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].local_id, 11);
        assert_eq!(decoded[0].mesh_id_bytes, Some(mesh_id));
    }

    #[test]
    fn object_feed_export_includes_mesh_id_when_known() {
        let mut connection = Connection::new(ConnectionConfig::default());
        let mesh_id = parse_uuid_bytes("10930d3b-1821-c584-a0c7-28a34999800d").expect("uuid bytes");
        connection.object_feed_upsert(
            77,
            Some([100, 120, 80]),
            None,
            Some(mesh_id),
            None,
            None,
            &[],
            None,
        );
        connection.refresh_object_feed_summary_export();

        let summary = connection.simulator_payload_decode_summary();
        assert_eq!(summary.object_feed_total_objects, 1);
        assert_eq!(summary.object_feed_objects.len(), 1);
        assert_eq!(
            summary.object_feed_objects[0].mesh_id.as_deref(),
            Some("10930d3b-1821-c584-a0c7-28a34999800d")
        );
    }

    #[test]
    fn decode_default_texture_id_from_texture_entry_reads_first_uuid() {
        let expected =
            parse_uuid_bytes("10930d3b-1821-c584-a0c7-28a34999800d").expect("uuid bytes");
        let mut texture_entry = Vec::new();
        texture_entry.extend_from_slice(&expected);
        texture_entry.extend_from_slice(&[0u8; 12]);
        let decoded =
            decode_default_texture_id_from_texture_entry(&texture_entry).expect("texture id");
        assert_eq!(decoded, expected);
    }

    #[test]
    fn decode_texture_entry_material_data_parses_default_and_face_overrides() {
        let default_texture =
            parse_uuid_bytes("947d4505-eb76-2ef5-c049-e7882881d689").expect("uuid bytes");
        let face_texture =
            parse_uuid_bytes("10930d3b-1821-c584-a0c7-28a34999800d").expect("uuid bytes");
        let material_id =
            parse_uuid_bytes("ac27119d-c8fd-2edc-6c28-049f70ec462c").expect("uuid bytes");
        let mut te = Vec::new();
        te.extend_from_slice(&default_texture);
        te.push(2); // face bitfield: face 1
        te.extend_from_slice(&face_texture);
        te.push(0); // texture overrides terminator
        te.extend_from_slice(&[255, 255, 255, 255]); // default rgba
        te.push(0); // rgba overrides terminator
        for _ in 0..5 {
            te.extend_from_slice(&10_000i16.to_le_bytes()); // default scale/offset/rot
            te.push(0); // no overrides
        }
        te.push(0); // default material byte
        te.push(0); // no material overrides
        te.push(0); // default media
        te.push(0); // no media overrides
        te.push(0); // default glow
        te.push(0); // no glow overrides
        te.extend_from_slice(&material_id); // default material id
        te.push(0); // no material-id overrides

        let decoded = decode_texture_entry_material_data(&te).expect("texture entry should decode");
        let default = decoded
            .default_face_material
            .as_ref()
            .expect("default face should exist");
        assert_eq!(default.texture_id_bytes, Some(default_texture));
        assert_eq!(default.material_id_bytes, Some(material_id));
        assert_eq!(decoded.face_material_overrides.len(), 1);
        assert_eq!(decoded.face_material_overrides[0].0, 1);
        assert_eq!(
            decoded.face_material_overrides[0].1.texture_id_bytes,
            Some(face_texture)
        );
    }

    #[test]
    fn decode_texture_entry_material_data_is_fail_soft_for_truncated_payload() {
        let default_texture =
            parse_uuid_bytes("947d4505-eb76-2ef5-c049-e7882881d689").expect("uuid bytes");
        let mut te = Vec::new();
        te.extend_from_slice(&default_texture);
        te.push(0); // texture overrides terminator
        te.extend_from_slice(&[255, 255, 255, 255]); // default rgba
        // truncated before the rest of the blocks

        let decoded = decode_texture_entry_material_data(&te)
            .expect("decoder should keep default on truncated payload");
        assert_eq!(
            decoded
                .default_face_material
                .as_ref()
                .and_then(|m| m.texture_id_bytes),
            Some(default_texture)
        );
    }

    #[test]
    fn object_feed_export_includes_texture_id_when_known() {
        let mut connection = Connection::new(ConnectionConfig::default());
        let texture_id =
            parse_uuid_bytes("947d4505-eb76-2ef5-c049-e7882881d689").expect("uuid bytes");
        connection.object_feed_upsert(
            77,
            Some([100, 120, 80]),
            None,
            None,
            Some(texture_id),
            None,
            &[],
            None,
        );
        connection.refresh_object_feed_summary_export();

        let summary = connection.simulator_payload_decode_summary();
        assert_eq!(summary.object_feed_total_objects, 1);
        assert_eq!(summary.object_feed_objects.len(), 1);
        assert_eq!(
            summary.object_feed_objects[0].texture_id.as_deref(),
            Some("947d4505-eb76-2ef5-c049-e7882881d689")
        );
    }

    #[test]
    fn decode_real_firestorm_object_update_extracts_mesh_id_from_extra_params() {
        let payload = decode_test_hex(
            "c0000000b8000c0001260400020f04000190ff0272c9653f00018133fece8e4a40ddbddee12178827bb0e900032f0400016666e63e9a99193f83bcca3f4cd6ce8d3c347740bca8f17f3f79b1ba4151af4b4247b88f429a61c14100201ac23c3f00098e723500043d090210100100046464000f8c0001c228d1cf4b5d4ba884f4899a0796aa979fffff9fe17f0010c09200013a367d1cbef16d437595e88c1e3aadb3880001499bfe35b7b1f7b0787faa2b2bd685ab8400017e4bb9a788167623eb3eb239d71cf30d0008803f0003803f0021770001446973706c61794e616d6520535452494e472052572044532053617368610a46697273744e616d6520535452494e47205257204453204b616c61636830370a4c6173744e616d6520535452494e47205257204453205265736964656e740a5469746c6520535452494e4720525720445320c4b9e28886000a01004373c9653f200c8eb5941bebef44f071438c1ef74426cf190002090300019e21803e9ea7943e9e21803e3c003c72c9653f7c010001101000056464000f770001861c019c3d20ff803c68198535269aa6023ed56050c54284e510b591e18b0f030600011728320001024e50510004803f0003803f001064a6c43aec38dd168df1b83a3081935f021cf09f1c76a4446d308b2c0738c3a6c9018894b993b8fc6cd710a74165656142c300013f00014174746163684974656d494420535452494e472052572044532038366664373434372d326233622d333062312d623931382d663230663065343634666464000a1801600001110003ac27119dc8fd2edc6c28049f70ec462c050042",
        );

        let objects = decode_object_update_ids_and_scales(&payload)
            .expect("real object update should decode");

        assert_eq!(objects.len(), 2);
        assert_eq!(objects[0].local_id, 1063635314);
        assert_eq!(objects[0].mesh_id_bytes, None);
        assert_eq!(objects[1].local_id, 1063635315);
        assert_eq!(
            objects[1].mesh_id_bytes,
            Some(
                parse_uuid_bytes("ac27119d-c8fd-2edc-6c28-049f70ec462c")
                    .expect("mesh uuid should parse")
            )
        );
    }

    #[test]
    fn object_feed_replay_preserves_real_firestorm_mesh_id_across_cached_and_terse_updates() {
        let object_update = decode_test_hex(
            "c0000000b8000c0001260400020f04000190ff0272c9653f00018133fece8e4a40ddbddee12178827bb0e900032f0400016666e63e9a99193f83bcca3f4cd6ce8d3c347740bca8f17f3f79b1ba4151af4b4247b88f429a61c14100201ac23c3f00098e723500043d090210100100046464000f8c0001c228d1cf4b5d4ba884f4899a0796aa979fffff9fe17f0010c09200013a367d1cbef16d437595e88c1e3aadb3880001499bfe35b7b1f7b0787faa2b2bd685ab8400017e4bb9a788167623eb3eb239d71cf30d0008803f0003803f0021770001446973706c61794e616d6520535452494e472052572044532053617368610a46697273744e616d6520535452494e47205257204453204b616c61636830370a4c6173744e616d6520535452494e47205257204453205265736964656e740a5469746c6520535452494e4720525720445320c4b9e28886000a01004373c9653f200c8eb5941bebef44f071438c1ef74426cf190002090300019e21803e9ea7943e9e21803e3c003c72c9653f7c010001101000056464000f770001861c019c3d20ff803c68198535269aa6023ed56050c54284e510b591e18b0f030600011728320001024e50510004803f0003803f001064a6c43aec38dd168df1b83a3081935f021cf09f1c76a4446d308b2c0738c3a6c9018894b993b8fc6cd710a74165656142c300013f00014174746163684974656d494420535452494e472052572044532038366664373434372d326233622d333062312d623931382d663230663065343634666464000a1801600001110003ac27119dc8fd2edc6c28049f70ec462c050042",
        );
        let local_id = 1063635315u32;

        let mut cached_body = Vec::new();
        cached_body.extend_from_slice(&1u64.to_le_bytes());
        cached_body.extend_from_slice(&0u16.to_le_bytes());
        cached_body.push(1);
        cached_body.extend_from_slice(&local_id.to_le_bytes());
        cached_body.extend_from_slice(&0u32.to_le_bytes());
        cached_body.extend_from_slice(&0u32.to_le_bytes());
        let cached_update =
            make_high_frequency_packet_with_body(LLUDP_OBJECT_UPDATE_CACHED_HIGH_ID, &cached_body);

        let mut terse_body = Vec::new();
        terse_body.extend_from_slice(&1u64.to_le_bytes());
        terse_body.extend_from_slice(&0u16.to_le_bytes());
        terse_body.push(1);
        terse_body.push(18);
        terse_body.extend_from_slice(&local_id.to_le_bytes());
        terse_body.push(0);
        terse_body.push(0);
        terse_body.extend_from_slice(&12.0f32.to_le_bytes());
        terse_body.extend_from_slice(&8.0f32.to_le_bytes());
        terse_body.extend_from_slice(&2.0f32.to_le_bytes());
        terse_body.extend_from_slice(&0u16.to_le_bytes());
        let terse_update = make_high_frequency_packet_with_body(
            LLUDP_IMPROVED_TERSE_OBJECT_UPDATE_HIGH_ID,
            &terse_body,
        );

        let mut connection = Connection::new(ConnectionConfig::default());
        connection.state = ConnectionState::LoggedIn;
        connection
            .observe_first_simulator_inbound_payload(&object_update)
            .expect("real object update should be observed");
        connection
            .observe_first_simulator_inbound_payload(&cached_update)
            .expect("cached update should be observed");
        connection
            .observe_first_simulator_inbound_payload(&terse_update)
            .expect("terse update should be observed");

        let summary = connection.simulator_payload_decode_summary();
        assert_eq!(summary.object_feed_total_objects, 2);
        assert_eq!(summary.object_feed_state_mesh_objects, 1);
        assert_eq!(summary.object_feed_export_objects, 2);
        assert_eq!(summary.object_feed_export_mesh_objects, 1);
        assert_eq!(summary.object_feed_object_update_mesh_hits, 1);
        let retained = summary
            .object_feed_objects
            .iter()
            .find(|obj| obj.local_id == local_id)
            .expect("mesh-bearing object should remain exported");
        assert_eq!(
            retained.mesh_id.as_deref(),
            Some("ac27119d-c8fd-2edc-6c28-049f70ec462c")
        );
        assert_eq!(retained.position_centi, Some([1200, 800, 200]));
    }

    #[test]
    fn object_feed_export_prioritizes_mesh_objects_when_truncated() {
        let mut connection = Connection::new(ConnectionConfig::default());
        let mesh_id = parse_uuid_bytes("10930d3b-1821-c584-a0c7-28a34999800d").expect("uuid bytes");
        let mesh_local_id = 1u32;
        connection.object_feed_upsert(
            mesh_local_id,
            None,
            None,
            Some(mesh_id),
            None,
            None,
            &[],
            None,
        );
        for local_id in 2..=129 {
            connection.object_feed_upsert(local_id, None, None, None, None, None, &[], None);
        }
        connection.refresh_object_feed_summary_export();

        let summary = connection.simulator_payload_decode_summary();
        assert_eq!(summary.object_feed_total_objects, 129);
        assert_eq!(summary.object_feed_state_mesh_objects, 1);
        assert_eq!(summary.object_feed_export_objects, 128);
        assert_eq!(summary.object_feed_export_mesh_objects, 1);
        assert!(summary.object_feed_export_truncated);
        assert_eq!(
            summary.object_feed_objects.first().map(|obj| obj.local_id),
            Some(mesh_local_id)
        );
        assert!(
            summary
                .object_feed_objects
                .iter()
                .any(|obj| obj.local_id == mesh_local_id && obj.mesh_id.is_some()),
            "mesh-bearing object should survive truncation"
        );
    }

    #[test]
    fn observe_coarse_location_update_updates_payload_decode_summary() {
        let mut connection = Connection::new(ConnectionConfig::default());
        connection.state = ConnectionState::LoggedIn;

        let payload = make_medium_frequency_packet_with_body(6, &[1, 64, 32, 12]);
        connection
            .observe_first_simulator_inbound_payload(&payload)
            .expect("coarse location update should be observed");

        let summary = connection.simulator_payload_decode_summary();
        assert_eq!(summary.coarse_location_updates, 1);
        assert_eq!(summary.coarse_location_last_count, Some(1));
        assert_eq!(summary.coarse_location_last_first, Some([64, 32, 12]));
        assert_eq!(summary.coarse_location_last_second, None);
        assert_eq!(summary.coarse_location_last_third, None);
    }

    #[test]
    fn observe_coarse_location_update_tracks_second_location_sample() {
        let mut connection = Connection::new(ConnectionConfig::default());
        connection.state = ConnectionState::LoggedIn;

        let payload = make_medium_frequency_packet_with_body(6, &[2, 64, 32, 12, 80, 48, 16]);
        connection
            .observe_first_simulator_inbound_payload(&payload)
            .expect("coarse location update should be observed");

        let summary = connection.simulator_payload_decode_summary();
        assert_eq!(summary.coarse_location_last_second, Some([80, 48, 16]));
        assert_eq!(summary.coarse_location_last_third, None);
    }

    #[test]
    fn observe_coarse_location_update_tracks_third_location_sample() {
        let mut connection = Connection::new(ConnectionConfig::default());
        connection.state = ConnectionState::LoggedIn;

        let payload =
            make_medium_frequency_packet_with_body(6, &[3, 64, 32, 12, 80, 48, 16, 96, 56, 20]);
        connection
            .observe_first_simulator_inbound_payload(&payload)
            .expect("coarse location update should be observed");

        let summary = connection.simulator_payload_decode_summary();
        assert_eq!(summary.coarse_location_last_third, Some([96, 56, 20]));
    }

    #[test]
    fn observe_health_message_updates_payload_decode_summary() {
        let mut connection = Connection::new(ConnectionConfig::default());
        connection.state = ConnectionState::LoggedIn;

        let mut payload = make_low_frequency_packet(138);
        payload.extend_from_slice(&0.62f32.to_le_bytes());
        connection
            .observe_first_simulator_inbound_payload(&payload)
            .expect("health message should be observed");

        let summary = connection.simulator_payload_decode_summary();
        assert_eq!(summary.health_updates, 1);
        assert_eq!(summary.health_last_basis_points, Some(6200));
    }

    #[test]
    fn observe_simulator_viewer_time_updates_payload_decode_summary() {
        let mut connection = Connection::new(ConnectionConfig::default());
        connection.state = ConnectionState::LoggedIn;

        let mut payload = make_low_frequency_packet(150);
        payload.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD, 0x01]);
        connection
            .observe_first_simulator_inbound_payload(&payload)
            .expect("simulator viewer time message should be observed");

        let summary = connection.simulator_payload_decode_summary();
        assert_eq!(summary.simulator_viewer_time_updates, 1);
        assert_eq!(summary.simulator_viewer_time_last_body_len, Some(5));
        assert_eq!(
            summary.simulator_viewer_time_last_signature,
            Some(0xDDCCBBAA)
        );
    }

    #[tokio::test]
    async fn observe_agent_movement_complete_advances_waiting_stage() {
        let listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("listener bind should succeed");
        let listener_addr = listener
            .local_addr()
            .expect("listener address should exist");

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": listener_addr.port(),
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })))
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        let adapter = SecondLifeAdapter;
        connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");
        connection
            .begin_first_simulator_handshake_scaffold()
            .expect("scaffold should initialize");
        connection
            .send_first_simulator_use_circuit_code()
            .await
            .expect("use circuit code should succeed");
        connection
            .send_first_simulator_complete_agent_movement()
            .await
            .expect("complete movement send should succeed");

        let mut buf = [0u8; 1024];
        timeout(Duration::from_secs(1), listener.recv_from(&mut buf))
            .await
            .expect("first outgoing datagram should be present")
            .expect("first outgoing datagram read should succeed");
        timeout(Duration::from_secs(1), listener.recv_from(&mut buf))
            .await
            .expect("second outgoing datagram should be present")
            .expect("second outgoing datagram read should succeed");

        let classification = connection
            .observe_first_simulator_inbound_payload(&make_low_frequency_packet(250))
            .expect("inbound payload observation should succeed");
        assert_eq!(
            classification.kind,
            FirstSimulatorInboundMessageKind::AgentMovementComplete
        );
        assert_eq!(
            connection
                .first_simulator_handshake_state()
                .expect("state should exist")
                .stage,
            FirstSimulatorHandshakeStage::AgentMovementComplete
        );
        let diagnostics = connection.first_simulator_handshake_receive_diagnostics();
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].advanced_stage);
        assert_eq!(diagnostics[0].observation_index, 1);
        assert_eq!(
            diagnostics[0].decode_source,
            FirstSimulatorInboundDecodeSource::PacketMessageNumber
        );
        assert_eq!(diagnostics[0].packet_message_number, Some(0xffff00fa));
    }

    #[tokio::test]
    async fn observe_out_of_order_agent_movement_complete_does_not_advance_stage() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": 15000,
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })))
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        let adapter = SecondLifeAdapter;
        connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");
        connection
            .begin_first_simulator_handshake_scaffold()
            .expect("scaffold should initialize");

        let classification = connection
            .observe_first_simulator_inbound_payload(&make_low_frequency_packet(250))
            .expect("inbound payload observation should succeed");
        assert_eq!(
            classification.kind,
            FirstSimulatorInboundMessageKind::AgentMovementComplete
        );
        assert_eq!(
            connection
                .first_simulator_handshake_state()
                .expect("state should exist")
                .stage,
            FirstSimulatorHandshakeStage::BootstrapPrerequisitesReady
        );
        let diagnostics = connection.first_simulator_handshake_receive_diagnostics();
        assert_eq!(diagnostics.len(), 1);
        assert!(!diagnostics[0].advanced_stage);
        assert_eq!(diagnostics[0].observation_index, 1);
    }

    #[tokio::test]
    async fn observe_json_agent_movement_complete_does_not_advance_waiting_stage() {
        let listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("listener bind should succeed");
        let listener_addr = listener
            .local_addr()
            .expect("listener address should exist");

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": listener_addr.port(),
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })))
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        let adapter = SecondLifeAdapter;
        connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");
        connection
            .begin_first_simulator_handshake_scaffold()
            .expect("scaffold should initialize");
        connection
            .send_first_simulator_use_circuit_code()
            .await
            .expect("use circuit code should succeed");
        connection
            .send_first_simulator_complete_agent_movement()
            .await
            .expect("complete movement send should succeed");

        let mut buf = [0u8; 1024];
        timeout(Duration::from_secs(1), listener.recv_from(&mut buf))
            .await
            .expect("first outgoing datagram should be present")
            .expect("first outgoing datagram read should succeed");
        timeout(Duration::from_secs(1), listener.recv_from(&mut buf))
            .await
            .expect("second outgoing datagram should be present")
            .expect("second outgoing datagram read should succeed");

        let classification = connection
            .observe_first_simulator_inbound_payload(br#"{"message":"AgentMovementComplete"}"#)
            .expect("inbound payload observation should succeed");
        assert_eq!(
            classification.kind,
            FirstSimulatorInboundMessageKind::AgentMovementComplete
        );
        assert_eq!(
            classification.decode_source,
            FirstSimulatorInboundDecodeSource::JsonField
        );
        assert_eq!(
            connection
                .first_simulator_handshake_state()
                .expect("state should exist")
                .stage,
            FirstSimulatorHandshakeStage::WaitingForAgentMovementComplete
        );
        let diagnostics = connection.first_simulator_handshake_receive_diagnostics();
        assert_eq!(diagnostics.len(), 1);
        assert!(!diagnostics[0].advanced_stage);
    }

    #[tokio::test]
    async fn receive_first_simulator_handshake_datagram_once_classifies_and_records_diagnostic() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": 15000,
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })))
            .mount(&server)
            .await;

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        let adapter = SecondLifeAdapter;
        connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");
        connection
            .begin_first_simulator_handshake_scaffold()
            .expect("scaffold should initialize");

        let bind_socket = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("bind socket should succeed");
        let bind_addr = bind_socket.local_addr().expect("bind address should exist");
        drop(bind_socket);
        let bind_text = bind_addr.to_string();

        tokio::spawn(async move {
            let sender = UdpSocket::bind("127.0.0.1:0")
                .await
                .expect("sender bind should succeed");
            tokio::time::sleep(Duration::from_millis(25)).await;
            let _ = sender
                .send_to(&make_low_frequency_packet(148), bind_addr)
                .await;
        });

        let classification = connection
            .receive_first_simulator_handshake_datagram_once(&bind_text, Duration::from_secs(1))
            .await
            .expect("one-shot receive should succeed");
        assert_eq!(
            classification.kind,
            FirstSimulatorInboundMessageKind::RegionHandshake
        );
        let diagnostics = connection.first_simulator_handshake_receive_diagnostics();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].kind,
            FirstSimulatorInboundMessageKind::RegionHandshake
        );
        assert!(!diagnostics[0].advanced_stage);
        assert_eq!(diagnostics[0].observation_index, 1);
        assert_eq!(
            diagnostics[0].decode_source,
            FirstSimulatorInboundDecodeSource::PacketMessageNumber
        );
        assert_eq!(diagnostics[0].packet_message_number, Some(0xffff0094));
    }

    #[tokio::test]
    async fn probe_first_simulator_handshake_once_sends_and_receives_on_same_socket() {
        let listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("listener bind should succeed");
        let listener_addr = listener
            .local_addr()
            .expect("listener address should exist");

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": listener_addr.port(),
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })))
            .mount(&server)
            .await;

        tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let (_, first_sender) = listener
                .recv_from(&mut buf)
                .await
                .expect("first handshake datagram should arrive");
            let (_, second_sender) = listener
                .recv_from(&mut buf)
                .await
                .expect("second handshake datagram should arrive");
            assert_eq!(first_sender, second_sender);
            let _ = listener
                .send_to(&make_low_frequency_packet(250), second_sender)
                .await;
        });

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        let adapter = SecondLifeAdapter;
        connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");

        let classification = connection
            .probe_first_simulator_handshake_once("127.0.0.1:0", Duration::from_secs(1))
            .await
            .expect("probe should receive inbound packet");
        assert_eq!(
            classification.kind,
            FirstSimulatorInboundMessageKind::AgentMovementComplete
        );
        assert_eq!(
            connection
                .first_simulator_handshake_state()
                .expect("state should exist")
                .stage,
            FirstSimulatorHandshakeStage::AgentMovementComplete
        );

        let send_diagnostics = connection.first_simulator_handshake_send_diagnostics();
        assert_eq!(send_diagnostics.len(), 2);
        assert!(send_diagnostics.iter().all(|diag| diag.success));
        assert_eq!(
            send_diagnostics[0].packet_id + 1,
            send_diagnostics[1].packet_id
        );
        assert_eq!(
            send_diagnostics[0].packet_message_number,
            Some(lludp_low_frequency_message_number(
                LLUDP_USE_CIRCUIT_CODE_LOW_ID
            ))
        );
        assert_eq!(
            send_diagnostics[1].packet_message_number,
            Some(lludp_low_frequency_message_number(
                LLUDP_COMPLETE_AGENT_MOVEMENT_LOW_ID
            ))
        );

        let receive_diagnostics = connection.first_simulator_handshake_receive_diagnostics();
        assert_eq!(receive_diagnostics.len(), 1);
        assert!(receive_diagnostics[0].advanced_stage);
        assert_eq!(receive_diagnostics[0].observation_index, 1);
        assert_eq!(
            receive_diagnostics[0].decode_source,
            FirstSimulatorInboundDecodeSource::PacketMessageNumber
        );
    }

    #[tokio::test]
    async fn open_social_circuit_reuses_retained_probe_socket_without_resending_handshake() {
        let listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("listener bind should succeed");
        let listener_addr = listener
            .local_addr()
            .expect("listener address should exist");

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": listener_addr.port(),
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })))
            .mount(&server)
            .await;

        let listener_task = tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let (_, first_sender) = listener
                .recv_from(&mut buf)
                .await
                .expect("first handshake datagram should arrive");
            let (_, second_sender) = listener
                .recv_from(&mut buf)
                .await
                .expect("second handshake datagram should arrive");
            assert_eq!(first_sender, second_sender);
            let _ = listener
                .send_to(&make_low_frequency_packet(250), second_sender)
                .await;
            let extra = timeout(Duration::from_millis(500), listener.recv_from(&mut buf)).await;
            (first_sender, extra.is_ok())
        });

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        let adapter = SecondLifeAdapter;
        connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");

        connection
            .probe_first_simulator_handshake_once("127.0.0.1:0", Duration::from_secs(1))
            .await
            .expect("probe should receive inbound packet");
        let send_count_before = connection
            .first_simulator_handshake_send_diagnostics()
            .len();

        let circuit = connection
            .open_social_circuit("127.0.0.1:0")
            .await
            .expect("social circuit should reuse retained socket");
        let (probe_sender, observed_extra_datagram) =
            listener_task.await.expect("listener task should complete");

        assert_eq!(
            circuit
                .socket
                .local_addr()
                .expect("social socket local address should exist"),
            probe_sender
        );
        assert!(!observed_extra_datagram);
        assert_eq!(
            connection
                .first_simulator_handshake_send_diagnostics()
                .len(),
            send_count_before
        );
        let socket_summary = connection.summarize_first_simulator_socket_diagnostics();
        assert_eq!(socket_summary.probe_bind_events, 1);
        assert_eq!(socket_summary.fresh_bind_events, 0);
        assert_eq!(socket_summary.retained_probe_events, 1);
        assert_eq!(socket_summary.reused_retained_probe_events, 1);
        assert_eq!(socket_summary.unique_local_ports.len(), 1);
        assert!(!socket_summary.split_local_port_detected);
    }

    #[tokio::test]
    async fn open_social_circuit_reuses_probe_socket_even_when_probe_times_out_before_amc() {
        let listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("listener bind should succeed");
        let listener_addr = listener
            .local_addr()
            .expect("listener address should exist");

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": listener_addr.port(),
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })))
            .mount(&server)
            .await;

        let listener_task = tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let (_, first_sender) = listener
                .recv_from(&mut buf)
                .await
                .expect("first handshake datagram should arrive");
            let (_, second_sender) = listener
                .recv_from(&mut buf)
                .await
                .expect("second handshake datagram should arrive");
            assert_eq!(first_sender, second_sender);
            let extra = timeout(Duration::from_millis(500), listener.recv_from(&mut buf)).await;
            (first_sender, extra.is_ok())
        });

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        let adapter = SecondLifeAdapter;
        connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");

        let report = connection
            .probe_first_simulator_handshake_window("127.0.0.1:0", Duration::from_millis(50), 2)
            .await
            .expect("probe should complete with timeout report");
        assert!(report.timed_out);
        assert_eq!(report.agent_movement_complete_observation_index, None);
        let send_count_before = connection
            .first_simulator_handshake_send_diagnostics()
            .len();

        let circuit = connection
            .open_social_circuit("127.0.0.1:0")
            .await
            .expect("social circuit should reuse timed-out probe socket");
        let (probe_sender, observed_extra_datagram) =
            listener_task.await.expect("listener task should complete");

        assert_eq!(
            circuit
                .socket
                .local_addr()
                .expect("social socket local address should exist"),
            probe_sender
        );
        assert!(!observed_extra_datagram);
        assert_eq!(
            connection
                .first_simulator_handshake_send_diagnostics()
                .len(),
            send_count_before
        );
    }

    #[tokio::test]
    async fn open_social_circuit_without_probe_binds_fresh_socket_and_sends_handshake() {
        let listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("listener bind should succeed");
        let listener_addr = listener
            .local_addr()
            .expect("listener address should exist");

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": listener_addr.port(),
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })))
            .mount(&server)
            .await;

        let listener_task = tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let (_, first_sender) = listener
                .recv_from(&mut buf)
                .await
                .expect("first handshake datagram should arrive");
            let (_, second_sender) = listener
                .recv_from(&mut buf)
                .await
                .expect("second handshake datagram should arrive");
            assert_eq!(first_sender, second_sender);
            first_sender
        });

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        let adapter = SecondLifeAdapter;
        connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");

        let circuit = connection
            .open_social_circuit("127.0.0.1:0")
            .await
            .expect("social circuit should bind a fresh socket");
        let handshake_sender = listener_task.await.expect("listener task should complete");

        assert_eq!(
            circuit
                .socket
                .local_addr()
                .expect("social socket local address should exist"),
            handshake_sender
        );
        assert_eq!(
            connection
                .first_simulator_handshake_send_diagnostics()
                .len(),
            2
        );
        let socket_summary = connection.summarize_first_simulator_socket_diagnostics();
        assert_eq!(socket_summary.probe_bind_events, 0);
        assert_eq!(socket_summary.fresh_bind_events, 1);
        assert_eq!(socket_summary.reused_retained_probe_events, 0);
        assert_eq!(socket_summary.unique_local_ports.len(), 1);
        assert!(!socket_summary.split_local_port_detected);
    }

    #[tokio::test]
    async fn send_use_circuit_code_on_circuit_to_port_sends_to_explicit_target() {
        let primary_listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("primary listener bind should succeed");
        let primary_addr = primary_listener
            .local_addr()
            .expect("primary listener address should exist");
        let secondary_listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("secondary listener bind should succeed");
        let secondary_addr = secondary_listener
            .local_addr()
            .expect("secondary listener address should exist");

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": primary_addr.port(),
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })))
            .mount(&server)
            .await;

        let primary_task = tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let _ = primary_listener
                .recv_from(&mut buf)
                .await
                .expect("use circuit should arrive");
            let _ = primary_listener
                .recv_from(&mut buf)
                .await
                .expect("complete movement should arrive");
        });

        let secondary_task = tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let (len, sender) = secondary_listener
                .recv_from(&mut buf)
                .await
                .expect("explicit use circuit should arrive");
            (len, sender, buf)
        });

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        let adapter = SecondLifeAdapter;
        connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");

        let circuit = connection
            .open_social_circuit("127.0.0.1:0")
            .await
            .expect("social circuit should open");
        primary_task.await.expect("primary task should complete");

        connection
            .send_use_circuit_code_on_circuit_to_port(&circuit, secondary_addr.port())
            .await
            .expect("explicit use circuit should send");

        let (len, sender, buf) = secondary_task
            .await
            .expect("secondary task should complete");
        assert_eq!(
            sender,
            circuit
                .socket
                .local_addr()
                .expect("social socket local address should exist")
        );
        let header =
            decode_first_simulator_packet_header(&buf[..len]).expect("packet header should decode");
        assert_eq!(
            header.message_number,
            u32::from(LLUDP_USE_CIRCUIT_CODE_LOW_ID) | 0xFFFF0000
        );
    }

    #[tokio::test]
    async fn send_complete_agent_movement_on_circuit_to_port_sends_to_explicit_target() {
        let primary_listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("primary listener bind should succeed");
        let primary_addr = primary_listener
            .local_addr()
            .expect("primary listener address should exist");
        let secondary_listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("secondary listener bind should succeed");
        let secondary_addr = secondary_listener
            .local_addr()
            .expect("secondary listener address should exist");

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": primary_addr.port(),
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })))
            .mount(&server)
            .await;

        let primary_task = tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let _ = primary_listener
                .recv_from(&mut buf)
                .await
                .expect("use circuit should arrive");
            let _ = primary_listener
                .recv_from(&mut buf)
                .await
                .expect("complete movement should arrive");
        });

        let secondary_task = tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let (len, sender) = secondary_listener
                .recv_from(&mut buf)
                .await
                .expect("explicit complete movement should arrive");
            (len, sender, buf)
        });

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        let adapter = SecondLifeAdapter;
        connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");

        let circuit = connection
            .open_social_circuit("127.0.0.1:0")
            .await
            .expect("social circuit should open");
        primary_task.await.expect("primary task should complete");

        connection
            .send_complete_agent_movement_on_circuit_to_port(&circuit, secondary_addr.port())
            .await
            .expect("explicit complete movement should send");

        let (len, sender, buf) = secondary_task
            .await
            .expect("secondary task should complete");
        assert_eq!(
            sender,
            circuit
                .socket
                .local_addr()
                .expect("social socket local address should exist")
        );
        let header =
            decode_first_simulator_packet_header(&buf[..len]).expect("packet header should decode");
        assert_eq!(
            header.message_number,
            lludp_low_frequency_message_number(LLUDP_COMPLETE_AGENT_MOVEMENT_LOW_ID)
        );
    }

    #[test]
    fn decode_region_handshake_handles_zero_coded_body() {
        let body = vec![0, 4, 1, 7, b'T', b'e', b's', b't', b'S', b'i', b'm'];
        let mut payload =
            encode_lludp_low_frequency_packet(1, LLUDP_REGION_HANDSHAKE_LOW_ID, &body);
        payload[0] |= LLUDP_ZERO_CODE_FLAG;

        let decoded = decode_region_handshake(&payload).expect("region handshake should decode");

        assert_eq!(decoded.region_flags, 0);
        assert_eq!(decoded.sim_name.as_deref(), Some("TestSim"));
    }

    #[tokio::test]
    async fn send_pending_region_handshake_reply_sends_reply_once() {
        let listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("listener bind should succeed");
        let listener_addr = listener
            .local_addr()
            .expect("listener address should exist");

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": listener_addr.port(),
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })))
            .mount(&server)
            .await;

        let listener_task = tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let _ = listener
                .recv_from(&mut buf)
                .await
                .expect("UseCircuitCode should arrive");
            let _ = listener
                .recv_from(&mut buf)
                .await
                .expect("CompleteAgentMovement should arrive");
            let (reply_len, _) = listener
                .recv_from(&mut buf)
                .await
                .expect("RegionHandshakeReply should arrive");
            let reply = buf[..reply_len].to_vec();
            let extra = timeout(Duration::from_millis(300), listener.recv_from(&mut buf)).await;
            (reply, extra.is_ok())
        });

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        let adapter = SecondLifeAdapter;
        connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");

        let circuit = connection
            .open_social_circuit("127.0.0.1:0")
            .await
            .expect("social circuit should open");
        connection.pending_region_handshake_reply_flags = Some(0x12345678);

        assert!(
            connection
                .send_pending_region_handshake_reply(&circuit)
                .await
                .expect("reply send should succeed")
        );
        assert!(
            !connection
                .send_pending_region_handshake_reply(&circuit)
                .await
                .expect("second reply should be skipped")
        );

        let (reply, saw_extra_datagram) = listener_task.await.expect("listener should complete");
        let header =
            decode_first_simulator_packet_header(&reply).expect("reply header should decode");
        assert_eq!(
            header.message_number,
            lludp_low_frequency_message_number(LLUDP_REGION_HANDSHAKE_REPLY_LOW_ID)
        );
        let body = &reply[header.body_offset..];
        let flags = u32::from_le_bytes(
            body[32..36]
                .try_into()
                .expect("RegionHandshakeReply flags should be present"),
        );
        assert_eq!(flags, viewer_region_handshake_reply_flags());
        assert!(!saw_extra_datagram);
    }

    #[tokio::test]
    async fn send_pending_region_handshake_reply_requires_observed_handshake_flags() {
        let listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("listener bind should succeed");
        let listener_addr = listener
            .local_addr()
            .expect("listener address should exist");

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": listener_addr.port(),
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })))
            .mount(&server)
            .await;

        let listener_task = tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let _ = listener
                .recv_from(&mut buf)
                .await
                .expect("UseCircuitCode should arrive");
            let _ = listener
                .recv_from(&mut buf)
                .await
                .expect("CompleteAgentMovement should arrive");
            timeout(Duration::from_millis(300), listener.recv_from(&mut buf))
                .await
                .is_ok()
        });

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        let adapter = SecondLifeAdapter;
        connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");

        let circuit = connection
            .open_social_circuit("127.0.0.1:0")
            .await
            .expect("social circuit should open");

        assert!(
            !connection
                .send_pending_region_handshake_reply(&circuit)
                .await
                .expect("reply should be skipped when no RegionHandshake observed")
        );

        let saw_reply = listener_task.await.expect("listener should complete");
        assert!(!saw_reply);
    }

    #[tokio::test]
    async fn send_pending_region_handshake_reply_prefers_observed_bootstrap_sender_endpoint() {
        let primary_listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("primary listener bind should succeed");
        let primary_addr = primary_listener
            .local_addr()
            .expect("primary listener address should exist");
        let secondary_listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("secondary listener bind should succeed");
        let secondary_addr = secondary_listener
            .local_addr()
            .expect("secondary listener address should exist");

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": primary_addr.port(),
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })))
            .mount(&server)
            .await;

        let primary_task = tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let _ = primary_listener
                .recv_from(&mut buf)
                .await
                .expect("UseCircuitCode should arrive on primary");
            let _ = primary_listener
                .recv_from(&mut buf)
                .await
                .expect("CompleteAgentMovement should arrive on primary");
            timeout(
                Duration::from_millis(300),
                primary_listener.recv_from(&mut buf),
            )
            .await
            .is_ok()
        });

        let secondary_task = tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let (len, _) = secondary_listener
                .recv_from(&mut buf)
                .await
                .expect("RegionHandshakeReply should arrive on secondary");
            buf[..len].to_vec()
        });

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        let adapter = SecondLifeAdapter;
        connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");

        let circuit = connection
            .open_social_circuit("127.0.0.1:0")
            .await
            .expect("social circuit should open");
        connection.pending_region_handshake_reply_flags = Some(0x12345678);
        connection.last_bootstrap_sender_endpoint = Some(secondary_addr);

        assert!(
            connection
                .send_pending_region_handshake_reply(&circuit)
                .await
                .expect("reply send should succeed")
        );

        let saw_unexpected_primary = primary_task.await.expect("primary task should complete");
        assert!(!saw_unexpected_primary);

        let reply = secondary_task
            .await
            .expect("secondary task should complete");
        let header =
            decode_first_simulator_packet_header(&reply).expect("reply header should decode");
        assert_eq!(
            header.message_number,
            lludp_low_frequency_message_number(LLUDP_REGION_HANDSHAKE_REPLY_LOW_ID)
        );
    }

    #[tokio::test]
    async fn send_handshake_reprime_bundle_sends_to_observed_and_primary_endpoints() {
        let primary_listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("primary listener bind should succeed");
        let primary_addr = primary_listener
            .local_addr()
            .expect("primary listener address should exist");
        let secondary_listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("secondary listener bind should succeed");
        let secondary_addr = secondary_listener
            .local_addr()
            .expect("secondary listener address should exist");

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": primary_addr.port(),
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })))
            .mount(&server)
            .await;

        let primary_task = tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let _ = primary_listener
                .recv_from(&mut buf)
                .await
                .expect("UseCircuitCode should arrive on primary");
            let _ = primary_listener
                .recv_from(&mut buf)
                .await
                .expect("CompleteAgentMovement should arrive on primary");
            let mut saw_use_circuit = false;
            let mut saw_complete_movement = false;
            for _ in 0..2 {
                let (len, _) = primary_listener
                    .recv_from(&mut buf)
                    .await
                    .expect("reprime datagram should arrive on primary");
                let header = decode_first_simulator_packet_header(&buf[..len])
                    .expect("header should decode on primary");
                if header.message_number
                    == lludp_low_frequency_message_number(LLUDP_USE_CIRCUIT_CODE_LOW_ID)
                {
                    saw_use_circuit = true;
                }
                if header.message_number
                    == lludp_low_frequency_message_number(LLUDP_COMPLETE_AGENT_MOVEMENT_LOW_ID)
                {
                    saw_complete_movement = true;
                }
            }
            (saw_use_circuit, saw_complete_movement)
        });

        let secondary_task = tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let mut saw_use_circuit = false;
            let mut saw_complete_movement = false;
            for _ in 0..2 {
                let (len, _) = secondary_listener
                    .recv_from(&mut buf)
                    .await
                    .expect("reprime datagram should arrive on secondary");
                let header = decode_first_simulator_packet_header(&buf[..len])
                    .expect("header should decode on secondary");
                if header.message_number
                    == lludp_low_frequency_message_number(LLUDP_USE_CIRCUIT_CODE_LOW_ID)
                {
                    saw_use_circuit = true;
                }
                if header.message_number
                    == lludp_low_frequency_message_number(LLUDP_COMPLETE_AGENT_MOVEMENT_LOW_ID)
                {
                    saw_complete_movement = true;
                }
            }
            (saw_use_circuit, saw_complete_movement)
        });

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        let adapter = SecondLifeAdapter;
        connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");

        let circuit = connection
            .open_social_circuit("127.0.0.1:0")
            .await
            .expect("social circuit should open");
        connection.last_bootstrap_sender_endpoint = Some(secondary_addr);

        let datagrams_sent = connection
            .send_handshake_reprime_bundle(&circuit)
            .await
            .expect("handshake reprime bundle should send");
        assert_eq!(datagrams_sent, 4);

        let (primary_saw_use, primary_saw_complete) =
            primary_task.await.expect("primary task should complete");
        assert!(primary_saw_use);
        assert!(primary_saw_complete);

        let (secondary_saw_use, secondary_saw_complete) = secondary_task
            .await
            .expect("secondary task should complete");
        assert!(secondary_saw_use);
        assert!(secondary_saw_complete);
    }

    #[tokio::test]
    async fn send_startup_interest_messages_sends_agent_throttle_height_width_update_animation_then_set_always_run()
     {
        let listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("listener bind should succeed");
        let listener_addr = listener
            .local_addr()
            .expect("listener address should exist");

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": listener_addr.port(),
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })))
            .mount(&server)
            .await;

        let listener_task = tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let _ = listener
                .recv_from(&mut buf)
                .await
                .expect("UseCircuitCode should arrive");
            let _ = listener
                .recv_from(&mut buf)
                .await
                .expect("CompleteAgentMovement should arrive");
            let (throttle_len, _) = listener
                .recv_from(&mut buf)
                .await
                .expect("AgentThrottle should arrive");
            let throttle = buf[..throttle_len].to_vec();
            let (height_width_len, _) = listener
                .recv_from(&mut buf)
                .await
                .expect("AgentHeightWidth should arrive");
            let height_width = buf[..height_width_len].to_vec();
            let (update_len, _) = listener
                .recv_from(&mut buf)
                .await
                .expect("AgentUpdate should arrive");
            let update = buf[..update_len].to_vec();
            let (animation_len, _) = listener
                .recv_from(&mut buf)
                .await
                .expect("AgentAnimation should arrive");
            let animation = buf[..animation_len].to_vec();
            let (set_always_run_len, _) = listener
                .recv_from(&mut buf)
                .await
                .expect("SetAlwaysRun should arrive");
            let set_always_run = buf[..set_always_run_len].to_vec();
            (throttle, height_width, update, animation, set_always_run)
        });

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        let adapter = SecondLifeAdapter;
        connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");
        connection
            .simulator_payload_decode_summary
            .agent_movement_complete_last_position = Some([10, 20, 30]);

        let circuit = connection
            .open_social_circuit("127.0.0.1:0")
            .await
            .expect("social circuit should open");
        connection
            .send_startup_interest_messages(&circuit)
            .await
            .expect("startup interest messages should send");

        let (throttle, height_width, update, animation, set_always_run) =
            listener_task.await.expect("listener should complete");
        let throttle_header =
            decode_first_simulator_packet_header(&throttle).expect("throttle header should decode");
        assert_eq!(
            throttle_header.message_number,
            lludp_low_frequency_message_number(LLUDP_AGENT_THROTTLE_LOW_ID)
        );
        let height_width_header = decode_first_simulator_packet_header(&height_width)
            .expect("height/width header should decode");
        assert_eq!(
            height_width_header.message_number,
            lludp_low_frequency_message_number(LLUDP_AGENT_HEIGHT_WIDTH_LOW_ID)
        );
        let height_width_body = height_width_header
            .body(&height_width)
            .expect("height/width body should exist");
        assert_eq!(height_width_body.len(), 44);
        assert_eq!(
            u32::from_le_bytes(
                height_width_body[32..36]
                    .try_into()
                    .expect("circuit code should be present")
            ),
            424242
        );
        assert_eq!(
            u32::from_le_bytes(
                height_width_body[36..40]
                    .try_into()
                    .expect("gen counter should be present")
            ),
            0
        );
        assert_eq!(
            u16::from_le_bytes(
                height_width_body[40..42]
                    .try_into()
                    .expect("height should be present")
            ),
            STARTUP_AGENT_HEIGHT
        );
        assert_eq!(
            u16::from_le_bytes(
                height_width_body[42..44]
                    .try_into()
                    .expect("width should be present")
            ),
            STARTUP_AGENT_WIDTH
        );
        let update_header =
            decode_first_simulator_packet_header(&update).expect("update header should decode");
        assert_eq!(
            update_header.message_number,
            u32::from(LLUDP_AGENT_UPDATE_HIGH_ID)
        );
        assert_eq!(update[0] & LLUDP_RELIABLE_FLAG, LLUDP_RELIABLE_FLAG);
        let animation_header = decode_first_simulator_packet_header(&animation)
            .expect("animation header should decode");
        assert_eq!(
            animation_header.message_number,
            u32::from(LLUDP_AGENT_ANIMATION_HIGH_ID)
        );
        assert_eq!(animation[0] & LLUDP_RELIABLE_FLAG, LLUDP_RELIABLE_FLAG);
        let animation_body = animation_header
            .body(&animation)
            .expect("animation body should exist");
        assert_eq!(animation_body[32], 1);
        assert_eq!(
            &animation_body[33..49],
            &parse_uuid_bytes(STARTUP_AGENT_ANIMATION_ID).expect("animation uuid should parse")
        );
        assert_eq!(animation_body[49], 0);
        assert_eq!(animation_body[50], 1);
        assert_eq!(animation_body[51], 0);
        let set_always_run_header = decode_first_simulator_packet_header(&set_always_run)
            .expect("set always run header should decode");
        assert_eq!(
            set_always_run_header.message_number,
            lludp_low_frequency_message_number(LLUDP_SET_ALWAYS_RUN_LOW_ID)
        );
        let set_always_run_body = set_always_run_header
            .body(&set_always_run)
            .expect("set always run body should exist");
        assert_eq!(set_always_run_body.len(), 33);
        assert_eq!(set_always_run_body[32], 0);
    }

    #[tokio::test]
    async fn send_startup_request_parity_messages_sends_expected_low_frequency_requests() {
        let listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("listener bind should succeed");
        let listener_addr = listener
            .local_addr()
            .expect("listener address should exist");

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": listener_addr.port(),
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })))
            .mount(&server)
            .await;

        let listener_task = tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let _ = listener
                .recv_from(&mut buf)
                .await
                .expect("UseCircuitCode should arrive");
            let _ = listener
                .recv_from(&mut buf)
                .await
                .expect("CompleteAgentMovement should arrive");
            let (mute_len, _) = listener
                .recv_from(&mut buf)
                .await
                .expect("MuteListRequest should arrive");
            let mute = buf[..mute_len].to_vec();
            let (money_len, _) = listener
                .recv_from(&mut buf)
                .await
                .expect("MoneyBalanceRequest should arrive");
            let money = buf[..money_len].to_vec();
            let (agent_len, _) = listener
                .recv_from(&mut buf)
                .await
                .expect("AgentDataUpdateRequest should arrive");
            let agent = buf[..agent_len].to_vec();
            (mute, money, agent)
        });

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        let adapter = SecondLifeAdapter;
        connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");

        let circuit = connection
            .open_social_circuit("127.0.0.1:0")
            .await
            .expect("social circuit should open");
        connection
            .send_startup_request_parity_messages(&circuit)
            .await
            .expect("startup request parity messages should send");

        let (mute, money, agent) = listener_task.await.expect("listener should complete");

        let mute_header =
            decode_first_simulator_packet_header(&mute).expect("mute header should decode");
        assert_eq!(
            mute_header.message_number,
            lludp_low_frequency_message_number(LLUDP_MUTE_LIST_REQUEST_LOW_ID)
        );
        let mute_body = mute_header.body(&mute).expect("mute body should exist");
        assert_eq!(mute_body.len(), 36);
        assert_eq!(
            u32::from_le_bytes(
                mute_body[32..36]
                    .try_into()
                    .expect("mute crc field should be present")
            ),
            0
        );

        let money_header =
            decode_first_simulator_packet_header(&money).expect("money header should decode");
        assert_eq!(
            money_header.message_number,
            lludp_low_frequency_message_number(LLUDP_MONEY_BALANCE_REQUEST_LOW_ID)
        );
        let money_body = money_header.body(&money).expect("money body should exist");
        assert_eq!(money_body.len(), 48);
        assert!(money_body[32..48].iter().all(|byte| *byte == 0));

        let agent_header =
            decode_first_simulator_packet_header(&agent).expect("agent header should decode");
        assert_eq!(
            agent_header.message_number,
            lludp_low_frequency_message_number(LLUDP_AGENT_DATA_UPDATE_REQUEST_LOW_ID)
        );
        let agent_body = agent_header.body(&agent).expect("agent body should exist");
        assert_eq!(agent_body.len(), 32);
    }

    #[tokio::test]
    async fn send_agent_update_on_circuit_can_send_non_reliable_keepalive() {
        let listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("listener bind should succeed");
        let listener_addr = listener
            .local_addr()
            .expect("listener address should exist");

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": listener_addr.port(),
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })))
            .mount(&server)
            .await;

        let listener_task = tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let _ = listener
                .recv_from(&mut buf)
                .await
                .expect("UseCircuitCode should arrive");
            let _ = listener
                .recv_from(&mut buf)
                .await
                .expect("CompleteAgentMovement should arrive");
            let (update_len, _) = listener
                .recv_from(&mut buf)
                .await
                .expect("AgentUpdate should arrive");
            buf[..update_len].to_vec()
        });

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        let adapter = SecondLifeAdapter;
        connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");

        let circuit = connection
            .open_social_circuit("127.0.0.1:0")
            .await
            .expect("social circuit should open");
        connection
            .send_agent_update_on_circuit(&circuit, false)
            .await
            .expect("agent update keepalive should send");

        let update = listener_task.await.expect("listener should complete");
        let update_header =
            decode_first_simulator_packet_header(&update).expect("update header should decode");
        assert_eq!(
            update_header.message_number,
            u32::from(LLUDP_AGENT_UPDATE_HIGH_ID)
        );
        assert_eq!(update[0] & LLUDP_RELIABLE_FLAG, 0);
    }

    #[tokio::test]
    async fn send_agent_update_on_circuit_appends_pending_ack_trailer_and_drains_queue() {
        let listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("listener bind should succeed");
        let listener_addr = listener
            .local_addr()
            .expect("listener address should exist");

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": listener_addr.port(),
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })))
            .mount(&server)
            .await;

        let listener_task = tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let _ = listener
                .recv_from(&mut buf)
                .await
                .expect("UseCircuitCode should arrive");
            let _ = listener
                .recv_from(&mut buf)
                .await
                .expect("CompleteAgentMovement should arrive");
            let (update_len, _) = listener
                .recv_from(&mut buf)
                .await
                .expect("AgentUpdate should arrive");
            buf[..update_len].to_vec()
        });

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        let adapter = SecondLifeAdapter;
        connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");

        let circuit = connection
            .open_social_circuit("127.0.0.1:0")
            .await
            .expect("social circuit should open");
        connection.pending_first_simulator_ack_ids = vec![0x01020304, 0x05060708];
        connection
            .send_agent_update_on_circuit(&circuit, false)
            .await
            .expect("agent update keepalive should send");

        let update = listener_task.await.expect("listener should complete");
        assert_eq!(update[0] & LLUDP_ACK_FLAG, LLUDP_ACK_FLAG);
        assert_eq!(
            u32::from_be_bytes(
                update[update.len() - 9..update.len() - 5]
                    .try_into()
                    .unwrap()
            ),
            0x01020304
        );
        assert_eq!(
            u32::from_be_bytes(
                update[update.len() - 5..update.len() - 1]
                    .try_into()
                    .unwrap()
            ),
            0x05060708
        );
        assert_eq!(update[update.len() - 1], 2);
        assert!(connection.pending_first_simulator_ack_ids.is_empty());
    }

    #[tokio::test]
    async fn flush_pending_ack_ids_on_circuit_sends_explicit_packet_ack_and_drains_queue() {
        let listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("listener bind should succeed");
        let listener_addr = listener
            .local_addr()
            .expect("listener address should exist");

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": listener_addr.port(),
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })))
            .mount(&server)
            .await;

        let listener_task = tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let _ = listener
                .recv_from(&mut buf)
                .await
                .expect("UseCircuitCode should arrive");
            let _ = listener
                .recv_from(&mut buf)
                .await
                .expect("CompleteAgentMovement should arrive");
            let (ack_len, _) = listener
                .recv_from(&mut buf)
                .await
                .expect("PacketAck should arrive");
            buf[..ack_len].to_vec()
        });

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        let adapter = SecondLifeAdapter;
        connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");

        let circuit = connection
            .open_social_circuit("127.0.0.1:0")
            .await
            .expect("social circuit should open");
        connection.pending_first_simulator_ack_ids = vec![0x01020304, 0x05060708];

        let flushed = connection
            .flush_pending_ack_ids_on_circuit(&circuit)
            .await
            .expect("ack flush should send");

        let ack_packet = listener_task.await.expect("listener should complete");
        let header =
            decode_first_simulator_packet_header(&ack_packet).expect("ack packet should decode");
        let body = header.body(&ack_packet).expect("ack body should decode");

        assert_eq!(
            header.message_number,
            lludp_low_frequency_message_number(LLUDP_PACKET_ACK_LOW_ID)
        );
        assert_eq!(body[0], 2);
        assert_eq!(
            u32::from_le_bytes(body[1..5].try_into().unwrap()),
            0x01020304
        );
        assert_eq!(
            u32::from_le_bytes(body[5..9].try_into().unwrap()),
            0x05060708
        );
        assert_eq!(flushed, 2);
        assert!(connection.pending_first_simulator_ack_ids.is_empty());
    }

    #[tokio::test]
    async fn flush_pending_object_cache_miss_ids_sends_request_multiple_objects_and_drains_queue() {
        let listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("listener bind should succeed");
        let listener_addr = listener
            .local_addr()
            .expect("listener address should exist");

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": listener_addr.port(),
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })))
            .mount(&server)
            .await;

        let listener_task = tokio::spawn(async move {
            let mut buf = [0u8; 2048];
            let _ = listener
                .recv_from(&mut buf)
                .await
                .expect("UseCircuitCode should arrive");
            let _ = listener
                .recv_from(&mut buf)
                .await
                .expect("CompleteAgentMovement should arrive");
            let (len, _) = listener
                .recv_from(&mut buf)
                .await
                .expect("RequestMultipleObjects should arrive");
            buf[..len].to_vec()
        });

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        let adapter = SecondLifeAdapter;
        connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");
        let circuit = connection
            .open_social_circuit("127.0.0.1:0")
            .await
            .expect("social circuit should open");

        connection.pending_object_cache_miss_ids = vec![1001, 1002];
        let flushed = connection
            .flush_pending_object_cache_miss_ids_on_circuit(&circuit)
            .await
            .expect("cache miss flush should succeed");

        let packet = listener_task.await.expect("listener should complete");
        let header = decode_first_simulator_packet_header(&packet)
            .expect("request-multiple packet should decode");
        let body = header.body(&packet).expect("request body should decode");

        assert_eq!(
            header.message_number,
            lludp_medium_frequency_message_number(LLUDP_REQUEST_MULTIPLE_OBJECTS_MEDIUM_ID)
        );
        assert_eq!(body[32], 2);
        assert_eq!(body[33], LLUDP_CACHE_MISS_TYPE_TOTAL);
        assert_eq!(u32::from_le_bytes(body[34..38].try_into().unwrap()), 1001);
        assert_eq!(body[38], LLUDP_CACHE_MISS_TYPE_TOTAL);
        assert_eq!(u32::from_le_bytes(body[39..43].try_into().unwrap()), 1002);
        assert_eq!(flushed, 2);
        assert!(connection.pending_object_cache_miss_ids.is_empty());
    }

    #[test]
    fn summarize_first_simulator_ack_forensics_reports_pending_and_appended_ack_state() {
        let mut connection = Connection::new(ConnectionConfig::default());
        connection.pending_first_simulator_ack_ids = vec![0x01020304, 0x05060708];
        connection.first_simulator_handshake_send_diagnostics.push(
            FirstSimulatorHandshakeSendDiagnostic {
                action: FirstSimulatorHandshakeAction::CompleteAgentMovement,
                target: String::from("127.0.0.1:13001"),
                packet_id: 7,
                packet_message_number: Some(u32::from(LLUDP_AGENT_UPDATE_HIGH_ID)),
                appended_ack_ids: vec![0x0A0B0C0D],
                payload_len: 42,
                elapsed_ms: 1,
                success: true,
                error: None,
            },
        );
        connection
            .first_simulator_handshake_receive_diagnostics
            .push(FirstSimulatorHandshakeReceiveDiagnostic {
                observation_index: 3,
                kind: FirstSimulatorInboundMessageKind::PacketAck,
                scope: FirstSimulatorInboundTrafficScope::TransportControl,
                payload_len: 12,
                stage_before: None,
                stage_after: None,
                advanced_stage: false,
                signal: String::from("packet:0xfffffffb"),
                decode_source: FirstSimulatorInboundDecodeSource::PacketMessageNumber,
                packet_message_number: Some(lludp_low_frequency_message_number(
                    LLUDP_PACKET_ACK_LOW_ID,
                )),
            });

        let summary = connection.summarize_first_simulator_ack_forensics();

        assert_eq!(summary.pending_ack_count, 2);
        assert_eq!(
            summary.pending_ack_ids_preview,
            vec![0x01020304, 0x05060708]
        );
        assert_eq!(summary.outbound_appended_ack_sends, 1);
        assert_eq!(summary.outbound_appended_ack_ids_last, vec![0x0A0B0C0D]);
        assert_eq!(summary.explicit_packet_ack_receives, 1);
        assert_eq!(summary.first_packet_ack_receive_index, Some(3));
    }

    #[test]
    fn summarize_first_simulator_startup_interest_gate_reports_pass_with_required_messages() {
        let mut connection = Connection::new(ConnectionConfig::default());
        connection.first_simulator_handshake_send_diagnostics.push(
            FirstSimulatorHandshakeSendDiagnostic {
                action: FirstSimulatorHandshakeAction::CompleteAgentMovement,
                target: String::from("127.0.0.1:13001"),
                packet_id: 11,
                packet_message_number: Some(lludp_low_frequency_message_number(
                    LLUDP_AGENT_THROTTLE_LOW_ID,
                )),
                appended_ack_ids: Vec::new(),
                payload_len: 20,
                elapsed_ms: 1,
                success: true,
                error: None,
            },
        );
        connection.first_simulator_handshake_send_diagnostics.push(
            FirstSimulatorHandshakeSendDiagnostic {
                action: FirstSimulatorHandshakeAction::CompleteAgentMovement,
                target: String::from("127.0.0.1:13001"),
                packet_id: 12,
                packet_message_number: Some(lludp_low_frequency_message_number(
                    LLUDP_AGENT_HEIGHT_WIDTH_LOW_ID,
                )),
                appended_ack_ids: Vec::new(),
                payload_len: 20,
                elapsed_ms: 1,
                success: true,
                error: None,
            },
        );
        connection.first_simulator_handshake_send_diagnostics.push(
            FirstSimulatorHandshakeSendDiagnostic {
                action: FirstSimulatorHandshakeAction::CompleteAgentMovement,
                target: String::from("127.0.0.1:13001"),
                packet_id: 13,
                packet_message_number: Some(u32::from(LLUDP_AGENT_UPDATE_HIGH_ID)),
                appended_ack_ids: Vec::new(),
                payload_len: 20,
                elapsed_ms: 1,
                success: true,
                error: None,
            },
        );

        let gate = connection.summarize_first_simulator_startup_interest_gate();
        assert!(gate.passed);
        assert!(gate.missing.is_empty());
        assert_eq!(gate.required.len(), 3);
        assert!(
            gate.required
                .iter()
                .any(|entry| entry.message == "AgentThrottle")
        );
        assert!(
            gate.required
                .iter()
                .any(|entry| entry.message == "AgentHeightWidth")
        );
        assert!(
            gate.required
                .iter()
                .any(|entry| entry.message == "AgentUpdate")
        );
    }

    #[test]
    fn summarize_first_simulator_startup_interest_gate_reports_missing_messages() {
        let mut connection = Connection::new(ConnectionConfig::default());
        connection.first_simulator_handshake_send_diagnostics.push(
            FirstSimulatorHandshakeSendDiagnostic {
                action: FirstSimulatorHandshakeAction::CompleteAgentMovement,
                target: String::from("127.0.0.1:13001"),
                packet_id: 21,
                packet_message_number: Some(lludp_low_frequency_message_number(
                    LLUDP_AGENT_THROTTLE_LOW_ID,
                )),
                appended_ack_ids: Vec::new(),
                payload_len: 20,
                elapsed_ms: 1,
                success: true,
                error: None,
            },
        );

        let gate = connection.summarize_first_simulator_startup_interest_gate();
        assert!(!gate.passed);
        assert_eq!(gate.required.len(), 1);
        assert_eq!(
            gate.missing,
            vec![
                String::from("AgentUpdate"),
                String::from("AgentHeightWidth"),
            ]
        );
    }

    #[test]
    fn summarize_first_simulator_receive_forensics_and_timeline_report_observed_packets() {
        let mut connection = Connection::new(ConnectionConfig::default());
        connection.state = ConnectionState::LoggedIn;

        connection
            .observe_first_simulator_inbound_payload(&make_low_frequency_packet(
                LLUDP_REGION_HANDSHAKE_LOW_ID,
            ))
            .expect("region handshake should observe");
        connection
            .observe_first_simulator_inbound_payload(&make_low_frequency_packet(
                LLUDP_REGION_HANDSHAKE_REPLY_LOW_ID,
            ))
            .expect("region handshake reply should observe");
        connection
            .observe_first_simulator_inbound_payload(&make_low_frequency_packet(
                LLUDP_PACKET_ACK_LOW_ID,
            ))
            .expect("packet ack should observe");
        connection
            .observe_first_simulator_inbound_payload(&make_low_frequency_packet(
                LLUDP_AGENT_MOVEMENT_COMPLETE_LOW_ID,
            ))
            .expect("agent movement complete should observe");
        connection
            .observe_first_simulator_inbound_payload(&make_high_frequency_packet(
                LLUDP_OBJECT_UPDATE_CACHED_HIGH_ID,
            ))
            .expect("object update cached should observe");
        connection
            .observe_first_simulator_inbound_payload(&make_high_frequency_packet(
                LLUDP_CAMERA_CONSTRAINT_HIGH_ID,
            ))
            .expect("camera constraint should observe");
        connection
            .observe_first_simulator_inbound_payload(&make_low_frequency_packet(
                LLUDP_GENERIC_MESSAGE_LOW_ID,
            ))
            .expect("generic message should observe");
        connection
            .observe_first_simulator_inbound_payload(&make_medium_frequency_packet(0x99))
            .expect("unknown packet should observe");

        let receive_summary = connection.summarize_first_simulator_receive_forensics();
        let timeline = connection.summarize_first_simulator_startup_timeline();
        let transcript = connection.summarize_first_simulator_startup_transcript(4);

        assert_eq!(receive_summary.receive_observations, 8);
        assert!(
            receive_summary
                .typed_kind_counts
                .iter()
                .any(
                    |(kind, count)| *kind == FirstSimulatorInboundMessageKind::RegionHandshake
                        && *count == 1
                )
        );
        assert!(receive_summary.raw_packet_message_numbers.iter().any(
            |(message_number, count)| *message_number
                == lludp_low_frequency_message_number(LLUDP_PACKET_ACK_LOW_ID)
                && *count == 1
        ));
        assert_eq!(receive_summary.unclassified_packet_message_numbers.len(), 1);
        assert_eq!(timeline.first_region_handshake_index, Some(1));
        assert_eq!(timeline.first_region_handshake_reply_index, Some(2));
        assert_eq!(timeline.first_packet_ack_index, Some(3));
        assert_eq!(timeline.first_agent_movement_complete_index, Some(4));
        assert_eq!(timeline.first_camera_constraint_index, Some(6));
        assert_eq!(timeline.first_generic_message_index, Some(7));
        assert_eq!(timeline.first_object_update_index, Some(5));
        assert_eq!(transcript.receive_events.len(), 4);
    }

    #[test]
    fn capability_url_classification_groups_simhost_and_cdn_families() {
        let simhost = classify_capability_url(
            "https://simhost-01eb29dc5b0bc96b7.agni.secondlife.io:12043/cap/example",
        );
        assert_eq!(simhost.family, CapabilityUrlFamily::SimulatorHost12043);
        assert_eq!(
            simhost.host.as_deref(),
            Some("simhost-01eb29dc5b0bc96b7.agni.secondlife.io")
        );
        assert_eq!(simhost.port, Some(12043));

        let asset_cdn =
            classify_capability_url("http://asset-cdn.glb.agni.lindenlab.com/?texture_id=test");
        assert_eq!(asset_cdn.family, CapabilityUrlFamily::AssetCdn);

        let bake = classify_capability_url(
            "http://bake-texture.glb.agni.lindenlab.com/texture/avatar/head/hash",
        );
        assert_eq!(bake.family, CapabilityUrlFamily::BakeTextureCdn);

        let inventory = summarize_seed_capability_inventory(&SeedCapabilityMap {
            entries: BTreeMap::from([
                (
                    String::from("EventQueueGet"),
                    String::from(
                        "https://simhost-01eb29dc5b0bc96b7.agni.secondlife.io:12043/cap/event",
                    ),
                ),
                (
                    String::from("GetTexture"),
                    String::from("http://asset-cdn.glb.agni.lindenlab.com/?texture_id=test"),
                ),
            ]),
        });
        assert_eq!(inventory.len(), 2);
        assert_eq!(
            inventory[0].classification.family,
            CapabilityUrlFamily::SimulatorHost12043
        );
        assert_eq!(
            inventory[1].classification.family,
            CapabilityUrlFamily::AssetCdn
        );
    }

    #[test]
    fn decode_health_message_ignores_ack_trailer_bytes() {
        let mut payload = mark_packet_reliable(make_low_frequency_packet_with_body(
            LLUDP_HEALTH_MESSAGE_LOW_ID,
            &1.0f32.to_le_bytes(),
        ));
        payload = append_lludp_ack_trailer(&payload, &[0x01020304]);

        let decoded = decode_health_message(&payload).expect("health message should decode");

        assert_eq!(decoded.health, 1.0);
    }

    #[tokio::test]
    async fn probe_first_simulator_handshake_window_collects_multiple_observations() {
        let listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("listener bind should succeed");
        let listener_addr = listener
            .local_addr()
            .expect("listener address should exist");

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": listener_addr.port(),
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })))
            .mount(&server)
            .await;

        tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let (_, sender) = listener
                .recv_from(&mut buf)
                .await
                .expect("first handshake datagram should arrive");
            let _ = listener
                .recv_from(&mut buf)
                .await
                .expect("second handshake datagram should arrive");
            let _ = listener
                .send_to(&make_low_frequency_packet(387), sender)
                .await;
            let _ = listener
                .send_to(&make_low_frequency_packet(250), sender)
                .await;
        });

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        let adapter = SecondLifeAdapter;
        connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");

        let report = connection
            .probe_first_simulator_handshake_window("127.0.0.1:0", Duration::from_secs(1), 3)
            .await
            .expect("probe window should succeed");
        assert_eq!(report.observations.len(), 2);
        assert!(!report.timed_out);
        assert_eq!(report.agent_movement_complete_observation_index, Some(2));
        assert_eq!(report.post_movement_observations, 0);
        let summary = report
            .post_boundary_summary
            .expect("post-boundary summary should exist once movement complete is observed");
        assert_eq!(summary.observations, 0);
        assert_eq!(summary.bootstrap_relevant, 0);
        assert_eq!(summary.transport_control, 0);
        assert_eq!(summary.likely_broader_traffic, 0);
        assert_eq!(summary.unknown, 0);
        assert!(summary.unknown_packet_message_numbers.is_empty());
        assert!(summary.repeated_unknown_packet_message_numbers.is_empty());
        assert!(summary.kinds.is_empty());
        assert_eq!(
            report.observations[0].classification.kind,
            FirstSimulatorInboundMessageKind::AgentDataUpdate
        );
        assert_eq!(
            report.observations[1].classification.kind,
            FirstSimulatorInboundMessageKind::AgentMovementComplete
        );
        assert!(connection.early_simulator_traffic_observations().is_empty());
        assert_eq!(
            connection
                .first_simulator_handshake_state()
                .expect("state should exist")
                .stage,
            FirstSimulatorHandshakeStage::AgentMovementComplete
        );
    }

    #[tokio::test]
    async fn probe_first_simulator_handshake_window_with_tail_preserves_observed_post_amc_order() {
        let listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("listener bind should succeed");
        let listener_addr = listener
            .local_addr()
            .expect("listener address should exist");

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": listener_addr.port(),
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })))
            .mount(&server)
            .await;

        tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let (_, sender) = listener
                .recv_from(&mut buf)
                .await
                .expect("first handshake datagram should arrive");
            let _ = listener
                .recv_from(&mut buf)
                .await
                .expect("second handshake datagram should arrive");
            let _ = listener
                .send_to(&make_low_frequency_packet(387), sender)
                .await;
            let _ = listener
                .send_to(&make_low_frequency_packet(1), sender)
                .await;
            let _ = listener
                .send_to(&make_low_frequency_packet(250), sender)
                .await;
            let _ = listener
                .send_to(&make_low_frequency_packet(0xFFFB), sender)
                .await;
            let _ = listener
                .send_to(&make_low_frequency_packet(138), sender)
                .await;
            let _ = listener
                .send_to(&make_low_frequency_packet(322), sender)
                .await;
            let _ = listener
                .send_to(&make_medium_frequency_packet(6), sender)
                .await;
            let _ = listener
                .send_to(&make_medium_frequency_packet(13), sender)
                .await;
            let _ = listener
                .send_to(&make_medium_frequency_packet(17), sender)
                .await;
        });

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        let adapter = SecondLifeAdapter;
        connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");

        let report = connection
            .probe_first_simulator_handshake_window_with_tail(
                "127.0.0.1:0",
                Duration::from_secs(1),
                12,
                6,
            )
            .await
            .expect("probe window with tail should succeed");
        assert_eq!(report.observations.len(), 9);
        assert!(!report.timed_out);
        assert_eq!(report.agent_movement_complete_observation_index, Some(3));
        assert_eq!(report.post_movement_observations, 6);
        assert_eq!(
            report.observations[0].classification.kind,
            FirstSimulatorInboundMessageKind::AgentDataUpdate
        );
        assert_eq!(
            report.observations[1].classification.kind,
            FirstSimulatorInboundMessageKind::TestMessage
        );
        assert_eq!(
            report.observations[2].classification.kind,
            FirstSimulatorInboundMessageKind::AgentMovementComplete
        );
        assert_eq!(
            report.observations[3].classification.kind,
            FirstSimulatorInboundMessageKind::PacketAck
        );
        assert_eq!(
            report.observations[4].classification.kind,
            FirstSimulatorInboundMessageKind::HealthMessage
        );
        assert_eq!(
            report.observations[5].classification.kind,
            FirstSimulatorInboundMessageKind::OnlineNotification
        );
        assert_eq!(
            report.observations[6].classification.kind,
            FirstSimulatorInboundMessageKind::CoarseLocationUpdate
        );
        assert_eq!(
            report.observations[7].classification.kind,
            FirstSimulatorInboundMessageKind::AttachedSound
        );
        assert_eq!(
            report.observations[8].classification.kind,
            FirstSimulatorInboundMessageKind::ViewerEffect
        );
        assert_eq!(
            report.observations[2].classification.scope,
            FirstSimulatorInboundTrafficScope::BootstrapRelevant
        );
        assert_eq!(
            report.observations[3].classification.scope,
            FirstSimulatorInboundTrafficScope::TransportControl
        );
        assert_eq!(
            report.observations[4].classification.scope,
            FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic
        );
        assert_eq!(
            report.observations[5].classification.scope,
            FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic
        );
        assert_eq!(
            report.observations[6].classification.scope,
            FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic
        );
        assert_eq!(
            report.observations[7].classification.scope,
            FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic
        );
        assert_eq!(
            report.observations[8].classification.scope,
            FirstSimulatorInboundTrafficScope::LikelyBroaderTraffic
        );
        let summary = report
            .post_boundary_summary
            .expect("post-boundary summary should exist once movement complete is observed");
        assert_eq!(summary.observations, 6);
        assert_eq!(summary.bootstrap_relevant, 0);
        assert_eq!(summary.transport_control, 1);
        assert_eq!(summary.likely_broader_traffic, 5);
        assert_eq!(summary.unknown, 0);
        assert!(summary.unknown_packet_message_numbers.is_empty());
        assert!(summary.repeated_unknown_packet_message_numbers.is_empty());
        assert_eq!(
            summary.kinds,
            vec![
                FirstSimulatorInboundMessageKind::PacketAck,
                FirstSimulatorInboundMessageKind::HealthMessage,
                FirstSimulatorInboundMessageKind::OnlineNotification,
                FirstSimulatorInboundMessageKind::CoarseLocationUpdate,
                FirstSimulatorInboundMessageKind::AttachedSound,
                FirstSimulatorInboundMessageKind::ViewerEffect,
            ]
        );
        let early_traffic = connection.early_simulator_traffic_observations();
        assert_eq!(early_traffic.len(), 5);
        assert_eq!(
            early_traffic[0].kind,
            EarlySimulatorTrafficKind::HealthMessage
        );
        assert_eq!(
            early_traffic[1].kind,
            EarlySimulatorTrafficKind::OnlineNotification
        );
        assert_eq!(
            early_traffic[2].kind,
            EarlySimulatorTrafficKind::CoarseLocationUpdate
        );
        assert_eq!(
            early_traffic[3].kind,
            EarlySimulatorTrafficKind::AttachedSound
        );
        assert_eq!(
            early_traffic[4].kind,
            EarlySimulatorTrafficKind::ViewerEffect
        );
        let early_summary = connection.summarize_early_simulator_traffic();
        assert_eq!(early_summary.observations, 5);
        assert_eq!(early_summary.health_message, 1);
        assert_eq!(early_summary.simulator_viewer_time_message, 0);
        assert_eq!(early_summary.online_notification, 1);
        assert_eq!(early_summary.viewer_effect, 1);
        assert_eq!(early_summary.coarse_location_update, 1);
        assert_eq!(early_summary.attached_sound, 1);
    }

    #[tokio::test]
    async fn probe_first_simulator_handshake_window_with_tail_summarizes_region_transition_and_repeated_unknowns()
     {
        let listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("listener bind should succeed");
        let listener_addr = listener
            .local_addr()
            .expect("listener address should exist");

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": listener_addr.port(),
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })))
            .mount(&server)
            .await;

        tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let (_, sender) = listener
                .recv_from(&mut buf)
                .await
                .expect("first handshake datagram should arrive");
            let _ = listener
                .recv_from(&mut buf)
                .await
                .expect("second handshake datagram should arrive");
            let _ = listener
                .send_to(&make_low_frequency_packet(387), sender)
                .await;
            let _ = listener
                .send_to(&make_low_frequency_packet(250), sender)
                .await;
            let _ = listener
                .send_to(&make_medium_frequency_packet(7), sender)
                .await;
            let _ = listener
                .send_to(&make_medium_frequency_packet(8), sender)
                .await;
            let _ = listener
                .send_to(&make_low_frequency_packet(42), sender)
                .await;
            let _ = listener
                .send_to(&make_low_frequency_packet(42), sender)
                .await;
            let _ = listener
                .send_to(&make_medium_frequency_packet(17), sender)
                .await;
        });

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        let adapter = SecondLifeAdapter;
        connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");

        let report = connection
            .probe_first_simulator_handshake_window_with_tail(
                "127.0.0.1:0",
                Duration::from_secs(1),
                12,
                5,
            )
            .await
            .expect("probe window with tail should succeed");

        assert!(!report.timed_out);
        assert_eq!(report.agent_movement_complete_observation_index, Some(2));
        assert_eq!(report.post_movement_observations, 5);

        let summary = report
            .post_boundary_summary
            .expect("post-boundary summary should exist once movement complete is observed");
        assert_eq!(summary.observations, 5);
        assert_eq!(summary.bootstrap_relevant, 0);
        assert_eq!(summary.transport_control, 0);
        assert_eq!(summary.region_transition_control, 2);
        assert_eq!(summary.likely_broader_traffic, 1);
        assert_eq!(summary.unknown, 2);
        assert_eq!(summary.crossed_region, 1);
        assert_eq!(summary.confirm_enable_simulator, 1);
        assert!(!summary.watched_region_transition_control_not_seen);
        assert_eq!(
            summary.kinds,
            vec![
                FirstSimulatorInboundMessageKind::CrossedRegion,
                FirstSimulatorInboundMessageKind::ConfirmEnableSimulator,
                FirstSimulatorInboundMessageKind::Irrelevant,
                FirstSimulatorInboundMessageKind::Irrelevant,
                FirstSimulatorInboundMessageKind::ViewerEffect,
            ]
        );
        assert_eq!(
            summary.unknown_packet_message_numbers,
            vec![0xffff002a, 0xffff002a]
        );
        assert_eq!(
            summary.repeated_unknown_packet_message_numbers,
            vec![0xffff002a]
        );

        let region_control_observations = connection.region_transition_control_observations();
        assert_eq!(region_control_observations.len(), 2);
        assert_eq!(
            region_control_observations[0].kind,
            RegionTransitionControlKind::CrossedRegion
        );
        assert_eq!(
            region_control_observations[1].kind,
            RegionTransitionControlKind::ConfirmEnableSimulator
        );
        let region_control_summary = connection.summarize_region_transition_control();
        assert_eq!(region_control_summary.observations, 2);
        assert_eq!(region_control_summary.crossed_region, 1);
        assert_eq!(region_control_summary.confirm_enable_simulator, 1);
        assert!(!region_control_summary.not_seen_in_run);
        let continuity = connection.continuity_summary();
        assert_eq!(continuity.phase, HandoffPhase::Confirming);
        assert_eq!(continuity.active_region_coords, Some([1000, 1001]));
        assert_eq!(continuity.previous_region_coords, None);

        let early_traffic = connection.early_simulator_traffic_observations();
        assert_eq!(early_traffic.len(), 1);
        assert_eq!(
            early_traffic[0].kind,
            EarlySimulatorTrafficKind::ViewerEffect
        );
        let early_summary = connection.summarize_early_simulator_traffic();
        assert_eq!(early_summary.observations, 1);
        assert_eq!(early_summary.viewer_effect, 1);
    }

    #[tokio::test]
    async fn probe_first_simulator_handshake_window_with_policy_uses_post_movement_timeout_and_can_stop_on_region_control()
     {
        let listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("listener bind should succeed");
        let listener_addr = listener
            .local_addr()
            .expect("listener address should exist");

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": listener_addr.port(),
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })))
            .mount(&server)
            .await;

        tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let (_, sender) = listener
                .recv_from(&mut buf)
                .await
                .expect("first handshake datagram should arrive");
            let _ = listener
                .recv_from(&mut buf)
                .await
                .expect("second handshake datagram should arrive");
            let _ = listener
                .send_to(&make_low_frequency_packet(387), sender)
                .await;
            let _ = listener
                .send_to(&make_low_frequency_packet(250), sender)
                .await;
            tokio::time::sleep(Duration::from_millis(80)).await;
            let _ = listener
                .send_to(&make_medium_frequency_packet(7), sender)
                .await;
            let _ = listener
                .send_to(&make_medium_frequency_packet(8), sender)
                .await;
        });

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        let adapter = SecondLifeAdapter;
        connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");

        let report = connection
            .probe_first_simulator_handshake_window_with_policy(
                "127.0.0.1:0",
                Duration::from_millis(20),
                10,
                5,
                Some(Duration::from_millis(200)),
                true,
            )
            .await
            .expect("probe window with policy should succeed");

        assert!(!report.timed_out);
        assert_eq!(
            report
                .observations
                .iter()
                .map(|obs| obs.classification.kind)
                .collect::<Vec<_>>(),
            vec![
                FirstSimulatorInboundMessageKind::AgentDataUpdate,
                FirstSimulatorInboundMessageKind::AgentMovementComplete,
                FirstSimulatorInboundMessageKind::CrossedRegion,
            ]
        );
        assert_eq!(report.agent_movement_complete_observation_index, Some(2));
        assert_eq!(report.post_movement_observations, 1);

        let summary = report
            .post_boundary_summary
            .expect("post-boundary summary should exist once movement complete is observed");
        assert_eq!(summary.observations, 1);
        assert_eq!(summary.region_transition_control, 1);
        assert_eq!(summary.crossed_region, 1);
        assert_eq!(summary.confirm_enable_simulator, 0);
        assert!(!summary.watched_region_transition_control_not_seen);

        let handoff_summary = connection.summarize_region_transition_control();
        assert_eq!(handoff_summary.observations, 1);
        assert_eq!(handoff_summary.crossed_region, 1);
        assert_eq!(handoff_summary.confirm_enable_simulator, 0);
        assert!(!handoff_summary.not_seen_in_run);
        assert_eq!(connection.continuity_summary().phase, HandoffPhase::Crossed);
    }

    #[test]
    fn record_region_continuity_observation_caps_neighbor_export() {
        let mut connection = Connection::new(ConnectionConfig::default());
        let neighbors = (0..32)
            .map(|idx| viewer_core::BoundedNeighborSummary {
                region_handle: idx as u64,
                region_x: idx,
                region_y: idx + 1,
            })
            .collect::<Vec<_>>();
        connection.record_region_continuity_observation(RegionContinuitySummary {
            phase: HandoffPhase::Crossed,
            outcome: viewer_core::HandoffOutcome::Normal,
            reason: viewer_core::HandoffReason::None,
            phase_age_ms: 0,
            last_probe_result: None,
            last_probe_time_unix_ms: None,
            active_region_coords: Some([1000, 1001]),
            previous_region_coords: Some([999, 1001]),
            neighbors,
        });
        assert_eq!(
            connection.continuity_summary().neighbors.len(),
            MAX_CONTINUITY_NEIGHBORS
        );
    }

    #[test]
    fn continuity_summary_phase_age_advances_after_phase_change() {
        let mut connection = Connection::new(ConnectionConfig::default());
        connection.record_region_continuity_observation(RegionContinuitySummary {
            phase: HandoffPhase::Crossed,
            outcome: viewer_core::HandoffOutcome::Normal,
            reason: viewer_core::HandoffReason::None,
            phase_age_ms: 0,
            last_probe_result: None,
            last_probe_time_unix_ms: None,
            active_region_coords: Some([1000, 1001]),
            previous_region_coords: None,
            neighbors: Vec::new(),
        });

        std::thread::sleep(Duration::from_millis(5));
        assert!(
            connection.continuity_summary().phase_age_ms >= 1,
            "phase age should increase over time after phase transition"
        );
    }

    #[test]
    fn observe_transition_control_does_not_regress_completed_phase() {
        let mut connection = Connection::new(ConnectionConfig::default());
        connection.state = ConnectionState::LoggedIn;
        connection.record_region_continuity_observation(RegionContinuitySummary {
            phase: HandoffPhase::Completed,
            outcome: viewer_core::HandoffOutcome::Normal,
            reason: viewer_core::HandoffReason::None,
            phase_age_ms: 0,
            last_probe_result: None,
            last_probe_time_unix_ms: None,
            active_region_coords: Some([1000, 1001]),
            previous_region_coords: Some([999, 1001]),
            neighbors: Vec::new(),
        });

        connection
            .observe_first_simulator_inbound_payload(&make_medium_frequency_packet(7))
            .expect("crossed-region observation should parse");

        assert_eq!(
            connection.continuity_summary().phase,
            HandoffPhase::Completed
        );
    }

    #[test]
    fn parse_event_queue_poll_from_json_extracts_scalar_fields() {
        let value = json!({
            "id": 9,
            "events": [{
                "message": "ChatFromSimulator",
                "body": {
                    "from_name": "Tester Resident",
                    "message": "hello world",
                    "channel": 0
                }
            }]
        });
        let poll = parse_event_queue_poll_from_json(&value).expect("poll parsing should succeed");
        assert_eq!(poll.id, Some(9));
        assert_eq!(poll.events.len(), 1);
        assert_eq!(poll.events[0].message, "ChatFromSimulator");
        assert_eq!(
            poll.events[0].fields.get("from_name").map(String::as_str),
            Some("Tester Resident")
        );
    }

    #[test]
    fn extract_nearby_chat_messages_filters_chat_like_events() {
        let connection = Connection::new(ConnectionConfig::default());
        let poll = EventQueuePollResult {
            id: Some(1),
            events: vec![
                EventQueueMessage {
                    message: String::from("ChatFromSimulator"),
                    fields: BTreeMap::from([
                        (String::from("from_name"), String::from("Alpha Resident")),
                        (String::from("message"), String::from("hello")),
                    ]),
                },
                EventQueueMessage {
                    message: String::from("Unrelated"),
                    fields: BTreeMap::new(),
                },
            ],
        };
        let nearby = connection.extract_nearby_chat_messages(&poll);
        assert_eq!(nearby.len(), 1);
        assert_eq!(nearby[0].sender, "Alpha Resident");
        assert_eq!(nearby[0].text, "hello");
    }

    #[test]
    fn parse_event_queue_poll_from_json_flattens_nested_fields() {
        let value = json!({
            "id": 4,
            "events": [{
                "message": "EnableSimulator",
                "body": {
                    "SimulatorInfo": {
                        "Handle": 123,
                        "IP": "16.144.39.130",
                        "Port": 13001
                    }
                }
            }]
        });

        let poll = parse_event_queue_poll_from_json(&value).expect("poll parsing should succeed");
        assert_eq!(poll.id, Some(4));
        assert_eq!(
            poll.events[0]
                .fields
                .get("SimulatorInfo.Handle")
                .map(String::as_str),
            Some("123")
        );
        assert_eq!(
            poll.events[0]
                .fields
                .get("SimulatorInfo.IP")
                .map(String::as_str),
            Some("16.144.39.130")
        );
        assert_eq!(
            poll.events[0]
                .fields
                .get("SimulatorInfo.Port")
                .map(String::as_str),
            Some("13001")
        );
    }

    #[test]
    fn parse_event_queue_poll_from_llsd_xml_flattens_nested_fields() {
        let body = br#"<llsd><map>
            <key>id</key><integer>7</integer>
            <key>events</key><array>
                <map>
                    <key>message</key><string>EstablishAgentCommunication</string>
                    <key>body</key><map>
                        <key>seed-capability</key><string>https://seed.example/cap</string>
                        <key>sim-ip-and-port</key><string>16.144.39.130:13001</string>
                    </map>
                </map>
            </array>
        </map></llsd>"#;

        let poll = parse_event_queue_poll_from_llsd_xml(body).expect("poll parsing should succeed");
        assert_eq!(poll.id, Some(7));
        assert_eq!(
            poll.events[0]
                .fields
                .get("seed-capability")
                .map(String::as_str),
            Some("https://seed.example/cap")
        );
        assert_eq!(
            poll.events[0]
                .fields
                .get("sim-ip-and-port")
                .map(String::as_str),
            Some("16.144.39.130:13001")
        );
    }

    #[test]
    fn parse_event_queue_poll_from_llsd_xml_preserves_binary_ip_field() {
        let body = br#"<llsd><map>
            <key>id</key><integer>8</integer>
            <key>events</key><array>
                <map>
                    <key>message</key><string>EnableSimulator</string>
                    <key>body</key><map>
                        <key>SimulatorInfo</key><array><map>
                            <key>IP</key><binary>EJAngg==</binary>
                            <key>Port</key><integer>13001</integer>
                        </map></array>
                    </map>
                </map>
            </array>
        </map></llsd>"#;

        let poll = parse_event_queue_poll_from_llsd_xml(body).expect("poll parsing should succeed");
        assert_eq!(poll.id, Some(8));
        assert_eq!(
            poll.events[0]
                .fields
                .get("SimulatorInfo[0].IP")
                .map(String::as_str),
            Some("EJAngg==")
        );
        assert_eq!(
            poll.events[0]
                .fields
                .get("SimulatorInfo[0].Port")
                .map(String::as_str),
            Some("13001")
        );
    }

    #[test]
    fn parse_event_queue_poll_from_llsd_xml_accepts_undef_root() {
        let body = br#"<llsd><undef /></llsd>"#;
        let poll = parse_event_queue_poll_from_llsd_xml(body).expect("poll parsing should succeed");
        assert_eq!(poll.id, None);
        assert!(poll.events.is_empty());
    }

    #[test]
    fn parse_event_queue_from_llsd_xml_accepts_undef_root() {
        let body = br#"<llsd><undef /></llsd>"#;
        let inspection =
            parse_event_queue_from_llsd_xml(body).expect("inspection parsing should succeed");
        assert!(!inspection.has_id);
        assert!(!inspection.has_events_array);
        assert_eq!(inspection.event_count, 0);
        assert!(inspection.top_level_keys.is_empty());
        assert!(inspection.event_names.is_empty());
    }

    #[test]
    fn parse_event_queue_poll_from_llsd_xml_accepts_root_array_with_map() {
        let body = br#"<llsd><array>
            <map>
                <key>id</key><integer>9</integer>
                <key>events</key><array>
                    <map>
                        <key>message</key><string>AgentStateUpdate</string>
                        <key>body</key><map><key>can_modify_navmesh</key><boolean>1</boolean></map>
                    </map>
                </array>
            </map>
        </array></llsd>"#;
        let poll = parse_event_queue_poll_from_llsd_xml(body).expect("poll parsing should succeed");
        assert_eq!(poll.id, Some(9));
        assert_eq!(poll.events.len(), 1);
        assert_eq!(poll.events[0].message, "AgentStateUpdate");
        assert_eq!(
            poll.events[0]
                .fields
                .get("can_modify_navmesh")
                .map(String::as_str),
            Some("1")
        );
    }

    #[test]
    fn extract_event_queue_simulator_targets_and_parcel_summaries() {
        let connection = Connection::new(ConnectionConfig::default());
        let poll = EventQueuePollResult {
            id: Some(2),
            events: vec![
                EventQueueMessage {
                    message: String::from("EnableSimulator"),
                    fields: BTreeMap::from([
                        (String::from("SimulatorInfo.Handle"), String::from("123")),
                        (
                            String::from("SimulatorInfo.IP"),
                            String::from("16.144.39.130"),
                        ),
                        (String::from("SimulatorInfo.Port"), String::from("13001")),
                    ]),
                },
                EventQueueMessage {
                    message: String::from("EstablishAgentCommunication"),
                    fields: BTreeMap::from([
                        (
                            String::from("seed-capability"),
                            String::from("https://seed.example/cap"),
                        ),
                        (
                            String::from("sim-ip-and-port"),
                            String::from("16.144.39.130:13001"),
                        ),
                    ]),
                },
                EventQueueMessage {
                    message: String::from("ParcelProperties"),
                    fields: BTreeMap::from([
                        (String::from("local_id"), String::from("55")),
                        (String::from("name"), String::from("Sandbox Parcel")),
                        (String::from("area"), String::from("4096")),
                    ]),
                },
            ],
        };

        let targets = connection.extract_event_queue_simulator_targets(&poll);
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].handle.as_deref(), Some("123"));
        assert_eq!(targets[0].ip.as_deref(), Some("16.144.39.130"));
        assert_eq!(targets[0].port.as_deref(), Some("13001"));
        assert_eq!(targets[0].endpoint_ip.as_deref(), Some("16.144.39.130"));
        assert_eq!(targets[0].endpoint_port, Some(13001));
        assert_eq!(
            targets[0].endpoint_source.as_deref(),
            Some("ip_port_fields")
        );
        assert_eq!(
            targets[1].seed_capability.as_deref(),
            Some("https://seed.example/cap")
        );
        assert_eq!(
            targets[1].sim_ip_and_port.as_deref(),
            Some("16.144.39.130:13001")
        );
        assert_eq!(targets[1].endpoint_ip.as_deref(), Some("16.144.39.130"));
        assert_eq!(targets[1].endpoint_port, Some(13001));
        assert_eq!(
            targets[1].endpoint_source.as_deref(),
            Some("sim_ip_and_port")
        );

        let parcels = connection.extract_event_queue_parcel_summaries(&poll);
        assert_eq!(parcels.len(), 1);
        assert_eq!(parcels[0].local_id.as_deref(), Some("55"));
        assert_eq!(parcels[0].name.as_deref(), Some("Sandbox Parcel"));
        assert_eq!(parcels[0].area.as_deref(), Some("4096"));

        assert_eq!(
            connection.summarize_event_queue_event_fields(&poll.events[2], 2),
            "area=4096;local_id=55"
        );
    }

    #[test]
    fn extract_event_queue_simulator_targets_decodes_binary_ip_endpoint() {
        let connection = Connection::new(ConnectionConfig::default());
        let poll = EventQueuePollResult {
            id: Some(3),
            events: vec![EventQueueMessage {
                message: String::from("EnableSimulator"),
                fields: BTreeMap::from([
                    (String::from("SimulatorInfo.IP"), String::from("EJAngg==")),
                    (String::from("SimulatorInfo.Port"), String::from("13001")),
                ]),
            }],
        };
        let targets = connection.extract_event_queue_simulator_targets(&poll);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].ip.as_deref(), Some("EJAngg=="));
        assert_eq!(targets[0].endpoint_ip.as_deref(), Some("16.144.39.130"));
        assert_eq!(targets[0].endpoint_port, Some(13001));
        assert_eq!(
            targets[0].endpoint_source.as_deref(),
            Some("simulatorinfo_binary_ip_port")
        );
    }

    #[tokio::test]
    async fn send_nearby_chat_uses_lludp_chat_from_viewer_and_decodes_response() {
        let listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("listener bind should succeed");
        let listener_addr = listener
            .local_addr()
            .expect("listener address should exist");

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": listener_addr.port(),
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })))
            .mount(&server)
            .await;

        tokio::spawn(async move {
            let mut buf = [0u8; 4096];
            let (_, sender) = listener
                .recv_from(&mut buf)
                .await
                .expect("use circuit should arrive");
            let (_, sender_2) = listener
                .recv_from(&mut buf)
                .await
                .expect("complete movement should arrive");
            assert_eq!(sender, sender_2);
            let (chat_len, chat_sender) = listener
                .recv_from(&mut buf)
                .await
                .expect("chat packet should arrive");
            assert_eq!(sender, chat_sender);
            let (flags, _, message_number, body) =
                decode_outbound_handshake_payload(&buf[..chat_len]);
            assert_eq!(flags & LLUDP_RELIABLE_FLAG, LLUDP_RELIABLE_FLAG);
            assert_eq!(
                message_number,
                lludp_low_frequency_message_number(LLUDP_CHAT_FROM_VIEWER_LOW_ID)
            );
            assert!(body.len() > 34);
            let msg_len = u16::from_le_bytes([body[32], body[33]]) as usize;
            let msg = &body[34..34 + msg_len];
            assert_eq!(msg.last().copied(), Some(0));
            assert_eq!(
                std::str::from_utf8(&msg[..msg_len - 1]).expect("message should be utf-8"),
                "test message"
            );
            let _ = listener
                .send_to(
                    &make_chat_from_simulator_packet("Echo Resident", "roger that"),
                    chat_sender,
                )
                .await;
        });

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        let adapter = SecondLifeAdapter;
        connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");

        let received = connection
            .send_nearby_chat("test message", "127.0.0.1:0", Duration::from_secs(1), 4)
            .await
            .expect("chat send should succeed");
        assert_eq!(received.len(), 1);
        assert_eq!(received[0].sender, "Echo Resident");
        assert_eq!(received[0].text, "roger that");
    }

    #[tokio::test]
    async fn poll_nearby_chat_on_circuit_reuses_existing_socket_without_resending_handshake() {
        let listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("listener bind should succeed");
        let listener_addr = listener
            .local_addr()
            .expect("listener address should exist");

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": true,
                "reason": "connect",
                "agent_id": "11111111-1111-1111-1111-111111111111",
                "session_id": "22222222-2222-2222-2222-222222222222",
                "secure_session_id": "33333333-3333-3333-3333-333333333333",
                "circuit_code": 424242,
                "sim_ip": "127.0.0.1",
                "sim_port": listener_addr.port(),
                "region_x": 1000,
                "region_y": 1001,
                "seed_capability": "https://seed-cap.example.invalid"
            })))
            .mount(&server)
            .await;

        let listener_task = tokio::spawn(async move {
            let mut buf = [0u8; 4096];
            let (_, sender) = listener
                .recv_from(&mut buf)
                .await
                .expect("use circuit should arrive");
            let (_, sender_2) = listener
                .recv_from(&mut buf)
                .await
                .expect("complete movement should arrive");
            assert_eq!(sender, sender_2);
            let _ = listener
                .send_to(
                    &make_chat_from_simulator_packet("Echo Resident", "reuse path"),
                    sender,
                )
                .await;
            let extra = timeout(Duration::from_millis(300), listener.recv_from(&mut buf)).await;
            extra.is_ok()
        });

        let mut connection = Connection::new(ConnectionConfig {
            endpoint: format!("{}/login", server.uri()),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        });
        connection.connect().await.expect("connect should succeed");
        let adapter = SecondLifeAdapter;
        connection
            .login_with_adapter(&adapter, make_intent(true))
            .await
            .expect("login should succeed");

        let circuit = connection
            .open_social_circuit("127.0.0.1:0")
            .await
            .expect("social circuit should open");
        let received = connection
            .poll_nearby_chat_on_circuit(&circuit, Duration::from_secs(1), 1)
            .await
            .expect("nearby poll should succeed");

        assert_eq!(received.len(), 1);
        assert_eq!(received[0].sender, "Echo Resident");
        assert_eq!(received[0].text, "reuse path");
        let saw_extra_datagram = listener_task.await.expect("listener should complete");
        assert!(!saw_extra_datagram);
        let socket_summary = connection.summarize_first_simulator_socket_diagnostics();
        assert_eq!(socket_summary.fresh_bind_events, 1);
        assert_eq!(socket_summary.reused_retained_probe_events, 0);
        assert_eq!(socket_summary.receive_events, 1);
        assert_eq!(socket_summary.unique_local_ports.len(), 1);
        assert!(!socket_summary.split_local_port_detected);
    }

    #[test]
    fn llsd_codec_decodes_buddy_list_entries() {
        let codec = LlsdLoginCodec;
        let body = br#"
<llsd><map>
<key>login</key><boolean>true</boolean>
<key>agent_id</key><string>11111111-1111-1111-1111-111111111111</string>
<key>session_id</key><string>22222222-2222-2222-2222-222222222222</string>
<key>secure_session_id</key><string>33333333-3333-3333-3333-333333333333</string>
<key>circuit_code</key><integer>424242</integer>
<key>sim_ip</key><string>127.0.0.1</string>
<key>sim_port</key><integer>13000</integer>
<key>region_x</key><integer>1000</integer>
<key>region_y</key><integer>1001</integer>
<key>seed_capability</key><string>https://seed-cap.example.invalid</string>
<key>buddy-list</key><array>
  <map>
    <key>buddy_id</key><uuid>aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa</uuid>
    <key>buddy_rights_has</key><integer>1</integer>
    <key>buddy_rights_given</key><integer>2</integer>
  </map>
</array>
</map></llsd>
"#;
        let response = codec
            .decode_response(body)
            .expect("llsd decode should succeed");
        assert_eq!(response.buddy_list.len(), 1);
        assert_eq!(
            response.buddy_list[0].buddy_id,
            "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa"
        );
        assert_eq!(response.buddy_list[0].rights_has, 1);
        assert_eq!(response.buddy_list[0].rights_given, 2);
    }

    #[test]
    fn decode_social_events_extracts_online_notification_ids() {
        let mut payload = make_low_frequency_packet(LLUDP_ONLINE_NOTIFICATION_LOW_ID);
        payload.push(2);
        payload.extend_from_slice(&[0xaa; 16]);
        payload.extend_from_slice(&[0xbb; 16]);
        let events = decode_social_events(&payload);
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], SocialEvent::FriendOnline { .. }));
    }

    #[test]
    fn parse_resolved_avatar_names_supports_json_agents_array() {
        let body = br#"{
            "agents":[
                {"id":"aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa","display_name":"Alpha Resident"}
            ]
        }"#;
        let names = parse_resolved_avatar_names(body, Some("application/json"))
            .expect("name response should parse");
        assert_eq!(names.len(), 1);
        assert_eq!(names[0].id, "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa");
        assert_eq!(names[0].display_name, "Alpha Resident");
    }

    #[test]
    fn parse_resolved_avatar_names_supports_llsd_agents_array() {
        let body = br#"
<llsd><map>
  <key>agents</key><array>
    <map>
      <key>id</key><uuid>bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb</uuid>
      <key>display_name</key><string>Bravo Resident</string>
    </map>
  </array>
</map></llsd>
"#;
        let names = parse_resolved_avatar_names(body, Some("application/llsd+xml"))
            .expect("name response should parse");
        assert_eq!(names.len(), 1);
        assert_eq!(names[0].id, "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb");
        assert_eq!(names[0].display_name, "Bravo Resident");
    }
}
