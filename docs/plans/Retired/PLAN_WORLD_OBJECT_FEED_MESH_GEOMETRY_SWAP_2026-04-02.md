# Plan: World Object Feed Mesh Geometry Swap (No Proxy Cubes When Mesh IDs Exist) 2026-04-02

## Scope
- Extend world object ingestion seam to carry decoded object-feed mesh IDs.
- Switch `viewer_core` world-object feed instance geometry from diagnostic cube to `GeometrySource::Mesh(mesh_id, 0)` when mesh ID is present.
- Preserve fallback diagnostic cube when mesh ID is absent.
- Add regression coverage for geometry promotion/demotion behavior.
- Run bounded live validation and capture evidence.

## Boundary Check
- `viewer_core` scene ownership only for geometry role/transform updates.
- `viewer_app`/`viewer_net` behavior unchanged except consumption of existing decoded fields.

## Validation Plan
- `cargo fmt --all`
- `cargo check -p viewer_core -p viewer_app -p viewer_net`
- `cargo test -p viewer_core`
- `cargo test -p viewer_app`
- bounded live `cargo run -p viewer_app` with log capture
