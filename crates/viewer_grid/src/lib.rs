use serde::{Deserialize, Serialize};
use thiserror::Error;

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
    pub password: String,
    pub start: String,
    pub agree_to_tos: bool,
    pub read_critical: bool,
    pub token: Option<String>,
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
    Success(SessionBootstrap),
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

        GridLoginRequest {
            method: String::from("login_to_simulator"),
            params: GridLoginRequestParams {
                username: intent.username.clone(),
                password: intent.password.clone(),
                start,
                agree_to_tos: intent.agree_to_tos,
                read_critical: intent.read_critical,
                token: intent.mfa_token.clone(),
            },
            // Minimal option set based on research note; expand later per adapter policy.
            options: vec![
                String::from("inventory-root"),
                String::from("inventory-skeleton"),
                String::from("buddy-list"),
                String::from("event_categories"),
                String::from("classified_categories"),
                String::from("max-agent-groups"),
                String::from("map-server-url"),
                String::from("voice-config"),
                String::from("login-flags"),
            ],
        }
    }

    fn interpret_login_response(
        &self,
        response: &GridLoginResponse,
    ) -> Result<GridLoginResult, GridAdapterError> {
        if response.login == Some(true) {
            return Ok(GridLoginResult::Success(extract_bootstrap(response)?));
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
