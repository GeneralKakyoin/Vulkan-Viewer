# Report: Object Ingress RegionObjects Typed Field Promotion (landimpact) (2026-04-01)

## Summary of implemented work
- Promoted `landimpact` into the bounded typed `RegionObjects` sample contract.
- Extended app-side `typed_sample=...` summary emission to include `landimpact` when present.
- Kept the existing bounded summary style and reconnect behavior unchanged.
- Live-validated on Ahern baseline with `cargo run -p viewer_app`.

## Files changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_OBJECT_INGRESS_REGION_OBJECTS_TYPED_FIELD_PROMOTION_LANDIMPACT_2026-04-01.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_REGION_OBJECTS_TYPED_FIELD_PROMOTION_LANDIMPACT_2026-04-01.md`
- `docs/reports/REPORT_OBJECT_INGRESS_REGION_OBJECTS_TYPED_FIELD_PROMOTION_LANDIMPACT_2026-04-01.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/OBJECT_INGRESS_STATUS.md`

## Validation run
- Passed:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net -p viewer_app`
  - `VIEWER_APP_LIVE_STARTUP=on VIEWER_APP_AUTO_TELEPORT_SLURL=secondlife://Ahern/50/60/70 VIEWER_APP_AUTO_TELEPORT_DELAY_TICKS=40 VIEWER_APP_REGION_OBJECTS_REPROBE_DELAY_TICKS=80 VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_region_objects_typed_landimpact_2026-04-01.jsonl cargo run -p viewer_app`
- Failed:
  - none
- Remaining unvalidated:
  - none for this slice
- Artifacts:
  - `artifacts/logs/live_region_objects_typed_landimpact_2026-04-01.out.log`
  - `artifacts/logs/live_region_objects_typed_landimpact_2026-04-01.err.log`
  - `artifacts/logs/network_debug_region_objects_typed_landimpact_2026-04-01.jsonl`

## Result status
- `typed_sample=...` now includes `landimpact` when present.
- Live evidence on Ahern baseline includes values such as:
  - `landimpact=1`
  - `landimpact=20`
- Reconnect and delayed re-probe behavior remained healthy in the same run.
- LLUDP object ingress remains unchanged (`RegionHandshake`/`RegionHandshakeReply`/`ObjectUpdate*` still absent).

## Risks or follow-up items
- `landimpact` may be absent for some sampled objects/regions; absence in future runs is not automatically a regression.
- Branch priority remains: keep typed-feed improvements bounded while running paired A/B reconnect captures.

## Learnings delta
- none
- Reason: this slice is a direct bounded field promotion using established pattern and did not introduce a new durable failure mode or strategy change.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/OBJECT_INGRESS_STATUS.md`
