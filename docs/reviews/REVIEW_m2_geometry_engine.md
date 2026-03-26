# REVIEW_m2_geometry_engine

## Verdict

**Approved (M2 deliverables materially satisfied for “geometry engine groundwork + renderable integration”).**

## Architecture and boundary fit

- Keeps GPU handle ownership inside `viewer_render`.
- Keeps geometry generation and domain types inside `viewer_core` and `viewer_asset` (CPU-side).
- Restores `Scene::sync_spatial()` as the frame boundary, preventing silent drift between transforms, world matrices, and culling structures.

## Correctness concerns

- `Scene::sync_spatial()` previously derived AABBs only from transform scale; the new approach derives world AABBs from a local AABB transformed by the world matrix (needed for procedural/sculpt geometry and rotated instances).
- Dynamic geometry upload is now enabled; vertex layout is consistent across built-in diagnostic meshes and dynamic meshes.

## Modularity / maintainability concerns

- The dynamic-geometry preparation path in `viewer_app` is still a “groundwork” bridge and currently uses dummy inputs for sculpt/mesh until the asset pipeline is wired.

## Validation adequacy

- `cargo fmt`, `cargo check`, and `cargo test` were run and passed.
- App launch was exercised via `STRESS_TEST=2` (time-bounded in this session); manual visual validation remains recommended.

## Open questions / follow-ups

- When the live asset pipeline is introduced, decide where sculpt-map and mesh bytes are decoded/fetched (likely `viewer_asset`) and how they are provided to `viewer_app` without crossing crate boundaries.

