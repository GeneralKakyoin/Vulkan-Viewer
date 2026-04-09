# Review: Implementation Object Feed Mesh ID Decode + Scheduling 2026-04-02

## Verdict
Approved.

## Architecture And Boundary Fit
- `viewer_net` owns new extra-param mesh decode and object-feed retention/export.
- `viewer_core` contract update is additive (`mesh_id: Option<String>`).
- `viewer_app` consumes the additive field for request scheduling without crossing crate ownership.

## Correctness Concerns
- Extra-param decode now recognizes sculpt/mesh IDs (`0x30`/`0x60`) with bounded length checks.
- Object-feed upsert/export preserves previously known values and emits normalized UUID strings.
- Scheduling path now unions fixture, visible scene meshes, and decoded object-feed mesh IDs.

## Modularity And Maintainability Concerns
- New helper extraction functions keep responsibilities clear.
- Tests cover parser behavior and deterministic capped extraction.

## Validation Adequacy
- Executed and passing:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_core -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_core`
  - `cargo test -p viewer_app`

## Risks And Open Questions
- Compressed update extra-param extraction is best-effort due variable packed layout.
- Capability permissions/runtime policy may still return `403` on valid mesh IDs.

## Learnings Delta Verdict
- none
- Reason: reinforces existing protocol-source anchoring; no new durable invariant identified.

## Required Revisions Or Approval Status
- Approved; no blocking revisions.
