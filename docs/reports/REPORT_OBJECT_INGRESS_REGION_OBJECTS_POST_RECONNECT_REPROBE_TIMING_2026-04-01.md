# Report: Object Ingress RegionObjects Post-Reconnect Reprobe Timing (2026-04-01)

## Summary of implemented work
- Implemented a bounded reconnect-only post-startup `RegionObjects` re-probe path in `viewer_app`.
- Added `VIEWER_APP_REGION_OBJECTS_REPROBE_DELAY_TICKS` (default `80`) to control one-shot re-probe timing.
- Added explicit protocol/log markers for:
  - `RegionObjects:reprobe_armed ...`
  - `RegionObjects:reprobe_ok ...`
  - `RegionObjects:reprobe_err ...`
- Added config parsing coverage in `viewer_app` tests and documented the new env knob.

## Files changed
- `crates/viewer_app/src/main.rs`
- `docs/TESTING_REFERENCE.md`
- `docs/reports/REPORT_OBJECT_INGRESS_REGION_OBJECTS_POST_RECONNECT_REPROBE_TIMING_2026-04-01.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/OBJECT_INGRESS_STATUS.md`
- `docs/LEARNINGS.md`

## Validation run
- Passed:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `CARGO_INCREMENTAL=0 cargo test -p viewer_net`
  - `CARGO_INCREMENTAL=0 cargo test -p viewer_app`
- Authoritative live run passed:
  - `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1 VIEWER_APP_LIVE_STARTUP=on VIEWER_APP_AUTO_TELEPORT_SLURL=secondlife://Morris/128/128/25 VIEWER_APP_AUTO_TELEPORT_DELAY_TICKS=40 VIEWER_APP_REGION_OBJECTS_REPROBE_DELAY_TICKS=80 VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_region_objects_post_reconnect_reprobe_timing_cargo_run_retry_2026-04-01.jsonl cargo run -p viewer_app`
- Live artifacts:
  - `artifacts/logs/live_region_objects_post_reconnect_reprobe_timing_cargo_run_retry_2026-04-01.out.log`
  - `artifacts/logs/live_region_objects_post_reconnect_reprobe_timing_cargo_run_retry_2026-04-01.err.log`
  - `artifacts/logs/network_debug_region_objects_post_reconnect_reprobe_timing_cargo_run_retry_2026-04-01.jsonl`

## Result status
- Implementation and unit/static validation are complete.
- Fresh-binary live validation is now complete via `cargo run -p viewer_app`.
- Re-probe behavior was observed explicitly:
  - `RegionObjects:reprobe_armed delay_ticks=80`
  - `post-reconnect re-probe ... typed_sample=none`
- In this capture, delayed re-probe did **not** recover typed object samples; post-reconnect emptiness persisted on the same simhost.

## Risks or follow-up items
- Timing-only interpretation is now weaker for this route: at least one delayed re-probe window still yielded `typed_sample=none`.
- Next branch should compare region-target behavior and/or capability semantics rather than assuming startup timing alone.

## Learnings delta
- added
- Added/updated:
  - L54: direct-binary live validation must follow an explicit successful binary build.
  - L55: for this target path, delayed post-reconnect re-probe did not recover `typed_sample`, so pure timing is not a sufficient explanation.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/OBJECT_INGRESS_STATUS.md`
- Updated `docs/LEARNINGS.md`
