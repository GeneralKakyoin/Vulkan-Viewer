use std::time::Duration;

use viewer_grid::{
    GridLoginResult, LoginIntent, SecondLifeAdapter, StartLocation, StartLocationIntent,
};
use viewer_net::{Connection, ConnectionConfig, LoginWireFormat};

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("LLSD login attempt failed: {err}");
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
        wire_format: LoginWireFormat::Llsd,
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
