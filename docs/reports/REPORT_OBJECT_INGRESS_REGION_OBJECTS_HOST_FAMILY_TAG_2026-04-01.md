# Report: Object Ingress RegionObjects Host-Family Transcript Tag (2026-04-01)

## Summary of implemented work
- Added a lightweight `host_family=...` transcript tag derived from capability URL host labels.
- Applied the tag to `RegionObjects` primary probe and post-reconnect re-probe lines (success/error paths) and matching protocol-event entries.
- Kept transport and protocol behavior unchanged.

## Files changed
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_OBJECT_INGRESS_REGION_OBJECTS_HOST_FAMILY_TAG_2026-04-01.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_REGION_OBJECTS_HOST_FAMILY_TAG_2026-04-01.md`
- `docs/reports/REPORT_OBJECT_INGRESS_REGION_OBJECTS_HOST_FAMILY_TAG_2026-04-01.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/OBJECT_INGRESS_STATUS.md`

## Validation run
- Passed:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_app`
  - `VIEWER_APP_LIVE_STARTUP=on VIEWER_APP_AUTO_TELEPORT_SLURL=secondlife://Ahern/50/60/70 VIEWER_APP_AUTO_TELEPORT_DELAY_TICKS=40 VIEWER_APP_REGION_OBJECTS_REPROBE_DELAY_TICKS=80 VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_region_objects_host_family_tag_2026-04-01.jsonl cargo run -p viewer_app`
- Failed:
  - none
- Remaining unvalidated:
  - none for this slice
- Artifacts:
  - `artifacts/logs/live_region_objects_host_family_tag_2026-04-01.out.log`
  - `artifacts/logs/live_region_objects_host_family_tag_2026-04-01.err.log`
  - `artifacts/logs/network_debug_region_objects_host_family_tag_2026-04-01.jsonl`

## Result status
- `RegionObjects` transcript lines now include explicit route identity tags such as:
  - `host_family=simhost-0629fe9f6de4b8693`
  - `host_family=simhost-0eec03118f78cfe1f`
- This enables direct filtering/grouping by host-family without parsing full URLs.
- In the bounded Ahern run used for validation, both pre-reconnect and post-reconnect windows remained `typed_sample=none`; this slice was diagnostics-only and did not change object-ingress behavior.

## Risks or follow-up items
- Host-family tags depend on URL host presence; fallback is `host_family=unknown-host` when host parsing fails.

## Learnings delta
- none
- Reason: diagnostics visibility improved, but no new durable failure mode or planning rule was discovered.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/OBJECT_INGRESS_STATUS.md`
