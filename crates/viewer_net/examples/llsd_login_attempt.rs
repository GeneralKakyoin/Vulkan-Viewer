use std::time::Duration;

use viewer_grid::{
    GridLoginResult, LoginIntent, SecondLifeAdapter, StartLocation, StartLocationIntent,
};
use viewer_net::{Connection, ConnectionConfig, ConnectionError, LoginWireFormat};

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("Login attempt failed: {err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let endpoint = required_env("VIEWER_LOGIN_ENDPOINT")?;
    let username = required_env("VIEWER_LOGIN_USERNAME")?;
    let password = required_env("VIEWER_LOGIN_PASSWORD")?;

    let start_location = parse_start_location(
        &std::env::var("VIEWER_LOGIN_START").unwrap_or_else(|_| String::from("last")),
    );
    let agree_to_tos = parse_bool_env("VIEWER_LOGIN_AGREE_TOS", false);
    let read_critical = parse_bool_env("VIEWER_LOGIN_READ_CRITICAL", true);
    let connect_timeout_secs = parse_u64_env("VIEWER_LOGIN_TIMEOUT_SECS", 15);
    let mfa_token = std::env::var("VIEWER_LOGIN_MFA_TOKEN").ok();
    let wire_format = parse_wire_format_env("VIEWER_LOGIN_WIRE_FORMAT", LoginWireFormat::Llsd);
    let fetch_seed_caps = parse_bool_env("VIEWER_FETCH_SEED_CAPS", false);
    let inspect_event_queue_once = parse_bool_env("VIEWER_INSPECT_EVENT_QUEUE_ONCE", false);

    let intent = LoginIntent {
        username,
        password,
        start_location,
        agree_to_tos,
        read_critical,
        mfa_token,
    };

    let mut connection = Connection::new(ConnectionConfig {
        endpoint,
        connect_timeout: Duration::from_secs(connect_timeout_secs),
        wire_format,
    });
    let adapter = SecondLifeAdapter;

    connection
        .connect()
        .await
        .map_err(|err| format!("connect failed: {err}"))?;

    match connection.login_with_trace(&adapter, intent).await {
        Ok((result, trace)) => {
            println!("Login outcome: {}", result_kind(&result));
            println!("Sanitized trace:\n{trace:#?}");
            println!("Interpreted result:\n{result:#?}");
            if fetch_seed_caps && matches!(result, GridLoginResult::Success(_)) {
                let caps = connection
                    .fetch_seed_capabilities()
                    .await
                    .map_err(|err| format!("seed capability fetch failed: {err}"))?;
                println!("Seed capability entries: {}", caps.entries.len());
                for name in caps.entries.keys() {
                    println!("Capability: {name}");
                }
                if inspect_event_queue_once {
                    if let Some(event_queue_url) = caps.entries.get("EventQueueGet") {
                        match connection.fetch_event_queue_once(event_queue_url).await {
                            Ok(inspection) => {
                                println!(
                                    "EventQueueGet one-shot: has_events={}, has_id={}, event_count={}",
                                    inspection.has_events_array,
                                    inspection.has_id,
                                    inspection.event_count
                                );
                                for event_name in &inspection.event_names {
                                    println!("Event: {event_name}");
                                }
                            }
                            Err(ConnectionError::EventQueueOneShotFailed { attempts, .. }) => {
                                println!("EventQueueGet one-shot failed after {} attempts", attempts.len());
                                for attempt in &attempts {
                                    println!(
                                        "Attempt {}: status={:?}, elapsed_ms={}, retryable={}, error_kind={}",
                                        attempt.attempt,
                                        attempt.status,
                                        attempt.elapsed_ms,
                                        attempt.retryable,
                                        attempt.error_kind
                                    );
                                    for (name, value) in &attempt.response_headers {
                                        println!("  Header: {name}={value}");
                                    }
                                }
                                return Err(String::from("event queue one-shot fetch failed"));
                            }
                            Err(err) => {
                                return Err(format!("event queue one-shot fetch failed: {err}"));
                            }
                        }
                    } else {
                        println!("EventQueueGet capability not present in seed map");
                    }
                }
            }
            Ok(())
        }
        Err(err) => {
            eprintln!("Login transport/protocol error: {err}");
            eprintln!("Error detail: {err:?}");
            Err(String::from("login did not complete successfully"))
        }
    }
}

fn required_env(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|_| {
        format!(
            "missing required environment variable {name}. Set VIEWER_LOGIN_ENDPOINT, VIEWER_LOGIN_USERNAME, and VIEWER_LOGIN_PASSWORD to run this example."
        )
    })
}

fn parse_bool_env(name: &str, default: bool) -> bool {
    match std::env::var(name) {
        Ok(value) => matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"),
        Err(_) => default,
    }
}

fn parse_u64_env(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default)
}

fn parse_wire_format_env(name: &str, default: LoginWireFormat) -> LoginWireFormat {
    match std::env::var(name) {
        Ok(value) => match value.trim().to_ascii_lowercase().as_str() {
            "json" => LoginWireFormat::Json,
            "llsd" => LoginWireFormat::Llsd,
            "xmlrpc" | "xml-rpc" => LoginWireFormat::XmlRpc,
            _ => default,
        },
        Err(_) => default,
    }
}

fn parse_start_location(value: &str) -> StartLocationIntent {
    if value.eq_ignore_ascii_case("home") {
        StartLocationIntent::Saved(StartLocation::Home)
    } else if value.eq_ignore_ascii_case("last") {
        StartLocationIntent::Saved(StartLocation::Last)
    } else {
        StartLocationIntent::Uri(value.to_string())
    }
}

fn result_kind(result: &GridLoginResult) -> &'static str {
    match result {
        GridLoginResult::Success(_) => "success",
        GridLoginResult::Redirect { .. } => "redirect",
        GridLoginResult::RequiresTos { .. } => "requires_tos",
        GridLoginResult::RequiresMfa { .. } => "requires_mfa",
        GridLoginResult::UpdateRequired { .. } => "update_required",
        GridLoginResult::Failed(_) => "failed",
    }
}
