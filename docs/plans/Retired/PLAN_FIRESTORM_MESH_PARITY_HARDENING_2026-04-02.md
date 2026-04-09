# Plan: Firestorm Mesh Parity Hardening (Compressed ExtraParams + Mesh Fetch Shaping) 2026-04-02

## Scope
- Implement structured `ObjectUpdateCompressed` field walking in `viewer_net` to locate and parse `ExtraParams` without speculative UUID scanning.
- Recover mesh asset IDs from compressed-update `ExtraParams` sculpt/mesh entries (`0x30` / `0x60`) when present.
- Align mesh URL selection and fetch behavior closer to Firestorm:
  - prefer Firestorm-style mesh URL shape (`/?mesh_id=` then `?mesh_id=`)
  - prefer capability ordering equivalent to Firestorm selection path (ViewerAsset, then GetMesh2, then GetMesh fallback)
  - add bounded mesh fetch request shaping with `Range` support fallback.
- Add/adjust unit tests for compressed mesh-ID decode and mesh URL selection.

## Current Known State
- Avoidable false-positive mesh scheduling from speculative compressed byte scans is already removed.
- Mesh rendering promotion path is present when decoded object-feed mesh IDs exist.
- Live runs still show `ViewerAsset mesh_id` probe `403` and often no mesh IDs from sampled object payloads.

## Files / Components Touched
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_grid/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- continuity/report docs for this slice.

## Boundary Check
- `viewer_net`: packet decode + HTTP fetch transport details only.
- `viewer_grid`: capability URL shaping policy.
- `viewer_app`: orchestration uses policy output; no protocol logic migration into app.

## Step Sequence
1. Add structured compressed payload walker in `viewer_net` up to `ExtraParams` and decode mesh ID via existing explicit parser.
2. Add compressed-update regression tests with and without mesh extra params.
3. Add mesh-specific URL candidate helper in `viewer_grid` that mirrors Firestorm query shaping and capability preference.
4. Update `viewer_app` mesh command path to use the mesh-specific helper.
5. Add bounded mesh fetch request shaping in `viewer_net` (`Accept` + `Range` first, fallback without range).
6. Run formatting, checks, targeted tests, and one bounded live validation capture.

## Validation Plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_grid -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_grid`
- `cargo test -p viewer_app`
- bounded live `cargo run -p viewer_app` with captured logs.

## Risks / Open Questions
- Compressed payload variants may differ by route/version; parser must fail-safe (no mesh ID) on unknown layouts.
- Range-first requests may still produce policy `403` for protected assets.

## Deferred Too Early Candidates
- Full Firestorm-equivalent multi-stage mesh header/body LOD byte-range loader in `viewer_net` (separate larger slice).

## Learnings Pre-check
- L63/L64: lane-level 4xx/5xx outcomes are informative; do not infer lane absence from non-2xx.
- L73: fixture mesh IDs remain required for deterministic transport verification.
- L74: RegionObjects mesh candidates are best-effort; LLUDP decode must remain a primary source.

## Completion Criteria
- Compressed object updates can produce mesh IDs from explicit extra params with tests.
- Mesh request URL/capability selection is Firestorm-aligned in policy and app call path.
- Mesh fetch path emits shaped range/non-range attempts and remains bounded.
- Validation commands pass; live capture artifact recorded.
