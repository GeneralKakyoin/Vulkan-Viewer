# REPORT: World Object Feed Mesh Geometry Swap (No Proxy Cubes When Mesh IDs Exist) 2026-04-02

## Summary Of Implemented Work
- Added `decoded_object_mesh_id: Option<String>` to `viewer_core::WorldObjectIngestionItem`.
- Updated `WorldObjectIngestionAdapter` to forward `DecodedWorldObjectFeedObject.mesh_id` into seam items.
- Updated `Scene::upsert_world_object_feed(...)` to select geometry per object:
  - `GeometrySource::Mesh(mesh_id, 0)` when mesh ID exists
  - `GeometrySource::Diagnostic(MeshKind::Cube)` fallback when absent
- Existing world-object instances now update geometry in place when mesh ID presence changes, including local AABB refresh.
- Added regression test:
  - `scene_world_object_feed_uses_live_mesh_geometry_when_mesh_id_present`

## Files Changed
- `crates/viewer_core/src/lib.rs`
- `docs/plans/PLAN_WORLD_OBJECT_FEED_MESH_GEOMETRY_SWAP_2026-04-02.md`
- `docs/reviews/REVIEW_PLAN_WORLD_OBJECT_FEED_MESH_GEOMETRY_SWAP_2026-04-02.md`
- `docs/reviews/REVIEW_IMPL_WORLD_OBJECT_FEED_MESH_GEOMETRY_SWAP_2026-04-02.md`
- `docs/reports/REPORT_WORLD_OBJECT_FEED_MESH_GEOMETRY_SWAP_2026-04-02.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`

## Validation Run
- `cargo fmt --all` -> PASS
- `cargo check -p viewer_core -p viewer_app -p viewer_net` -> PASS
- `cargo test -p viewer_core` -> PASS
- `cargo test -p viewer_app` -> PASS

## Live Validation
- Command: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`
- Artifacts:
  - `artifacts/logs/live_mesh_no_proxy_retest_2026-04-02_191011.log`
  - `artifacts/logs/network_debug_mesh_no_proxy_retest_2026-04-02_191011.jsonl`
- Result:
  - LLUDP object ingress was healthy (`lludp_object_gate: PASS`, object updates observed)
  - No `mesh_fetch: queued` lines in this bounded window (no mesh IDs promoted from current observed payloads)

## Result Status
- PASS for implementation scope.
- Live readiness remains conditional on mesh ID availability in decoded object stream.

## Risks / Follow-Up
- If mesh IDs are absent in current route packets, objects remain fallback cubes.
- Next unblock is richer/verified mesh-ID extraction for additional object-update layouts where available.

## Learnings Delta
- none
- Reason: behavior confirms existing known constraint (mesh promotion requires mesh IDs present in observed payloads).
