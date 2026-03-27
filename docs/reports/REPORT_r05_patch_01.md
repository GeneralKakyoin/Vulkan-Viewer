# Execution Report: R05 Patch 01 (Transparent Sort Tie-break)

## Summary
Aligned transparent draw ordering with `PLAN_R05` by changing the tie-break to use `instance_id` (no `geometry_key`) when `distance_sq` is equal.

## Files changed
- `crates/viewer_render/src/draw_helpers.rs`
- `docs/plans/PLAN_R05_PATCH_01.md`
- `docs/reviews/REVIEW_impl_r05.md`

## Validation run
- `cargo fmt`: PASS
- `cargo check`: PASS
- `cargo test -p viewer_core -p viewer_render`: PASS

## Result status
Complete.

## Risks / follow-ups
- None.
