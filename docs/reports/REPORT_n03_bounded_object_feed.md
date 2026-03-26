# REPORT: N03 Bounded World/Object Decode Expansion for Richer Render Feed

## Summary of implemented work
- Added bounded LLUDP classification + minimal decode for the object-update family:
  - `ObjectUpdate` (high 12, zerocoded): extracts local IDs + scale and updates a bounded feed.
  - `ObjectUpdateCompressed` (high 13): extracts local IDs from the packed `Data` block.
  - `ObjectUpdateCached` (high 14): extracts local IDs.
  - `ImprovedTerseObjectUpdate` (high 15): extracts local IDs from the packed `Data` block.
  - `KillObject` (high 16): extracts local IDs to remove from the feed.
- Implemented a bounded object-feed state in `viewer_net` and exported it via `SimulatorPayloadDecodeSummary` with explicit caps and truncation signaling.
- Bridged the object feed through `LiveVisualSnapshot` and the world-ingestion seam into `viewer_core::Scene`.
- Added seam-driven scene lifecycle for world-object feed proxies:
  - create/update per local ID
  - remove on explicit kill signals
  - conservative removal when feed export is truncated (no mass-delete on truncation)
- Added tests for object-feed decode and seam/scene lifecycle behavior.

## Files changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_net/examples/llsd_login_attempt.rs`
- `crates/viewer_core/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `crates/viewer_ui/src/lib.rs`
- `docs/plans/PLAN_N03.md`
- `docs/reviews/REVIEW_plan_n03.md`
- `docs/plans/DEFERRED_FEATURES.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/plans/PLAN.md`

## Validation run
- `cargo fmt` (pass)
- `cargo check` (pass; warnings only from existing `wgpu` deprecated copy type aliases)
- `cargo test -p viewer_net` (pass)
- `cargo test -p viewer_core` (pass)
- `cargo test -p viewer_app` (pass)
- `cargo test` (pass)
- Runtime smoke (time-bounded; process was stopped by timeout):
  - `VIEWER_APP_LIVE_STARTUP=off`
  - `cargo run -p viewer_app`

## Result status
Complete for N03 scope.

## Risks or follow-up items
- Object update payload parsing remains intentionally minimal and bounded. Deeper payload parity (materials, per-face texture entry semantics) remains deferred to later milestones.
- `ObjectUpdate` is zerocoded; decode is bounded and defensive, but additional live observation may reveal variants that require further hardening.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md` to reflect N03 execution completion.
- Updated `docs/HANDOFF.md` to set the next step to `U04` planning/execution.
- Updated `docs/plans/PLAN.md` “Current next planning targets” to start from `U04`.
