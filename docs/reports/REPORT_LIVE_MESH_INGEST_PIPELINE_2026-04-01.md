# REPORT: Live Mesh Ingest Pipeline (2026-04-01)

## Summary of Implemented Work
- Added bounded mesh HTTP fetch helper in `viewer_net`:
  - `fetch_mesh_asset_bytes(urls, timeout)` with mesh Accept header handling.
- Completed `viewer_app` live mesh command/update lane:
  - `LiveFeedCommand::RequestMesh { id, lod }`
  - `LiveFeedUpdate::MeshAsset` / `LiveFeedUpdate::MeshAssetFailed`
  - worker command handling now resolves URL candidates from `GetMesh2` / `GetMesh` / `ViewerAsset` and fetches bytes.
- Added mesh ingest state to app orchestration:
  - fixture IDs from `VIEWER_FIXTURE_MESHES`
  - requested/failed/live-bytes tracking
  - per-frame mesh request tick (`tick_scene_meshes`) for fixture + visible mesh geometry.
- Wired geometry path to feed fetched bytes into mesh cache decode path:
  - `geometry_cache.get_mesh(uuid, lod, bytes)`.
- Added targeted tests:
  - `viewer_net`: empty mesh candidate guard.
  - `viewer_app`: fixture mesh CSV normalization helper.
- Updated testing reference with `VIEWER_FIXTURE_MESHES`.

## Files Changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/TESTING_REFERENCE.md`

## Validation Run
- `cargo fmt --all` -> PASS
- `cargo check -p viewer_net -p viewer_app` -> PASS
- `cargo test -p viewer_net fetch_mesh_asset_bytes_requires_candidate_urls -- --nocapture` -> PASS
- `cargo test -p viewer_app fixture_mesh_ids_from_csv_normalizes_and_filters -- --nocapture` -> PASS
- bounded live run (no forced fixture mesh):
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_ASSET_SOURCE_MODE=live`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_live_mesh_ingest_probe_2026-04-01.jsonl`
  - `cargo run -p viewer_app`
- bounded live run (forced fixture mesh UUID):
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_ASSET_SOURCE_MODE=live`
  - `VIEWER_FIXTURE_MESHES=00000000-0000-0000-0000-000000000001`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_live_mesh_fixture_probe_2026-04-01.jsonl`
  - `cargo run -p viewer_app` (time-bounded; command timeout after evidence capture)

## Runtime Evidence
- Mesh command lane executes in live runtime:
  - `artifacts/logs/live_mesh_fixture_probe_2026-04-01.log:60`
    - `mesh_fetch: queued id=00000000-0000-0000-0000-000000000001 lod=0 candidates=5`
- Mesh fetch failure classification is surfaced with exact HTTP reason:
  - `artifacts/logs/live_mesh_fixture_probe_2026-04-01.log:61`
    - `mesh_fetch: failed ... reason=MissingCapability detail=http status 403 Forbidden ...`

## Result Status
- Implemented and validated: mesh ingest transport/orchestration path is now active and observable.
- Not yet proven in this run: `mesh_fetch: ready` (requires at least one valid, permission-allowed mesh UUID).

## Risks / Follow-up
- The current forced fixture UUID is intentionally synthetic and returns deterministic 403.
- Next probe to close remaining gap:
  - run with one known valid mesh UUID and capture `mesh_fetch: ready ...`.

## Learnings Delta
- `added` (L73): forced fixture mesh IDs are the fastest deterministic way to verify mesh command lane execution when scene visibility does not include mesh geometry.

## Continuity Updates Performed
- `docs/TESTING_REFERENCE.md` updated with `VIEWER_FIXTURE_MESHES`.
- `docs/CURRENT_STATE.md` updated.
- `docs/HANDOFF.md` updated.
- `docs/LEARNINGS.md` updated.
