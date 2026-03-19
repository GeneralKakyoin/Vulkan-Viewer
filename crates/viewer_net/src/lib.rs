use quick_xml::Reader;
use quick_xml::events::Event;
use reqwest::{
    Method, StatusCode,
    header::{ACCEPT, CONTENT_TYPE},
};
use roxmltree::{Document, Node};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::net::UdpSocket;
use tokio::time::timeout;
use viewer_grid::{
    GridAdapterError, GridLoginAdapter, GridLoginRequest, GridLoginResponse, GridLoginResult,
    LoginIntent, SessionBootstrap,
};

const MAX_LOGIN_REDIRECTS: usize = 4;
const MAX_EVENT_QUEUE_ONE_SHOT_ATTEMPTS: usize = 3;
const EVENT_QUEUE_ONE_SHOT_MIN_TIMEOUT: Duration = Duration::from_secs(35);
const LLSD_XML_CONTENT_TYPE: &str = "application/llsd+xml";
const LLUDP_PACKET_ID_SIZE: usize = 6;
const LLUDP_MINIMUM_VALID_PACKET_SIZE: usize = LLUDP_PACKET_ID_SIZE + 1;
const LLUDP_MESSAGE_PREFIX: u8 = 0xFF;
const LLUDP_RELIABLE_FLAG: u8 = 0x40;
const LLUDP_LOW_FREQUENCY_PREFIX: u32 = 0xFFFF0000;
const LLUDP_USE_CIRCUIT_CODE_LOW_ID: u16 = 3;
const LLUDP_COMPLETE_AGENT_MOVEMENT_LOW_ID: u16 = 249;
const LLUDP_TEST_MESSAGE_LOW_ID: u16 = 1;
const LLUDP_REGION_HANDSHAKE_LOW_ID: u16 = 148;
const LLUDP_HEALTH_MESSAGE_LOW_ID: u16 = 138;
const LLUDP_SIMULATOR_VIEWER_TIME_LOW_ID: u16 = 150;
const LLUDP_ENABLE_SIMULATOR_LOW_ID: u16 = 151;
const LLUDP_AGENT_MOVEMENT_COMPLETE_LOW_ID: u16 = 250;
const LLUDP_AGENT_DATA_UPDATE_LOW_ID: u16 = 387;
const LLUDP_PACKET_ACK_LOW_ID: u16 = 0xFFFB;
const LLUDP_ONLINE_NOTIFICATION_LOW_ID: u16 = 322;
const LLUDP_VIEWER_EFFECT_MEDIUM_ID: u8 = 17;
const LLUDP_COARSE_LOCATION_UPDATE_MEDIUM_ID: u8 = 6;
const LLUDP_ATTACHED_SOUND_MEDIUM_ID: u8 = 13;
const DEFAULT_SEED_CAPABILITY_REQUEST: &[&str] = &[
    "EventQueueGet",
    "SimulatorFeatures",
    "MapLayer",
    "ViewerAsset",
];

pub trait LoginCodec {
    fn content_type(&self) -> &str;
    fn encode_request(&self, request: &GridLoginRequest) -> Result<Vec<u8>, CodecError>;
    fn decode_response(&self, body: &[u8]) -> Result<GridLoginResponse, CodecError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoginWireFormat {
    Json,
    Llsd,
    XmlRpc,
}

impl Default for LoginWireFormat {
    fn default() -> Self {
        LoginWireFormat::Json
    }
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
            login: map.get("login").and_then(|value| llsd_to_bool(value)),
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
        xmlrpc_member_string(&mut xml, "platform_version", &request.params.platform_version);
        xmlrpc_member_string(&mut xml, "host_id", &request.params.host_id);
        xmlrpc_member_string(&mut xml, "machine_hash", &request.params.machine_hash);
        xmlrpc_member_array_of_strings(&mut xml, "options", &request.options);

        xml.push_str("</struct></value></param></params></methodCall>");
        Ok(xml.into_bytes())
    }

    fn decode_response(&self, body: &[u8]) -> Result<GridLoginResponse, CodecError> {
        let text = std::str::from_utf8(body).map_err(|err| CodecError::Deserialize(err.to_string()))?;
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
                )))
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct LegacyLoginName {
    first: String,
    last: String,
}

