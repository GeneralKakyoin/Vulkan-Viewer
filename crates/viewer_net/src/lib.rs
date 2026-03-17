use reqwest::{Method, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use thiserror::Error;
use viewer_grid::{
    GridAdapterError, GridLoginAdapter, GridLoginRequest, GridLoginResponse, GridLoginResult,
    LoginIntent, SessionBootstrap,
};

const MAX_LOGIN_REDIRECTS: usize = 4;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub first_name: String,
    pub last_name: String,
    pub password: String,
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

        let request = adapter.shape_login_request(&intent);
        let mut endpoint = self.config.endpoint.clone();
        let mut http_method = Method::POST;
        let mut redirects = 0;

        loop {
            let response = self
                .http_transport_login(&request, &endpoint, http_method.clone())
                .await?;
            let result = adapter.interpret_login_response(&response)?;

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

                    endpoint = next_url;
                    http_method = parsed_method;
                }
                GridLoginResult::Success(bootstrap) => {
                    self.apply_bootstrap_session(&bootstrap);
                    self.state = ConnectionState::LoggedIn;
                    return Ok(GridLoginResult::Success(bootstrap));
                }
                terminal => return Ok(terminal),
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
    ) -> Result<GridLoginResponse, ConnectionError> {
        let client = reqwest::Client::builder()
            .timeout(self.config.connect_timeout)
            .build()?;

        let response = client
            .request(method, endpoint)
            .json(request)
            .send()
            .await?;
        let status = response.status();
        let raw_json: Value = response.json().await?;

        if !status.is_success() {
            return Err(ConnectionError::HttpStatus {
                status,
                body: raw_json.to_string(),
            });
        }

        let normalized = normalize_login_response(raw_json);
        serde_json::from_value(normalized)
            .map_err(|err| ConnectionError::InvalidResponse(err.to_string()))
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
}
