# Walkthrough: N11 Region Handoff Hardening

This milestone hardened the region transition diagnostics and handoff stability by implementing a formal diagnostic contract, semantic classification, and network-level guards.

## Key Accomplishments

### 1. Hardened Diagnostic Contracts (`viewer_core`)
Consolidated the handoff phase model into `viewer_core` and extended `RegionContinuitySummary` to include diagnostic fields for outcome, reason, and timing.

```rust
pub struct RegionContinuitySummary {
    pub phase: HandoffPhase,
    pub outcome: HandoffOutcome,
    pub reason: HandoffReason,
    pub phase_age_ms: u64,
    // ... coordinates and neighbors ...
}
```

### 2. Monotonic Phase Timing & Guards (`viewer_net`)
`viewer_net` now tracks the exact `Instant` each phase changed and computes a deterministic `phase_age_ms`. It also implements a priority-based guard to prevent out-of-order UDP packets from causing "phase flickering."

```rust
// In viewer_net::Connection
pub fn record_region_continuity_observation(&mut self, summary: RegionContinuitySummary) {
    let current_priority = self.continuity_summary.phase.priority();
    let next_priority = summary.phase.priority();
    if next_priority < current_priority && summary.phase != HandoffPhase::None {
        return; // Guard: no regressing backward!
    }
    // ...
}
```

### 3. Semantic Transition Classifier (`viewer_grid`)
A new policy-level classifier in `viewer_grid` maps raw transport timing to human-readable outcomes and failure reasons.

*   **Normal**: Phase completed or confirming within expected windows (e.g., < 5s for confirmation).
*   **Degraded**: Phase age has exceeded soft thresholds (e.g., > 5s in Confirming).
*   **Stalled**: Phase age has exceeded hard thresholds (e.g., > 15s in Confirming).

### 4. Visibility & UI Diagnostics (`viewer_ui`)
The diagnostics panel now features a color-coded status chip for handoff health and an expanded detail line.

> [!TIP]
> **Normal** status is green. **Degraded** is yellow. **STALLED** is bright red.

## Verification Results

### Automated Tests
*   `cargo test -p viewer_core`: **PASSED**
*   `cargo test -p viewer_grid`: **PASSED** (including `continuity::tests`)
*   `cargo check -p viewer_app`: **PASSED**

### Manual Smoke Check
The diagnostics UI was updated to render these fields. In an offline state, the UI correctly displays the "Presence & Continuity" panel with the default `Normal` / `None` / `0ms` diagnostics.

---

## Technical Learnings
- **UDP Order Sensitivity**: LLUDP region transition packets are not guaranteed to arrive in order. Monotonic phase priority is required for stability.
- **Inherited Coordinates**: Maintaining coordinate stability by inheriting previous values during partial net updates prevents jumpy position diagnostics in the UI.
