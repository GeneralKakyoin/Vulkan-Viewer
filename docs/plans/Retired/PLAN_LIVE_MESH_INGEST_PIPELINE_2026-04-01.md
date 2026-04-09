# Plan: Live Mesh Ingest Pipeline (2026-04-01)

## Scope
- Add a bounded mesh fetch/request path in `viewer_app` so visible/fixture mesh IDs can be fetched from `ViewerAsset` (`mesh_id`) and fed into `geometry_cache.get_mesh(...)`.
- Keep transport and capability semantics within existing crate boundaries.

## Current Known State
- Texture ingest path is live and proven.
- Mesh render path currently calls `get_mesh(uuid, lod, &[])`, so no live mesh bytes are ever supplied.
- Lane probes show `mesh_id` requests can currently return 403 for dummy IDs.

## Files and Components Touched
- `crates/viewer_app/src/main.rs`
- `crates/viewer_net/src/lib.rs`
- `docs/TESTING_REFERENCE.md`
- plan/review/report + continuity docs

## Boundary Check
- `viewer_app`: orchestration, request scheduling, ingest state.
- `viewer_net`: bounded HTTP helper for mesh candidate fetch.
- `viewer_grid`: unchanged semantics usage (query-shape helpers only).

## Step Sequence
1. Add mesh fetch helper in `viewer_net` (`fetch_mesh_asset_bytes`).
2. Add `LiveFeedCommand::RequestMesh` and `LiveFeedUpdate::MeshAsset/MeshAssetFailed`.
3. Add `viewer_app` mesh request state:
   - fixture mesh IDs (`VIEWER_FIXTURE_MESHES`)
   - requested/failed tracking
   - downloaded mesh byte cache keyed by `(uuid,lod)`.
4. Add `tick_scene_meshes(...)` to request fixture + visible mesh IDs.
5. Update geometry prep path to pass fetched bytes to `geometry_cache.get_mesh(uuid,lod,bytes)`.
6. Add targeted tests for mesh env parsing and command/update wiring where practical.
7. Validate and run a bounded live capture with mesh-fetch relay evidence.

## Validation Plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- targeted tests for touched parsing/helpers
- bounded live run with:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_ASSET_SOURCE_MODE=live`
  - `VIEWER_FIXTURE_MESHES=<id>`
  - capture `mesh_fetch: queued|ready|failed` lines

## Risks and Open Questions
- Unknown mesh IDs will return deterministic 4xx; pipeline may validate as `queued/failed` without `ready`.
- Success requires at least one valid mesh asset UUID.

## Deferred-Too-Early Candidates
- Full mesh decode diagnostics/format introspection in UI panel.

## Learnings Pre-Check
- L64, L65, L66, L72.

## Exact Completion Criteria
- Mesh bytes can be requested and routed to `geometry_cache.get_mesh`.
- Runtime emits `mesh_fetch` relay lines proving command execution.
- Documentation and continuity updates complete.
