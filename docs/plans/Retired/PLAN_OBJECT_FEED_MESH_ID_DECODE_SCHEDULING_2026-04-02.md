# Plan: Object Feed Mesh ID Decode + Scheduling Parity (Firestorm + OpenSim) 2026-04-02

## Scope
- Decode mesh asset UUIDs from LLUDP object update extra params in `viewer_net`.
- Retain/export mesh IDs in object-feed summaries.
- Propagate mesh IDs through `viewer_app` -> `viewer_core` snapshot mapping.
- Include decoded object-feed mesh IDs in `tick_scene_meshes` scheduling so live mesh fetch requests can be issued from decoded object payloads.
- Add targeted tests for extra-param mesh decode and scheduling input extraction.

## Current Known State
- Mesh fetch pipeline (`RequestMesh`, URL candidates, `fetch_mesh_asset_bytes`) exists.
- Object feed currently carried local id/scale/position/object UUID, but not mesh UUID.
- RegionObjects candidate extraction is present but not sufficient as sole live source.

## Files And Components Touched
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_core/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/reviews/REVIEW_PLAN_OBJECT_FEED_MESH_ID_DECODE_SCHEDULING_2026-04-02.md`
- `docs/reviews/REVIEW_IMPL_OBJECT_FEED_MESH_ID_DECODE_SCHEDULING_2026-04-02.md`
- `docs/reports/REPORT_OBJECT_FEED_MESH_ID_DECODE_SCHEDULING_2026-04-02.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`

## Boundary Check
- `viewer_net`: protocol decode/state only.
- `viewer_app`: mapping + request scheduling only.
- `viewer_core`: additive snapshot contract field only.
- No `viewer_grid` behavior changes.

## Firestorm/OpenSim References
- `reference/firestorm/indra/llprimitive/llprimitive.h` (`PARAMS_SCULPT = 0x30`)
- `reference/firestorm/indra/llprimitive/llprimitive.cpp` (extra params handling)
- `reference/opensimulator/nebadon2025-opensimulator/OpenSim/Framework/PrimitiveBaseShape.cs` (`SculptEP=0x30`, `MeshEP=0x60`, `MeshFlagsEP=0x70`, extra-param read path)
- `reference/opensimulator/nebadon2025-opensimulator/OpenSim/Region/ClientStack/Linden/UDP/LLClientView.cs` (object update block includes `ExtraParams`)

## Validation Plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_core -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_core`
- `cargo test -p viewer_app`

## Risks And Open Questions
- Compressed object update payloads are format-variable; extra-param extraction there is best-effort scan-based in this slice.
- Live mesh capability access may still return non-retryable HTTP statuses (e.g., 403) independent of decode correctness.

## Deferred-Too-Early Candidates
- None for this bounded slice.

## Learnings Pre-Check
- L10/L15 protocol-source anchoring is required; no speculative packet constants.

## Completion Criteria
- Object-feed export includes optional `mesh_id` where present.
- `tick_scene_meshes` includes decoded object-feed mesh IDs.
- Validation ladder passes for touched crates.
