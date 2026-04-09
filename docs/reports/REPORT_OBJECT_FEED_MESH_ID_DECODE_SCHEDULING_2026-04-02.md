# REPORT: Object Feed Mesh ID Decode + Scheduling (Firestorm + OpenSim) 2026-04-02

## Summary Of Implemented Work
- Added mesh asset tracking to decoded object feed in `viewer_net`:
  - `DecodedObjectFeedObject` now exports optional `mesh_id`.
  - Internal object-feed state now retains optional `mesh_id_bytes`.
- Implemented extra-param mesh decode aligned to Firestorm/OpenSim constants:
  - parses ExtraParams entry stream (`count`, repeated `type/in_use/len/data`).
  - supports sculpt and mesh param IDs (`0x30`, `0x60`).
  - accepts sculpt payload as mesh only when sculpt type indicates mesh.
- Wired decoded mesh IDs through app/core contracts:
  - `viewer_core::DecodedWorldObjectFeedObject` now includes optional `mesh_id`.
  - `viewer_app` maps `viewer_net` object-feed `mesh_id` into core snapshot objects.
- Updated live mesh scheduling:
  - `tick_scene_meshes` now unions fixture meshes, visible scene mesh geometry, and decoded object-feed mesh IDs.
  - added deterministic capped helper extraction for decoded object-feed mesh IDs.
- Added tests:
  - `viewer_net`: extra-param sculpt-mesh decode + object-feed mesh export.
  - `viewer_app`: deterministic/capped decoded mesh-ID extraction.

## Firestorm/OpenSim References Used
- `reference/firestorm/indra/llprimitive/llprimitive.h`
- `reference/firestorm/indra/llprimitive/llprimitive.cpp`
- `reference/opensimulator/nebadon2025-opensimulator/OpenSim/Framework/PrimitiveBaseShape.cs`
- `reference/opensimulator/nebadon2025-opensimulator/OpenSim/Region/ClientStack/Linden/UDP/LLClientView.cs`

## Files Changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_core/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_OBJECT_FEED_MESH_ID_DECODE_SCHEDULING_2026-04-02.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_FEED_MESH_ID_DECODE_SCHEDULING_2026-04-02.md`
- `docs/reviews/REVIEW_IMPL_OBJECT_FEED_MESH_ID_DECODE_SCHEDULING_2026-04-02.md`
- `docs/reports/REPORT_OBJECT_FEED_MESH_ID_DECODE_SCHEDULING_2026-04-02.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`

## Validation Run
- `cargo fmt --all` -> PASS
- `cargo check -p viewer_net -p viewer_core -p viewer_app` -> PASS
- `cargo test -p viewer_net` -> PASS
- `cargo test -p viewer_core` -> PASS
- `cargo test -p viewer_app` -> PASS

## Result Status
- PASS (scoped mesh decode/scheduling implementation complete)

## Risks Or Follow-Up Items
- Compressed object-update extra-param extraction remains best-effort because compressed packed layout is flag-dependent.
- Live capability/runtime policy can still block mesh bytes (`403`) despite correct mesh ID decode/scheduling.

## Learnings Delta
- none
- Reason: this slice reinforced existing protocol parsing discipline; no new durable invariant emerged.

## Continuity Updates Performed
- Updated `docs/CURRENT_STATE.md` with new mesh decode/scheduling status.
- Replaced `docs/HANDOFF.md` with latest handoff for this completed slice.
