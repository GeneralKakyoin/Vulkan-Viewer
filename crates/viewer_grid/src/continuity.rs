use viewer_core::{HandoffOutcome, HandoffPhase, HandoffReason, RegionContinuitySummary};

/// Semantic classifier for region handoff diagnostics.
///
/// Maps raw transport observations and timing into policy-level outcomes and reasons.
pub fn classify_handoff_diagnostics(
    summary: &RegionContinuitySummary,
) -> (HandoffOutcome, HandoffReason) {
    let phase = summary.phase;
    let age = summary.phase_age_ms;

    match phase {
        HandoffPhase::None => (HandoffOutcome::Normal, HandoffReason::None),
        HandoffPhase::Crossed => {
            if age > 10_000 {
                (HandoffOutcome::Stalled, HandoffReason::StaleWindowExceeded)
            } else if age > 4_000 {
                (HandoffOutcome::Degraded, HandoffReason::LateConfirmation)
            } else {
                (HandoffOutcome::Normal, HandoffReason::None)
            }
        }
        HandoffPhase::Confirming => {
            if age > 15_000 {
                (HandoffOutcome::Stalled, HandoffReason::StaleWindowExceeded)
            } else if age > 5_000 {
                (HandoffOutcome::Degraded, HandoffReason::LateConfirmation)
            } else {
                (HandoffOutcome::Normal, HandoffReason::None)
            }
        }
        HandoffPhase::Completed => {
            // Completed is usually normal unless it was reached via a degraded path,
            // but for now we just mark it normal.
            (HandoffOutcome::Normal, HandoffReason::None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classification_logic() {
        let mut summary = RegionContinuitySummary {
            phase: HandoffPhase::Confirming,
            phase_age_ms: 1000,
            ..Default::default()
        };

        let (outcome, reason) = classify_handoff_diagnostics(&summary);
        assert_eq!(outcome, HandoffOutcome::Normal);
        assert_eq!(reason, HandoffReason::None);

        summary.phase_age_ms = 6000;
        let (outcome, reason) = classify_handoff_diagnostics(&summary);
        assert_eq!(outcome, HandoffOutcome::Degraded);
        assert_eq!(reason, HandoffReason::LateConfirmation);

        summary.phase_age_ms = 16000;
        let (outcome, reason) = classify_handoff_diagnostics(&summary);
        assert_eq!(outcome, HandoffOutcome::Stalled);
        assert_eq!(reason, HandoffReason::StaleWindowExceeded);
    }
}
