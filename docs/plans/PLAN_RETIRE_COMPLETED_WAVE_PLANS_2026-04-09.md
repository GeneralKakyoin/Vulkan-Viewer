# Plan: Retire Completed Wave Plans (2026-04-09)

## Objective
Retire completed Wave-series helper extraction plans by moving them from `docs/plans/` to `docs/plans/Retired/`, while preserving continuity and documenting the retirement action.

## Scope
- Move completed Wave plan files (Wave 1 through Wave 9) and the associated Wave 8/9 closure plan into `docs/plans/Retired/`.
- Update continuity docs to record the retirement action and point to the unchanged functional next step.
- Produce an execution report and implementation review for traceability.

## Current known state
- Wave 1 through Wave 9 extraction work is already completed and has corresponding report/review artifacts.
- Plans currently remain in `docs/plans/` root and are not yet retired.

## Files and components touched
- `docs/plans/PLAN_RETIRE_COMPLETED_WAVE_PLANS_2026-04-09.md` (new)
- `docs/reviews/REVIEW_PLAN_RETIRE_COMPLETED_WAVE_PLANS_2026-04-09.md` (new)
- Move completed wave plans to `docs/plans/Retired/`
- `docs/reviews/REVIEW_IMPL_RETIRE_COMPLETED_WAVE_PLANS_2026-04-09.md` (new)
- `docs/reports/REPORT_RETIRE_COMPLETED_WAVE_PLANS_2026-04-09.md` (new)
- `docs/CURRENT_STATE.md` (modify)
- `docs/HANDOFF.md` (replace latest handoff)

## Boundary check
- Documentation/process-only operation.
- No runtime or crate-boundary behavior changes.

## Step sequence
1. Add plan + plan review artifact for this retirement task.
2. Move completed Wave plan files into `docs/plans/Retired/`.
3. Add implementation review + report.
4. Update `CURRENT_STATE.md` and `HANDOFF.md`.
5. Run `cargo fmt --all` and `cargo check --workspace`.

## Validation plan
- `cargo fmt --all`
- `cargo check --workspace`

## Risks and open questions
- Existing documents may reference old root-plan paths; retirement action should be clearly documented in continuity artifacts.

## Deferred-too-early candidates captured
- None for this docs/process retirement task.

## Learnings pre-check
- L82 applies as process context (crate-local placement discipline), but no new code placement changes are introduced.

## Completion criteria
- Selected completed Wave plans are present in `docs/plans/Retired/` and absent from `docs/plans/` root.
- Continuity docs and report/review artifacts capture the move and next step.
- Validation commands pass.
