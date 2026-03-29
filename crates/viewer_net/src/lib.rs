use quick_xml::Reader;
use quick_xml::events::Event;
use reqwest::{
    Method, StatusCode,
    header::{ACCEPT, CONTENT_TYPE},
};
use roxmltree::{Document, Node};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
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

const MAX_LOGIN_REDIRECTS: usize = 4;
const MAX_EVENT_QUEUE_ONE_SHOT_ATTEMPTS: usize = 3;
const EVENT_QUEUE_ONE_SHOT_MIN_TIMEOUT: Duration = Duration::from_secs(35);
const EVENT_QUEUE_ONE_SHOT_RETRY_BASE_DELAY: Duration = Duration::from_millis(250);
const EVENT_QUEUE_ONE_SHOT_RETRY_MAX_DELAY: Duration = Duration::from_secs(2);
const LLSD_XML_CONTENT_TYPE: &str = "application/llsd+xml";
const LLUDP_PACKET_ID_SIZE: usize = 6;
const LLUDP_MINIMUM_VALID_PACKET_SIZE: usize = LLUDP_PACKET_ID_SIZE + 1;
const LLUDP_MESSAGE_PREFIX: u8 = 0xFF;
const LLUDP_RELIABLE_FLAG: u8 = 0x40;
const LLUDP_LOW_FREQUENCY_PREFIX: u32 = 0xFFFF0000;
const LLUDP_USE_CIRCUIT_CODE_LOW_ID: u16 = 3;
const LLUDP_CHAT_FROM_VIEWER_LOW_ID: u16 = 80;
const LLUDP_COMPLETE_AGENT_MOVEMENT_LOW_ID: u16 = 249;
const LLUDP_TEST_MESSAGE_LOW_ID: u16 = 1;
const LLUDP_REGION_HANDSHAKE_LOW_ID: u16 = 148;
const LLUDP_HEALTH_MESSAGE_LOW_ID: u16 = 138;
const LLUDP_CHAT_FROM_SIMULATOR_LOW_ID: u16 = 139;
const LLUDP_SIMULATOR_VIEWER_TIME_LOW_ID: u16 = 150;
const LLUDP_ENABLE_SIMULATOR_LOW_ID: u16 = 151;
const LLUDP_AGENT_MOVEMENT_COMPLETE_LOW_ID: u16 = 250;
const LLUDP_AGENT_DATA_UPDATE_LOW_ID: u16 = 387;
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
const LLUDP_OBJECT_UPDATE_HIGH_ID: u8 = 12;
const LLUDP_OBJECT_UPDATE_COMPRESSED_HIGH_ID: u8 = 13;
const LLUDP_OBJECT_UPDATE_CACHED_HIGH_ID: u8 = 14;
const LLUDP_IMPROVED_TERSE_OBJECT_UPDATE_HIGH_ID: u8 = 15;
const LLUDP_KILL_OBJECT_HIGH_ID: u8 = 16;
const LLUDP_VIEWER_EFFECT_MEDIUM_ID: u8 = 17;
const LLUDP_COARSE_LOCATION_UPDATE_MEDIUM_ID: u8 = 6;
const LLUDP_ATTACHED_SOUND_MEDIUM_ID: u8 = 13;
const LLUDP_CROSSED_REGION_MEDIUM_ID: u8 = 7;
const LLUDP_CONFIRM_ENABLE_SIMULATOR_MEDIUM_ID: u8 = 8;
const MAX_OBJECT_FEED_OBJECTS: usize = 1024;
const MAX_OBJECT_FEED_EXPORT_OBJECTS: usize = 128;
const MAX_OBJECT_FEED_RECENT_KILLS: usize = 128;
const MAX_ZEROCODED_BODY_BYTES: usize = 256 * 1024;
const MAX_CONTINUITY_NEIGHBORS: usize = 8;
const MAX_CONTINUITY_OBSERVATIONS: usize = 16;
const DEFAULT_SEED_CAPABILITY_REQUEST: &[&str] = &[
    "EventQueueGet",
    "AgentProfile",
    "GetTexture",
    "GetDisplayNames",
    "SimulatorFeatures",
    "MapLayer",
    "ViewerAsset",
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

fn llsd_string(key: &str, value: &str) -> String {
    format!(
        "<key>{}</key><string>{}</string>",
        escape_xml(key),
        escape_xml(value)
    )
}

fn llsd_boolean(key: &str, value: bool) -> String {
    format!("<key>{}</key><boolean>{}</boolean>", escape_xml(key), value)
}

fn xmlrpc_member_string(xml: &mut String, name: &str, value: &str) {
    xml.push_str("<member>");
    xml.push_str(&format!("<name>{}</name>", escape_xml(name)));
    xml.push_str(&format!(
        "<value><string>{}</string></value>",
        escape_xml(value)
    ));
    xml.push_str("</member>");
}

fn xmlrpc_member_bool(xml: &mut String, name: &str, value: bool) {
    let xmlrpc_bool = if value { "1" } else { "0" };
    xml.push_str("<member>");
    xml.push_str(&format!("<name>{}</name>", escape_xml(name)));
    xml.push_str(&format!("<value><boolean>{xmlrpc_bool}</boolean></value>"));
    xml.push_str("</member>");
}

fn xmlrpc_member_array_of_strings(xml: &mut String, name: &str, values: &[String]) {
    xml.push_str("<member>");
    xml.push_str(&format!("<name>{}</name>", escape_xml(name)));
    xml.push_str("<value><array><data>");
    for value in values {
        xml.push_str(&format!(
            "<value><string>{}</string></value>",
            escape_xml(value)
        ));
    }
    xml.push_str("</data></array></value>");
    xml.push_str("</member>");
}

#[derive(Debug, Clone)]
enum XmlRpcValue {
    String(String),
    Int(i64),
    Bool(bool),
    Struct(HashMap<String, XmlRpcValue>),
    Array(Vec<XmlRpcValue>),
}

fn first_child_with_tag<'a, 'i>(node: Node<'a, 'i>, tag: &str) -> Option<Node<'a, 'i>> {
    node.children()
        .find(|child| child.is_element() && child.has_tag_name(tag))
}

fn parse_xmlrpc_value(value_node: Node<'_, '_>) -> Result<XmlRpcValue, CodecError> {
    let typed_child = value_node.children().find(|child| child.is_element());
    let Some(typed_child) = typed_child else {
        let text = value_node
            .text()
            .map(str::trim)
            .unwrap_or_default()
            .to_string();
        return Ok(XmlRpcValue::String(text));
    };

    match typed_child.tag_name().name() {
        "string" => Ok(XmlRpcValue::String(
            typed_child.text().unwrap_or_default().to_string(),
        )),
        "int" | "i4" => {
            let parsed = typed_child
                .text()
                .unwrap_or_default()
                .trim()
                .parse::<i64>()
                .map_err(|err| CodecError::Deserialize(err.to_string()))?;
            Ok(XmlRpcValue::Int(parsed))
        }
        "boolean" => {
            let bool_text = typed_child.text().unwrap_or_default().trim();
            let parsed = matches!(bool_text, "1" | "true" | "TRUE");
            Ok(XmlRpcValue::Bool(parsed))
        }
        "double" => Ok(XmlRpcValue::String(
            typed_child.text().unwrap_or_default().to_string(),
        )),
        "struct" => {
            let mut map = HashMap::new();
            for member in typed_child
                .children()
                .filter(|child| child.has_tag_name("member"))
            {
                let name = first_child_with_tag(member, "name")
                    .and_then(|name_node| name_node.text())
                    .ok_or_else(|| {
                        CodecError::Deserialize(String::from(
                            "xml-rpc struct member missing name text",
                        ))
                    })?
                    .to_string();
                let child_value = first_child_with_tag(member, "value").ok_or_else(|| {
                    CodecError::Deserialize(String::from("xml-rpc struct member missing value"))
                })?;
                let parsed_value = parse_xmlrpc_value(child_value)?;
                map.insert(name, parsed_value);
            }
            Ok(XmlRpcValue::Struct(map))
        }
        "array" => {
            let data_node = first_child_with_tag(typed_child, "data").ok_or_else(|| {
                CodecError::Deserialize(String::from("xml-rpc array missing data node"))
            })?;
            let mut values = Vec::new();
            for child_value in data_node
                .children()
                .filter(|child| child.has_tag_name("value"))
            {
                values.push(parse_xmlrpc_value(child_value)?);
            }
            Ok(XmlRpcValue::Array(values))
        }
        other => Err(CodecError::Deserialize(format!(
            "unsupported xml-rpc value type: {other}"
        ))),
    }
}

fn xmlrpc_as_string(value: &XmlRpcValue) -> Option<String> {
    match value {
        XmlRpcValue::String(text) => Some(text.clone()),
        XmlRpcValue::Int(value) => Some(value.to_string()),
        XmlRpcValue::Bool(value) => Some(value.to_string()),
        XmlRpcValue::Struct(_) => None,
        XmlRpcValue::Array(values) => Some(
            values
                .iter()
                .filter_map(xmlrpc_as_string)
                .collect::<Vec<_>>()
                .join(","),
        ),
    }
}

fn xmlrpc_as_bool(value: &XmlRpcValue) -> Option<bool> {
    match value {
        XmlRpcValue::Bool(value) => Some(*value),
        XmlRpcValue::Int(value) => Some(*value != 0),
        XmlRpcValue::String(text) => match text.to_ascii_lowercase().as_str() {
            "true" | "1" => Some(true),
            "false" | "0" => Some(false),
            _ => None,
        },
        XmlRpcValue::Struct(_) | XmlRpcValue::Array(_) => None,
    }
}

fn xmlrpc_as_u32(value: &XmlRpcValue) -> Option<u32> {
    match value {
        XmlRpcValue::Int(value) => u32::try_from(*value).ok(),
        XmlRpcValue::String(text) => text.parse::<u32>().ok(),
        XmlRpcValue::Bool(value) => Some(if *value { 1 } else { 0 }),
        XmlRpcValue::Struct(_) | XmlRpcValue::Array(_) => None,
    }
}

fn xmlrpc_as_u16(value: &XmlRpcValue) -> Option<u16> {
    xmlrpc_as_u32(value).and_then(|value| u16::try_from(value).ok())
}

fn xmlrpc_as_buddy_list(value: &XmlRpcValue) -> Option<Vec<FriendBootstrapEntry>> {
    let XmlRpcValue::Array(items) = value else {
        return None;
    };
    let mut buddies = Vec::new();
    for item in items {
        let XmlRpcValue::Struct(map) = item else {
            continue;
        };
        let buddy_id = map
            .get("buddy_id")
            .and_then(xmlrpc_as_string)
            .unwrap_or_default();
        if buddy_id.is_empty() {
            continue;
        }
        let rights_has = map
            .get("buddy_rights_has")
            .and_then(xmlrpc_as_u32)
            .map(|v| v as i32)
            .unwrap_or_default();
        let rights_given = map
            .get("buddy_rights_given")
            .and_then(xmlrpc_as_u32)
            .map(|v| v as i32)
            .unwrap_or_default();
        buddies.push(FriendBootstrapEntry {
            buddy_id,
            rights_has,
            rights_given,
        });
    }
    Some(buddies)
}

fn escape_xml(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            other => escaped.push(other),
        }
    }
    escaped
}

