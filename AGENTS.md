# AGENTS.md

This file is the canonical repository contract for coding agents working in this repo.
It is vendor-neutral. It defines repository law, workflow, continuity rules, validation, and safety rails.
Tool-specific setup lives outside this file.

## 1. Purpose

This repository exists to build and evolve the viewer in bounded, reviewable steps.
Agents must preserve repository continuity, architectural boundaries, and previous progress.

The default behavior is disciplined incremental progress, not opportunistic refactoring.

## 2. Authority and conflict handling

Treat this file as strong repository guidance.

If instructions conflict, use this precedence order:

1. Explicit user direction in the current task
2. This `AGENTS.md`
3. Approved task plan under `docs/plans/`
4. Nested repo guidance, if introduced later
5. Older historical notes or stale docs

If a conflict is material, warn the user, pause, and wait for direction.
Do not silently pick a side.

## 3. Read order before work

### Tiny task minimum
Read these before starting even a small scoped task:

1. `AGENTS.md`
2. `docs/CURRENT_STATE.md`
3. `docs/HANDOFF.md`
4. Relevant code and files for the task

### Normal task full path
For any non-trivial task, read in this order:

1. `AGENTS.md`
2. `docs/CURRENT_STATE.md`
3. `docs/HANDOFF.md`
4. `docs/ARCHITECTURE.md`
5. `docs/INTERFACES.md`
6. `docs/TASKS.md`
7. `docs/MASTER_PLAN.md`
8. `docs/LEARNINGS.md`
9. `docs/FIELD_GUIDE.md`
10. `docs/PLAN.md`
11. Relevant Firestorm and research docs
12. Relevant code

Do not ask the user questions that can be answered from these files.

## 4. Approved workflow

Meaningful implementation must follow this sequence:

1. Understand the current repo state
2. Create or update a written plan in `docs/plans/`
3. Capture any "valid but too early" feature candidates in `docs/plans/DEFERRED_FEATURES.md`
4. Review the plan before implementation
5. Implement only the approved plan
6. Validate the result with required checks
7. Write a completion report in `docs/reports/`
8. Update continuity docs
9. Perform mandatory `docs/LEARNINGS.md` delta review (add/update entry, or explicitly record why no durable learning was found)
10. Leave a precise handoff in `docs/HANDOFF.md`

Planning and implementation are separate phases.
Do not skip planning for meaningful work.

## 5. Roles

This repo uses role names, not model names.

- **Planner**: prepares the task plan
- **Plan reviewer**: reviews plan correctness, architecture fit, modularity, risks, and milestone alignment
- **Implementer**: executes the approved plan without expanding scope
- **Implementation reviewer**: checks the resulting change, validation quality, and boundary compliance
- **Integrator**: prepares the final branch, report, and continuity updates when needed

A single model may perform more than one role, but the roles remain conceptually separate.

## 6. Scope control

Implement only what the approved plan authorizes.

If a better approach is discovered mid-task, stop and request plan revision.
Do not silently widen scope.

Do not perform broad cleanup, opportunistic refactors, file moves, structural reorganization, or documentation renames unless one of these is true:

- the approved plan explicitly includes it
- it is required to complete the planned change
- it is required by a durable learning or hard repository invariant
- the user explicitly approves it

When ownership or location is non-obvious, consult docs/FIELD_GUIDE.md before asking the user or doing broad repo search.

## 7. Architecture and boundary discipline

Respect crate ownership and interface boundaries at all times.

If the next step would cross crate boundaries, change architecture, or introduce a materially different design path, stop and ask.

Do not use Firestorm as an architecture template.
Use it only as behavior and protocol reference.

When protocol behavior matters:

- cite the exact Firestorm reference used
- separate protocol research from implementation unless the task is explicitly small and contained
- do not make speculative protocol changes without repo or Firestorm evidence

## 8. Ambiguity rules

Pause and ask when any of the following is true:

- the task would change architecture or crate boundaries
- the plan is incomplete or contradictory
- the task requires destructive Git action
- the task requires dependency or system package changes not already approved
- there are two materially different approaches with different long-term consequences
- the user’s direction conflicts with repository law or the approved plan

Do not pause for minor local ambiguity that can be resolved from repo evidence.
Use a best-effort assumption and state it.

## 9. Editing model

Both configured tools may edit files, but not at the same time.
Assume only one active writer at a time.

Default operating pattern:

- planner: deeper review-oriented model
- reviewer: deeper review-oriented model
- writer: implementation-oriented model

