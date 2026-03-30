# REPORT_single_live_texture_first_visible

## Summary of implemented work
- Added a shared robust texture decode path in `viewer_asset` (`decode_texture_rgba8`) with decode order:
  1. standard image decoders
  2. `jpeg2k` decoder
  3. `justjp2` fallback path
- Updated live texture ingestion in `viewer_app` to use shared decode instead of PNG-only decode.
- Updated `tick_scene_textures(...)` request composition so scene-visible textures can be requested even when `VIEWER_FIXTURE_TEXTURES` is empty; first visible ID is now used as the live trigger when there are no seed IDs.
- Updated `viewer_grid::AssetCapabilityPolicy` `ViewerAsset` fallback URL to query-style `/?texture_id=<id>`.
- Added explicit `live_requests_failed_missing_capability` metric bucket in `viewer_core::AssetContinuityMetrics`.
- Wired missing-capability failure accounting in `viewer_asset` and diagnostics display in `viewer_ui`.
- Added targeted tests across `viewer_asset`, `viewer_app`, `viewer_grid`, and `viewer_ui`.
- Verified message-template guardrail: this slice did not modify LLUDP message ID mappings/classification.

## Files changed
- `crates/viewer_core/src/lib.rs`
- `crates/viewer_asset/Cargo.toml`
- `crates/viewer_asset/src/lib.rs`
- `crates/viewer_asset/src/texture_fixture.rs`
- `crates/viewer_app/src/main.rs`
- `crates/viewer_grid/src/lib.rs`
- `crates/viewer_ui/src/lib.rs`
- `docs/plans/PLAN_SINGLE_LIVE_TEXTURE_FIRST_VISIBLE.md`
- `docs/reviews/REVIEW_PLAN_SINGLE_LIVE_TEXTURE_FIRST_VISIBLE.md`
- `docs/reviews/REVIEW_IMPL_SINGLE_LIVE_TEXTURE_FIRST_VISIBLE.md`
- `docs/reports/REPORT_single_live_texture_first_visible.md`
- `docs/RESEARCH/SL_UPSTREAM_SOURCE_AUDIT_2026-03-29.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`

## Validation run
- `cargo fmt --all` -> PASSED
- `cargo test -p viewer_asset -p viewer_grid -p viewer_ui -p viewer_app` -> PASSED
- `cargo check --workspace` -> PASSED
- `cargo test --workspace` -> PASSED
- Connected run attempt:
  - Command:
    - `VIEWER_APP_LIVE_STARTUP=on`
    - `VIEWER_ASSET_SOURCE_MODE=live`
    - `STRESS_TEST=screenshot`
    - `VIEWER_TEST_SCREENSHOT_DIR=artifacts/screenshots_live_texture_n16`
    - `VIEWER_TEST_SCREENSHOT_EVERY_N_FRAMES=1`
    - `VIEWER_TEST_SCREENSHOT_MAX_FRAMES=1`
    - `cargo run -p viewer_app`
  - Initial result: TIMED OUT in this environment with credentials unset.
  - Follow-up run (after `.env` setup): process exited cleanly (`exit 0`) with runtime output:
    - `startup: live worker starting`
    - `login: login successful`
  - Follow-up screenshot reviewed: `artifacts/screenshots_live_texture_n16_connected/viewer_test_0001.png`.
  - Live-texture proof status: still pending. No `texture_fetch` success evidence appeared in captured output, and screenshot remained fallback-colored baseline.

## Result status
- Code implementation and offline/CI-level validation: completed.
- Connected acceptance criterion (real SL texture render proof): pending due missing observable live texture fetch/render evidence in current scene capture.

## Risks or follow-up items
- Must re-run connected validation with diagnostics visible or targeted texture ID seeding to confirm at least one SL texture is fetched live and rendered in-scene.

## Learnings delta
none - No durable learning identified; changes followed existing architecture/process constraints.

## Continuity updates performed
- Added plan/review/report artifacts for this slice.
- Updated current state and handoff to reflect implementation completion and connected-validation next step.
