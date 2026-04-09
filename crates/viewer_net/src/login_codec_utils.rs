use quick_xml::Reader;
use quick_xml::events::Event;
use roxmltree::{Document, Node};
use std::collections::HashMap;

use super::{CodecError, FriendBootstrapEntry, parse_llsd_scalar_map};

pub(super) fn llsd_string(key: &str, value: &str) -> String {
    format!(
        "<key>{}</key><string>{}</string>",
        escape_xml(key),
        escape_xml(value)
    )
}

pub(super) fn llsd_boolean(key: &str, value: bool) -> String {
    format!("<key>{}</key><boolean>{}</boolean>", escape_xml(key), value)
}

pub(super) fn xmlrpc_member_string(xml: &mut String, name: &str, value: &str) {
    xml.push_str("<member>");
    xml.push_str(&format!("<name>{}</name>", escape_xml(name)));
    xml.push_str(&format!(
        "<value><string>{}</string></value>",
        escape_xml(value)
    ));
    xml.push_str("</member>");
}

pub(super) fn xmlrpc_member_bool(xml: &mut String, name: &str, value: bool) {
    let xmlrpc_bool = if value { "1" } else { "0" };
    xml.push_str("<member>");
    xml.push_str(&format!("<name>{}</name>", escape_xml(name)));
    xml.push_str(&format!("<value><boolean>{xmlrpc_bool}</boolean></value>"));
    xml.push_str("</member>");
}

pub(super) fn xmlrpc_member_array_of_strings(xml: &mut String, name: &str, values: &[String]) {
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
pub(super) enum XmlRpcValue {
    String(String),
    Int(i64),
    Bool(bool),
    Struct(HashMap<String, XmlRpcValue>),
    Array(Vec<XmlRpcValue>),
}

pub(super) fn first_child_with_tag<'a, 'i>(node: Node<'a, 'i>, tag: &str) -> Option<Node<'a, 'i>> {
    node.children()
        .find(|child| child.is_element() && child.has_tag_name(tag))
}

pub(super) fn parse_xmlrpc_value(value_node: Node<'_, '_>) -> Result<XmlRpcValue, CodecError> {
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

pub(super) fn xmlrpc_as_string(value: &XmlRpcValue) -> Option<String> {
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

pub(super) fn xmlrpc_as_bool(value: &XmlRpcValue) -> Option<bool> {
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

pub(super) fn xmlrpc_as_u32(value: &XmlRpcValue) -> Option<u32> {
    match value {
        XmlRpcValue::Int(value) => u32::try_from(*value).ok(),
        XmlRpcValue::String(text) => text.parse::<u32>().ok(),
        XmlRpcValue::Bool(value) => Some(if *value { 1 } else { 0 }),
        XmlRpcValue::Struct(_) | XmlRpcValue::Array(_) => None,
    }
}

pub(super) fn xmlrpc_as_u16(value: &XmlRpcValue) -> Option<u16> {
    xmlrpc_as_u32(value).and_then(|value| u16::try_from(value).ok())
}

pub(super) fn xmlrpc_as_buddy_list(value: &XmlRpcValue) -> Option<Vec<FriendBootstrapEntry>> {
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

pub(super) fn escape_xml(text: &str) -> String {
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

pub(super) fn parse_llsd_map(body: &[u8]) -> Result<HashMap<String, LlsdValue>, CodecError> {
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

pub(super) fn parse_llsd_buddy_list(body: &[u8]) -> Result<Vec<FriendBootstrapEntry>, CodecError> {
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

pub(super) enum LlsdValue {
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

pub(super) fn llsd_to_string(value: &LlsdValue) -> Option<String> {
    match value {
        LlsdValue::String(text) => Some(text.clone()),
        LlsdValue::Integer(num) => Some(num.to_string()),
        LlsdValue::Bool(flag) => Some(flag.to_string()),
    }
}

pub(super) fn llsd_to_bool(value: &LlsdValue) -> Option<bool> {
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

pub(super) fn llsd_to_u32(value: &LlsdValue) -> Option<u32> {
    value.as_integer().and_then(|i| u32::try_from(i).ok())
}

pub(super) fn llsd_to_u16(value: &LlsdValue) -> Option<u16> {
    value.as_integer().and_then(|i| u16::try_from(i).ok())
}
