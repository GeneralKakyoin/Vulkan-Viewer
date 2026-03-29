# Review: N15 Implementation

## Verdict
Approved.

## Architecture and boundary fit
- `viewer_app` now enforces retry cooldown plus explicit single-flight probe guard.
- `viewer_net` probe execution remains transport-owned (`execute_continuity_probe`).
- `viewer_ui` remains display/intent-only with added recovery status text.
- No crate boundary collapse detected.

## Correctness concerns
- Addressed: retry requests are rejected when unavailable or already in flight.
- Addressed: in-flight flag is cleared on probe result update.
- Addressed: queue-send failure maps to `Unavailable` and resets probe retry timestamp.

## Modularity and maintainability concerns
- Recovery gating remains centralized in `compute_recovery_action` + app dispatch.
- UI formatting helper is localized and read-only.

## Validation adequacy
- `cargo fmt --all`: passed.
- `cargo check --workspace`: passed.
- `cargo test -p viewer_core -p viewer_ui -p viewer_app -p viewer_net`: passed.
- `cargo test --workspace`: passed.
- `VIEWER_APP_LIVE_STARTUP=off STRESS_TEST=screenshot ... cargo run -p viewer_app`: passed, screenshot manually reviewed.

## Risks and open questions
- Connected-live continuity probe behavior is still unvalidated in this pass (requires `VIEWER_LOGIN_*` environment).

## Learnings delta verdict
none - No new durable lesson beyond existing recovery/process constraints.

## Required revisions or approval status
No required revisions.
