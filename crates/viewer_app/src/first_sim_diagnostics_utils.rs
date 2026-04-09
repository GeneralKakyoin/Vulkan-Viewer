use super::*;

pub(super) fn emit_first_simulator_socket_summary(
    tx: &mpsc::Sender<LiveFeedUpdate>,
    connection: &Connection,
    label: &str,
) {
    let summary = connection.summarize_first_simulator_socket_diagnostics();
    let ports = if summary.unique_local_ports.is_empty() {
        String::from("none")
    } else {
        summary
            .unique_local_ports
            .iter()
            .map(u16::to_string)
            .collect::<Vec<_>>()
            .join(",")
    };
    let tail = summarize_first_simulator_socket_tail(connection, 5);
    emit_relay(
        tx,
        RuntimeRelayLevel::Info,
        "first_sim_socket",
        &format!(
            "{label}: ports={ports} split={} events={} probe_binds={} fresh_binds={} retained={} reused={} sends={} receives={} tail={tail}",
            summary.split_local_port_detected,
            summary.events,
            summary.probe_bind_events,
            summary.fresh_bind_events,
            summary.retained_probe_events,
            summary.reused_retained_probe_events,
            summary.send_events,
            summary.receive_events,
        ),
    );
}

pub(super) fn emit_first_simulator_forensics_summary(
    tx: &mpsc::Sender<LiveFeedUpdate>,
    connection: &Connection,
    label: &str,
) {
    let ack = connection.summarize_first_simulator_ack_forensics();
    let receive = connection.summarize_first_simulator_receive_forensics();
    let timeline = connection.summarize_first_simulator_startup_timeline();
    let transcript =
        connection.summarize_first_simulator_startup_transcript(FIRST_SIM_TRANSCRIPT_TAIL_LEN);
    let startup_interest = connection.summarize_first_simulator_startup_interest_gate();
    let decode = connection.simulator_payload_decode_summary();
    emit_relay(
        tx,
        RuntimeRelayLevel::Info,
        "first_sim_forensics",
        &format!(
            "{label} ack: pending={} preview={} appended_sends={} last_appended={} explicit_packet_ack={} first_packet_ack_idx={}",
            ack.pending_ack_count,
            format_u32_list_hex(&ack.pending_ack_ids_preview),
            ack.outbound_appended_ack_sends,
            format_u32_list_hex(&ack.outbound_appended_ack_ids_last),
            ack.explicit_packet_ack_receives,
            format_optional_index(ack.first_packet_ack_receive_index),
        ),
    );
    emit_relay(
        tx,
        RuntimeRelayLevel::Info,
        "first_sim_forensics",
        &format!(
            "{label} recv: obs={} raw={} unclassified={} typed={}",
            receive.receive_observations,
            format_message_number_counts(&receive.raw_packet_message_numbers),
            format_message_number_counts(&receive.unclassified_packet_message_numbers),
            format_receive_kind_counts(&receive.typed_kind_counts),
        ),
    );
    emit_relay(
        tx,
        RuntimeRelayLevel::Info,
        "first_sim_forensics",
        &format!(
            "{label} timeline: region_handshake={} region_handshake_reply={} amc={} packet_ack={} camera_constraint={} generic_message={} object_update={}",
            format_optional_index(timeline.first_region_handshake_index),
            format_optional_index(timeline.first_region_handshake_reply_index),
            format_optional_index(timeline.first_agent_movement_complete_index),
            format_optional_index(timeline.first_packet_ack_index),
            format_optional_index(timeline.first_camera_constraint_index),
            format_optional_index(timeline.first_generic_message_index),
            format_optional_index(timeline.first_object_update_index),
        ),
    );
    emit_relay(
        tx,
        RuntimeRelayLevel::Info,
        "first_sim_forensics",
        &format!(
            "{label} region_handshake_observed index={} updates={} sim_name={}",
            format_optional_index(timeline.first_region_handshake_index),
            decode.region_handshake_updates,
            decode
                .region_handshake_last_sim_name
                .as_deref()
                .unwrap_or("none"),
        ),
    );
    let (reply_send_count, first_reply_send_index) =
        summarize_region_handshake_reply_send_evidence(connection);
    emit_relay(
        tx,
        RuntimeRelayLevel::Info,
        "first_sim_forensics",
        &format!(
            "{label} region_handshake_reply_send count={} first_send_idx={} observed_reply_idx={}",
            reply_send_count,
            format_optional_index(first_reply_send_index),
            format_optional_index(timeline.first_region_handshake_reply_index),
        ),
    );
    emit_relay(
        tx,
        RuntimeRelayLevel::Info,
        "first_sim_forensics",
        &format_startup_interest_gate_line(label, &startup_interest),
    );
    emit_relay(
        tx,
        RuntimeRelayLevel::Info,
        "first_sim_forensics",
        &format!(
            "{label} transcript: sends={} receives={}",
            format_transcript_side(&transcript.send_events),
            format_transcript_side(&transcript.receive_events),
        ),
    );
    emit_relay(
        tx,
        RuntimeRelayLevel::Info,
        "first_sim_forensics",
        &format_lludp_startup_object_gate_line(label, &timeline, decode),
    );
}

