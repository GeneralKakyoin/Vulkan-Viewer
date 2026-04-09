use reqwest::header::{HeaderMap, SET_COOKIE};
use std::collections::BTreeMap;

pub(super) fn extract_cookie_header_from_set_cookie(headers: &HeaderMap) -> Option<String> {
    let mut cookie_pairs = Vec::new();
    for set_cookie in headers.get_all(SET_COOKIE) {
        let Ok(raw) = set_cookie.to_str() else {
            continue;
        };
        let pair = raw.split(';').next().unwrap_or_default().trim();
        if pair.is_empty() {
            continue;
        }
        let Some((name, value)) = pair.split_once('=') else {
            continue;
        };
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        cookie_pairs.push((name.to_string(), value.trim().to_string()));
    }
    if cookie_pairs.is_empty() {
        return None;
    }
    let mut deduped = BTreeMap::new();
    for (name, value) in cookie_pairs {
        deduped.insert(name, value);
    }
    Some(
        deduped
            .into_iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join("; "),
    )
}

pub(super) fn merge_cookie_header(base: Option<&str>, update: Option<&str>) -> Option<String> {
    let mut merged = BTreeMap::new();
    for source in [base, update] {
        let Some(source) = source else {
            continue;
        };
        for token in source.split(';') {
            let token = token.trim();
            if token.is_empty() {
                continue;
            }
            let Some((name, value)) = token.split_once('=') else {
                continue;
            };
            let name = name.trim();
            if name.is_empty() {
                continue;
            }
            merged.insert(name.to_string(), value.trim().to_string());
        }
    }
    if merged.is_empty() {
        return None;
    }
    Some(
        merged
            .into_iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join("; "),
    )
}
