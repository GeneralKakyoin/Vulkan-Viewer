# HANDOFF: Milestone R08 Avatar Appearance and Attachment Foundation

## What Changed
- Added bounded avatar appearance contracts in `viewer_core`:
  - `AvatarAppearanceSummary`
  - `AvatarAttachmentProxy`
  - `MAX_R08_ATTACHMENTS_PER_AVATAR = 8`
- Added `InstanceRole::WorldAvatarAttachmentProxy` and attachment lifecycle handling in `Scene::apply_world_object_ingestion_seam(...)`.
- Added seam-side attachment payload collection on `WorldObjectIngestionSeam::avatar_attachments` so attachment lifecycle remains seam-owned.
- Added deterministic attachment projection in `viewer_app` from the existing avatar samples into the seam path.
- Extended `viewer_ui` diagnostics with avatar/attachment proxy counts.
- Added `viewer_core` tests for bounded deterministic attachment projection and seam create/remove lifecycle.

## Validation Run
- `cargo fmt --all`: PASSED
- `cargo check --workspace`: PASSED
- `cargo test -p viewer_core`: PASSED
- `cargo test -p viewer_app`: PASSED
- `cargo test -p viewer_ui`: PASSED
- `cargo test -p viewer_render`: PASSED
- `cargo test --workspace`: PASSED
- `VIEWER_APP_LIVE_STARTUP=off STRESS_TEST=screenshot VIEWER_TEST_SCREENSHOT_DIR=artifacts/screenshots_r08_smoke VIEWER_TEST_SCREENSHOT_EVERY_N_FRAMES=1 VIEWER_TEST_SCREENSHOT_MAX_FRAMES=1 cargo run -p viewer_app`: PASSED, produced `artifacts/screenshots_r08_smoke/viewer_test_0001.png`

## Exact Current State
- R08 is implemented and validated on the current workspace baseline.
- Avatar body proxies continue to use the existing `MeshKind::AvatarProxy` path.
- Attachment proxies are deterministic, bounded, and removed when the seam payload disappears.
- `viewer_app` now projects attachment proxies from the current avatar samples before applying the seam.
- `viewer_ui` shows avatar and attachment proxy counts in diagnostics.

## Exact Next Step
- Begin `U09` planning/implementation from the validated R08 baseline.

## Blockers / Risks
- No active blockers remain from the R08 implementation.
- Live online worker verification was not run in this pass; validation was offline/bounded only.