fn classify_legacy_login_name(raw: &str) -> LegacyLoginName {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return LegacyLoginName {
            first: String::new(),
            last: String::new(),
        };
    }

    if let Some((first, last)) = trimmed.split_once('.') {
        if !first.is_empty() && !last.is_empty() {
            return LegacyLoginName {
                first: first.to_string(),
                last: last.to_string(),
            };
        }
    }

    let mut parts = trimmed.split_whitespace();
    if let Some(first) = parts.next() {
        let remainder: Vec<&str> = parts.collect();
        if !remainder.is_empty() {
            return LegacyLoginName {
                first: first.to_string(),
                last: remainder.join(" "),
            };
        }
    }

    LegacyLoginName {
        first: trimmed.to_string(),
        last: String::from("Resident"),
    }
}

fn split_legacy_name(username: &str) -> Option<(String, String)> {
    let trimmed = username.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some((first, last)) = trimmed.split_once('.') {
        if !first.is_empty() && !last.is_empty() {
            return Some((first.to_string(), last.to_string()));
        }
    }

    let mut parts = trimmed.split_whitespace();
    let first = parts.next()?;
    let remainder: Vec<&str> = parts.collect();
    if remainder.is_empty() {
        return None;
    }

    Some((first.to_string(), remainder.join(" ")))
}

