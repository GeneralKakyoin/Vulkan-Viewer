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
    let inspect_simulator_features_once =
        parse_bool_env("VIEWER_INSPECT_SIMULATOR_FEATURES_ONCE", false);
    let inspect_map_layer_once = parse_bool_env("VIEWER_INSPECT_MAP_LAYER_ONCE", false);
    let inspect_first_sim_handshake_once =
        parse_bool_env("VIEWER_INSPECT_FIRST_SIM_HANDSHAKE_ONCE", false);
    let first_sim_receive_bind = std::env::var("VIEWER_FIRST_SIM_RECEIVE_BIND")
        .unwrap_or_else(|_| String::from("0.0.0.0:0"));
    let first_sim_receive_timeout_secs =
        parse_u64_env("VIEWER_FIRST_SIM_RECEIVE_TIMEOUT_SECS", 5);
    let first_sim_receive_max_packets =
        parse_u64_env("VIEWER_FIRST_SIM_RECEIVE_MAX_PACKETS", 3) as usize;
    let first_sim_post_movement_tail_packets =
        parse_u64_env("VIEWER_FIRST_SIM_POST_MOVEMENT_TAIL_PACKETS", 0) as usize;
    let first_sim_post_movement_timeout_secs =
        parse_optional_u64_env("VIEWER_FIRST_SIM_POST_MOVEMENT_TIMEOUT_SECS");
    let first_sim_stop_on_region_control =
        parse_bool_env("VIEWER_FIRST_SIM_STOP_ON_REGION_CONTROL", false);

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
                if inspect_simulator_features_once {
                    if let Some(sim_features_url) = caps.entries.get("SimulatorFeatures") {
                        let inspection = connection
                            .fetch_simulator_features_once(sim_features_url)
                            .await
                            .map_err(|err| format!("simulator features fetch failed: {err}"))?;
                        println!(
                            "SimulatorFeatures one-shot: keys={}, scalar_values={}, complex_values={}",
                            inspection.top_level_keys.len(),
                            inspection.scalar_values.len(),
                            inspection.complex_value_types.len()
                        );
                        for key in &inspection.top_level_keys {
                            println!("SimulatorFeatures key: {key}");
                        }
                    } else {
                        println!("SimulatorFeatures capability not present in seed map");
                    }
                }
                if inspect_map_layer_once {
                    if let Some(map_layer_url) = caps.entries.get("MapLayer") {
                        let inspection = connection
                            .fetch_map_layer_once(map_layer_url)
                            .await
                            .map_err(|err| format!("map layer fetch failed: {err}"))?;
                        println!(
                            "MapLayer one-shot: keys={}, scalar_values={}, complex_values={}",
                            inspection.top_level_keys.len(),
                            inspection.scalar_values.len(),
                            inspection.complex_value_types.len()
                        );
                        for key in &inspection.top_level_keys {
                            println!("MapLayer key: {key}");
                        }
                    } else {
                        println!("MapLayer capability not present in seed map");
                    }
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
                if inspect_first_sim_handshake_once {
                    inspect_first_simulator_handshake_once(
                        &mut connection,
                        &first_sim_receive_bind,
                        Duration::from_secs(first_sim_receive_timeout_secs),
                        first_sim_receive_max_packets,
                        first_sim_post_movement_tail_packets,
                        first_sim_post_movement_timeout_secs.map(Duration::from_secs),
                        first_sim_stop_on_region_control,
                    )
                    .await?;
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

fn parse_optional_u64_env(name: &str) -> Option<u64> {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
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

async fn inspect_first_simulator_handshake_once(
    connection: &mut Connection,
    receive_bind: &str,
    receive_timeout: Duration,
    max_packets: usize,
    post_movement_tail_packets: usize,
    post_movement_timeout: Option<Duration>,
    stop_on_region_control: bool,
) -> Result<(), String> {
    println!(
        "First-simulator receive window attempt: bind={}, timeout_secs={}, max_packets={}, post_movement_tail_packets={}, post_movement_timeout_secs={:?}, stop_on_region_control={}",
        receive_bind,
        receive_timeout.as_secs(),
        max_packets,
        post_movement_tail_packets,
        post_movement_timeout.map(|d| d.as_secs()),
        stop_on_region_control
    );
    match connection
        .probe_first_simulator_handshake_window_with_policy(
            receive_bind,
            receive_timeout,
            max_packets,
            post_movement_tail_packets,
            post_movement_timeout,
            stop_on_region_control,
        )
        .await
    {
        Ok(report) => {
            println!(
                "First-simulator receive report: observations={}, timed_out={}, movement_complete_observation_index={:?}, post_movement_observations={}",
                report.observations.len(),
                report.timed_out,
                report.agent_movement_complete_observation_index,
                report.post_movement_observations
            );
            if let Some(summary) = &report.post_boundary_summary {
                println!(
                    "Post-boundary summary: observations={}, bootstrap_relevant={}, transport_control={}, region_transition_control={}, likely_broader_traffic={}, unknown={}, crossed_region={}, confirm_enable_simulator={}, watched_region_transition_control_not_seen={}",
                    summary.observations,
                    summary.bootstrap_relevant,
                    summary.transport_control,
                    summary.region_transition_control,
                    summary.likely_broader_traffic,
                    summary.unknown,
                    summary.crossed_region,
                    summary.confirm_enable_simulator,
                    summary.watched_region_transition_control_not_seen
                );
                if !summary.unknown_packet_message_numbers.is_empty() {
                    println!(
                        "Post-boundary unknown packet numbers: {:?}",
                        summary.unknown_packet_message_numbers
                    );
                }
                if !summary.repeated_unknown_packet_message_numbers.is_empty() {
                    println!(
                        "Post-boundary repeated unknown packet numbers: {:?}",
                        summary.repeated_unknown_packet_message_numbers
                    );
                }
                for (idx, kind) in summary.kinds.iter().enumerate() {
                    println!("Post-boundary kind {}: {:?}", idx + 1, kind);
                }
            }
            for obs in &report.observations {
                println!(
                    "Observation {}: kind={:?}, scope={:?}, source={:?}, signal={}, packet_message_number={:?}, payload_len={}",
                    obs.observation_index,
                    obs.classification.kind,
                    obs.classification.scope,
                    obs.classification.decode_source,
                    obs.classification.signal,
                    obs.classification.packet_message_number,
                    obs.payload_len
                );
            }
        }
        Err(err) => {
            println!("First-simulator one-shot receive failed: {err}");
            println!("First-simulator receive error detail: {err:?}");
        }
    }

    let send_diagnostics = connection.first_simulator_handshake_send_diagnostics();
    println!(
        "First-simulator send diagnostics entries: {}",
        send_diagnostics.len()
    );
    for diag in send_diagnostics {
        println!(
            "Send diag: action={:?}, target={}, packet_id={}, packet_message_number={:?}, payload_len={}, elapsed_ms={}, success={}, error={:?}",
            diag.action,
            diag.target,
            diag.packet_id,
            diag.packet_message_number,
            diag.payload_len,
            diag.elapsed_ms,
            diag.success,
            diag.error
        );
    }

    let receive_diagnostics = connection.first_simulator_handshake_receive_diagnostics();
    println!(
        "First-simulator receive diagnostics entries: {}",
        receive_diagnostics.len()
    );
    for diag in receive_diagnostics {
        println!(
            "Receive diag: kind={:?}, scope={:?}, source={:?}, packet_message_number={:?}, payload_len={}, stage_before={:?}, stage_after={:?}, advanced_stage={}, signal={}",
            diag.kind,
            diag.scope,
            diag.decode_source,
            diag.packet_message_number,
            diag.payload_len,
            diag.stage_before,
            diag.stage_after,
            diag.advanced_stage,
            diag.signal
        );
    }

    let early_traffic = connection.early_simulator_traffic_observations();
    let early_summary = connection.summarize_early_simulator_traffic();
    println!(
        "Early traffic summary: observations={}, health={}, simulator_viewer_time={}, online_notification={}, viewer_effect={}, coarse_location_update={}, attached_sound={}",
        early_summary.observations,
        early_summary.health_message,
        early_summary.simulator_viewer_time_message,
        early_summary.online_notification,
        early_summary.viewer_effect,
        early_summary.coarse_location_update,
        early_summary.attached_sound
    );
    println!(
        "Early simulator traffic observations: {}",
        early_traffic.len()
    );
    for obs in early_traffic {
        println!(
            "Early traffic: observation_index={}, kind={:?}, packet_message_number={:?}, payload_len={}, signal={}",
            obs.observation_index, obs.kind, obs.packet_message_number, obs.payload_len, obs.signal
        );
    }

    let handoff_summary = connection.summarize_region_transition_control();
    println!(
        "Region-transition control summary: observations={}, crossed_region={}, confirm_enable_simulator={}, not_seen_in_run={}",
        handoff_summary.observations,
        handoff_summary.crossed_region,
        handoff_summary.confirm_enable_simulator,
        handoff_summary.not_seen_in_run
    );
    let handoff_observations = connection.region_transition_control_observations();
    println!(
        "Region-transition control observations: {}",
        handoff_observations.len()
    );
    for obs in handoff_observations {
        println!(
            "Region-transition control: observation_index={}, kind={:?}, packet_message_number={:?}, payload_len={}, signal={}",
            obs.observation_index, obs.kind, obs.packet_message_number, obs.payload_len, obs.signal
        );
    }

    Ok(())
}
