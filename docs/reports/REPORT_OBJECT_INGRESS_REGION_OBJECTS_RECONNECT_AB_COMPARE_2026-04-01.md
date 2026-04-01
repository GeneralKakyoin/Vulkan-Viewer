# Report: Object Ingress RegionObjects Reconnect A/B Compare (Morris vs Ahern) (2026-04-01)

## Summary of implemented work
- Executed paired bounded reconnect captures with identical knobs and different targets:
  - `secondlife://Morris/128/128/25`
  - `secondlife://Ahern/50/60/70`
- Compared primary probe and delayed re-probe outcomes from both runs.
- Produced a single branch decision based on cross-target evidence.

## Files changed
- `docs/plans/PLAN_OBJECT_INGRESS_REGION_OBJECTS_RECONNECT_AB_COMPARE_2026-04-01.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_REGION_OBJECTS_RECONNECT_AB_COMPARE_2026-04-01.md`
- `docs/reports/REPORT_OBJECT_INGRESS_REGION_OBJECTS_RECONNECT_AB_COMPARE_2026-04-01.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/OBJECT_INGRESS_STATUS.md`
- `docs/LEARNINGS.md`

## Validation run
- Passed:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `VIEWER_APP_LIVE_STARTUP=on VIEWER_APP_AUTO_TELEPORT_SLURL=secondlife://Morris/128/128/25 VIEWER_APP_AUTO_TELEPORT_DELAY_TICKS=40 VIEWER_APP_REGION_OBJECTS_REPROBE_DELAY_TICKS=80 VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_region_objects_reconnect_ab_morris_2026-04-01.jsonl cargo run -p viewer_app`
  - `VIEWER_APP_LIVE_STARTUP=on VIEWER_APP_AUTO_TELEPORT_SLURL=secondlife://Ahern/50/60/70 VIEWER_APP_AUTO_TELEPORT_DELAY_TICKS=40 VIEWER_APP_REGION_OBJECTS_REPROBE_DELAY_TICKS=80 VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_region_objects_reconnect_ab_ahern_2026-04-01.jsonl cargo run -p viewer_app`
- Failed:
  - none
- Remaining unvalidated:
  - none for this slice
- Artifacts:
  - `artifacts/logs/live_region_objects_reconnect_ab_morris_2026-04-01.out.log`
  - `artifacts/logs/live_region_objects_reconnect_ab_morris_2026-04-01.err.log`
  - `artifacts/logs/network_debug_region_objects_reconnect_ab_morris_2026-04-01.jsonl`
  - `artifacts/logs/live_region_objects_reconnect_ab_ahern_2026-04-01.out.log`
  - `artifacts/logs/live_region_objects_reconnect_ab_ahern_2026-04-01.err.log`
  - `artifacts/logs/network_debug_region_objects_reconnect_ab_ahern_2026-04-01.jsonl`

## Result status
- Morris run:
  - pre-reconnect on `simhost-04e63a701b66ed282...` showed rich typed samples with `landimpact`.
  - post-reconnect on `simhost-0a962ce03cdb50c3e...` showed `typed_sample=none` on both primary and delayed re-probe.
- Ahern run:
  - pre-reconnect on `simhost-0a962ce03cdb50c3e...` showed `typed_sample=none`.
  - post-reconnect on `simhost-04e63a701b66ed282...` showed rich typed samples with `landimpact=1|20` on both primary and delayed re-probe.
- Conclusion:
  - behavior is reproducibly target/simhost dependent under identical timing knobs.

## Risks or follow-up items
- Region content and simhost assignment may vary over time; this conclusion should be rechecked if routing changes.
- LLUDP object ingress remains unresolved and independent.

## Learnings delta
- added
- Added L57: paired A/B captures with identical knobs are required for reconnect branch decisions because outcomes can invert across simhost paths.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/OBJECT_INGRESS_STATUS.md`
- Updated `docs/LEARNINGS.md`
