# Report: EventQueue Cap Rotation and Object Render Correctness (2026-04-09)

## Summary of implemented work
- Added bounded EventQueue cap-not-found re-prime behavior in `viewer_app`:
  - when `event_queue_cap_not_found_before_reconnect` threshold is reached, app now attempts a seed-cap refresh and EventQueue URL rebind before reconnect.
  - on successful re-prime, EventQueue URL/pending probe URLs/readiness/inventory are refreshed, and EventQueue ack + failure counters reset.
  - on re-prime failure or missing EventQueue capability, app falls back to reconnect with explicit reason telemetry.
- Added `viewer_app` unit coverage for capability re-prime mapping extraction from refreshed seed capabilities.
- Added `viewer_core` math regression tests to lock transform matrix correctness for identity rotation and translation/scale mapping.

## Files changed
- `crates/viewer_app/src/main.rs`
- `crates/viewer_core/src/math_utils.rs`
- `docs/plans/PLAN_EVENT_QUEUE_CAP_ROTATION_AND_OBJECT_RENDER_CORRECTNESS_2026-04-09.md`
- `docs/reviews/REVIEW_PLAN_EVENT_QUEUE_CAP_ROTATION_AND_OBJECT_RENDER_CORRECTNESS_2026-04-09.md`
- `docs/reviews/REVIEW_IMPL_EVENT_QUEUE_CAP_ROTATION_AND_OBJECT_RENDER_CORRECTNESS_2026-04-09.md`
- `docs/reports/REPORT_EVENT_QUEUE_CAP_ROTATION_AND_OBJECT_RENDER_CORRECTNESS_2026-04-09.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`

## Validation run
- `cargo fmt --all` -> pass
- `cargo check --workspace` -> pass
- `cargo test -p viewer_net` -> pass
- `cargo test -p viewer_core` -> pass
- `cargo test -p viewer_render` -> pass
- bounded live run:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_APP_LLUDP_STARTUP_PARITY_BUNDLE=on`
  - `VIEWER_APP_CAPABILITY_PROBES_REQUIRE_EVENT_QUEUE_OK=true`
  - `VIEWER_APP_EVENT_QUEUE_CAP_NOT_FOUND_BEFORE_RECONNECT=3`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_event_queue_cap_rotation_and_render_correctness_2026-04-09.jsonl`
  - `cargo run -p viewer_app`
  - command result: harness timeout (expected bounded capture), artifact captured and reviewed

## Result status
- Implemented: yes
- Build/test validation: green on all targeted commands
- Live verification: partial success
  - new re-prime branch executed in live evidence (`cap_reprime:start` + failure telemetry)
  - when late-session cap invalidation occurred, refresh path returned `404 cap not found` and reconnect fallback executed

## Risks or follow-up items
- Primary follow-up: add bounded alternate EventQueue re-prime source path (for example, recent followed seed-cap URLs) for cases where current seed-cap fetch itself is invalid.
- Rendering parity remains a broader downstream track; this slice only added deterministic math regression coverage.

## Learnings delta
none — no new durable learning identified; current behavior aligns with existing EventQueue cap lifecycle learnings and improves runtime observability.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md` with latest notable changes and validation summary.
- Replaced `docs/HANDOFF.md` with latest exact state, next step, and risks.
- Added implementation review artifact under `docs/reviews/`.
