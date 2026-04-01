# Report: Object Ingress RegionObjects Reprobe Target Comparison (2026-04-01)

## Summary of implemented work
- Executed a bounded live reconnect run using `cargo run -p viewer_app` with:
  - `VIEWER_APP_AUTO_TELEPORT_SLURL=secondlife://Ahern/50/60/70`
  - `VIEWER_APP_AUTO_TELEPORT_DELAY_TICKS=40`
  - `VIEWER_APP_REGION_OBJECTS_REPROBE_DELAY_TICKS=80`
- Verified that the run captured both a negative and positive `RegionObjects` reconnect outcome in one session.
- Updated continuity and status docs to reflect route/target-dependent behavior rather than a timing-only framing.

## Files changed
- `docs/reports/REPORT_OBJECT_INGRESS_REGION_OBJECTS_REPROBE_TARGET_COMPARISON_2026-04-01.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/OBJECT_INGRESS_STATUS.md`
- `docs/LEARNINGS.md`

## Validation run
- Passed:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `VIEWER_APP_LIVE_STARTUP=on VIEWER_APP_AUTO_TELEPORT_SLURL=secondlife://Ahern/50/60/70 VIEWER_APP_AUTO_TELEPORT_DELAY_TICKS=40 VIEWER_APP_REGION_OBJECTS_REPROBE_DELAY_TICKS=80 VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_region_objects_post_reconnect_reprobe_target_ahern_2026-04-01.jsonl cargo run -p viewer_app`
- Failed:
  - none
- Remaining unvalidated:
  - none for this slice
- Artifacts:
  - `artifacts/logs/live_region_objects_post_reconnect_reprobe_target_ahern_2026-04-01.out.log`
  - `artifacts/logs/live_region_objects_post_reconnect_reprobe_target_ahern_2026-04-01.err.log`
  - `artifacts/logs/network_debug_region_objects_post_reconnect_reprobe_target_ahern_2026-04-01.jsonl`

## Result status
- Pre-reconnect probe on `simhost-0a962ce03cdb50c3e...` returned `typed_sample=none`.
- Reconnect target (`Ahern`) on `simhost-04e63a701b66ed282...` returned rich UUID-keyed typed samples on:
  - primary probe
  - delayed re-probe (`delay_ticks=80`)
- This confirms:
  - delayed re-probe can succeed
  - reconnect emptiness is not universal
  - branch decisions must compare across targets

## Risks or follow-up items
- Single-target results remain high risk for misclassification.
- LLUDP ingress remains unresolved and separate from this `RegionObjects` lane.

## Learnings delta
- added
- Added L56: reconnect `RegionObjects` outcomes must be compared across at least two targets before deciding branch priority.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/OBJECT_INGRESS_STATUS.md`
- Updated `docs/LEARNINGS.md`
