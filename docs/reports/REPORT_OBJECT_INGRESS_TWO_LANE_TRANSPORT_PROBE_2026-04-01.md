# Report: Object Ingress Two-Lane Transport Probe (2026-04-01)

## Summary of implemented work
- Added a new one-shot capability transport probe API in `viewer_net`:
  - `probe_capability_transport_once(url)` -> `status`, `content_type`, `body_bytes`
- Added deterministic two-lane target selection in `viewer_app` from non-baseline seed capabilities (max two host families).
- Added startup protocol/relay logging for lane probe start/result:
  - `LaneProbe:start ...`
  - `LaneProbe:ok ...` / `LaneProbe:err ...`
  - `parallel_protocol` relay line with target + classified URL + transport result.
- Added tests for:
  - `viewer_net` probe metadata reporting
  - `viewer_app` lane target selection behavior.

## Files changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_OBJECT_INGRESS_TWO_LANE_TRANSPORT_PROBE_2026-04-01.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_TWO_LANE_TRANSPORT_PROBE_2026-04-01.md`
- `docs/reviews/REVIEW_IMPL_OBJECT_INGRESS_TWO_LANE_TRANSPORT_PROBE_2026-04-01.md`

## Validation run
- `cargo fmt --all`: PASSED
- `cargo check -p viewer_net -p viewer_app`: PASSED
- `cargo test -p viewer_net`: PASSED (104 tests)
- `cargo test -p viewer_app`: PASSED (55 tests)
- bounded live run:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_two_lane_transport_probe_2026-04-01.jsonl`
  - `cargo run -p viewer_app` (bounded capture)

## Result status
- Both newly surfaced lanes now have concrete transport evidence:
  - `asset-cdn` probe (`ViewerAsset`) -> `status=403`, `content_type=application/xml`, `body_bytes=275`
  - `simhost-...` probe (`SimulatorFeatures`) -> `status=503`, `content_type=text/plain`, `body_bytes=13`
- This confirms lane reachability/behavior differences and removes ambiguity about whether those lanes are responding at all.
- LLUDP object ingress remains blocked (`ObjectUpdate*` still absent in the same run).

## Risks or follow-up items
- Current probes are transport-level only and do not include lane-specific required request shaping.
- Next slice should test lane-specific invocation shape for selected targets before drawing semantic conclusions.

## Learnings delta
- added
- Added L63 to record that non-baseline lane probes can return non-2xx yet still provide actionable lane-presence evidence.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/OBJECT_INGRESS_STATUS.md`
- Updated `docs/LEARNINGS.md`
