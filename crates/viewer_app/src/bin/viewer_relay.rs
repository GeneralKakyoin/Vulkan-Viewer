use dotenvy::dotenv;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::time::Duration;
use viewer_grid::{
    GridLoginResult, LoginIntent, SecondLifeAdapter, StartLocation, StartLocationIntent,
};
use viewer_net::{Connection, ConnectionConfig, LoginWireFormat, SocialEvent};

fn parse_bool_like(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn parse_wire_format(value: &str) -> LoginWireFormat {
    match value.trim().to_ascii_lowercase().as_str() {
        "json" => LoginWireFormat::Json,
        "xmlrpc" | "xml-rpc" => LoginWireFormat::XmlRpc,
        _ => LoginWireFormat::Llsd,
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

fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn relay_line(category: &str, message: &str) -> String {
    format!("[{}] {category}: {message}", now_unix_ms())
}

fn append_jsonl(path: &str, category: &str, message: &str) {
    let line = format!(
        "{{\"ts\":{},\"category\":\"{}\",\"message\":\"{}\"}}\n",
        now_unix_ms(),
        category.replace('"', "'"),
        message.replace('"', "'"),
    );
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(line.as_bytes());
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let _ = dotenv();
    let endpoint = match std::env::var("VIEWER_LOGIN_ENDPOINT") {
        Ok(v) => v,
        Err(_) => {
            eprintln!("missing VIEWER_LOGIN_ENDPOINT");
            return;
        }
    };
    let username = match std::env::var("VIEWER_LOGIN_USERNAME") {
        Ok(v) => v,
        Err(_) => {
            eprintln!("missing VIEWER_LOGIN_USERNAME");
            return;
        }
    };
    let password = match std::env::var("VIEWER_LOGIN_PASSWORD") {
        Ok(v) => v,
        Err(_) => {
            eprintln!("missing VIEWER_LOGIN_PASSWORD");
            return;
        }
    };
    let wire_format = std::env::var("VIEWER_LOGIN_WIRE_FORMAT")
        .ok()
        .as_deref()
        .map(parse_wire_format)
        .unwrap_or(LoginWireFormat::Llsd);
    let start_location = std::env::var("VIEWER_LOGIN_START")
        .ok()
        .as_deref()
        .map(parse_start_location)
        .unwrap_or(StartLocationIntent::Saved(StartLocation::Last));
    let bind = std::env::var("VIEWER_FIRST_SIM_RECEIVE_BIND")
        .unwrap_or_else(|_| String::from("0.0.0.0:0"));
    let log_path = std::env::var("VIEWER_RELAY_LOG_PATH")
        .unwrap_or_else(|_| String::from("logs/viewer_relay.jsonl"));
    let _ = fs::create_dir_all("logs");
    let reconnect_enabled = std::env::var("VIEWER_RELAY_RECONNECT")
        .map(|v| parse_bool_like(&v))
        .unwrap_or(true);

    let adapter = SecondLifeAdapter;
    let mut reconnect_attempt = 0u32;
    println!("{}", relay_line("startup", "viewer_relay starting"));
    loop {
        let mut connection = Connection::new(ConnectionConfig {
            endpoint: endpoint.clone(),
            connect_timeout: Duration::from_secs(15),
            wire_format,
        });
        if let Err(err) = connection.connect().await {
            let msg = format!("connect failed: {err}");
            println!("{}", relay_line("connect", &msg));
            append_jsonl(&log_path, "connect", &msg);
            if !reconnect_enabled {
                break;
            }
            reconnect_attempt = reconnect_attempt.saturating_add(1);
            tokio::time::sleep(Duration::from_millis(
                (500 * (1 << reconnect_attempt.min(6))).min(15_000),
            ))
            .await;
            continue;
        }
        let intent = LoginIntent {
            username: username.clone(),
            password: password.clone(),
            start_location: start_location.clone(),
            agree_to_tos: false,
            read_critical: true,
            mfa_token: None,
        };
        let login = connection.login_with_trace(&adapter, intent).await;
        let (result, _trace) = match login {
            Ok(v) => v,
            Err(err) => {
                let msg = format!("login failed: {err}");
                println!("{}", relay_line("login", &msg));
                append_jsonl(&log_path, "login", &msg);
                if !reconnect_enabled {
                    break;
                }
                reconnect_attempt = reconnect_attempt.saturating_add(1);
                tokio::time::sleep(Duration::from_millis(
                    (500 * (1 << reconnect_attempt.min(6))).min(15_000),
                ))
                .await;
                continue;
            }
        };
        let bootstrap = match result {
            GridLoginResult::Success(v) => v,
            other => {
                let msg = format!("login terminal result: {other:?}");
                println!("{}", relay_line("login", &msg));
                append_jsonl(&log_path, "login", &msg);
                if !reconnect_enabled {
                    break;
                }
                reconnect_attempt = reconnect_attempt.saturating_add(1);
                tokio::time::sleep(Duration::from_millis(
                    (500 * (1 << reconnect_attempt.min(6))).min(15_000),
                ))
                .await;
                continue;
            }
        };
        let msg = format!(
            "connected as {} with {} bootstrap friends",
            bootstrap.agent_id,
            bootstrap.buddy_list.len()
        );
        println!("{}", relay_line("connected", &msg));
        append_jsonl(&log_path, "connected", &msg);

        let social_circuit = match connection.open_social_circuit(&bind).await {
            Ok(c) => c,
            Err(err) => {
                let msg = format!("social circuit failed: {err}");
                println!("{}", relay_line("social", &msg));
                append_jsonl(&log_path, "social", &msg);
                if !reconnect_enabled {
                    break;
                }
                reconnect_attempt = reconnect_attempt.saturating_add(1);
                tokio::time::sleep(Duration::from_millis(
                    (500 * (1 << reconnect_attempt.min(6))).min(15_000),
                ))
                .await;
                continue;
            }
        };
        let _ = connection
            .send_retrieve_instant_messages(&social_circuit)
            .await;
        reconnect_attempt = 0;
        loop {
            match connection
                .poll_social_events(&social_circuit, Duration::from_millis(250), 24)
                .await
            {
                Ok(events) => {
                    for event in events {
                        let msg = match event {
                            SocialEvent::FriendOnline { agent_id } => format!("{agent_id} online"),
                            SocialEvent::FriendOffline { agent_id } => {
                                format!("{agent_id} offline")
                            }
                            SocialEvent::FriendRights {
                                agent_id,
                                related_id,
                                rights,
                            } => format!(
                                "rights changed agent={agent_id} related={related_id} rights={rights}"
                            ),
                            SocialEvent::DirectIm(im) => {
                                format!(
                                    "im from={} to={} text={}",
                                    im.from_id, im.to_id, im.message
                                )
                            }
                        };
                        println!("{}", relay_line("social", &msg));
                        append_jsonl(&log_path, "social", &msg);
                    }
                }
                Err(err) => {
                    let msg = format!("social poll failed: {err}");
                    println!("{}", relay_line("social", &msg));
                    append_jsonl(&log_path, "social", &msg);
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(800)).await;
        }
        if !reconnect_enabled {
            break;
        }
    }
}
