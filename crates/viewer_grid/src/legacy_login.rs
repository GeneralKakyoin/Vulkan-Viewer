use md5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyLoginName {
    pub first: String,
    pub last: String,
}

impl LegacyLoginName {
    pub fn new(first: String, last: String) -> Self {
        Self { first, last }
    }
}

pub fn classify_legacy_login_name(raw: &str) -> LegacyLoginName {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return LegacyLoginName::new(String::new(), String::new());
    }

    if let Some((first, last)) = trimmed.split_once('.') {
        if !first.is_empty() && !last.is_empty() {
            return LegacyLoginName::new(first.to_string(), last.to_string());
        }
    }

    let mut parts = trimmed.split_whitespace();
    if let Some(first) = parts.next() {
        let remainder: Vec<&str> = parts.collect();
        if !remainder.is_empty() {
            return LegacyLoginName::new(first.to_string(), remainder.join(" "));
        }
    }

    LegacyLoginName::new(trimmed.to_string(), String::from("Resident"))
}

pub fn split_legacy_name(username: &str) -> Option<(String, String)> {
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

pub fn normalize_legacy_passwd(passwd: &str) -> String {
    if passwd.starts_with("$1$") {
        return passwd.to_string();
    }

    let digest = md5::compute(passwd.as_bytes());
    format!("$1${digest:x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_preserves_legacy_prefix() {
        assert_eq!(
            normalize_legacy_passwd("$1$alreadyhashed"),
            "$1$alreadyhashed"
        );
    }

    #[test]
    fn normalize_hashes_plain_text() {
        assert_eq!(
            normalize_legacy_passwd("secret"),
            "$1$5ebe2294ecd0e0f08eab7690d2a6ee69"
        );
    }

    #[test]
    fn split_returns_last_resident_when_space_suffix() {
        assert_eq!(
            split_legacy_name("First Last"),
            Some((String::from("First"), String::from("Last")))
        );
    }
}
