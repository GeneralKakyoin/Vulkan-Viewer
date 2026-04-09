# Report: World Object Feed True Placement + Scale (2026-04-03)

## Summary of implemented work
- Updated `viewer_core::world_object_feed_proxy_transform(...)` so decoded world-object-feed positions are no longer compressed into a small debug cluster.
- Applied explicit axis mapping from decoded object-feed coordinates to scene world space:
  - position: `X,Y,Z` -> `X,Z,Y`
  - scale: `X,Y,Z` -> `X,Z,Y`
- Preserved deterministic ring fallback for objects with missing decoded position data.
- Added focused regression tests for decoded position/scale mapping and fallback behavior.

## Files changed
- `crates/viewer_core/src/lib.rs`
- `docs/plans/PLAN_WORLD_OBJECT_FEED_TRUE_PLACEMENT_SCALE_2026-04-03.md`
- `docs/reviews/REVIEW_PLAN_WORLD_OBJECT_FEED_TRUE_PLACEMENT_SCALE_2026-04-03.md`
- `docs/reviews/REVIEW_IMPL_WORLD_OBJECT_FEED_TRUE_PLACEMENT_SCALE_2026-04-03.md`
- `docs/reports/REPORT_WORLD_OBJECT_FEED_TRUE_PLACEMENT_SCALE_2026-04-03.md`
- `docs/plans/DEFERRED_FEATURES.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`

## Validation run
- `cargo fmt --all` -> PASS
- `cargo check -p viewer_core` -> PASS
- `cargo test -p viewer_core` -> PASS
- `cargo check -p viewer_app` -> PASS
- bounded live launch:
  - command: `VIEWER_APP_LIVE_STARTUP=on VIEWER_FIXTURE_MESHES=0 VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_world_object_true_placement_2026-04-03.jsonl cargo run -p viewer_app`
  - result: process timed out by harness (expected for interactive runtime), with healthy in-window ingress evidence in the generated log (`lludp_object_gate: PASS`, object-feed tick summaries present)

## Result status
- Complete for this slice.
- Object-feed placement now follows decoded in-region position and axis-correct scale instead of debug-cluster compression.

## Risks or follow-up items
- Visual framing may need tuning in some capture modes now that object positions are spread by real in-region coordinates.
- Object rotation parity is still pending and should be handled in a separate bounded slice.

## Learnings delta
- `none`
- Reason: this was an expected application of existing learnings (`L76`) rather than a newly discovered durable trap.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/plans/DEFERRED_FEATURES.md`