fn parse_llsd_map(body: &[u8]) -> Result<HashMap<String, LlsdValue>, CodecError> {
    let mut reader = Reader::from_reader(body);
    reader.trim_text(true);
    let mut map = HashMap::new();
    let mut map_depth: usize = 0;
    let mut current_key: Option<String> = None;
    let mut current_tag: Option<TagType> = None;

    loop {
        let event = match reader.read_event() {
            Ok(event) => event,
            Err(err) => return Err(CodecError::Deserialize(err.to_string())),
        };

        match event {
            Event::Start(ref e) => match e.name().as_ref() {
                b"map" => map_depth += 1,
                b"key" if map_depth == 1 => current_tag = Some(TagType::Key),
                b"string" if map_depth == 1 => current_tag = Some(TagType::String),
                b"integer" if map_depth == 1 => current_tag = Some(TagType::Integer),
                b"boolean" if map_depth == 1 => current_tag = Some(TagType::Boolean),
                _ => {}
            },
            Event::Empty(ref e) => {
                if map_depth == 1 {
                    if let Some(key) = current_key.take() {
                        match e.name().as_ref() {
                            b"true" => {
                                map.insert(key, LlsdValue::Bool(true));
                            }
                            b"false" => {
                                map.insert(key, LlsdValue::Bool(false));
                            }
                            _ => {}
                        }
                    }
                    current_tag = None;
                }
            }
            Event::Text(e) => {
                if map_depth == 1
                    && let Ok(text) = e.unescape()
                {
                    let text = text.into_owned();
                    match current_tag.take() {
                        Some(TagType::Key) => {
                            current_key = Some(text);
                        }
                        Some(tag) => {
                            if let Some(key) = current_key.take() {
                                match tag {
                                    TagType::String => {
                                        map.insert(key, LlsdValue::String(text));
                                    }
                                    TagType::Integer => {
                                        if let Ok(value) = text.parse::<i64>() {
                                            map.insert(key, LlsdValue::Integer(value));
                                        }
                                    }
                                    TagType::Boolean => {
                                        let normalized =
                                            matches!(text.to_lowercase().as_str(), "true" | "1");
                                        map.insert(key, LlsdValue::Bool(normalized));
                                    }
                                    TagType::Key => {}
                                }
                            }
                        }
                        None => {}
                    }
                }
            }
            Event::End(ref e) => {
                if e.name().as_ref() == b"map" {
                    map_depth = map_depth.saturating_sub(1);
                }
                current_tag = None;
            }
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(map)
}

fn parse_llsd_buddy_list(body: &[u8]) -> Result<Vec<FriendBootstrapEntry>, CodecError> {
    let text = std::str::from_utf8(body).map_err(|err| CodecError::Deserialize(err.to_string()))?;
    let doc = Document::parse(text).map_err(|err| CodecError::Deserialize(err.to_string()))?;
    let top_map = doc
        .descendants()
        .find(|node| node.has_tag_name("map"))
        .ok_or_else(|| CodecError::Deserialize(String::from("missing llsd map")))?;
    let children: Vec<Node<'_, '_>> = top_map
        .children()
        .filter(|node| node.is_element())
        .collect();
    let mut idx = 0usize;
    while idx + 1 < children.len() {
        let key_node = children[idx];
        let value_node = children[idx + 1];
        if key_node.has_tag_name("key") && key_node.text().unwrap_or_default() == "buddy-list" {
            if !value_node.has_tag_name("array") {
                return Ok(Vec::new());
            }
            let mut buddies = Vec::new();
            for item in value_node
                .children()
                .filter(|node| node.has_tag_name("map"))
            {
                let map = parse_llsd_scalar_map(item);
                let buddy_id = map.get("buddy_id").cloned().unwrap_or_default();
                if buddy_id.is_empty() {
                    continue;
                }
                let rights_has = map
                    .get("buddy_rights_has")
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or_default();
                let rights_given = map
                    .get("buddy_rights_given")
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or_default();
                buddies.push(FriendBootstrapEntry {
                    buddy_id,
                    rights_has,
                    rights_given,
                });
            }
            return Ok(buddies);
        }
        idx += 2;
    }
    Ok(Vec::new())
}

enum TagType {
    Key,
    String,
    Integer,
    Boolean,
}

enum LlsdValue {
    String(String),
    Integer(i64),
    Bool(bool),
}

impl LlsdValue {
    fn as_integer(&self) -> Option<i64> {
        match self {
            LlsdValue::Integer(value) => Some(*value),
            LlsdValue::String(text) => text.parse().ok(),
            LlsdValue::Bool(_) => None,
        }
    }
}

fn llsd_to_string(value: &LlsdValue) -> Option<String> {
    match value {
        LlsdValue::String(text) => Some(text.clone()),
        LlsdValue::Integer(num) => Some(num.to_string()),
        LlsdValue::Bool(flag) => Some(flag.to_string()),
    }
}

fn llsd_to_bool(value: &LlsdValue) -> Option<bool> {
    match value {
        LlsdValue::Bool(flag) => Some(*flag),
        LlsdValue::String(text) => match text.to_lowercase().as_str() {
            "true" | "1" => Some(true),
            "false" | "0" => Some(false),
            _ => None,
        },
        LlsdValue::Integer(num) => Some(*num != 0),
    }
}

fn llsd_to_u32(value: &LlsdValue) -> Option<u32> {
    value.as_integer().and_then(|i| u32::try_from(i).ok())
}

fn llsd_to_u16(value: &LlsdValue) -> Option<u16> {
    value.as_integer().and_then(|i| u16::try_from(i).ok())
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
    pub payload_len: usize,
    pub elapsed_ms: u128,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FirstSimulatorInboundMessageKind {
    TestMessage,
    PacketAck,
    AgentMovementComplete,
    RegionHandshake,
    HealthMessage,
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
    ObjectUpdate,
    ObjectUpdateCompressed,
    ObjectUpdateCached,
    ImprovedTerseObjectUpdate,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecodedSimulatorViewerTimeMessage {
    pub body_len: u16,
    pub signature: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecodedObjectFeedObject {
    pub local_id: u32,
    pub scale_centi: Option<[u16; 3]>,
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
    pub object_feed_kill_messages: usize,
    pub object_feed_decode_dropped: usize,
    pub object_feed_evicted: usize,
    pub object_feed_total_objects: usize,
    pub object_feed_export_truncated: bool,
    pub object_feed_objects: Vec<DecodedObjectFeedObject>,
    pub object_feed_recent_kills: Vec<u32>,
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
    first_simulator_handshake_send_diagnostics: Vec<FirstSimulatorHandshakeSendDiagnostic>,
    first_simulator_handshake_receive_diagnostics: Vec<FirstSimulatorHandshakeReceiveDiagnostic>,
    early_simulator_traffic_observations: Vec<EarlySimulatorTrafficObservation>,
    region_transition_control_observations: Vec<RegionTransitionControlObservation>,
    simulator_payload_decode_summary: SimulatorPayloadDecodeSummary,
    object_feed_objects: BTreeMap<u32, ObjectFeedObjectState>,
    object_feed_recent_kills: Vec<u32>,
    object_feed_tick: u64,
    next_first_simulator_packet_id: u32,
    continuity_summary: RegionContinuitySummary,
    last_phase_change_at: Instant,
}

#[derive(Debug, Clone, Copy, Default)]
struct ObjectFeedObjectState {
    last_seen_tick: u64,
    scale_centi: Option<[u16; 3]>,
}

impl Connection {
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            state: ConnectionState::Disconnected,
            session: None,
            first_simulator_handshake_prerequisites: None,
            first_simulator_handshake_state: None,
            first_simulator_handshake_send_diagnostics: Vec::new(),
            first_simulator_handshake_receive_diagnostics: Vec::new(),
            early_simulator_traffic_observations: Vec::new(),
            region_transition_control_observations: Vec::new(),
            simulator_payload_decode_summary: SimulatorPayloadDecodeSummary::default(),
            object_feed_objects: BTreeMap::new(),
            object_feed_recent_kills: Vec::new(),
            object_feed_tick: 0,
            next_first_simulator_packet_id: 1,
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

    fn object_feed_upsert(&mut self, local_id: u32, scale_centi: Option<[u16; 3]>) {
        if local_id == 0 {
            return;
        }
        self.object_feed_tick = self.object_feed_tick.saturating_add(1);
        let entry = self.object_feed_objects.entry(local_id).or_default();
        entry.last_seen_tick = self.object_feed_tick;
        if scale_centi.is_some() {
            entry.scale_centi = scale_centi;
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
            .object_feed_export_truncated =
            self.object_feed_objects.len() > MAX_OBJECT_FEED_EXPORT_OBJECTS;

        let mut entries: Vec<(u32, ObjectFeedObjectState)> = self
            .object_feed_objects
            .iter()
            .map(|(id, state)| (*id, *state))
            .collect();
        entries.sort_by(|a, b| {
            b.1.last_seen_tick
                .cmp(&a.1.last_seen_tick)
                .then_with(|| a.0.cmp(&b.0))
        });
        entries.truncate(MAX_OBJECT_FEED_EXPORT_OBJECTS);
        let mut export: Vec<DecodedObjectFeedObject> = entries
            .into_iter()
            .map(|(local_id, state)| DecodedObjectFeedObject {
                local_id,
                scale_centi: state.scale_centi,
            })
            .collect();
        export.sort_by_key(|obj| obj.local_id);
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
        self.first_simulator_handshake_send_diagnostics.clear();
        self.first_simulator_handshake_receive_diagnostics.clear();
        self.early_simulator_traffic_observations.clear();
        self.region_transition_control_observations.clear();
        self.simulator_payload_decode_summary = SimulatorPayloadDecodeSummary::default();
        self.object_feed_objects.clear();
        self.object_feed_recent_kills.clear();
        self.object_feed_tick = 0;
        self.next_first_simulator_packet_id = 1;
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
        self.first_simulator_handshake_send_diagnostics.clear();
        self.first_simulator_handshake_receive_diagnostics.clear();
        self.early_simulator_traffic_observations.clear();
        self.region_transition_control_observations.clear();
        self.simulator_payload_decode_summary = SimulatorPayloadDecodeSummary::default();
        self.object_feed_objects.clear();
        self.object_feed_recent_kills.clear();
        self.object_feed_tick = 0;
        self.next_first_simulator_packet_id = 1;
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
            && let Some(sim_name) = decode_region_handshake_sim_name(payload)
        {
            self.simulator_payload_decode_summary
                .region_handshake_updates += 1;
            self.simulator_payload_decode_summary
                .region_handshake_last_sim_name = Some(sim_name);
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
                    if let Some(objects) = decode_object_update_ids_and_scales(payload) {
                        for (local_id, scale_centi) in objects {
                            self.object_feed_upsert(local_id, scale_centi);
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
                    if let Some(ids) = decode_object_update_compressed_local_ids(payload) {
                        for local_id in ids {
                            self.object_feed_upsert(local_id, None);
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
                    if let Some(ids) = decode_object_update_cached_local_ids(payload) {
                        for local_id in ids {
                            self.object_feed_upsert(local_id, None);
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
                    if let Some(ids) = decode_improved_terse_object_update_local_ids(payload) {
                        for local_id in ids {
                            self.object_feed_upsert(local_id, None);
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
        let mut buf = vec![0u8; 2048];

        let recv = timeout(wait_timeout, socket.recv_from(&mut buf)).await;
        let (received_len, _) = match recv {
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

        self.observe_first_simulator_inbound_payload(&buf[..received_len])
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

        if self.first_simulator_handshake_state.is_none() {
            self.begin_first_simulator_handshake_scaffold()?;
        }

        let socket = UdpSocket::bind(bind).await.map_err(|err| {
            ConnectionError::InvalidFirstSimulatorReceiveBind {
                bind: bind.to_string(),
                reason: err.to_string(),
            }
        })?;

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
            let (received_len, _) = match recv {
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

            let classification =
                self.observe_first_simulator_inbound_payload(&buf[..received_len])?;
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
        let client = reqwest::Client::builder()
            .timeout(self.config.connect_timeout.max(Duration::from_secs(15)))
            .build()?;
        let base = image_cap_url.trim_end_matches('/');
        let candidates = [
            format!("{base}?texture_id={asset_id}"),
            format!("{base}/?texture_id={asset_id}"),
            format!("{base}/{asset_id}"),
            format!("{base}?id={asset_id}"),
            format!("{base}?asset_id={asset_id}"),
        ];
        let mut last_error: Option<ConnectionError> = None;
        for url in &candidates {
            let response = match client
                .get(url)
                .header(ACCEPT, "image/x-j2c,image/jp2,image/*,*/*")
                .send()
                .await
            {
                Ok(response) => response,
                Err(err) => {
                    last_error = Some(ConnectionError::Http(err));
                    continue;
                }
            };
            let status = response.status();
            let bytes = response.bytes().await?;
            if status.is_success() {
                return Ok(bytes.to_vec());
            }
            last_error = Some(ConnectionError::HttpStatus {
                status,
                body: String::from_utf8_lossy(&bytes).to_string(),
            });
        }
        Err(last_error.unwrap_or_else(|| {
            ConnectionError::CapabilityDecode(String::from("profile image fetch failed"))
        }))
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
        let (prerequisites, socket) = self.prepare_chat_socket(bind).await?;

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
        let (_prerequisites, socket) = self.prepare_chat_socket(bind).await?;
        self.receive_nearby_chat_on_socket(&socket, bind, receive_timeout, receive_max_packets)
            .await
    }

    pub async fn open_social_circuit(
        &mut self,
        bind: &str,
    ) -> Result<SocialCircuit, ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }
        let (prerequisites, socket) = self.prepare_chat_socket(bind).await?;
        Ok(SocialCircuit {
            bind: bind.to_string(),
            target: prerequisites.target,
            socket,
        })
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
        for _ in 0..receive_max_packets {
            if started.elapsed() > Duration::from_secs(8) {
                break;
            }
            let recv = timeout(receive_timeout, circuit.socket.recv_from(&mut buf)).await;
            let (received_len, _) = match recv {
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
            let _ = self.observe_first_simulator_inbound_payload(packet);
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
        let mut events = Vec::new();
        let mut buf = vec![0u8; 4096];
        for _ in 0..receive_max_packets {
            let recv = timeout(receive_timeout, circuit.socket.recv_from(&mut buf)).await;
            let (received_len, _) = match recv {
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
            let _ = self.observe_first_simulator_inbound_payload(payload);
            events.extend(decode_social_events(payload));
        }
        Ok(events)
    }

    pub async fn fetch_simulator_features_once(
        &self,
        simulator_features_url: &str,
    ) -> Result<SimulatorFeaturesInspection, ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }

        let client = reqwest::Client::builder()
            .timeout(self.config.connect_timeout)
            .build()?;

        let response = client
            .get(simulator_features_url)
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

        parse_simulator_features_response(&bytes, content_type.as_deref())
    }

    pub async fn fetch_map_layer_once(
        &self,
        map_layer_url: &str,
    ) -> Result<MapLayerInspection, ConnectionError> {
        if self.state != ConnectionState::LoggedIn {
            return Err(ConnectionError::InvalidState(self.state));
        }

        let client = reqwest::Client::builder()
            .timeout(self.config.connect_timeout)
            .build()?;

        let response = client
            .get(map_layer_url)
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
        self.first_simulator_handshake_send_diagnostics.clear();
        self.first_simulator_handshake_receive_diagnostics.clear();
        self.early_simulator_traffic_observations.clear();
        self.region_transition_control_observations.clear();
        self.simulator_payload_decode_summary = SimulatorPayloadDecodeSummary::default();
        self.object_feed_objects.clear();
        self.object_feed_recent_kills.clear();
        self.object_feed_tick = 0;
        self.next_first_simulator_packet_id = 1;
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
                        payload_len: payload.len(),
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
        let send_result = socket.send_to(payload, socket_addr).await;
        let elapsed_ms = started.elapsed().as_millis();

        match send_result {
            Ok(sent_len) if sent_len == payload.len() => {
                self.first_simulator_handshake_send_diagnostics.push(
                    FirstSimulatorHandshakeSendDiagnostic {
                        action,
                        target: target_text,
                        packet_id,
                        packet_message_number,
                        payload_len: payload.len(),
                        elapsed_ms,
                        success: true,
                        error: None,
                    },
                );
                Ok(())
            }
            Ok(sent_len) => {
                let reason = format!("partial datagram send ({sent_len}/{})", payload.len());
                self.first_simulator_handshake_send_diagnostics.push(
                    FirstSimulatorHandshakeSendDiagnostic {
                        action,
                        target: target_text.clone(),
                        packet_id,
                        packet_message_number,
                        payload_len: payload.len(),
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
                        payload_len: payload.len(),
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
        for _ in 0..receive_max_packets {
            let recv = timeout(receive_timeout, socket.recv_from(&mut buf)).await;
            let (received_len, _) = match recv {
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
            let _ = self.observe_first_simulator_inbound_payload(payload);
            if let Some(chat) = decode_chat_from_simulator(payload) {
                nearby.push(chat);
            }
        }
        Ok(nearby)
    }
}

pub async fn fetch_asset_bytes(url: &str, timeout: Duration) -> Result<Vec<u8>, ConnectionError> {
    let client = reqwest::Client::builder()
        .timeout(timeout.max(Duration::from_secs(1)))
        .build()?;
    let response = client.get(url).send().await?;
    if response.status().is_success() {
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    } else {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("unreadable body"));
        Err(ConnectionError::HttpStatus { status, body })
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

fn decode_lludp_packet_id(payload: &[u8]) -> Option<u32> {
    if payload.len() < LLUDP_PACKET_ID_SIZE {
        return None;
    }
    let id_bytes: [u8; 4] = payload[1..5].try_into().ok()?;
    Some(u32::from_be_bytes(id_bytes))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FirstSimulatorPacketHeader {
    message_number: u32,
    body_offset: usize,
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

    Some(FirstSimulatorPacketHeader {
        message_number,
        body_offset: header_start + consumed_header_bytes,
    })
}

fn classify_first_simulator_inbound_from_packet(
    payload: &[u8],
) -> Option<FirstSimulatorInboundClassification> {
    let header = decode_first_simulator_packet_header(payload)?;
    let signal = format!("packet:0x{:08x}", header.message_number);

    match header.message_number {
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

fn decode_coarse_location_update(payload: &[u8]) -> Option<DecodedCoarseLocationUpdate> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number
        != lludp_medium_frequency_message_number(LLUDP_COARSE_LOCATION_UPDATE_MEDIUM_ID)
    {
        return None;
    }
    let body = payload.get(header.body_offset..)?;
    let location_count = *body.first()?;
    let needed = 1usize.saturating_add(usize::from(location_count).saturating_mul(3));
    if body.len() < needed {
        return None;
    }
    let count = usize::from(location_count);
    // Message-template shape:
    // [Location variable count:u8][count * (x:u8,y:u8,z:u8)][Index You:i16,Prey:i16][AgentData variable count:u8][count * AgentID:uuid]
    // We decode location entries first (always), then index/agent blocks when present.
    let mut avatars = Vec::with_capacity(count);
    let mut first_location = None;
    let mut second_location = None;
    let mut third_location = None;
    for idx in 0..count {
        let base = 1 + idx * 3;
        let xyz = [body[base], body[base + 1], body[base + 2]];
        if idx == 0 {
            first_location = Some(xyz);
        } else if idx == 1 {
            second_location = Some(xyz);
        } else if idx == 2 {
            third_location = Some(xyz);
        }
        avatars.push(DecodedCoarseAvatar {
            agent_id: None,
            xyz,
        });
    }
    let mut self_index: Option<u16> = None;
    let mut offset = 1usize.saturating_add(count.saturating_mul(3));
    if body.len() >= offset + 4 {
        let you = i16::from_le_bytes(body.get(offset..offset + 2)?.try_into().ok()?);
        let _prey = i16::from_le_bytes(body.get(offset + 2..offset + 4)?.try_into().ok()?);
        if you >= 0 {
            let you_idx = you as usize;
            if you_idx < count {
                self_index = Some(you as u16);
            }
        }
        offset += 4;
    }
    if body.len() > offset {
        let agent_count = usize::from(*body.get(offset)?);
        offset += 1;
        let max_assign = agent_count.min(count);
        if body.len() >= offset + max_assign * 16 {
            for idx in 0..max_assign {
                let raw: [u8; 16] = body
                    .get(offset..offset + 16)
                    .and_then(|s| s.try_into().ok())?;
                offset += 16;
                let id = format_uuid_bytes(raw);
                if !is_null_uuid(&id)
                    && let Some(entry) = avatars.get_mut(idx)
                {
                    entry.agent_id = Some(id);
                }
            }
        }
    }
    Some(DecodedCoarseLocationUpdate {
        location_count,
        first_location,
        second_location,
        third_location,
        avatars,
        self_index,
    })
}

fn decode_health_message(payload: &[u8]) -> Option<DecodedHealthMessage> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number != lludp_low_frequency_message_number(LLUDP_HEALTH_MESSAGE_LOW_ID) {
        return None;
    }
    let body = payload.get(header.body_offset..)?;
    let health_bytes: [u8; 4] = body.get(0..4)?.try_into().ok()?;
    let health = f32::from_le_bytes(health_bytes);
    if !health.is_finite() {
        return None;
    }
    Some(DecodedHealthMessage { health })
}

fn decode_zerocoded_body(body: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(body.len().min(4096));
    let mut i = 0usize;
    while i < body.len() {
        let b = body[i];
        i += 1;
        if b != 0 {
            out.push(b);
            continue;
        }
        let zeros = *body.get(i)? as usize;
        i += 1;
        if out.len().saturating_add(zeros) > MAX_ZEROCODED_BODY_BYTES {
            return None;
        }
        out.extend(std::iter::repeat_n(0u8, zeros));
    }
    Some(out)
}

fn quantize_vector3_centi(vec: [f32; 3]) -> Option<[u16; 3]> {
    let mut out = [0u16; 3];
    for (idx, v) in vec.iter().copied().enumerate() {
        if !v.is_finite() {
            return None;
        }
        let scaled = (v * 100.0).round();
        if scaled < 0.0 {
            return None;
        }
        out[idx] = u16::try_from(scaled as i64).ok()?;
    }
    Some(out)
}

fn read_u8(body: &[u8], offset: &mut usize) -> Option<u8> {
    let v = *body.get(*offset)?;
    *offset += 1;
    Some(v)
}

fn read_u16_le(body: &[u8], offset: &mut usize) -> Option<u16> {
    let bytes: [u8; 2] = body.get(*offset..(*offset + 2))?.try_into().ok()?;
    *offset += 2;
    Some(u16::from_le_bytes(bytes))
}

fn read_u32_le(body: &[u8], offset: &mut usize) -> Option<u32> {
    let bytes: [u8; 4] = body.get(*offset..(*offset + 4))?.try_into().ok()?;
    *offset += 4;
    Some(u32::from_le_bytes(bytes))
}

fn read_u64_le(body: &[u8], offset: &mut usize) -> Option<u64> {
    let bytes: [u8; 8] = body.get(*offset..(*offset + 8))?.try_into().ok()?;
    *offset += 8;
    Some(u64::from_le_bytes(bytes))
}

fn read_f32_le(body: &[u8], offset: &mut usize) -> Option<f32> {
    let bytes: [u8; 4] = body.get(*offset..(*offset + 4))?.try_into().ok()?;
    *offset += 4;
    Some(f32::from_le_bytes(bytes))
}

fn read_vector3f(body: &[u8], offset: &mut usize) -> Option<[f32; 3]> {
    let x = read_f32_le(body, offset)?;
    let y = read_f32_le(body, offset)?;
    let z = read_f32_le(body, offset)?;
    Some([x, y, z])
}

fn decode_object_update_ids_and_scales(payload: &[u8]) -> Option<Vec<(u32, Option<[u16; 3]>)>> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number != u32::from(LLUDP_OBJECT_UPDATE_HIGH_ID) {
        return None;
    }

    let body_raw = payload.get(header.body_offset..)?;
    let body = decode_zerocoded_body(body_raw)?;
    let mut offset = 0usize;

    // RegionData single
    let _region_handle = read_u64_le(&body, &mut offset)?;
    let _time_dilation = read_u16_le(&body, &mut offset)?;

    // ObjectData variable
    let count = read_u8(&body, &mut offset)? as usize;
    let mut out = Vec::with_capacity(count.min(32));
    for _ in 0..count {
        let local_id = read_u32_le(&body, &mut offset)?;
        offset += 1; // State
        offset += 16; // FullID
        offset += 4; // CRC
        offset += 1; // PCode
        offset += 1; // Material
        offset += 1; // ClickAction
        let scale = read_vector3f(&body, &mut offset)?;
        let scale_centi = quantize_vector3_centi(scale);

        // ObjectData (packed), variable 1 (u8 length)
        let object_data_len = read_u8(&body, &mut offset)? as usize;
        offset = offset.checked_add(object_data_len)?;

        // ParentID, UpdateFlags
        offset += 4;
        offset += 4;

        // Path/Profile params (fixed sizes)
        offset += 1; // PathCurve
        offset += 1; // ProfileCurve
        offset += 2; // PathBegin
        offset += 2; // PathEnd
        offset += 1; // PathScaleX
        offset += 1; // PathScaleY
        offset += 1; // PathShearX
        offset += 1; // PathShearY
        offset += 1; // PathTwist
        offset += 1; // PathTwistBegin
        offset += 1; // PathRadiusOffset
        offset += 1; // PathTaperX
        offset += 1; // PathTaperY
        offset += 1; // PathRevolutions
        offset += 1; // PathSkew
        offset += 2; // ProfileBegin
        offset += 2; // ProfileEnd
        offset += 2; // ProfileHollow

        // TextureEntry (var 2), TextureAnim (var 1), NameValue (var 2), Data (var 2), Text (var 1)
        let texture_entry_len = read_u16_le(&body, &mut offset)? as usize;
        offset = offset.checked_add(texture_entry_len)?;
        let texture_anim_len = read_u8(&body, &mut offset)? as usize;
        offset = offset.checked_add(texture_anim_len)?;
        let name_value_len = read_u16_le(&body, &mut offset)? as usize;
        offset = offset.checked_add(name_value_len)?;
        let data_len = read_u16_le(&body, &mut offset)? as usize;
        offset = offset.checked_add(data_len)?;
        let text_len = read_u8(&body, &mut offset)? as usize;
        offset = offset.checked_add(text_len)?;

        // TextColor fixed 4, MediaURL var 1, PSBlock var 1, ExtraParams var 1
        offset += 4;
        let media_url_len = read_u8(&body, &mut offset)? as usize;
        offset = offset.checked_add(media_url_len)?;
        let ps_len = read_u8(&body, &mut offset)? as usize;
        offset = offset.checked_add(ps_len)?;
        let extra_params_len = read_u8(&body, &mut offset)? as usize;
        offset = offset.checked_add(extra_params_len)?;

        // Sound UUID, OwnerID UUID, Gain f32, Flags u8, Radius f32
        offset += 16;
        offset += 16;
        offset += 4;
        offset += 1;
        offset += 4;

        // JointType u8, JointPivot vec3, JointAxisOrAnchor vec3
        offset += 1;
        offset += 12;
        offset += 12;

        out.push((local_id, scale_centi));
    }
    Some(out)
}

fn decode_object_update_cached_local_ids(payload: &[u8]) -> Option<Vec<u32>> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number != u32::from(LLUDP_OBJECT_UPDATE_CACHED_HIGH_ID) {
        return None;
    }
    let body = payload.get(header.body_offset..)?;
    let mut offset = 0usize;
    offset += 8; // RegionHandle
    offset += 2; // TimeDilation
    let count = read_u8(body, &mut offset)? as usize;
    let mut out = Vec::with_capacity(count.min(32));
    for _ in 0..count {
        let id = read_u32_le(body, &mut offset)?;
        offset += 4; // CRC
        offset += 4; // UpdateFlags
        out.push(id);
    }
    Some(out)
}

fn decode_object_update_compressed_local_ids(payload: &[u8]) -> Option<Vec<u32>> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number != u32::from(LLUDP_OBJECT_UPDATE_COMPRESSED_HIGH_ID) {
        return None;
    }
    let body = payload.get(header.body_offset..)?;
    let mut offset = 0usize;
    offset += 8; // RegionHandle
    offset += 2; // TimeDilation
    let count = read_u8(body, &mut offset)? as usize;
    let mut out = Vec::with_capacity(count.min(32));
    for _ in 0..count {
        offset += 4; // UpdateFlags
        let data_len = read_u16_le(body, &mut offset)? as usize;
        let data = body.get(offset..offset + data_len)?;
        offset += data_len;
        if data.len() >= 20 {
            let local_id_bytes: [u8; 4] = data.get(16..20)?.try_into().ok()?;
            out.push(u32::from_le_bytes(local_id_bytes));
        }
    }
    Some(out)
}

fn decode_improved_terse_object_update_local_ids(payload: &[u8]) -> Option<Vec<u32>> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number != u32::from(LLUDP_IMPROVED_TERSE_OBJECT_UPDATE_HIGH_ID) {
        return None;
    }
    let body = payload.get(header.body_offset..)?;
    let mut offset = 0usize;
    offset += 8; // RegionHandle
    offset += 2; // TimeDilation
    let count = read_u8(body, &mut offset)? as usize;
    let mut out = Vec::with_capacity(count.min(32));
    for _ in 0..count {
        let data_len = read_u8(body, &mut offset)? as usize;
        let data = body.get(offset..offset + data_len)?;
        offset += data_len;
        if data.len() >= 4 {
            let local_id_bytes: [u8; 4] = data.get(0..4)?.try_into().ok()?;
            out.push(u32::from_le_bytes(local_id_bytes));
        }
        let tex_len = read_u16_le(body, &mut offset)? as usize;
        offset = offset.checked_add(tex_len)?;
    }
    Some(out)
}

fn decode_kill_object_local_ids(payload: &[u8]) -> Option<Vec<u32>> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number != u32::from(LLUDP_KILL_OBJECT_HIGH_ID) {
        return None;
    }
    let body = payload.get(header.body_offset..)?;
    let mut offset = 0usize;
    let count = read_u8(body, &mut offset)? as usize;
    let mut out = Vec::with_capacity(count.min(32));
    for _ in 0..count {
        out.push(read_u32_le(body, &mut offset)?);
    }
    Some(out)
}

fn decode_region_handshake_sim_name(payload: &[u8]) -> Option<String> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number != lludp_low_frequency_message_number(LLUDP_REGION_HANDSHAKE_LOW_ID) {
        return None;
    }
    let body = payload.get(header.body_offset..)?;
    let mut offset = 0usize;
    offset += 4; // RegionFlags
    offset += 1; // SimAccess
    let sim_name = read_var_string_u8(body, &mut offset)?;
    if sim_name.trim().is_empty() {
        return None;
    }
    Some(sim_name)
}

fn read_vector3f_i32(body: &[u8], offset: &mut usize) -> Option<[i32; 3]> {
    if body.len() < *offset + 12 {
        return None;
    }
    let x = f32::from_le_bytes(body.get(*offset..(*offset + 4))?.try_into().ok()?);
    *offset += 4;
    let y = f32::from_le_bytes(body.get(*offset..(*offset + 4))?.try_into().ok()?);
    *offset += 4;
    let z = f32::from_le_bytes(body.get(*offset..(*offset + 4))?.try_into().ok()?);
    *offset += 4;
    Some([x.round() as i32, y.round() as i32, z.round() as i32])
}

fn decode_agent_movement_complete(payload: &[u8]) -> Option<DecodedAgentMovementComplete> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number
        != lludp_low_frequency_message_number(LLUDP_AGENT_MOVEMENT_COMPLETE_LOW_ID)
    {
        return None;
    }
    let body = payload.get(header.body_offset..)?;
    let mut offset = 0usize;
    offset += 16; // AgentID
    offset += 16; // SessionID
    let position = read_vector3f_i32(body, &mut offset)?;
    offset += 12; // LookAt
    let region_handle = u64::from_le_bytes(body.get(offset..offset + 8)?.try_into().ok()?);
    Some(DecodedAgentMovementComplete {
        position,
        region_handle,
    })
}

fn decode_chat_from_simulator(payload: &[u8]) -> Option<NearbyChatMessage> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number != lludp_low_frequency_message_number(LLUDP_CHAT_FROM_SIMULATOR_LOW_ID)
    {
        return None;
    }
    let body = payload.get(header.body_offset..)?;

    let mut offset = 0usize;
    let from_name_len = usize::from(*body.get(offset)?);
    offset += 1;
    let from_name = std::str::from_utf8(body.get(offset..offset + from_name_len)?)
        .ok()?
        .trim_end_matches('\0')
        .to_string();
    offset += from_name_len;

    offset += 16; // SourceID
    offset += 16; // OwnerID
    offset += 1; // SourceType
    offset += 1; // ChatType
    offset += 1; // Audible
    offset += 12; // Position

    let msg_len_bytes: [u8; 2] = body.get(offset..offset + 2)?.try_into().ok()?;
    let msg_len = usize::from(u16::from_le_bytes(msg_len_bytes));
    offset += 2;
    let text = std::str::from_utf8(body.get(offset..offset + msg_len)?)
        .ok()?
        .trim_end_matches('\0')
        .to_string();

    Some(NearbyChatMessage {
        sender: if from_name.is_empty() {
            String::from("unknown")
        } else {
            from_name
        },
        text,
        source: String::from("ChatFromSimulator"),
    })
}

fn decode_social_events(payload: &[u8]) -> Vec<SocialEvent> {
    let Some(header) = decode_first_simulator_packet_header(payload) else {
        return Vec::new();
    };
    let body = match payload.get(header.body_offset..) {
        Some(v) => v,
        None => return Vec::new(),
    };
    if header.message_number == lludp_low_frequency_message_number(LLUDP_ONLINE_NOTIFICATION_LOW_ID)
    {
        return decode_online_offline_notification_events(body, true);
    }
    if header.message_number
        == lludp_low_frequency_message_number(LLUDP_OFFLINE_NOTIFICATION_LOW_ID)
    {
        return decode_online_offline_notification_events(body, false);
    }
    if header.message_number == lludp_low_frequency_message_number(LLUDP_CHANGE_USER_RIGHTS_LOW_ID)
    {
        return decode_change_user_rights_events(body);
    }
    if header.message_number
        == lludp_low_frequency_message_number(LLUDP_IMPROVED_INSTANT_MESSAGE_LOW_ID)
    {
        return decode_improved_instant_message_event(body)
            .into_iter()
            .collect();
    }
    Vec::new()
}

fn decode_online_offline_notification_events(body: &[u8], online: bool) -> Vec<SocialEvent> {
    let mut out = Vec::new();
    let count = match body.first() {
        Some(v) => usize::from(*v),
        None => return out,
    };
    let mut offset = 1usize;
    for _ in 0..count {
        let raw: [u8; 16] = match body
            .get(offset..offset + 16)
            .and_then(|s| s.try_into().ok())
        {
            Some(v) => v,
            None => break,
        };
        offset += 16;
        let agent_id = format_uuid_bytes(raw);
        if online {
            out.push(SocialEvent::FriendOnline { agent_id });
        } else {
            out.push(SocialEvent::FriendOffline { agent_id });
        }
    }
    out
}

fn decode_change_user_rights_events(body: &[u8]) -> Vec<SocialEvent> {
    let mut out = Vec::new();
    if body.len() < 17 {
        return out;
    }
    let agent_id = match body.get(0..16).and_then(|s| s.try_into().ok()) {
        Some(raw) => format_uuid_bytes(raw),
        None => return out,
    };
    let rights_count = usize::from(body[16]);
    let mut offset = 17usize;
    for _ in 0..rights_count {
        let related_id = match body
            .get(offset..offset + 16)
            .and_then(|s| s.try_into().ok())
        {
            Some(raw) => format_uuid_bytes(raw),
            None => break,
        };
        offset += 16;
        let rights = match body
            .get(offset..offset + 4)
            .and_then(|s| s.try_into().ok())
            .map(i32::from_le_bytes)
        {
            Some(v) => v,
            None => break,
        };
        offset += 4;
        out.push(SocialEvent::FriendRights {
            agent_id: agent_id.clone(),
            related_id,
            rights,
        });
    }
    out
}

fn decode_improved_instant_message_event(body: &[u8]) -> Option<SocialEvent> {
    if body.len() < 32 + 1 + 16 + 4 + 16 + 12 + 1 + 1 + 16 + 4 + 1 + 2 + 2 {
        return None;
    }
    let mut offset = 0usize;
    let from_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    offset += 16;
    offset += 16; // agent session id
    offset += 1; // from_group
    let to_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    offset += 16;
    offset += 4; // parent estate
    offset += 16; // region id
    offset += 12; // position
    offset += 1; // offline
    let dialog = *body.get(offset)?;
    offset += 1;
    let session_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    offset += 16;
    let timestamp = u32::from_le_bytes(body.get(offset..offset + 4)?.try_into().ok()?);
    offset += 4;
    let from_name_len = usize::from(*body.get(offset)?);
    offset += 1;
    let from_name = std::str::from_utf8(body.get(offset..offset + from_name_len)?)
        .ok()?
        .trim_end_matches('\0')
        .to_string();
    offset += from_name_len;
    let message_len = usize::from(u16::from_le_bytes(
        body.get(offset..offset + 2)?.try_into().ok()?,
    ));
    offset += 2;
    let message = std::str::from_utf8(body.get(offset..offset + message_len)?)
        .ok()?
        .trim_end_matches('\0')
        .to_string();
    Some(SocialEvent::DirectIm(DirectImPayload {
        from_id,
        to_id,
        session_id,
        from_name,
        message,
        dialog,
        timestamp,
    }))
}

fn decode_legacy_avatar_properties_reply(
    payload: &[u8],
    expected_avatar_id: &str,
) -> Option<AgentProfileData> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number
        != lludp_low_frequency_message_number(LLUDP_AVATAR_PROPERTIES_REPLY_LOW_ID)
    {
        return None;
    }
    let body = payload.get(header.body_offset..)?;
    if body.len() < 68 {
        return None;
    }
    let mut offset = 0usize;
    offset += 16; // AgentID
    let avatar_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    if !expected_avatar_id.is_empty() && !avatar_id.eq_ignore_ascii_case(expected_avatar_id) {
        return None;
    }
    offset += 16;
    let sl_image_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    offset += 16;
    let fl_image_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    offset += 16;
    let partner_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    offset += 16;

    let sl_about_text = read_var_string_u16(body, &mut offset)?;
    let fl_about_text = read_var_string_u8(body, &mut offset)?;
    let born_on = read_var_string_u8(body, &mut offset)?;
    let profile_url = read_var_string_u8(body, &mut offset)?;
    let _caption = read_var_string_u8(body, &mut offset)?;
    let flags = i32::from_le_bytes(body.get(offset..offset + 4)?.try_into().ok()?);

    Some(AgentProfileData {
        id: avatar_id,
        profile_url: if profile_url.is_empty() {
            None
        } else {
            Some(profile_url)
        },
        sl_about_text,
        fl_about_text,
        notes: String::new(),
        sl_image_id: if is_null_uuid(&sl_image_id) {
            None
        } else {
            Some(sl_image_id)
        },
        fl_image_id: if is_null_uuid(&fl_image_id) {
            None
        } else {
            Some(fl_image_id)
        },
        partner_id: if is_null_uuid(&partner_id) {
            None
        } else {
            Some(partner_id)
        },
        member_since: if born_on.is_empty() {
            None
        } else {
            Some(born_on)
        },
        online: Some((flags & (1 << 4)) != 0),
        allow_publish: Some((flags & (1 << 0)) != 0),
        identified: Some((flags & (1 << 2)) != 0),
        transacted: Some((flags & (1 << 3)) != 0),
        display_name: None,
        username: None,
        groups: Vec::new(),
        picks: Vec::new(),
        pick_details: Vec::new(),
        classifieds: Vec::new(),
        classified_details: Vec::new(),
    })
}

fn decode_legacy_avatar_groups_reply(
    payload: &[u8],
    expected_avatar_id: &str,
) -> Option<Vec<AgentProfileGroup>> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number != lludp_low_frequency_message_number(LLUDP_AVATAR_GROUPS_REPLY_LOW_ID)
    {
        return None;
    }
    let body = payload.get(header.body_offset..)?;
    if body.len() < 33 {
        return None;
    }
    let mut offset = 0usize;
    offset += 16; // AgentID
    let avatar_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    if !expected_avatar_id.is_empty() && !avatar_id.eq_ignore_ascii_case(expected_avatar_id) {
        return None;
    }
    offset += 16;
    let group_count = usize::from(*body.get(offset)?);
    offset += 1;
    let mut groups = Vec::new();
    for _ in 0..group_count {
        if body.len() < offset + 8 + 1 + 16 + 16 {
            break;
        }
        offset += 8; // GroupPowers
        offset += 1; // AcceptNotices
        let _group_title = read_var_string_u8(body, &mut offset)?;
        let group_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
        offset += 16;
        let group_name = read_var_string_u8(body, &mut offset)?;
        let insignia_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
        offset += 16;
        groups.push(AgentProfileGroup {
            id: group_id,
            name: group_name,
            image_id: if is_null_uuid(&insignia_id) {
                None
            } else {
                Some(insignia_id)
            },
        });
    }
    Some(groups)
}

fn decode_legacy_avatar_notes_reply(payload: &[u8], expected_avatar_id: &str) -> Option<String> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number != lludp_low_frequency_message_number(LLUDP_AVATAR_NOTES_REPLY_LOW_ID)
    {
        return None;
    }
    let body = payload.get(header.body_offset..)?;
    if body.len() < 32 {
        return None;
    }
    let mut offset = 0usize;
    offset += 16; // AgentID
    let target_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    if !expected_avatar_id.is_empty() && !target_id.eq_ignore_ascii_case(expected_avatar_id) {
        return None;
    }
    offset += 16;
    read_var_string_u16(body, &mut offset)
}

fn decode_legacy_avatar_picks_reply(
    payload: &[u8],
    expected_avatar_id: &str,
) -> Option<Vec<AgentProfilePick>> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number != lludp_low_frequency_message_number(LLUDP_AVATAR_PICKS_REPLY_LOW_ID)
    {
        return None;
    }
    let body = payload.get(header.body_offset..)?;
    if body.len() < 33 {
        return None;
    }
    let mut offset = 0usize;
    offset += 16; // AgentID
    let target_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    if !expected_avatar_id.is_empty() && !target_id.eq_ignore_ascii_case(expected_avatar_id) {
        return None;
    }
    offset += 16;
    let mut picks = Vec::new();
    while offset < body.len() {
        if body.len() < offset + 16 {
            break;
        }
        let pick_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
        offset += 16;
        let pick_name = read_var_string_u8(body, &mut offset)?;
        picks.push(AgentProfilePick {
            id: pick_id,
            name: pick_name,
        });
    }
    Some(picks)
}

fn decode_legacy_pick_info_reply(
    payload: &[u8],
    expected_avatar_id: &str,
) -> Option<AgentProfilePickDetails> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number != lludp_low_frequency_message_number(LLUDP_PICK_INFO_REPLY_LOW_ID) {
        return None;
    }
    let body = payload.get(header.body_offset..)?;
    if body.len() < 16 + 16 + 1 + 16 {
        return None;
    }
    let mut offset = 0usize;
    offset += 16; // AgentID
    let pick_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    offset += 16;
    let creator_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    if !expected_avatar_id.is_empty() && !creator_id.eq_ignore_ascii_case(expected_avatar_id) {
        return None;
    }
    offset += 16;
    offset += 1; // top_pick
    let parcel_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    offset += 16;
    let name = read_var_string_u8(body, &mut offset)?;
    let desc = read_var_string_u16(body, &mut offset)?;
    let snapshot_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    offset += 16;
    let _user = read_var_string_u8(body, &mut offset)?;
    let _original_name = read_var_string_u8(body, &mut offset)?;
    let sim_name = read_var_string_u8(body, &mut offset)?;
    let global_position = read_vector3d_i32(body, &mut offset)?;
    Some(AgentProfilePickDetails {
        id: pick_id,
        name: if name.is_empty() { None } else { Some(name) },
        description: if desc.is_empty() { None } else { Some(desc) },
        snapshot_id: if is_null_uuid(&snapshot_id) {
            None
        } else {
            Some(snapshot_id)
        },
        parcel_id: if is_null_uuid(&parcel_id) {
            None
        } else {
            Some(parcel_id)
        },
        sim_name: if sim_name.is_empty() {
            None
        } else {
            Some(sim_name)
        },
        parcel_name: None,
        global_position: Some(global_position),
    })
}

fn decode_legacy_avatar_classifieds_reply(
    payload: &[u8],
    expected_avatar_id: &str,
) -> Option<Vec<AgentProfileClassified>> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number
        != lludp_low_frequency_message_number(LLUDP_AVATAR_CLASSIFIED_REPLY_LOW_ID)
    {
        return None;
    }
    let body = payload.get(header.body_offset..)?;
    if body.len() < 32 {
        return None;
    }
    let mut offset = 0usize;
    offset += 16; // AgentID
    let target_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    if !expected_avatar_id.is_empty() && !target_id.eq_ignore_ascii_case(expected_avatar_id) {
        return None;
    }
    offset += 16;
    let mut classifieds = Vec::new();
    while offset < body.len() {
        if body.len() < offset + 16 {
            break;
        }
        let classified_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
        offset += 16;
        let name = read_var_string_u8(body, &mut offset)?;
        classifieds.push(AgentProfileClassified {
            id: classified_id,
            name,
        });
    }
    Some(classifieds)
}

fn decode_legacy_classified_info_reply(
    payload: &[u8],
    expected_avatar_id: &str,
) -> Option<AgentProfileClassifiedDetails> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number
        != lludp_low_frequency_message_number(LLUDP_CLASSIFIED_INFO_REPLY_LOW_ID)
    {
        return None;
    }
    let body = payload.get(header.body_offset..)?;
    if body.len() < 16 + 16 + 4 + 4 + 4 {
        return None;
    }
    let mut offset = 0usize;
    offset += 16; // AgentID
    let classified_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    offset += 16;
    let creator_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    if !expected_avatar_id.is_empty() && !creator_id.eq_ignore_ascii_case(expected_avatar_id) {
        return None;
    }
    offset += 16;
    offset += 4; // creation date
    offset += 4; // expiration date
    let category = u32::from_le_bytes(body.get(offset..offset + 4)?.try_into().ok()?);
    offset += 4;
    let name = read_var_string_u8(body, &mut offset)?;
    let desc = read_var_string_u16(body, &mut offset)?;
    let parcel_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    offset += 16;
    offset += 4; // parent estate
    let snapshot_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    offset += 16;
    let sim_name = read_var_string_u8(body, &mut offset)?;
    let global_position = read_vector3d_i32(body, &mut offset)?;
    let parcel_name = read_var_string_u8(body, &mut offset)?;
    let flags = *body.get(offset)?;
    offset += 1;
    let price_for_listing = i32::from_le_bytes(body.get(offset..offset + 4)?.try_into().ok()?);

    Some(AgentProfileClassifiedDetails {
        id: classified_id,
        name: if name.is_empty() { None } else { Some(name) },
        description: if desc.is_empty() { None } else { Some(desc) },
        snapshot_id: if is_null_uuid(&snapshot_id) {
            None
        } else {
            Some(snapshot_id)
        },
        parcel_id: if is_null_uuid(&parcel_id) {
            None
        } else {
            Some(parcel_id)
        },
        sim_name: if sim_name.is_empty() {
            None
        } else {
            Some(sim_name)
        },
        parcel_name: if parcel_name.is_empty() {
            None
        } else {
            Some(parcel_name)
        },
        global_position: Some(global_position),
        category: Some(category),
        flags: Some(flags),
        price_for_listing: Some(price_for_listing),
    })
}

fn read_var_string_u8(body: &[u8], offset: &mut usize) -> Option<String> {
    let len = usize::from(*body.get(*offset)?);
    *offset += 1;
    let text = std::str::from_utf8(body.get(*offset..(*offset + len))?).ok()?;
    *offset += len;
    Some(text.trim_end_matches('\0').to_string())
}

fn read_var_string_u16(body: &[u8], offset: &mut usize) -> Option<String> {
    let len = usize::from(u16::from_le_bytes(
        body.get(*offset..(*offset + 2))?.try_into().ok()?,
    ));
    *offset += 2;
    let text = std::str::from_utf8(body.get(*offset..(*offset + len))?).ok()?;
    *offset += len;
    Some(text.trim_end_matches('\0').to_string())
}

fn read_vector3d_i32(body: &[u8], offset: &mut usize) -> Option<[i32; 3]> {
    if body.len() < *offset + 24 {
        return None;
    }
    let x = f64::from_le_bytes(body.get(*offset..(*offset + 8))?.try_into().ok()?);
    *offset += 8;
    let y = f64::from_le_bytes(body.get(*offset..(*offset + 8))?.try_into().ok()?);
    *offset += 8;
    let z = f64::from_le_bytes(body.get(*offset..(*offset + 8))?.try_into().ok()?);
    *offset += 8;
    Some([x.round() as i32, y.round() as i32, z.round() as i32])
}

fn is_null_uuid(uuid: &str) -> bool {
    uuid == "00000000-0000-0000-0000-000000000000"
}

fn decode_simulator_viewer_time_message(
    payload: &[u8],
) -> Option<DecodedSimulatorViewerTimeMessage> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number
        != lludp_low_frequency_message_number(LLUDP_SIMULATOR_VIEWER_TIME_LOW_ID)
    {
        return None;
    }
    let body = payload.get(header.body_offset..)?;
    let body_len = u16::try_from(body.len()).ok()?;
    let signature = body.get(0..4).and_then(|bytes| {
        let raw: [u8; 4] = bytes.try_into().ok()?;
        Some(u32::from_le_bytes(raw))
    });
    Some(DecodedSimulatorViewerTimeMessage {
        body_len,
        signature,
    })
}

fn health_to_basis_points(value: f32) -> u16 {
    let normalized = if value > 1.0 {
        (value / 100.0).clamp(0.0, 1.0)
    } else {
        value.clamp(0.0, 1.0)
    };
    (normalized * 10_000.0).round() as u16
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
            if let Some(body_map) = event_map.get("body").and_then(Value::as_object) {
                for (key, value) in body_map {
                    if let Some(text) = json_scalar_to_string(value) {
                        fields.insert(key.clone(), text);
                    }
                }
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
    let map = doc
        .descendants()
        .find(|node| node.has_tag_name("map"))
        .ok_or_else(|| ConnectionError::CapabilityDecode(String::from("missing llsd map")))?;

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
    let map = doc
        .descendants()
        .find(|node| node.has_tag_name("map"))
        .ok_or_else(|| ConnectionError::CapabilityDecode(String::from("missing llsd map")))?;

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
    let map = doc
        .descendants()
        .find(|node| node.has_tag_name("map"))
        .ok_or_else(|| ConnectionError::CapabilityDecode(String::from("missing llsd map")))?;

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
                    fields = parse_llsd_scalar_map(value_node);
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
    let map = doc
        .descendants()
        .find(|node| node.has_tag_name("map"))
        .ok_or_else(|| ConnectionError::CapabilityDecode(String::from("missing llsd map")))?;

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
    use wiremock::matchers::{body_partial_json, body_string_contains, header, method, path};
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
            .and(body_string_contains("<string>AgentProfile</string>"))
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
        for id in [42u32, 99u32] {
            body.push(4); // Data len (var 1)
            body.extend_from_slice(&id.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes()); // TextureEntry len (var 2)
        }
        let payload =
            make_high_frequency_packet_with_body(LLUDP_IMPROVED_TERSE_OBJECT_UPDATE_HIGH_ID, &body);
        let ids =
            decode_improved_terse_object_update_local_ids(&payload).expect("terse should decode");
        assert_eq!(ids, vec![42, 99]);
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
    fn decode_object_update_compressed_extracts_local_ids() {
        let mut body = Vec::new();
        body.extend_from_slice(&1u64.to_le_bytes()); // RegionHandle
        body.extend_from_slice(&0u16.to_le_bytes()); // TimeDilation
        body.push(1); // object count
        body.extend_from_slice(&0u32.to_le_bytes()); // UpdateFlags
        let mut data = vec![0u8; 16]; // UUID
        data.extend_from_slice(&55u32.to_le_bytes()); // LocalID
        data.push(9); // PCode
        let data_len = u16::try_from(data.len()).expect("len fits u16");
        body.extend_from_slice(&data_len.to_le_bytes());
        body.extend_from_slice(&data);
        let payload =
            make_high_frequency_packet_with_body(LLUDP_OBJECT_UPDATE_COMPRESSED_HIGH_ID, &body);
        let ids = decode_object_update_compressed_local_ids(&payload)
            .expect("compressed update should decode");
        assert_eq!(ids, vec![55]);
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
