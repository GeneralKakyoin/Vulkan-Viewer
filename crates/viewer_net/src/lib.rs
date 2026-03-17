use quick_xml::Reader;
use quick_xml::events::Event;
use reqwest::{Method, StatusCode, header::CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;
use viewer_grid::{
    GridAdapterError, GridLoginAdapter, GridLoginRequest, GridLoginResponse, GridLoginResult,
    LoginIntent, SessionBootstrap,
};

const MAX_LOGIN_REDIRECTS: usize = 4;

pub trait LoginCodec {
    fn content_type(&self) -> &str;
    fn encode_request(&self, request: &GridLoginRequest) -> Result<Vec<u8>, CodecError>;
    fn decode_response(&self, body: &[u8]) -> Result<GridLoginResponse, CodecError>;
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
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            endpoint: String::from("https://example.invalid"),
            connect_timeout: Duration::from_secs(10),
        }
    }
}

pub struct LlsdLoginCodec;

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
        xml.push_str(&llsd_string("password", &request.params.password));
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
}

/// Minimal networking boundary.
/// Real protocol transport and grid-specific behavior will plug in later.
#[derive(Debug)]
pub struct Connection {
    config: ConnectionConfig,
    state: ConnectionState,
    session: Option<Session>,
}

impl Connection {
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            state: ConnectionState::Disconnected,
            session: None,
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
        });
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
        let codec = JsonLoginCodec;
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
                .http_transport_login(&request, &endpoint, http_method.clone(), &codec)
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
        self.state = ConnectionState::Disconnected;
        Ok(())
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
        });
    }
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
    use viewer_grid::{GridLoginResult, SecondLifeAdapter, StartLocation, StartLocationIntent};
    use wiremock::matchers::{body_partial_json, method, path};
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
        assert!(encoded.contains("<string>inventory-root</string>"));
        assert!(encoded.contains("<key>options</key><array>"));
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
}
