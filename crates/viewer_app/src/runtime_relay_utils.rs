use super::*;

pub(super) fn emit_relay(
    tx: &mpsc::Sender<LiveFeedUpdate>,
    level: RuntimeRelayLevel,
    category: &str,
    message: &str,
) {
    let ts = now_unix_ms();
    let line = format!("[{ts}] {category}: {message}");
    println!("{line}");
    let _ = fs::create_dir_all("logs");
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("logs/viewer_app_runtime.jsonl")
    {
        let json_line = format!(
            "{{\"ts\":{},\"category\":\"{}\",\"message\":\"{}\"}}\n",
            ts,
            category.replace('"', "'"),
            message.replace('"', "'")
        );
        let _ = file.write_all(json_line.as_bytes());
    }
    let event = RuntimeRelayEvent {
        at_unix_ms: ts,
        level,
        category: category.to_string(),
        message: message.to_string(),
    };
    if is_network_debug_category(category) {
        append_network_debug_log(&event);
    }
    let _ = tx.send(LiveFeedUpdate::Relay(event));
}

pub(super) fn is_network_debug_category(category: &str) -> bool {
    matches!(
        category,
        "parallel_protocol"
            | "event_queue"
            | "first_sim_socket"
            | "first_sim_forensics"
            | "first_sim_ack"
            | "object_feed"
            | "region_objects"
            | "social"
    )
}

pub(super) fn network_debug_log_path() -> PathBuf {
    std::env::var("VIEWER_NETWORK_DEBUG_LOG_PATH")
        .ok()
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| PathBuf::from("logs/network_debug.jsonl"))
}

pub(super) fn append_network_debug_log(event: &RuntimeRelayEvent) {
    let path = network_debug_log_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let json_line = format!(
            "{{\"ts\":{},\"level\":\"{:?}\",\"category\":\"{}\",\"message\":\"{}\"}}\n",
            event.at_unix_ms,
            event.level,
            event.category.replace('"', "'"),
            event.message.replace('"', "'")
        );
        let _ = file.write_all(json_line.as_bytes());
    }
}

pub(super) fn append_network_debug_line(debug: &mut NetworkDebugState, title: &str, line: String) {
    let mut lines = debug
        .sections
        .iter()
        .find(|section| section.title == title)
        .map(|section| section.lines.clone())
        .unwrap_or_default();
    lines.push(line);
    if lines.len() > NETWORK_DEBUG_SECTION_MAX_LINES {
        let keep_from = lines.len() - NETWORK_DEBUG_SECTION_MAX_LINES;
        lines.drain(0..keep_from);
    }
    debug.set_section_lines(title, lines);
}
