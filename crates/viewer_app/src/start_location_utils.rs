use super::*;

pub(super) fn parse_start_location(value: &str) -> StartLocationIntent {
    normalize_start_location_input(value)
        .unwrap_or_else(|_| StartLocationIntent::Uri(value.to_string()))
}

pub(super) fn parse_bool_like(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

pub(super) fn is_canonical_uuid_like(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 36 {
        return false;
    }
    for (idx, ch) in bytes.iter().enumerate() {
        let is_dash = matches!(idx, 8 | 13 | 18 | 23);
        if is_dash {
            if *ch != b'-' {
                return false;
            }
            continue;
        }
        if !ch.is_ascii_hexdigit() {
            return false;
        }
    }
    true
}

pub(super) fn parse_region_name_from_start_location(value: &str) -> Option<String> {
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

pub(super) fn describe_start_location_intent(start_location: &StartLocationIntent) -> String {
    match start_location {
        StartLocationIntent::Saved(StartLocation::Home) => String::from("home"),
        StartLocationIntent::Saved(StartLocation::Last) => String::from("last"),
        StartLocationIntent::Uri(uri) => uri.clone(),
    }
}

pub(super) fn apply_reconnect_teleport_request(
    tx: &mpsc::Sender<LiveFeedUpdate>,
    active_start_location: &mut StartLocationIntent,
    reconnect_reason: &mut Option<String>,
    should_reconnect: &mut bool,
    slurl: &str,
    source: &str,
) -> bool {
    if *should_reconnect {
        emit_relay(
            tx,
            RuntimeRelayLevel::Warn,
            "teleport",
            &format!(
                "teleport request ignored source={} input={} reason=reconnect_already_pending",
                source, slurl
            ),
        );
        return false;
    }

    match normalize_start_location_input(slurl) {
        Ok(start_location) => {
            let target = describe_start_location_intent(&start_location);
            emit_relay(
                tx,
                RuntimeRelayLevel::Info,
                "teleport",
                &format!(
                    "reconnect teleport requested source={} input={} target={}",
                    source, slurl, target
                ),
            );
            *active_start_location = start_location;
            *reconnect_reason = Some(format!("teleporting to {target}"));
            *should_reconnect = true;
            true
        }
        Err(err) => {
            emit_relay(
                tx,
                RuntimeRelayLevel::Warn,
                "teleport",
                &format!(
                    "teleport request rejected source={} input={} reason={}",
                    source, slurl, err
                ),
            );
            false
        }
    }
}

pub(super) fn normalize_start_location_input(value: &str) -> Result<StartLocationIntent> {
    let raw = value.trim();
    if raw.is_empty() {
        anyhow::bail!("empty start location");
    }
    if raw.eq_ignore_ascii_case("home") {
        return Ok(StartLocationIntent::Saved(StartLocation::Home));
    }
    if raw.eq_ignore_ascii_case("last") {
        return Ok(StartLocationIntent::Saved(StartLocation::Last));
    }
    if raw.to_ascii_lowercase().starts_with("uri:") {
        return Ok(StartLocationIntent::Uri(raw.to_string()));
    }
    if let Some(uri) = normalize_slurl_to_login_uri(raw) {
        return Ok(StartLocationIntent::Uri(uri));
    }
    if raw.contains("://") {
        anyhow::bail!("unsupported SLURL/start-location format");
    }
    Ok(StartLocationIntent::Uri(raw.to_string()))
}

pub(super) fn normalize_slurl_to_login_uri(value: &str) -> Option<String> {
    parse_secondlife_location_components(value)
        .map(|(region, x, y, z)| format!("uri:{region}&{x}&{y}&{z}"))
}

pub(super) fn parse_secondlife_location_components(value: &str) -> Option<(String, i32, i32, i32)> {
    let raw = value.trim();
    if raw.is_empty() {
        return None;
    }
    let lower = raw.to_ascii_lowercase();
    if lower.starts_with("secondlife:///app/teleport/") {
        let tail = &raw["secondlife:///app/teleport/".len()..];
        return parse_location_tail(tail);
    }
    if lower.starts_with("secondlife:///app/region/") {
        let tail = &raw["secondlife:///app/region/".len()..];
        return parse_location_tail(tail);
    }
    if lower.starts_with("secondlife://") {
        let tail = &raw["secondlife://".len()..];
        return parse_location_tail(tail);
    }
    if (lower.starts_with("https://maps.secondlife.com/secondlife/")
        || lower.starts_with("http://maps.secondlife.com/secondlife/"))
        && let Some(idx) = lower.find("/secondlife/")
    {
        let tail = &raw[idx + "/secondlife/".len()..];
        return parse_location_tail(tail);
    }
    None
}

pub(super) fn parse_location_tail(tail: &str) -> Option<(String, i32, i32, i32)> {
    let trimmed = tail.trim_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    let without_query = trimmed.split(['?', '#']).next().unwrap_or(trimmed);
    let mut segments = without_query
        .split('/')
        .filter(|segment| !segment.is_empty());
    let region = percent_decode_component(segments.next()?)?;
    let x = segments.next().and_then(parse_i32_segment).unwrap_or(128);
    let y = segments.next().and_then(parse_i32_segment).unwrap_or(128);
    let z = segments.next().and_then(parse_i32_segment).unwrap_or(0);
    Some((region, x, y, z))
}

pub(super) fn parse_i32_segment(value: &str) -> Option<i32> {
    let decoded = percent_decode_component(value)?;
    decoded.parse::<i32>().ok()
}

pub(super) fn percent_decode_component(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut idx = 0usize;
    while idx < bytes.len() {
        match bytes[idx] {
            b'%' if idx + 2 < bytes.len() => {
                let hi = decode_hex_digit(bytes[idx + 1])?;
                let lo = decode_hex_digit(bytes[idx + 2])?;
                decoded.push((hi << 4) | lo);
                idx += 3;
            }
            b'+' => {
                decoded.push(b' ');
                idx += 1;
            }
            byte => {
                decoded.push(byte);
                idx += 1;
            }
        }
    }
    String::from_utf8(decoded).ok()
}

pub(super) fn decode_hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