pub(super) fn summarize_region_handshake_reply_send_evidence(
    connection: &Connection,
) -> (usize, Option<usize>) {
    const REGION_HANDSHAKE_REPLY_MESSAGE_NUMBER: u32 = 0xFFFF_0000 | 149;
    let mut first_index = None;
    let mut count = 0usize;
    for (idx, diag) in connection
        .first_simulator_handshake_send_diagnostics()
        .iter()
        .enumerate()
    {
        if diag.packet_message_number == Some(REGION_HANDSHAKE_REPLY_MESSAGE_NUMBER) {
            count = count.saturating_add(1);
            if first_index.is_none() {
                first_index = Some(idx + 1);
            }
        }
    }
    (count, first_index)
}

pub(super) fn summarize_first_simulator_socket_tail(
    connection: &Connection,
    limit: usize,
) -> String {
    let diagnostics = connection.first_simulator_socket_diagnostics();
    if diagnostics.is_empty() {
        return String::from("none");
    }
    let start = diagnostics.len().saturating_sub(limit);
    diagnostics[start..]
        .iter()
        .map(format_first_simulator_socket_diagnostic)
        .collect::<Vec<_>>()
        .join(" | ")
}

pub(super) fn format_first_simulator_socket_diagnostic(
    diagnostic: &viewer_net::FirstSimulatorSocketDiagnostic,
) -> String {
    let local_addr = diagnostic.local_addr.as_deref().unwrap_or("?");
    let remote_target = diagnostic.remote_target.as_deref().unwrap_or("?");
    let message_number = diagnostic
        .packet_message_number
        .map(|number| format!("{number:#x}"))
        .unwrap_or_else(|| String::from("-"));
    let payload_len = diagnostic
        .payload_len
        .map(|len| len.to_string())
        .unwrap_or_else(|| String::from("-"));
    format!(
        "#{}:{:?}@{}->{} m={} len={} {}",
        diagnostic.event_index,
        diagnostic.kind,
        local_addr,
        remote_target,
        message_number,
        payload_len,
        diagnostic.reason,
    )
}

pub(super) fn format_optional_index(index: Option<usize>) -> String {
    index
        .map(|value| value.to_string())
        .unwrap_or_else(|| String::from("none"))
}

pub(super) fn has_nonzero_local_id_evidence(
    decoded: &viewer_net::SimulatorPayloadDecodeSummary,
) -> bool {
    decoded
        .object_feed_objects
        .iter()
        .any(|obj| obj.local_id > 0)
}

pub(super) fn format_local_id_preview(
    decoded: &viewer_net::SimulatorPayloadDecodeSummary,
    limit: usize,
) -> String {
    let mut local_ids: Vec<u32> = decoded
        .object_feed_objects
        .iter()
        .filter_map(|obj| (obj.local_id > 0).then_some(obj.local_id))
        .collect();
    local_ids.sort_unstable();
    local_ids.dedup();
    if local_ids.is_empty() {
        return String::from("none");
    }
    local_ids
        .into_iter()
        .take(limit.max(1))
        .map(|local_id| local_id.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

pub(super) fn format_lludp_startup_object_gate_line(
    label: &str,
    timeline: &viewer_net::FirstSimulatorStartupTimelineSummary,
    decoded: &viewer_net::SimulatorPayloadDecodeSummary,
) -> String {
    let has_object_update = timeline.first_object_update_index.is_some();
    let has_local_ids = has_nonzero_local_id_evidence(decoded);
    let verdict = if has_object_update && has_local_ids {
        "PASS"
    } else {
        "FAIL"
    };
    format!(
        "{label} lludp_object_gate: verdict={verdict} object_update={} update_messages={} total_objects={} local_ids={}",
        format_optional_index(timeline.first_object_update_index),
        decoded.object_feed_update_messages,
        decoded.object_feed_total_objects,
        format_local_id_preview(decoded, 8),
    )
}

pub(super) fn format_startup_interest_gate_line(
    label: &str,
    gate: &viewer_net::FirstSimulatorStartupInterestGateSummary,
) -> String {
    let verdict = if gate.passed { "PASS" } else { "FAIL" };
    let required = if gate.required.is_empty() {
        String::from("none")
    } else {
        gate.required
            .iter()
            .map(|entry| {
                format!(
                    "{}@{}#{}",
                    entry.message, entry.order_index, entry.packet_id
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    };
    let missing = if gate.missing.is_empty() {
        String::from("none")
    } else {
        gate.missing.join(",")
    };
    let classification = if gate.passed {
        "ok"
    } else {
        "startup_send_path_defect"
    };
    format!(
        "{label} startup_interest_gate: verdict={verdict} required={required} missing={missing} classification={classification}"
    )
}

pub(super) fn format_u32_list_hex(values: &[u32]) -> String {
    if values.is_empty() {
        return String::from("none");
    }
    values
        .iter()
        .map(|value| format!("0x{value:08x}"))
        .collect::<Vec<_>>()
        .join(",")
}

pub(super) fn format_message_number_counts(entries: &[(u32, usize)]) -> String {
    if entries.is_empty() {
        return String::from("none");
    }
    entries
        .iter()
        .map(|(message_number, count)| format!("0x{message_number:08x}:{count}"))
        .collect::<Vec<_>>()
        .join(",")
}

pub(super) fn format_receive_kind_counts(
    entries: &[(viewer_net::FirstSimulatorInboundMessageKind, usize)],
) -> String {
    if entries.is_empty() {
        return String::from("none");
    }
    entries
        .iter()
        .map(|(kind, count)| format!("{kind:?}:{count}"))
        .collect::<Vec<_>>()
        .join(",")
}

pub(super) fn format_transcript_side(entries: &[String]) -> String {
    if entries.is_empty() {
        return String::from("none");
    }
    entries.join(" | ")
}
