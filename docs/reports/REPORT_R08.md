# REPORT_R08 Avatar Appearance and Attachment Render Foundation

## Summary
Implemented the R08 bounded avatar appearance baseline by adding typed avatar appearance and attachment proxy contracts in `viewer_core`, projecting deterministic attachment proxies from the existing avatar sample path in `viewer_app`, and wiring seam-owned lifecycle handling for attachment proxies in `Scene::apply_world_object_ingestion_seam(...)`.

## Files Changed
- `crates/viewer_core/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `crates/viewer_ui/src/lib.rs`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/LEARNINGS.md`

## Validation Run
- `cargo fmt --all`
- `cargo check --workspace`
- `cargo test -p viewer_core`
- `cargo test -p viewer_app`
- `cargo test -p viewer_ui`
- `cargo test -p viewer_render`
- `cargo test --workspace`
- `VIEWER_APP_LIVE_STARTUP=off STRESS_TEST=screenshot VIEWER_TEST_SCREENSHOT_DIR=artifacts/screenshots_r08_smoke VIEWER_TEST_SCREENSHOT_EVERY_N_FRAMES=1 VIEWER_TEST_SCREENSHOT_MAX_FRAMES=1 cargo run -p viewer_app`

## Result Status
- Passed: all validation commands above.
- Failed: none.
- Remaining unvalidated: live online worker / login path verification was not run for this milestone.

## Risks / Follow-Up
- Attachment proxies are intentionally bounded and synthetic in R08; future avatar parity work will need real attachment semantics before this can represent live content.
- The renderer path itself did not need new GPU branching because attachments reuse the existing `MeshKind::AvatarProxy` path.

## Learnings Delta
- `added`: L17 captures the seam-side payload preference for attachment data derived from app-local avatar samples.

## Continuity Updates
- Updated `docs/CURRENT_STATE.md` to reflect R08 completion and the next milestone focus.
- Replaced `docs/HANDOFF.md` with the R08 handoff state and next step.
- Added the R08 durable learning entry in `docs/LEARNINGS.md`.
