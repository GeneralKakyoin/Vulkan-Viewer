# Plan: Repo Crate-Local Editing Policy and Documentation Propagation (2026-04-04)

## Objective
Codify repository law that future changes should be implemented in the owning crate and the nearest behavior-focused subfile/module (creating new files/modules when needed), and propagate that policy through all high-signal docs used by future agents.

## Scope
- In scope:
  - Update repository contract and architecture/interface/process docs with explicit crate-local/subfile-local editing rules.
  - Update continuity docs so the policy is visible in latest state/handoff context.
  - Record durable learning for this policy.
- Out of scope:
  - Production code refactors.
  - Module splits in this task.
  - Runtime/protocol behavior changes.

## Current known state
- The codebase has very large monolithic files in multiple crates.
- A staged cross-crate split plan exists (`PLAN_CROSS_CRATE_BEHAVIOR_FUNCTION_SPLIT_2026-04-04.md`), but the policy is not yet fully embedded across canonical docs.
- Future agents primarily read `AGENTS.md`, `CURRENT_STATE.md`, and `HANDOFF.md` first; broader tasks add architecture/interface/task docs.

## Files and components touched
- `AGENTS.md`
- `docs/MASTER_PLAN.md`
- `docs/ARCHITECTURE.md`
- `docs/INTERFACES.md`
- `docs/TASKS.md`
- `docs/agents/CODEX.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/LEARNINGS.md`
- `docs/reviews/REVIEW_PLAN_REPO_CRATE_LOCAL_EDITING_POLICY_2026-04-04.md`
- `docs/reports/REPORT_REPO_CRATE_LOCAL_EDITING_POLICY_2026-04-04.md`

## Boundary check
- No crate ownership changes.
- No protocol behavior changes.
- No code-path edits.
- Firestorm remains behavior/protocol reference only; this policy is structural hygiene within this repo architecture.

## Step sequence
1. Add explicit crate-local/subfile-local editing law to `AGENTS.md`.
2. Mirror durable policy language in `MASTER_PLAN.md`.
3. Reinforce boundary-local implementation guidance in `ARCHITECTURE.md` and `INTERFACES.md`.
4. Add planning requirement in `TASKS.md` to include file-placement strategy.
5. Update `docs/agents/CODEX.md` for default writer behavior alignment.
6. Update continuity docs (`CURRENT_STATE.md`, `HANDOFF.md`) with the new policy as current repo truth.
7. Perform LEARNINGS delta review and add durable learning entry if warranted.
8. Validate with required checks and write completion report.

## Validation plan
- `cargo fmt --all`
- `cargo check`

## Risks and open questions
- Risk: wording could be interpreted as "never touch large files."
  - Mitigation: policy text explicitly allows bounded exceptions when required by approved plan.
- Risk: conflict with existing "do not use Firestorm as architecture template."
  - Mitigation: language states this is locality discipline only, not architecture borrowing.
- Open question: whether to enforce this via lint/tooling later.
  - Deferred for future process/tooling milestone.

## Deferred-too-early candidates captured
- No new deferred feature identified in this task.
- `docs/plans/DEFERRED_FEATURES.md` unchanged.

## Learnings pre-check
- Applicable learnings:
  - L06 (boundary collapse risk under pressure)
  - L70 (planning discipline before repeated churn)
- No conflict with existing learnings; this task strengthens them.

## Completion criteria
- Policy is explicitly present in all canonical planning/boundary docs used during agent startup/read order.
- `CURRENT_STATE.md` and `HANDOFF.md` reflect the policy as latest repo truth.
- `LEARNINGS.md` updated with durable lesson verdict (`added`/`updated`/`none`).
- Validation commands pass and completion report is written.
