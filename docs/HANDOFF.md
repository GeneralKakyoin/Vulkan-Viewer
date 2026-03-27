# HANDOFF: R12 Environment Gap Fixes Applied

## What Changed
- Fixed R12 fog depth computation in `viewer_render` by switching from `clip_position.w` to world-space camera distance.
- Activated `fog.density` in shader fog math and kept fog bounded by configured start/end range.
- Extended `viewer_core::EnvironmentState` with additive controls:
  - `time_of_day_normalized`
  - `sky_enabled`
  - `fog_enabled`
- Added `EnvironmentState::sanitized()` to clamp invalid environment values before renderer use.
- Updated `viewer_app` with `derive_environment_from_snapshot(...)` and per-frame environment mapping from live snapshot.
- Expanded environment diagnostics in `viewer_ui` to show time-of-day, sky/fog flags, sky top/bottom, and fog values.
- Added targeted tests in `viewer_core`, `viewer_render`, and `viewer_app` for environment defaults/sanitization, clear-color behavior, and mapping determinism.

## Validation Run
- `cargo fmt --all`: PASSED
- `cargo check -p viewer_core -p viewer_render -p viewer_app -p viewer_ui`: PASSED
- `cargo test -p viewer_core -p viewer_render -p viewer_app -p viewer_ui`: PASSED
- `VIEWER_APP_LIVE_STARTUP=off STRESS_TEST=screenshot VIEWER_TEST_SCREENSHOT_DIR=artifacts/screenshots_r12_gapfix_smoke VIEWER_TEST_SCREENSHOT_EVERY_N_FRAMES=1 VIEWER_TEST_SCREENSHOT_MAX_FRAMES=2 cargo run -p viewer_app`: PASSED

## Exact Current State
- R12 environment path now uses bounded, sanitized contracts with explicit enable flags and time-of-day scalar.
- Fog now responds to distance and density as intended.
- Sky top and bottom colors are both used in runtime rendering behavior.

## Exact Next Step
- Re-run workspace-wide strict lint validation (`cargo clippy --workspace --all-targets -- -D warnings`) and either:
  - address remaining warnings (including `DEBUG_CLIP_SPACE_TRIANGLE`), or
  - explicitly defer warning cleanup in a scoped follow-up plan.

## Blockers / Risks
- No functional blocker found.
- `viewer_render` still emits a dead-code warning for `DEBUG_CLIP_SPACE_TRIANGLE`; not functionally harmful but clippy-strict workflows may fail until cleaned.
