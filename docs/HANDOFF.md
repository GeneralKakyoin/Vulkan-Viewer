# HANDOFF: N15 Continuity Probe Command Wiring

## What Changed
- Implemented `Retry Continuity Probe` as a live worker command path (UI intent -> app dispatch -> net probe execution -> result update).
- Added deterministic retry guards in `viewer_app`:
  - cooldown enforcement (`RECOVERY_PROBE_COOLDOWN_MS`)
  - single-flight rejection when probe already in flight
  - unavailable mapping when probe command cannot be queued
- Preserved probe result propagation to continuity diagnostics:
  - `continuity.last_probe_result`
  - `continuity.last_probe_time_unix_ms`
- Added diagnostics status line text for last recovery action in `viewer_ui`.
- Added/updated N15 artifacts:
  - `docs/reviews/REVIEW_PLAN_N15.md`
  - `docs/reviews/REVIEW_IMPL_N15.md`
  - `docs/reports/REPORT_N15.md`
  - `docs/plans/DEFERRED_FEATURES.md` promotion for U14 deferred probe wiring

## Validation Run
- `cargo fmt --all`: PASSED
- `cargo check --workspace`: PASSED
- `cargo test -p viewer_core -p viewer_ui -p viewer_app -p viewer_net`: PASSED
- `cargo test --workspace`: PASSED
- `VIEWER_APP_LIVE_STARTUP=off STRESS_TEST=screenshot VIEWER_TEST_SCREENSHOT_DIR=artifacts/screenshots_n15_smoke VIEWER_TEST_SCREENSHOT_EVERY_N_FRAMES=1 VIEWER_TEST_SCREENSHOT_MAX_FRAMES=1 cargo run -p viewer_app`: PASSED
- Screenshot manually reviewed: `artifacts/screenshots_n15_smoke/viewer_test_0001.png`

## Exact Current State
- N15 bounded continuity probe retry path is wired and validated offline.
- Retry probe now exposes explicit unavailable/in-flight/cooldown/completed statuses through existing recovery result surfaces.
- Workspace still reports non-blocking warning in `viewer_render` (`DEBUG_CLIP_SPACE_TRIANGLE` dead code).

## Exact Next Step
- Run connected live validation for retry-probe behavior and continuity diagnostics transitions with valid `VIEWER_LOGIN_*` environment credentials.

## Blockers / Risks
- Connected verification remains pending due credential/environment dependency.
- No `docs/plans/PLAN_N16.md` exists in this workspace; only `PLAN_N15.md` was implementable in this pass.
