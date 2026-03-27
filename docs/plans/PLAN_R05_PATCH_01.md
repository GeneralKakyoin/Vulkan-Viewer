# Plan: R05 Patch 01 (Transparent Sort Tie-break)

## Scope
Align `viewer_render` transparent draw ordering with `docs/plans/PLAN_R05.md`:
- `transparent`: `(distance_sq desc, instance_id asc)`

No other sorting rule changes, no pipeline changes, no contract changes.

## Current known state
- `crates/viewer_render/src/draw_helpers.rs` sorts transparent as:
  - `(distance_sq desc, geometry_key asc, instance_id asc)`
- `PLAN_R05` specifies transparent should tie-break by `instance_id` directly (no `geometry_key`).

## Files touched
- `crates/viewer_render/src/draw_helpers.rs`
- `docs/reviews/REVIEW_impl_r05.md` (update verdict/notes after fix)
- `docs/HANDOFF.md` (optional: note patch if needed)
- `docs/reports/REPORT_r05_patch_01.md` (execution report)

## Boundary check
- No crate boundary changes.
- `viewer_render` remains the sole owner of draw submission ordering policy.

## Step sequence
1. Update transparent sort to `(distance_sq desc, instance_id asc)`.
2. Add/adjust a unit test to cover the transparent tie-break when distances are equal.
3. Validate with `fmt`/`check`/targeted tests.
4. Update review/handoff/report.

## Validation plan
- `cargo fmt`
- `cargo check`
- `cargo test -p viewer_core -p viewer_render`

## Risks / open questions
- None expected (deterministic ordering tweak only).

## Completion criteria
- Code matches plan’s transparent ordering tie-break.
- Targeted tests cover the tie-break and pass.
