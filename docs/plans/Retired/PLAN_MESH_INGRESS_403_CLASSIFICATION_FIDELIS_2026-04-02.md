# Plan: Mesh Ingress and 403 Classification Hardening (Fidelis Route) 2026-04-02

## Scope
- Add LLUDP `ObjectExtraParams` decode support in `viewer_net` and feed decoded mesh IDs into object-feed state.
- Improve mesh fetch failure classification in `viewer_app` so 403 causes are bucketed with actionable detail.
- Keep mesh request shaping/cap order from prior Firestorm parity slice; no architecture expansion.
- Run bounded live validation at `secondlife://Fidelis/51/72/24` targeting first `mesh_fetch: ready` evidence path.

## Current Known State
- Compressed object updates now parse explicit `ExtraParams` path for mesh IDs.
- Firestorm-style mesh URL policy and range-first fetch fallback are implemented.
- Live runs still show `ViewerAsset mesh_id` probes at 403 and no in-window `mesh_fetch` queue events.

## Files / Components Touched
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- continuity/report docs for this slice.

## Boundary Check
- `viewer_net`: protocol decode + transport error surface.
- `viewer_app`: orchestration/relay classification only.
- No crate-boundary redesign.

## Step Sequence
1. Implement LLUDP `ObjectExtraParams` packet decode to extract local_id + mesh_id from explicit extra-param payload.
2. Integrate decoded `ObjectExtraParams` events into object-feed upsert path.
3. Add tests for `ObjectExtraParams` decode and object-feed mesh-id promotion.
4. Add mesh fetch error bucketing helper in `viewer_app` for 403 body classes.
5. Run fmt/check/tests and bounded live runs on Fidelis route.

## Validation Plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- bounded live `cargo run -p viewer_app` with Fidelis start and captured logs.

## Risks / Open Questions
- `ObjectExtraParams` may arrive sparsely depending on simulator/state changes.
- 403s may remain access-policy bounded even with better ID ingress.

## Deferred Too Early Candidates
- Full packet replay harness and full multi-stage mesh LOD transfer policy remain deferred.

## Learnings Pre-check
- L73: use deterministic fixture/candidate forcing for mesh-lane verification.
- L74: RegionObjects mesh candidates are best-effort only.

## Completion Criteria
- `ObjectExtraParams` decode path exists with tests and updates object-feed mesh IDs.
- Mesh 403 classifications are explicit in runtime relays/logs.
- Fidelis bounded run artifacts captured with updated diagnostics.
