# Report: Object Ingress RegionObjects Typed Feed Live Validation (2026-03-31)

## Summary of implemented work
- Ran a bounded live validation pass against the current `RegionObjects` typed-feed promotion.
- Confirmed that the new `typed_sample=...` relay appears on-wire in the `region_objects` summary.
- Captured fresh stdout/err and JSONL artifacts for the current build.

## Files changed
- `docs/plans/PLAN_OBJECT_INGRESS_REGION_OBJECTS_TYPED_FEED_LIVE_VALIDATION_2026-03-31.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_REGION_OBJECTS_TYPED_FEED_LIVE_VALIDATION_2026-03-31.md`
- `docs/reports/REPORT_OBJECT_INGRESS_REGION_OBJECTS_TYPED_FEED_LIVE_VALIDATION_2026-03-31.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/OBJECT_INGRESS_STATUS.md`

## Validation run
- bounded direct-binary live run with auto-teleport enabled:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_APP_AUTO_TELEPORT_SLURL=secondlife://Ahern/50/60/70`
  - `VIEWER_APP_AUTO_TELEPORT_DELAY_TICKS=40`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_region_objects_typed_feed_live_validation_rerun_2026-03-31.jsonl`
  - `target/debug/viewer_app.exe`
- artifacts:
  - `artifacts/logs/live_region_objects_typed_feed_live_validation_rerun_2026-03-31.out.log`
  - `artifacts/logs/live_region_objects_typed_feed_live_validation_rerun_2026-03-31.err.log`
  - `artifacts/logs/network_debug_region_objects_typed_feed_live_validation_rerun_2026-03-31.jsonl`

## Result status
- Live validation passed for the new `typed_sample=...` relay.
- The relay remained readable and surfaced trusted fields including `profile`, `name`, `linkset_use`, `walkability`, `position`, `description_shape`, and `owner`.
- LLUDP object ingress remains unchanged:
  - `RegionHandshake = none`
  - `RegionHandshakeReply = none`
  - `ObjectUpdate* = none`
  - `update_messages=0`
  - `total_objects=0`

## Risks or follow-up items
- This live run validated the new summary on-wire, but it did not broaden into a different visible object family; the reconnect landed back on the same region-content family (`Object`, `bamboo`).
- If the next goal is broader object sampling rather than summary validation, use a different reconnect target for the next live pass.

## Learnings delta
- none
- No durable learning identified; this slice validated the already-planned summary successfully but did not reveal a new protocol or architectural constraint.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/OBJECT_INGRESS_STATUS.md`
