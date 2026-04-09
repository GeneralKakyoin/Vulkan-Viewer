# Plan: Backfill Missing Impl Reviews and Retire Completed Dated Plans (2026-04-09)

## Objective
For completed dated tactical plans that have reports but lack implementation-review artifacts, backfill missing `REVIEW_IMPL_*` files and retire the plans into `docs/plans/Retired/`.

## Scope
- Target only `docs/plans/PLAN_*_YYYY-MM-DD.md` files where:
  - matching `docs/reports/REPORT_<slug>.md` exists
  - matching `docs/reviews/REVIEW_IMPL_<slug>.md` is missing
- Create the missing implementation-review files.
- Move the corresponding completed plans to `docs/plans/Retired/`.
- Update continuity docs and write a completion report.

## Current known state
- Several completed dated plans remain in `docs/plans/` root because implementation-review artifacts are missing.
- User requested retiring plans that are done but missing artifact coverage.

## Files and components touched
- `docs/plans/PLAN_BACKFILL_IMPL_REVIEW_AND_RETIRE_COMPLETED_DATED_PLANS_2026-04-09.md` (new)
- `docs/reviews/REVIEW_PLAN_BACKFILL_IMPL_REVIEW_AND_RETIRE_COMPLETED_DATED_PLANS_2026-04-09.md` (new)
- Multiple `docs/reviews/REVIEW_IMPL_<slug>.md` files (new, backfill)
- Multiple `docs/plans/PLAN_*_YYYY-MM-DD.md` moved to `docs/plans/Retired/`
- `docs/reviews/REVIEW_IMPL_BACKFILL_IMPL_REVIEW_AND_RETIRE_COMPLETED_DATED_PLANS_2026-04-09.md` (new)
- `docs/reports/REPORT_BACKFILL_IMPL_REVIEW_AND_RETIRE_COMPLETED_DATED_PLANS_2026-04-09.md` (new)
- `docs/CURRENT_STATE.md` (modify)
- `docs/HANDOFF.md` (replace latest)

## Boundary check
- Documentation/process-only action.
- No runtime, crate, or protocol behavior changes.

## Step sequence
1. Compute eligible dated-plan candidates using report/missing-impl-review gate.
2. Create missing `REVIEW_IMPL_*` for those candidates.
3. Move eligible plans to `docs/plans/Retired/`.
4. Add task implementation-review and report artifacts.
5. Update continuity docs and run required validation.

## Validation plan
- `cargo fmt --all`
- `cargo check --workspace`

## Risks and open questions
- Some docs may still reference old root plan paths after retirement.

## Deferred-too-early candidates captured
- None.

## Learnings pre-check
- L82 applies as process context; no new code-locality change is introduced.

## Completion criteria
- All eligible dated plans with report+missing-impl-review have backfilled `REVIEW_IMPL_*` files.
- Those plans are moved to `docs/plans/Retired/`.
- Continuity docs and report reflect exactly what moved and why.
- Validation passes.