fn normalize_legacy_passwd(passwd: &str) -> String {
    if passwd.starts_with("$1$") {
        return passwd.to_string();
    }

    let digest = md5::compute(passwd.as_bytes());
    format!("$1${digest:x}")
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
            for member in typed_child.children().filter(|child| child.has_tag_name("member")) {
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
            for child_value in data_node.children().filter(|child| child.has_tag_name("value")) {
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
                if map_depth == 1 {
                    if let Ok(text) = e.unescape() {
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
                                            let normalized = matches!(
                                                text.to_lowercase().as_str(),
                                                "true" | "1"
                                            );
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
    SimulatorViewerTimeMessage,
    EnableSimulator,
    AgentDataUpdate,
    OnlineNotification,
    ViewerEffect,
    CoarseLocationUpdate,
    AttachedSound,
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
    pub likely_broader_traffic: usize,
    pub unknown: usize,
    pub kinds: Vec<FirstSimulatorInboundMessageKind>,
    pub unknown_packet_message_numbers: Vec<u32>,
    pub repeated_unknown_packet_message_numbers: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EarlySimulatorTrafficKind {
    HealthMessage,
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
    pub simulator_viewer_time_message: usize,
    pub online_notification: usize,
    pub viewer_effect: usize,
    pub coarse_location_update: usize,
    pub attached_sound: usize,
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
pub struct EventQueueAttemptDiagnostic {
    pub attempt: usize,
    pub status: Option<u16>,
    pub elapsed_ms: u128,
    pub retryable: bool,
    pub error_kind: String,
    pub response_headers: Vec<(String, String)>,
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
    next_first_simulator_packet_id: u32,
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
            next_first_simulator_packet_id: 1,
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

    pub fn summarize_early_simulator_traffic(&self) -> EarlySimulatorTrafficSummary {
        let mut summary = EarlySimulatorTrafficSummary::default();
        for observation in &self.early_simulator_traffic_observations {
            summary.observations += 1;
            match observation.kind {
                EarlySimulatorTrafficKind::HealthMessage => summary.health_message += 1,
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
        self.next_first_simulator_packet_id = 1;
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
        if self.state != ConnectionState::Connected {
            return Err(ConnectionError::InvalidState(self.state));
        }

        let request = adapter.shape_login_request(&intent);
        let mut endpoint = self.config.endpoint.clone();
        let mut http_method = Method::POST;
        let mut redirects = 0;
        let codec = self.config.wire_format.codec();
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

        self.advance_first_simulator_handshake_scaffold(FirstSimulatorHandshakeStage::UseCircuitCode)
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
        let stage_before = self.first_simulator_handshake_state.as_ref().map(|s| s.stage);
        let mut advanced_stage = false;

        if classification.kind == FirstSimulatorInboundMessageKind::AgentMovementComplete
            && classification.decode_source == FirstSimulatorInboundDecodeSource::PacketMessageNumber
            && stage_before == Some(FirstSimulatorHandshakeStage::WaitingForAgentMovementComplete)
        {
            self.mark_first_simulator_agent_movement_complete_received()?;
            advanced_stage = true;
        }

        let stage_after = self.first_simulator_handshake_state.as_ref().map(|s| s.stage);
        let observation_index = self.first_simulator_handshake_receive_diagnostics.len() + 1;
        self.first_simulator_handshake_receive_diagnostics
            .push(FirstSimulatorHandshakeReceiveDiagnostic {
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
            });

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
                })
            }
            Err(_) => {
                return Err(ConnectionError::FirstSimulatorReceiveTimedOut {
                    bind: bind.to_string(),
                    timeout_ms: wait_timeout.as_millis(),
                })
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
        let mut post_boundary_likely_broader_traffic = 0usize;
        let mut post_boundary_unknown = 0usize;
        let mut post_boundary_kinds = Vec::new();
        let mut post_boundary_unknown_packet_message_numbers = Vec::new();
        for _ in 0..max_packets {
            let mut buf = vec![0u8; 2048];
            let recv = timeout(wait_timeout, socket.recv_from(&mut buf)).await;
            let (received_len, _) = match recv {
                Ok(Ok(parts)) => parts,
                Ok(Err(err)) => {
                    return Err(ConnectionError::FirstSimulatorReceiveFailed {
                        bind: bind.to_string(),
                        reason: err.to_string(),
                    })
                }
                Err(_) => {
                    timed_out = true;
                    break;
                }
            };

            let classification = self.observe_first_simulator_inbound_payload(&buf[..received_len])?;
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
                post_boundary_kinds.push(classification.kind);
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
                post_boundary_kinds.push(classification.kind);
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
                likely_broader_traffic: post_boundary_likely_broader_traffic,
                unknown: post_boundary_unknown,
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
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()?;

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
                    let retryable = err.is_timeout() || err.is_connect() || err.is_request();
                    attempts.push(EventQueueAttemptDiagnostic {
                        attempt,
                        status: None,
                        elapsed_ms,
                        retryable,
                        error_kind: format!("transport:{err}"),
                        response_headers: Vec::new(),
                    });
                    if retryable && attempt < MAX_EVENT_QUEUE_ONE_SHOT_ATTEMPTS {
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
            attempts.push(EventQueueAttemptDiagnostic {
                attempt,
                status: Some(status.as_u16()),
                elapsed_ms,
                retryable,
                error_kind: format!("http:{status}"),
                response_headers: headers,
            });
            if retryable && attempt < MAX_EVENT_QUEUE_ONE_SHOT_ATTEMPTS {
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
        self.next_first_simulator_packet_id = 1;
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

    let header = &payload[LLUDP_PACKET_ID_SIZE..];
    if header.is_empty() {
        return None;
    }

    let message_number = if header[0] != LLUDP_MESSAGE_PREFIX {
        u32::from(header[0])
    } else if payload.len() >= LLUDP_MINIMUM_VALID_PACKET_SIZE + 1
        && header.get(1).is_some()
        && header[1] != LLUDP_MESSAGE_PREFIX
    {
        (u32::from(LLUDP_MESSAGE_PREFIX) << 8) | u32::from(header[1])
    } else if payload.len() >= LLUDP_MINIMUM_VALID_PACKET_SIZE + 3
        && header.len() >= 4
        && header[1] == LLUDP_MESSAGE_PREFIX
    {
        let low = u16::from_be_bytes([header[2], header[3]]);
        lludp_low_frequency_message_number(low)
    } else {
        return None;
    };

    Some(FirstSimulatorPacketHeader { message_number })
}

fn classify_first_simulator_inbound_from_packet(
    payload: &[u8],
) -> Option<FirstSimulatorInboundClassification> {
    let header = decode_first_simulator_packet_header(payload)?;
    let signal = format!("packet:0x{:08x}", header.message_number);

    match header.message_number {
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

fn classify_first_simulator_inbound_message(payload: &[u8]) -> FirstSimulatorInboundClassification {
    if let Some(classification) = classify_first_simulator_inbound_from_packet(payload) {
        return classification;
    }

    let text = String::from_utf8_lossy(payload);
    let lowered = text.to_ascii_lowercase();

    if let Ok(json) = serde_json::from_slice::<Value>(payload) {
        if let Some(kind) = classify_first_simulator_inbound_from_json(&json) {
            return kind;
        }
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

fn extract_llsd_event_messages(array_node: Node<'_, '_>, inspection: &mut EventQueueInspection) -> usize {
    let mut count = 0usize;
    for event_map in array_node.children().filter(|node| node.has_tag_name("map")) {
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
                inspection.scalar_values.insert(
                    key_name,
                    value_node.text().unwrap_or_default().to_string(),
                );
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
        if obj.get("reason").is_none() {
            if let Some(reason) = data_reason {
                obj.insert(String::from("reason"), reason);
            }
        }

        let data_message = obj
            .get("data")
            .and_then(Value::as_object)
            .and_then(|data| data.get("message"))
            .cloned();
        if obj.get("message").is_none() {
            if let Some(message) = data_message {
                obj.insert(String::from("message"), message);
            }
        }

        let data_message_id = obj
            .get("data")
            .and_then(Value::as_object)
            .and_then(|data| data.get("message_id"))
            .cloned();
        if obj.get("message_id").is_none() {
            if let Some(message_id) = data_message_id {
                obj.insert(String::from("message_id"), message_id);
            }
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
        assert!(encoded.contains(
            "<key>passwd</key><string>$1$5ebe2294ecd0e0f08eab7690d2a6ee69</string>"
        ));
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
        assert!(encoded.contains(
            "<key>passwd</key><string>$1$5ebe2294ecd0e0f08eab7690d2a6ee69</string>"
        ));
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
            <key>MapLayer</key><string>https://cap.example/map</string>
        </map></llsd>"#;

        Mock::given(method("POST"))
            .and(path("/seed"))
            .and(header("content-type", "application/llsd+xml"))
            .and(body_string_contains("<llsd><array>"))
            .and(body_string_contains("<string>EventQueueGet</string>"))
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
            .and(body_string_contains("<key>done</key><boolean>false</boolean>"))
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
            vec!["EnableSimulator".to_string(), "ParcelProperties".to_string()]
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
            ConnectionError::EventQueueOneShotFailed { attempts_len, attempts } => {
                assert_eq!(attempts_len, 1);
                assert_eq!(attempts.len(), 1);
                assert_eq!(attempts[0].status, Some(400));
                assert!(!attempts[0].retryable);
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
            ConnectionError::EventQueueOneShotFailed { attempts_len, attempts } => {
                assert_eq!(attempts_len, MAX_EVENT_QUEUE_ONE_SHOT_ATTEMPTS);
                assert_eq!(attempts.len(), MAX_EVENT_QUEUE_ONE_SHOT_ATTEMPTS);
                assert!(attempts.iter().all(|attempt| attempt.retryable));
                assert!(attempts.iter().all(|attempt| attempt.status == Some(500)));
            }
            other => panic!("expected EventQueueOneShotFailed, got {other:?}"),
        }
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

        assert!(inspection.top_level_keys.iter().any(|k| k == "MeshRezEnabled"));
        assert!(inspection.top_level_keys.iter().any(|k| k == "OpenSimExtras"));
        assert_eq!(
            inspection.scalar_values.get("Channel").map(String::as_str),
            Some("Second Life Server")
        );
        assert_eq!(
            inspection.complex_value_types.get("OpenSimExtras").map(String::as_str),
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
            inspection.complex_value_types.get("MapBlocks").map(String::as_str),
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
            .advance_first_simulator_handshake_scaffold(FirstSimulatorHandshakeStage::UseCircuitCode)
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
                assert_eq!(from, FirstSimulatorHandshakeStage::BootstrapPrerequisitesReady);
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
        let listener_addr = listener.local_addr().expect("listener address should exist");

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
        assert_eq!(u32::from_le_bytes(body[0..4].try_into().expect("code bytes")), 424242);

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
    async fn send_first_simulator_complete_agent_movement_sends_datagram_and_waits_for_movement_complete(
    ) {
        let listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("listener bind should succeed");
        let listener_addr = listener.local_addr().expect("listener address should exist");

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
                assert_eq!(from, FirstSimulatorHandshakeStage::BootstrapPrerequisitesReady);
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
        assert_eq!(region.kind, FirstSimulatorInboundMessageKind::RegionHandshake);
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
        assert_eq!(enable.kind, FirstSimulatorInboundMessageKind::EnableSimulator);
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

        let agent_data =
            classify_first_simulator_inbound_message(&make_low_frequency_packet(387));
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

        let packet_ack = classify_first_simulator_inbound_message(&make_low_frequency_packet(0xFFFB));
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
        assert_eq!(viewer_effect.kind, FirstSimulatorInboundMessageKind::ViewerEffect);
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
        assert_eq!(coarse_location_update.packet_message_number, Some(0x0000ff06));

        let attached_sound =
            classify_first_simulator_inbound_message(&make_medium_frequency_packet(13));
        assert_eq!(attached_sound.kind, FirstSimulatorInboundMessageKind::AttachedSound);
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
        assert_eq!(irrelevant.kind, FirstSimulatorInboundMessageKind::Irrelevant);
        assert_eq!(irrelevant.scope, FirstSimulatorInboundTrafficScope::Unknown);
        assert_eq!(
            irrelevant.decode_source,
            FirstSimulatorInboundDecodeSource::Unknown
        );
        assert!(irrelevant.packet_message_number.is_some());
        assert!(irrelevant.signal.starts_with("packet:0x"));

        let unknown_packet = classify_first_simulator_inbound_message(&make_low_frequency_packet(42));
        assert_eq!(unknown_packet.kind, FirstSimulatorInboundMessageKind::Irrelevant);
        assert_eq!(unknown_packet.scope, FirstSimulatorInboundTrafficScope::Unknown);
        assert_eq!(
            unknown_packet.decode_source,
            FirstSimulatorInboundDecodeSource::Unknown
        );
        assert_eq!(unknown_packet.signal, "packet:0xffff002a:unmapped");
        assert_eq!(unknown_packet.packet_message_number, Some(0xffff002a));
    }

    #[tokio::test]
    async fn observe_agent_movement_complete_advances_waiting_stage() {
        let listener = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("listener bind should succeed");
        let listener_addr = listener.local_addr().expect("listener address should exist");

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
        let listener_addr = listener.local_addr().expect("listener address should exist");

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
        let listener_addr = listener.local_addr().expect("listener address should exist");

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
        assert_eq!(send_diagnostics[0].packet_id + 1, send_diagnostics[1].packet_id);
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
        let listener_addr = listener.local_addr().expect("listener address should exist");

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
        let listener_addr = listener.local_addr().expect("listener address should exist");

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
        assert_eq!(early_traffic[0].kind, EarlySimulatorTrafficKind::HealthMessage);
        assert_eq!(early_traffic[1].kind, EarlySimulatorTrafficKind::OnlineNotification);
        assert_eq!(
            early_traffic[2].kind,
            EarlySimulatorTrafficKind::CoarseLocationUpdate
        );
        assert_eq!(early_traffic[3].kind, EarlySimulatorTrafficKind::AttachedSound);
        assert_eq!(early_traffic[4].kind, EarlySimulatorTrafficKind::ViewerEffect);
        let early_summary = connection.summarize_early_simulator_traffic();
        assert_eq!(early_summary.observations, 5);
        assert_eq!(early_summary.health_message, 1);
        assert_eq!(early_summary.simulator_viewer_time_message, 0);
        assert_eq!(early_summary.online_notification, 1);
        assert_eq!(early_summary.viewer_effect, 1);
        assert_eq!(early_summary.coarse_location_update, 1);
        assert_eq!(early_summary.attached_sound, 1);
    }
}
