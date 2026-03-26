# REPORT_m2_geometry_engine

## Summary

Completed the missing M2 integration pieces so procedural/sculpt meshes can be uploaded and rendered as real dynamic geometry (instead of falling back to cubes), and restored correct per-frame spatial synchronization so world matrices and AABBs remain consistent with transforms.

Follow-up: fixed a bug in `LLVolume` circle-path extrusion that produced planar (flat) meshes by (a) correctly orienting the circle path in 3D and (b) treating integer-revolution circle paths as closed (no end caps, wrapped side stitching). Also fixed `PathPt` default rotation so twist can apply on line paths.

## Files changed

- `crates/viewer_core/src/lib.rs`
- `crates/viewer_render/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `crates/viewer_core/src/geometry/llvolume.rs`
- `Cargo.toml`
- `docs/plans/PLAN_M2.md`

## Validation run

Commands:
- `cargo fmt`
- `cargo check`
- `cargo test`
- `cargo test -p viewer_core`
- `$env:STRESS_TEST='2'; $env:VIEWER_APP_LIVE_STARTUP='off'; cargo run -p viewer_app`

Results:
- `cargo fmt`: pass
- `cargo check`: pass
- `cargo test`: pass
- `cargo test -p viewer_core`: pass
- `cargo run -p viewer_app`: app launched; run was time-bounded in this session and the process was terminated after launch.

## Result status

**Done**
- Dynamic geometry upload is enabled in the per-frame preparation path (procedural + sculpt) and is renderable by `viewer_render`.
- Scene spatial sync is active again each frame, and world AABBs are derived from per-instance local bounds transformed by the world matrix.

**Not addressed (still backlog / later milestones)**
- Asset-backed feeding of real sculpt-map pixels and real mesh bytes into `GeometrySource::Sculpt` / `GeometrySource::Mesh` from live world ingestion.
- LOD selection heuristics beyond the current groundwork.
- VRAM-budgeted eviction policies for dynamic GPU meshes.

## Risks / follow-ups

- If `GeometrySource::Mesh` is instantiated with empty data, dynamic upload is skipped and the renderer falls back to the diagnostic cube (expected until a real asset pipeline is wired).
- Manual visual validation is still recommended using `STRESS_TEST=2` to confirm the torture test renders procedurally generated geometry (not cubes).

## Continuity updates performed

- Updated `docs/plans/PLAN_M2.md` validation results to reflect executed validation in this session.
