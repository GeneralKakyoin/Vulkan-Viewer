# REPORT: Testing Reference Compendium (Flags, Modes, and Commands)

## Summary of implemented work

- Added `docs/TESTING_REFERENCE.md` as the canonical, model-friendly compendium of:
  - validation commands used as “tests” (`cargo fmt`, `cargo check`, targeted `cargo test`, runtime smoke)
  - deterministic runtime verification modes (`STRESS_TEST=...`) and tuning knobs
  - supported `VIEWER_*` env vars that act as test/verification controls
- Updated documentation to point to the new canonical reference:
  - `README.md`
  - `docs/TASKS.md`
  - `docs/FIELD_GUIDE.md`
  - `docs/CURRENT_STATE.md`
- Fixed a `cargo check` build break in `viewer_app` by updating `LiveVisualSnapshot` initializers to include newly-added fields.

## Files changed

- `docs/TESTING_REFERENCE.md`
- `README.md`
- `docs/TASKS.md`
- `docs/MASTER_PLAN.md`
- `docs/CURRENT_STATE.md`
- `docs/FIELD_GUIDE.md`
- `docs/plans/PLAN_testing_reference_compendium.md`
- `docs/reviews/REVIEW_plan_testing_reference_compendium.md`
- `docs/reports/REPORT_testing_reference_compendium.md`
- `crates/viewer_app/src/main.rs`

## Validation run

Commands run:

- `cargo fmt`
- `cargo check`

Results:

- Passed:
  - `cargo fmt`
  - `cargo check` (with existing non-blocking `wgpu` deprecation warnings in `viewer_render`)
- Failed:
  - None in final validation set
- Remains unvalidated:
  - No runtime smoke was run for this docs-focused change

## Result status

Complete for scoped plan.

## Risks or follow-up items

- `wgpu` deprecated copy type aliases remain in `viewer_render` (already tracked in handoffs/reports; non-blocking).
- The compendium can drift if new env vars/test modes are added without updating `docs/TESTING_REFERENCE.md`.

## Continuity updates performed

- Added plan + plan review artifacts for this docs task.
- Updated `docs/CURRENT_STATE.md`.
- Updated `docs/HANDOFF.md`.