Tool-specific mappings live outside this file.

## 10. Validation requirements

No change may be called complete without validation.

Default validation ladder:

- always run `cargo fmt`
- always run `cargo check`
- run targeted tests for touched crates or modules when applicable
- run broader `cargo test` when behavior changed materially or milestone scope warrants it
- run `cargo run -p viewer_app` when runtime, startup, UI, rendering, or end-to-end behavior changed

Use targeted checks early and workspace-wide checks before milestone completion when scope justifies it.

If validation is blocked, do not claim completion.
State exactly what was implemented, what was validated, what could not be validated, and why.

Every completion report must state exactly:

- what commands ran
- what passed
- what failed
- what remains unvalidated

## 11. Git safety rules

Protect repository progress.

Before any Git write action, inspect Git state.
Before any commit, confirm:

- current branch name
- changed files
- no unrelated files included

Never run destructive Git commands without explicit user approval.
This includes at minimum:

- `git reset --hard`
- `git clean -fd`
- forceful checkout over local work
- deleting branches
- rebasing published branches
- any pull strategy that can overwrite local work

Agents may create branches and commits.
They must not push, merge, or perform any remote-affecting action without explicit user approval.
Always prompt first.

Recommended branch naming:

- `ai/feat-<slug>`
- `ai/fix-<slug>`
- `ai/docs-<slug>`
- `ai/plan-<slug>`

Use Conventional Commits style for commit messages.

## 12. Dependency and environment changes

Installing repo dependencies or system packages requires approval unless the approved plan already authorizes it.
Document every dependency or environment change in the execution report.

## 13. Continuity documents

Continuity is mandatory.

### `docs/CURRENT_STATE.md`
Describe what is true now.
Keep history minimal.
A tiny "latest notable changes" section is allowed at the top.

### `docs/HANDOFF.md`
This is latest handoff only.
It must not become a running journal.
It should contain:

- what changed
- validation run
- exact current state
- exact next step
- blockers or risks

### `docs/LEARNINGS.md`
Store durable lessons, recurring traps, and known dead ends.
Do not use it for disposable task notes.
`LEARNINGS.md` interaction is mandatory for every meaningful task:

- planner/reviewer/implementer must check whether the task produced a durable lesson
- if yes, add or update a numbered learning entry
- if no, state "No durable learning identified" with a one-line reason in the review/report artifact

### `docs/TASKS.md`
Keep this focused on process guidance and active priorities.
Avoid long historical logs.

### `docs/plans/`
Store approved task plans and the canonical deferred-feature list (`DEFERRED_FEATURES.md`).

### `docs/reviews/`
Store plan reviews and implementation reviews.

### `docs/reports/`
Store execution reports as `REPORT_<slug>.md`.

The actor that makes the final material change must update continuity docs.
Review-only tasks that materially change repo understanding should also update continuity artifacts when relevant.

## 14. Output expectations

Plans, reviews, and reports must be explicit, structured, and concise.
Use markdown headings.
Do not use bloated narrative.

### Required for plans
A plan should include:

- scope
- current known state
- files and components touched
- boundary check
- step sequence
- validation plan
- risks and open questions
- deferred-too-early candidates captured (and synced to `docs/plans/DEFERRED_FEATURES.md` when present)
- learnings pre-check (which existing learning entries constrain this plan)
- exact completion criteria

### Required for reviews
A review should include:

- verdict
- architecture and boundary fit
- correctness concerns
- modularity and maintainability concerns
- validation adequacy
- risks and open questions
- learnings delta verdict (`add` / `update` / `none` with reason)
- required revisions or approval status

### Required for execution reports
A report should include:

- summary of implemented work
- files changed
- validation run
- result status
- risks or follow-up items
- learnings delta (`added` / `updated` / `none` with reason)
- continuity updates performed

## 15. Research rules

When repo evidence is insufficient, research before changing behavior.
For protocol work, consult the relevant Firestorm reference and cite the exact file used.

Do not invent protocol behavior.
Do not change protocol-sensitive code from vague memory.

## 16. Preferred operating behavior

Show useful progress early.
Do not falsely reassure the user.
Do not claim something works unless it was actually validated.
Do not keep asking questions already answerable from repo materials.
Do not optimize for speed at the expense of correctness, continuity, or architectural cleanliness.

## 17. What this file is not

This file does not define:

- current project phase
- current active milestone details
- tool-specific runtime settings
- provider-specific prompting syntax

Those belong elsewhere.
