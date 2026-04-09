# Plan: Firestorm Object Decode Fail-Soft Parity (2026-04-03)

## Scope
- Harden LLUDP object mesh-ID decode so malformed/truncated object blocks do not invalidate valid neighboring objects in the same packet.
- Keep behavior aligned with Firestorm's object-by-object tolerance model.

## Current Known State
- `ObjectUpdateCompressed`, `ObjectExtraParams`, and extra-param mesh decode already exist.
- Prior decode paths could fail whole-packet extraction on malformed segments, reducing mesh-ID discovery reliability.

## Files / Components Touched
- `crates/viewer_net/src/lib.rs`
- Continuity artifacts for this slice (`docs/reviews/`, `docs/reports/`, `docs/CURRENT_STATE.md`, `docs/HANDOFF.md`)

## Boundary Check
- Protocol decode behavior remains in `viewer_net`.
- No crate-boundary changes or architecture movement.

## Step Sequence
1. Make `decode_mesh_asset_id_from_extra_params(...)` fail-soft on trailing/truncated data.
2. Make `decode_object_extra_params_mesh_updates(...)` preserve valid entries when some entries are malformed.
3. Make `decode_object_update_compressed_objects(...)` preserve valid blocks when later blocks are malformed/truncated.
4. Add regression tests for each fail-soft path.
5. Run `cargo fmt`, `cargo check`, and targeted/full `viewer_net` tests.

## Validation Plan
- `cargo fmt --all`
- `cargo check -p viewer_net`
- `cargo test -p viewer_net decode_mesh_asset_id_from_extra_params_tolerates_trailing_bytes -- --nocapture`
- `cargo test -p viewer_net decode_object_update_compressed_tolerates_mixed_valid_and_malformed_blocks -- --nocapture`
- `cargo test -p viewer_net decode_object_extra_params_tolerates_invalid_entry_and_keeps_valid_following_entry -- --nocapture`
- `cargo test -p viewer_net`

## Risks / Open Questions
- Fail-soft parsing can mask malformed tail bytes; diagnostics still need explicit malformed-packet visibility if later triage requires it.
- This does not itself guarantee live mesh IDs; upstream packet availability and access policy still apply.

## Deferred Too Early Candidates
- None for this bounded slice.

## Learnings Pre-check
- L74: RegionObjects mesh candidates are best-effort only.
- L73: deterministic forcing is still needed for transport-lane verification.
- L77: startup receive/ACK timing can be decisive for object ingress before decode improvements matter.

## Completion Criteria
- Decoder preserves valid mesh-ID extraction in mixed valid+malformed payloads.
- New regression tests pass.
- Required validation commands pass.
