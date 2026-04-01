# HANDOFF: Object Ingress Post-Reconnect Reprobe Timing Live-Validated (2026-04-01)

## What Changed
- Added reconnect-only bounded delayed `RegionObjects` re-probe logic in `viewer_app`.
- Added `VIEWER_APP_REGION_OBJECTS_REPROBE_DELAY_TICKS` (default `80`) for timing control.
- Added explicit protocol/log markers for re-probe arming/success/failure.
- Completed authoritative live validation via `cargo run -p viewer_app` with re-probe enabled.
- Updated testing reference and continuity artifacts for this branch.

## Validation Run
- `cargo fmt --all`: PASSED
- `cargo check -p viewer_net -p viewer_app`: PASSED
- `CARGO_INCREMENTAL=0 cargo test -p viewer_net`: PASSED
- `CARGO_INCREMENTAL=0 cargo test -p viewer_app`: PASSED
- authoritative live validation:
  - `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1 VIEWER_APP_LIVE_STARTUP=on VIEWER_APP_AUTO_TELEPORT_SLURL=secondlife://Morris/128/128/25 VIEWER_APP_AUTO_TELEPORT_DELAY_TICKS=40 VIEWER_APP_REGION_OBJECTS_REPROBE_DELAY_TICKS=80 VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_region_objects_post_reconnect_reprobe_timing_cargo_run_retry_2026-04-01.jsonl cargo run -p viewer_app`: PASSED

## Exact Current State
- The re-probe timing logic is implemented and live-validated.
- Re-probe fired on reconnect (`RegionObjects:reprobe_armed`) and executed (`post-reconnect re-probe ...`), but still returned `typed_sample=none` in this run.
- LLUDP object ingress remains unchanged:
  - `RegionHandshake` absent
  - `RegionHandshakeReply` absent
  - `ObjectUpdate*` absent
  - `update_messages=0`
  - `total_objects=0`

## Exact Next Step
1. Run one bounded reconnect capture with a different SLURL target and compare `primary probe` vs `re-probe` output.
2. Decide whether post-reconnect `typed_sample=none` is route/region dependent.
3. If another target still stays empty after re-probe, prioritize LLUDP branch re-entry with this capability-lane constraint documented.

## Blockers / Risks
- For the tested target path, delayed re-probe did not recover typed samples, so a timing-only explanation is now weaker.
- LLUDP object ingress remains unresolved and separate.
