use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod continuity;
pub mod legacy_login;

pub use legacy_login::{
    LegacyLoginName, classify_legacy_login_name, normalize_legacy_passwd, split_legacy_name,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StartLocation {
    Last,
    Home,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StartLocationIntent {
    Saved(StartLocation),
    Uri(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginIntent {
    pub username: String,
    pub password: String,
    pub start_location: StartLocationIntent,
    pub agree_to_tos: bool,
    pub read_critical: bool,
    pub mfa_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridLoginRequest {
    pub method: String,
    pub params: GridLoginRequestParams,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridLoginRequestParams {
    pub username: String,
    #[serde(rename = "passwd")]
    pub password: String,
    pub start: String,
    pub agree_to_tos: bool,
    pub read_critical: bool,
    pub token: Option<String>,
    pub channel: String,
    pub version: String,
    pub platform: String,
    pub platform_version: String,
    pub host_id: String,
    pub machine_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GridLoginResponse {
    pub login: Option<bool>,
    pub reason: Option<String>,
    pub message: Option<String>,
    pub message_id: Option<String>,
    pub next_url: Option<String>,
    pub next_method: Option<String>,
    pub agent_id: Option<String>,
    pub session_id: Option<String>,
    pub secure_session_id: Option<String>,
    pub circuit_code: Option<u32>,
    pub sim_ip: Option<String>,
    pub sim_port: Option<u16>,
    pub region_x: Option<u32>,
    pub region_y: Option<u32>,
    pub seed_capability: Option<String>,
    pub start_location: Option<String>,
    pub look_at: Option<String>,
    pub home: Option<String>,
    pub motd: Option<String>,
    #[serde(default, rename = "buddy-list")]
    pub buddy_list: Vec<FriendBootstrapEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FriendBootstrapEntry {
    #[serde(default, rename = "buddy_id")]
    pub buddy_id: String,
    #[serde(default, rename = "buddy_rights_has")]
    pub rights_has: i32,
    #[serde(default, rename = "buddy_rights_given")]
    pub rights_given: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionBootstrap {
    pub agent_id: String,
    pub session_id: String,
    pub secure_session_id: String,
    pub circuit_code: u32,
    pub first_sim: FirstSimulator,
    pub seed_capability: String,
    pub start_location: Option<String>,
    pub look_at: Option<String>,
    pub home: Option<String>,
    pub motd: Option<String>,
    pub buddy_list: Vec<FriendBootstrapEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstSimulator {
    pub sim_ip: String,
    pub sim_port: u16,
    pub region_x: u32,
    pub region_y: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GridLoginResult {
    Success(Box<SessionBootstrap>),
    Redirect {
        next_url: String,
        next_method: String,
    },
    RequiresTos {
        message: Option<String>,
    },
    RequiresMfa {
        message: Option<String>,
    },
    UpdateRequired {
        message: Option<String>,
    },
    Failed(GridLoginError),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridLoginError {
    pub class: GridLoginErrorClass,
    pub reason: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GridLoginErrorClass {
    AuthFailed,
    SessionConflict,
    ServiceUnavailable,
    ViewerRejected,
    InvalidResponse,
    Transport,
    Unknown,
}

#[derive(Debug, Error)]
pub enum GridAdapterError {
    #[error("missing bootstrap field: {0}")]
    MissingField(&'static str),
}

const SECOND_LIFE_CLIENT_CHANNEL: &str = "rust-viewer";
const SECOND_LIFE_CLIENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const SECOND_LIFE_PLATFORM: &str = std::env::consts::OS;
const SECOND_LIFE_PLATFORM_VERSION: &str = env!("CARGO_PKG_VERSION");
const SECOND_LIFE_HOST_ID: &str = "rust-viewer-host";
const SECOND_LIFE_MACHINE_HASH: &str = "rust-viewer-machine-hash";
const SECOND_LIFE_LOGIN_OPTIONS: &[&str] = &[
    "inventory-root",
    "inventory-skeleton",
    "buddy-list",
    "event_categories",
    "classified_categories",
    "max-agent-groups",
    "map-server-url",
    "voice-config",
    "login-flags",
    "event_queue",
];

/// Grid boundary for login shaping and response interpretation.
/// Transport/session mechanics remain in viewer_net.
pub trait GridLoginAdapter {
    fn grid_id(&self) -> &'static str;
    fn shape_login_request(&self, intent: &LoginIntent) -> GridLoginRequest;
    fn interpret_login_response(
        &self,
        response: &GridLoginResponse,
    ) -> Result<GridLoginResult, GridAdapterError>;
}

#[derive(Debug, Default)]
pub struct SecondLifeAdapter;

impl GridLoginAdapter for SecondLifeAdapter {
    fn grid_id(&self) -> &'static str {
        "secondlife"
    }

    fn shape_login_request(&self, intent: &LoginIntent) -> GridLoginRequest {
        let start = match &intent.start_location {
            StartLocationIntent::Saved(StartLocation::Last) => String::from("last"),
            StartLocationIntent::Saved(StartLocation::Home) => String::from("home"),
            StartLocationIntent::Uri(uri) => uri.clone(),
        };

        let normalized_password = normalize_legacy_passwd(&intent.password);
        GridLoginRequest {
            method: String::from("login_to_simulator"),
            params: GridLoginRequestParams {
                username: intent.username.clone(),
                password: normalized_password,
                start,
                agree_to_tos: intent.agree_to_tos,
                read_critical: intent.read_critical,
                token: intent.mfa_token.clone(),
                channel: String::from(SECOND_LIFE_CLIENT_CHANNEL),
                version: String::from(SECOND_LIFE_CLIENT_VERSION),
                platform: String::from(SECOND_LIFE_PLATFORM),
                platform_version: String::from(SECOND_LIFE_PLATFORM_VERSION),
                host_id: String::from(SECOND_LIFE_HOST_ID),
                machine_hash: String::from(SECOND_LIFE_MACHINE_HASH),
            },
            options: SECOND_LIFE_LOGIN_OPTIONS
                .iter()
                .map(|item| item.to_string())
                .collect(),
        }
    }

    fn interpret_login_response(
        &self,
        response: &GridLoginResponse,
    ) -> Result<GridLoginResult, GridAdapterError> {
        if response.login == Some(true) {
            return Ok(GridLoginResult::Success(Box::new(extract_bootstrap(
                response,
            )?)));
        }

        let reason = response.reason.as_deref().unwrap_or_default();
        let message = response.message.clone();

        let result = match reason {
            "indeterminate" => {
                let next_url = response
                    .next_url
                    .clone()
                    .ok_or(GridAdapterError::MissingField("next_url"))?;
                let next_method = response
                    .next_method
                    .clone()
                    .ok_or(GridAdapterError::MissingField("next_method"))?;
                GridLoginResult::Redirect {
                    next_url,
                    next_method,
                }
            }
            "tos" => GridLoginResult::RequiresTos { message },
            "mfa_challenge" => GridLoginResult::RequiresMfa { message },
            "update" | "optional" => GridLoginResult::UpdateRequired { message },
            _ => GridLoginResult::Failed(classify_failure(response)),
        };

        Ok(result)
    }
}

fn extract_bootstrap(response: &GridLoginResponse) -> Result<SessionBootstrap, GridAdapterError> {
    Ok(SessionBootstrap {
        agent_id: response
            .agent_id
            .clone()
            .ok_or(GridAdapterError::MissingField("agent_id"))?,
        session_id: response
            .session_id
            .clone()
            .ok_or(GridAdapterError::MissingField("session_id"))?,
        secure_session_id: response
            .secure_session_id
            .clone()
            .ok_or(GridAdapterError::MissingField("secure_session_id"))?,
        circuit_code: response
            .circuit_code
            .ok_or(GridAdapterError::MissingField("circuit_code"))?,
        first_sim: FirstSimulator {
            sim_ip: response
                .sim_ip
                .clone()
                .ok_or(GridAdapterError::MissingField("sim_ip"))?,
            sim_port: response
                .sim_port
                .ok_or(GridAdapterError::MissingField("sim_port"))?,
            region_x: response
                .region_x
                .ok_or(GridAdapterError::MissingField("region_x"))?,
            region_y: response
                .region_y
                .ok_or(GridAdapterError::MissingField("region_y"))?,
        },
        seed_capability: response
            .seed_capability
            .clone()
            .ok_or(GridAdapterError::MissingField("seed_capability"))?,
        start_location: response.start_location.clone(),
        look_at: response.look_at.clone(),
        home: response.home.clone(),
        motd: response.motd.clone(),
        buddy_list: response.buddy_list.clone(),
    })
}

fn classify_failure(response: &GridLoginResponse) -> GridLoginError {
    let reason = response.reason.clone();
    let class = match response.reason.as_deref() {
        Some("key") => GridLoginErrorClass::AuthFailed,
        Some("presence") => GridLoginErrorClass::SessionConflict,
        Some("connect") => GridLoginErrorClass::Transport,
        Some("critical") => GridLoginErrorClass::ServiceUnavailable,
        Some("viewer") => GridLoginErrorClass::ViewerRejected,
        Some("parse") => GridLoginErrorClass::InvalidResponse,
        Some(_) => GridLoginErrorClass::Unknown,
        None => GridLoginErrorClass::Unknown,
    };

    GridLoginError {
        class,
        reason,
        message: response.message.clone(),
    }
}

pub struct AssetCapabilityPolicy;

impl AssetCapabilityPolicy {
    pub fn texture_url_candidates(
        capabilities: &std::collections::BTreeMap<String, String>,
        asset_id: &viewer_core::AssetID,
    ) -> Vec<String> {
        if asset_id.is_empty() {
            return Vec::new();
        }

        let mut urls = Vec::new();
        if let Some(base_url) = capabilities.get("GetTexture") {
            extend_unique(
                &mut urls,
                Self::texture_url_candidates_from_base(base_url, asset_id.as_str()),
            );
        }
        if let Some(base_url) = capabilities.get("ViewerAsset") {
            extend_unique(
                &mut urls,
                Self::texture_url_candidates_from_base(base_url, asset_id.as_str()),
            );
        }
        urls
    }

    pub fn texture_url_candidates_from_base(base_url: &str, asset_id: &str) -> Vec<String> {
        let base = base_url.trim().trim_end_matches('/');
        let asset_id = asset_id.trim();
        if base.is_empty() || asset_id.is_empty() {
            return Vec::new();
        }

        let mut urls = Vec::new();
        extend_unique(
            &mut urls,
            [
                format!("{base}/?texture_id={asset_id}"),
                format!("{base}?texture_id={asset_id}"),
                format!("{base}/{asset_id}"),
                format!("{base}?id={asset_id}"),
                format!("{base}?asset_id={asset_id}"),
            ],
        );
        urls
    }

    pub fn select_texture_url(
        capabilities: &std::collections::BTreeMap<String, String>,
        asset_id: &viewer_core::AssetID,
    ) -> Option<String> {
        Self::texture_url_candidates(capabilities, asset_id)
            .into_iter()
            .next()
    }
}

fn extend_unique<I>(out: &mut Vec<String>, candidates: I)
where
    I: IntoIterator<Item = String>,
{
    for candidate in candidates {
        if !out.contains(&candidate) {
            out.push(candidate);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_intent() -> LoginIntent {
        LoginIntent {
            username: String::from("test.user"),
            password: String::from("secret"),
            start_location: StartLocationIntent::Saved(StartLocation::Last),
            agree_to_tos: true,
            read_critical: true,
            mfa_token: None,
        }
    }

    #[test]
    fn secondlife_request_populates_metadata() {
        let request = SecondLifeAdapter.shape_login_request(&make_intent());
        assert_eq!(request.params.channel, SECOND_LIFE_CLIENT_CHANNEL);
        assert_eq!(request.params.version, SECOND_LIFE_CLIENT_VERSION);
        assert_eq!(request.params.platform, SECOND_LIFE_PLATFORM);
        assert_eq!(
            request.params.platform_version,
            SECOND_LIFE_PLATFORM_VERSION
        );
        assert_eq!(request.params.host_id, SECOND_LIFE_HOST_ID);
        assert_eq!(request.params.machine_hash, SECOND_LIFE_MACHINE_HASH);
    }

    #[test]
    fn secondlife_request_honors_uri_start() {
        let mut intent = make_intent();
        intent.start_location = StartLocationIntent::Uri(String::from("uri://some.where"));
        let request = SecondLifeAdapter.shape_login_request(&intent);
        assert_eq!(request.params.start, "uri://some.where");
    }

    #[test]
    fn secondlife_request_options_include_event_queue() {
        let request = SecondLifeAdapter.shape_login_request(&make_intent());
        assert!(request.options.iter().any(|opt| opt == "event_queue"));
    }
    #[test]
    fn secondlife_request_hashes_password() {
        let request = SecondLifeAdapter.shape_login_request(&make_intent());
        assert_eq!(
            request.params.password,
            "$1$5ebe2294ecd0e0f08eab7690d2a6ee69"
        );
    }

    #[test]
    fn asset_policy_prioritizes_get_texture() {
        let mut caps = std::collections::BTreeMap::new();
        caps.insert("GetTexture".to_string(), "http://cdn".to_string());
        caps.insert("ViewerAsset".to_string(), "http://fallback".to_string());

        let id = viewer_core::AssetID::new("test-id");
        let url = AssetCapabilityPolicy::select_texture_url(&caps, &id).unwrap();
        assert_eq!(url, "http://cdn/?texture_id=test-id");
    }

    #[test]
    fn asset_policy_exposes_firestorm_style_first_then_legacy_fallbacks() {
        let candidates =
            AssetCapabilityPolicy::texture_url_candidates_from_base("http://cdn", "test-id");
        assert_eq!(
            candidates,
            vec![
                "http://cdn/?texture_id=test-id",
                "http://cdn?texture_id=test-id",
                "http://cdn/test-id",
                "http://cdn?id=test-id",
                "http://cdn?asset_id=test-id",
            ]
        );
    }

    #[test]
    fn asset_policy_falls_back_to_viewer_asset() {
        let mut caps = std::collections::BTreeMap::new();
        caps.insert("ViewerAsset".to_string(), "http://fallback".to_string());

        let id = viewer_core::AssetID::new("test-id");
        let url = AssetCapabilityPolicy::select_texture_url(&caps, &id).unwrap();
        assert_eq!(url, "http://fallback/?texture_id=test-id");
    }

    #[test]
    fn asset_policy_returns_none_on_missing_caps_or_id() {
        let caps = std::collections::BTreeMap::new();
        let id = viewer_core::AssetID::new("test-id");
        assert!(AssetCapabilityPolicy::select_texture_url(&caps, &id).is_none());

        let mut caps = std::collections::BTreeMap::new();
        caps.insert("GetTexture".to_string(), "http://cdn".to_string());
        assert!(
            AssetCapabilityPolicy::select_texture_url(&caps, &viewer_core::AssetID::default())
                .is_none()
        );
        assert!(
            AssetCapabilityPolicy::texture_url_candidates(&caps, &viewer_core::AssetID::default())
                .is_empty()
        );
    }
}
