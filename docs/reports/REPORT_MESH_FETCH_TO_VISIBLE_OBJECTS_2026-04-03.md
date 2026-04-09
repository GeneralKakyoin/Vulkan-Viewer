# Report: Mesh Fetch To Visible Objects (2026-04-03)

## Summary of implemented work
- Added typed mesh cache lookup status in `viewer_asset` so the app can distinguish empty data, cached-ready meshes, decoded meshes, and decode failures.
- Implemented SL mesh asset format detection and a real binary-LLSD + zlib mesh decoder in `viewer_asset`.
- Extended `viewer_app` live mesh state into lifecycle states (`Requested`, `Fetched`, `Decoded`, `Failed`) and promoted decoded meshes into renderer uploads.
- Seeded the screenshot torture scene with a deterministic synthetic SL mesh asset so decode/upload/render can be validated offline.
- Added bounded mesh verification env wiring and summary reporting.

## Files changed
- `crates/viewer_asset/Cargo.toml`
- `crates/viewer_asset/src/lib.rs`
- `crates/viewer_asset/src/sl_mesh_loader.rs`
- `crates/viewer_app/src/main.rs`
- `docs/TESTING_REFERENCE.md`
- `docs/plans/DEFERRED_FEATURES.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/LEARNINGS.md`

## Validation run
- `cargo fmt --all` -> PASS
- `cargo check -p viewer_asset -p viewer_app` -> PASS
- `cargo test -p viewer_asset` -> PASS
- `cargo test -p viewer_app` -> PASS
- offline screenshot smoke:
  - `VIEWER_APP_LIVE_STARTUP=off`
  - `STRESS_TEST=screenshot`
  - `VIEWER_TEST_SCREENSHOT_DIR=artifacts/screenshots_mesh_visibility_offline_verify_2026-04-03`
  - `VIEWER_TEST_SCREENSHOT_EVERY_N_FRAMES=1`
  - `VIEWER_TEST_SCREENSHOT_MAX_FRAMES=1`
  - `VIEWER_APP_MESH_VERIFY=on`
  - `VIEWER_APP_MESH_VERIFY_ID=debug-secondlife-mesh`
  - `VIEWER_APP_MESH_VERIFY_LOG_PATH=artifacts/logs/mesh_visibility_offline_verify_2026-04-03.jsonl`
  - `VIEWER_APP_MESH_VERIFY_SCREENSHOT_DIR=artifacts/screenshots_mesh_visibility_offline_verify_2026-04-03`
  - `.\\target\\debug\\viewer_app.exe *> artifacts\\logs\\mesh_visibility_offline_verify_2026-04-03.out.log`
  - Result: screenshot artifact captured; process did not self-exit before tool timeout
- controlled offline screenshot smoke:
  - `Start-Process` wrapper around `.\\target\\debug\\viewer_app.exe` with equivalent env vars
  - Result: screenshot artifact captured and process stopped cleanly

## Result status
- Geometry-first milestone complete:
  - SL mesh bytes decode into `ProcessedMesh`
  - decoded mesh geometry uploads into the renderer
  - offline screenshot verification shows non-placeholder mesh geometry visible in the viewer
- One verification artifact remains partial:
  - the compact mesh-verification JSON log path was configured but no JSONL file was emitted in the bounded offline runs

## Risks or follow-up items
- Full live object material/texture parity is still deferred.
- The compact mesh-verification JSON log sink needs a targeted follow-up if it is required as a canonical lifecycle artifact.
- Live login instability prevented using this task to prove the same path on a fresh live object-visibility run.

## Learnings delta
- `added`
- Added `L79` to capture the deterministic synthetic SL mesh verification pattern.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/TESTING_REFERENCE.md`
- Updated `docs/plans/DEFERRED_FEATURES.md`
- Updated `docs/LEARNINGS.md`
