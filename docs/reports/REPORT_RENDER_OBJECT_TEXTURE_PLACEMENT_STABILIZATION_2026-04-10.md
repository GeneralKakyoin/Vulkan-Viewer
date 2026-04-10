# REPORT_RENDER_OBJECT_TEXTURE_PLACEMENT_STABILIZATION_2026-04-10

## Summary of Implemented Work
- Stabilized world-object-feed proxy placement in `viewer_core`:
  - `upsert_world_object_feed(...)` now passes prior instance transform into transform derivation when present.
  - `world_object_feed_proxy_transform(...)` now:
    - reuses prior position when new position payload is absent
    - defaults first-sighting positionless objects to anchor (removes local-id ring scatter)
    - reuses prior scale/rotation for partial updates when new payload fields are absent
- Corrected SL→scene axis conversion to a right-handed mapping (`x,z,-y`) in both:
  - `viewer_asset::sl_mesh_loader::sl_to_scene_axis(...)`
  - `viewer_core::world_object_feed_proxy_transform(...)` position/rotation mapping
- Improved texture lane reliability/observability in `viewer_app`:
  - increased `LIVE_TEXTURE_FETCH_MAX_INFLIGHT` from `4` to `16`
  - extended `texture_fetch` ready relay with byte length/signature
  - added `texture_asset` decode-stage relay lines for decode success/failure, including bytes/signature and decoded dimensions/failure reason
- Added/updated tests:
  - `viewer_core`: `scene_world_object_feed_positionless_update_retains_existing_position`
  - `viewer_asset`: `sl_to_scene_axis_maps_z_up_to_y_up` (updated expectation)
  - `viewer_core`: `scene_world_object_feed_maps_decoded_rotation_quaternion_to_scene_axes` (updated expectation)

## Files Changed
- `crates/viewer_core/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `crates/viewer_asset/src/sl_mesh_loader.rs`
- `docs/plans/PLAN_RENDER_OBJECT_TEXTURE_PLACEMENT_STABILIZATION_2026-04-10.md`
- `docs/reviews/REVIEW_PLAN_RENDER_OBJECT_TEXTURE_PLACEMENT_STABILIZATION_2026-04-10.md`
- `docs/reviews/REVIEW_IMPL_RENDER_OBJECT_TEXTURE_PLACEMENT_STABILIZATION_2026-04-10.md`
- `docs/reports/REPORT_RENDER_OBJECT_TEXTURE_PLACEMENT_STABILIZATION_2026-04-10.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`

## Validation Run
Commands run:
- `cargo fmt --all`
- `cargo check -p viewer_core -p viewer_app`
- `cargo test -p viewer_core scene_world_object_feed_positionless_update_retains_existing_position -- --nocapture`
- `cargo test -p viewer_app test_merge_texture_request_ids_preserves_left_priority_order -- --nocapture`
- `cargo fmt --all` (after axis-conversion adjustment)
- `cargo check -p viewer_asset -p viewer_core -p viewer_app`
- `cargo test -p viewer_asset sl_to_scene_axis_maps_z_up_to_y_up -- --nocapture`
- `cargo test -p viewer_core scene_world_object_feed_maps_decoded_rotation_quaternion_to_scene_axes -- --nocapture`
- `cargo test -p viewer_core scene_world_object_feed_positionless_update_retains_existing_position -- --nocapture`
- `cargo test -p viewer_app test_merge_texture_request_ids_preserves_left_priority_order -- --nocapture`
- `VIEWER_APP_LIVE_STARTUP=off STRESS_TEST=screenshot VIEWER_TEST_SCREENSHOT_DIR=artifacts/screenshots_render_object_texture_placement_2026-04-10_b VIEWER_TEST_SCREENSHOT_EVERY_N_FRAMES=1 VIEWER_TEST_SCREENSHOT_MAX_FRAMES=1 cargo run -p viewer_app`

What passed:
- All listed commands completed successfully.
- Manual screenshot review performed on:
  - `artifacts/screenshots_render_object_texture_placement_2026-04-10_b/viewer_test_0001.png`
- Manual visual verdict:
  - Screenshot smoke scene remained stable and rendered as expected for offline screenshot mode.

What failed:
- None.

What remains unvalidated:
- Live in-world visual parity for the user’s reported chair case (orientation/texture richness) has not yet been re-captured in this run because that requires a connected live scene repro from user environment.

## Result Status
- Bounded implementation complete and validated at code/test/smoke level.
- Follow-up live confirmation still required for final user-scene parity signoff.

## Risks / Follow-up Items
- Some texture IDs in prior runtime artifacts return `403 MissingCapability`; those assets will still render missing/flat until transport capability access succeeds.
- Next diagnostic branch (if needed): add renderer-side per-draw material bind status counters for Ready/Loading/Missing IDs.

## Learnings Delta
- `none` — No durable learning identified; this slice was an incremental stabilization and observability pass over known pathways.

## Continuity Updates Performed
- Added latest-notable-change section to `docs/CURRENT_STATE.md`.
- Replaced `docs/HANDOFF.md` with latest handoff for this slice.
