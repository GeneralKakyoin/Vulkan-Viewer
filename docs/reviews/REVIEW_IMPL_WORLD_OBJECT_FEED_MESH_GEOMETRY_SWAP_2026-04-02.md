# Review: Implementation World Object Feed Mesh Geometry Swap 2026-04-02

## Verdict
Approved.

## Architecture And Boundary Fit
- `viewer_core` now maps decoded mesh IDs to mesh geometry for world-object feed instances.
- Fallback diagnostic behavior remains in `viewer_core` when mesh IDs are absent.

## Correctness Concerns
- Existing instances now update transform/color/geometry in-place and refresh local AABB when geometry changes.
- Seam contract gained additive `decoded_object_mesh_id` and adapter now forwards `obj.mesh_id`.

## Validation Adequacy
- `cargo fmt --all` PASS
- `cargo check -p viewer_core -p viewer_app -p viewer_net` PASS
- `cargo test -p viewer_core` PASS
- `cargo test -p viewer_app` PASS
- bounded live run executed with logs captured.

## Risks And Open Questions
- Live route still may not surface mesh IDs in observed object-update payloads, so promotion path can remain idle.
- Asset-host policy can still deny some mesh fetches with `403`.

## Learnings Delta Verdict
- none
- Reason: no new durable invariant beyond existing mesh-ID availability constraints.
