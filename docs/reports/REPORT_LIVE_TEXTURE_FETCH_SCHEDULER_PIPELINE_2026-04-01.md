# Report: Live Texture Fetch Scheduler Pipeline (2026-04-01)

## Summary Of Implemented Work
- Replaced ad-hoc `RequestTexture` fire-and-forget spawning in `viewer_app` worker with a bounded live texture scheduler pipeline:
  - queued requests (`AssetID` dedupe)
  - priority-aware dispatch
  - bounded in-flight cap
  - completion handling lane
  - bounded retry/backoff for retryable failures
- Preserved Firestorm-aligned capability URL shaping by continuing to use `viewer_grid::AssetCapabilityPolicy::texture_url_candidates(...)`.
- Preserved transport ownership by continuing to use `viewer_net::fetch_texture_asset_bytes(...)`.
- Updated app ingest decode for `LiveFeedUpdate::TextureAsset` from PNG-only to `viewer_asset::decode_texture_rgba8(...)` (J2C + PNG capable).
- Added unit tests for retry policy and retry backoff helper behavior.

## Files Changed
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_LIVE_TEXTURE_FETCH_SCHEDULER_PIPELINE_2026-04-01.md`
- `docs/reviews/REVIEW_PLAN_LIVE_TEXTURE_FETCH_SCHEDULER_PIPELINE_2026-04-01.md`
- `docs/reviews/REVIEW_IMPL_LIVE_TEXTURE_FETCH_SCHEDULER_PIPELINE_2026-04-01.md`
- `docs/reports/REPORT_LIVE_TEXTURE_FETCH_SCHEDULER_PIPELINE_2026-04-01.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/LEARNINGS.md`

## Validation Run
- `cargo fmt --all` -> PASSED
- `cargo check -p viewer_app -p viewer_asset -p viewer_net -p viewer_grid` -> PASSED
- `cargo test -p viewer_app -p viewer_asset` -> PASSED
- Runtime smoke:
  - command:
    - `VIEWER_APP_LIVE_STARTUP=off`
    - `STRESS_TEST=screenshot`
    - `VIEWER_TEST_SCREENSHOT_DIR=artifacts/screenshots_live_texture_scheduler_smoke_2026-04-01`
    - `VIEWER_TEST_SCREENSHOT_EVERY_N_FRAMES=1`
    - `VIEWER_TEST_SCREENSHOT_MAX_FRAMES=1`
    - `cargo run -p viewer_app`
  - result: timeout-bounded run ended with tool timeout, but screenshot capture succeeded:
    - `artifacts/screenshots_live_texture_scheduler_smoke_2026-04-01/viewer_test_0001.png`
  - manual screenshot review: PASS (expected scene rendered, no obvious corruption/regression)

## Result Status
- Implementation complete for bounded scheduler/pipeline slice.
- Validation complete for formatting, compilation, tests, and bounded runtime visual smoke.

## Risks Or Follow-Up Items
- Tune scheduler constants with live evidence under denser texture request pressure.
- Evaluate whether bounded `Range` support is needed for parity-depth follow-up.

## Learnings Delta
- `added`: `L65` (live texture ingest must use shared decode supporting J2C, not PNG-only decode assumptions).

## Continuity Updates Performed
- Updated `docs/CURRENT_STATE.md` with latest notable changes for live texture scheduler pipeline.
- Replaced `docs/HANDOFF.md` with current exact handoff for this slice.
- Updated `docs/LEARNINGS.md` with durable lesson `L65`.
