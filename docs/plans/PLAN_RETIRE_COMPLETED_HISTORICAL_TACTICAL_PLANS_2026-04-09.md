# Plan: Retire Completed Historical Tactical Plans (2026-04-09)

## Objective
Retire previously completed tactical plans (including object-ingress branches) from `docs/plans/` root into `docs/plans/Retired/` using completion evidence gates.

## Scope
- Select dated tactical plans in `docs/plans/` root where both implementation artifacts exist:
  - `docs/reports/REPORT_<slug>.md`
  - `docs/reviews/REVIEW_IMPL_<slug>.md`
- Move eligible plans into `docs/plans/Retired/`.
- Update continuity docs and produce task review/report artifacts.

## Current known state
- Many historical tactical plans remain in `docs/plans/` despite completed report/review evidence.
- User explicitly requested prior completed plans be retired, citing object-ingress plans.

## Files and components touched
- `docs/plans/PLAN_RETIRE_COMPLETED_HISTORICAL_TACTICAL_PLANS_2026-04-09.md` (new)
- `docs/reviews/REVIEW_PLAN_RETIRE_COMPLETED_HISTORICAL_TACTICAL_PLANS_2026-04-09.md` (new)
- Multiple `docs/plans/PLAN_*_YYYY-MM-DD.md` files moved to `docs/plans/Retired/`
- `docs/reviews/REVIEW_IMPL_RETIRE_COMPLETED_HISTORICAL_TACTICAL_PLANS_2026-04-09.md` (new)
- `docs/reports/REPORT_RETIRE_COMPLETED_HISTORICAL_TACTICAL_PLANS_2026-04-09.md` (new)
- `docs/CURRENT_STATE.md` (modify)
- `docs/HANDOFF.md` (replace latest handoff)

## Boundary check
- Documentation/process-only move.
- No runtime behavior or crate-boundary changes.

## Step sequence
1. Add plan + plan-review artifacts for this retirement pass.
2. Compute eligible tactical plans using report+implementation-review evidence.
3. Move eligible plans into `docs/plans/Retired/`.
4. Add implementation-review + report artifacts.
5. Update continuity docs and validate.

## Validation plan
- `cargo fmt --all`
- `cargo check --workspace`

## Risks and open questions
- Historical docs may still reference root plan paths; continuity docs will record retirement-path lookup.

## Deferred-too-early candidates captured
- None (process-only task).

## Learnings pre-check
- L82 process/locality discipline applies as context only; no code changes introduced.

## Completion criteria
- Eligible completed tactical plans (including completed object-ingress plans) are moved to `docs/plans/Retired/`.
- Continuity and report/review artifacts reflect the move and unchanged functional next step.
- Validation commands pass.
