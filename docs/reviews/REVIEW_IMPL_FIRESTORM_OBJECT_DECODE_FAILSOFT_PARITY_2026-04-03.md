# Review: Implementation Firestorm Object Decode Fail-Soft Parity (2026-04-03)

## Verdict
- Approved.

## Architecture / Boundary Fit
- Implementation remains fully inside `viewer_net` decode/test scope.
- No boundary violations across `viewer_app`, `viewer_core`, `viewer_grid`, or UI crates.

## Correctness Concerns
- `decode_mesh_asset_id_from_extra_params(...)` now preserves decoded mesh IDs even if trailing bytes are malformed.
- `decode_object_extra_params_mesh_updates(...)` now skips malformed entries and keeps valid following entries.
- `decode_object_update_compressed_objects(...)` now keeps valid decoded objects and tolerates malformed/truncated subsequent blocks.

## Modularity / Maintainability
- Added explicit regression tests for each tolerance path.
- Existing decode semantics (message gating and UUID filtering) are preserved.

## Validation Adequacy
- `cargo fmt --all` PASS
- `cargo check -p viewer_net` PASS
- targeted fail-soft tests PASS
- `cargo test -p viewer_net` PASS

## Risks / Open Questions
- Runtime mesh fetch can still be blocked by upstream capability policy (`403`) or lack of mesh IDs in captured window.

## Learnings Delta Verdict
- none — no new durable rule beyond already captured startup/mesh-ingress learnings.

## Approval Status
- Implementation accepted for this slice.
