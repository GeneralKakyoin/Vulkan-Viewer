use super::*;

pub(super) fn should_submit_on_enter(enter_pressed: bool, shift_held: bool) -> bool {
    enter_pressed && !shift_held
}

pub(super) fn connection_chip(connection: &ChatConnectionState) -> (&'static str, egui::Color32) {
    match connection {
        ChatConnectionState::Disabled => ("disabled", egui::Color32::GRAY),
        ChatConnectionState::Connecting => ("connecting", egui::Color32::YELLOW),
        ChatConnectionState::Connected => ("connected", egui::Color32::GREEN),
        ChatConnectionState::Reconnecting => ("reconnecting", egui::Color32::YELLOW),
        ChatConnectionState::Failed(_) => ("failed", egui::Color32::RED),
    }
}

pub(super) fn send_chip(send_status: &ChatSendStatus) -> Option<(&'static str, egui::Color32)> {
    match send_status {
        ChatSendStatus::Idle => None,
        ChatSendStatus::Sending => Some(("sending", egui::Color32::YELLOW)),
        ChatSendStatus::Sent => Some(("sent", egui::Color32::LIGHT_GREEN)),
        ChatSendStatus::Failed(_) => Some(("send failed", egui::Color32::RED)),
    }
}

pub(super) fn avatar_render_mode_label(mode: AvatarRenderMode) -> &'static str {
    match mode {
        AvatarRenderMode::Proxy => "proxy",
        AvatarRenderMode::FallbackBox => "fallback-box",
    }
}

pub(super) fn session_reason_label(reason: SessionUxReason) -> &'static str {
    match reason {
        SessionUxReason::DisabledByConfig => "disabled-by-config",
        SessionUxReason::MissingConfig => "missing-config",
        SessionUxReason::ConnectTransport => "connect-transport",
        SessionUxReason::LoginTransport => "login-transport",
        SessionUxReason::LoginAuth => "login-auth",
        SessionUxReason::LoginRequiresTos => "login-requires-tos",
        SessionUxReason::LoginRequiresMfa => "login-requires-mfa",
        SessionUxReason::LoginUpdateRequired => "login-update-required",
        SessionUxReason::ConnectionLost => "connection-lost",
        SessionUxReason::Other => "other",
    }
}

pub(super) fn probe_result_label(code: ProbeResultCode) -> &'static str {
    match code {
        ProbeResultCode::Success => "success",
        ProbeResultCode::Timeout => "timeout",
        ProbeResultCode::TransportError => "transport-error",
        ProbeResultCode::HttpFailure => "http-failure",
        ProbeResultCode::Unavailable => "unavailable",
    }
}

pub(super) fn recovery_result_label(result: &RecoveryActionResult) -> String {
    let action = match result.action {
        RecoveryAction::RetryContinuityProbe => "retry continuity probe",
        RecoveryAction::RefreshVisibleAssets => "refresh visible assets",
        RecoveryAction::ClearRecoveryBanner => "clear recovery banner",
    };
    let status = match result.code {
        RecoveryResultCode::Accepted => "accepted".to_string(),
        RecoveryResultCode::CooldownActive => {
            if let Some(remaining) = result.cooldown_remaining_ms {
                format!("cooldown ({} ms remaining)", remaining)
            } else {
                String::from("cooldown")
            }
        }
        RecoveryResultCode::Unavailable => String::from("unavailable"),
        RecoveryResultCode::Completed(code) => {
            format!("completed ({})", probe_result_label(code))
        }
    };
    if let Some(detail) = &result.detail {
        format!("{action}: {status} - {detail}")
    } else {
        format!("{action}: {status}")
    }
}

pub(super) fn session_status_chip(
    status: &SessionUxStatus,
) -> (&'static str, egui::Color32, Option<SessionUxReason>) {
    match status {
        SessionUxStatus::Disabled { reason } => ("disabled", egui::Color32::GRAY, *reason),
        SessionUxStatus::Starting => ("starting", egui::Color32::YELLOW, None),
        SessionUxStatus::Connected => ("connected", egui::Color32::GREEN, None),
        SessionUxStatus::Reconnecting { reason } => {
            ("reconnecting", egui::Color32::YELLOW, *reason)
        }
        SessionUxStatus::Failed { reason } => ("failed", egui::Color32::RED, Some(*reason)),
    }
}

pub(super) fn handoff_outcome_chip(outcome: HandoffOutcome) -> (&'static str, egui::Color32) {
    match outcome {
        HandoffOutcome::Normal => ("normal", egui::Color32::GREEN),
        HandoffOutcome::Degraded => ("degraded", egui::Color32::YELLOW),
        HandoffOutcome::Stalled => ("STALLED", egui::Color32::RED),
    }
}

pub(super) fn handoff_reason_label(reason: HandoffReason) -> &'static str {
    match reason {
        HandoffReason::None => "none",
        HandoffReason::LateConfirmation => "late_confirmation",
        HandoffReason::StaleWindowExceeded => "stale_window",
        HandoffReason::MissingCrossedRegion => "missing_crossed",
        HandoffReason::NetworkJitter => "jitter",
    }
}

pub(super) fn sorted_friend_ids_for_filter(
    social_state: &SocialState,
    filter: ThreadFilter,
) -> Vec<String> {
    let mut ids: Vec<String> = social_state.friends.iter().map(|f| f.id.clone()).collect();
    match filter {
        ThreadFilter::All => {
            ids.sort_by(|a, b| {
                let a_name = social_state
                    .friends
                    .iter()
                    .find(|f| &f.id == a)
                    .map(SocialState::friend_display_label)
                    .unwrap_or_else(|| a.clone());
                let b_name = social_state
                    .friends
                    .iter()
                    .find(|f| &f.id == b)
                    .map(SocialState::friend_display_label)
                    .unwrap_or_else(|| b.clone());
                a_name.cmp(&b_name)
            });
        }
        ThreadFilter::Online => {
            ids.retain(|id| {
                social_state
                    .friends
                    .iter()
                    .find(|f| &f.id == id)
                    .map(|f| f.online)
                    .unwrap_or(false)
            });
            ids.sort_by(|a, b| {
                let a_last = social_state
                    .thread_for_participant(a)
                    .map(|t| t.last_activity_unix_ms)
                    .unwrap_or(0);
                let b_last = social_state
                    .thread_for_participant(b)
                    .map(|t| t.last_activity_unix_ms)
                    .unwrap_or(0);
                b_last.cmp(&a_last).then_with(|| a.cmp(b))
            });
        }
        ThreadFilter::Recent => {
            let mut by_recent = social_state.sorted_thread_participants_by_recent();
            for id in ids {
                if !by_recent.iter().any(|existing| existing == &id) {
                    by_recent.push(id);
                }
            }
            ids = by_recent;
        }
    }
    ids
}
