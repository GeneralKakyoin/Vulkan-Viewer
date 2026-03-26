# TASKS.md

A living task guide for the viewer project.

This file has two jobs only:
1. define how tasks should be planned and executed
2. record the current active priorities

Do not use this file as a milestone history log.
Do not duplicate `CURRENT_STATE.md`, `HANDOFF.md`, or the rendering roadmap in full.

See:
- `AGENTS.md` for repository law
- `docs/CURRENT_STATE.md` for present truth
- `docs/HANDOFF.md` for the latest exact next step
- `docs/ARCHITECTURE.md` and `docs/INTERFACES.md` for boundaries
- `docs/FIELD_GUIDE.md` for non-obvious ownership and lookup points
- `docs/LEARNINGS.md` for durable lessons and dead ends

---

## HOW TO PLAN A TASK

For any meaningful task:

1. **Read the required docs first.**
   - Tiny task minimum:
     - `AGENTS.md`
     - `docs/CURRENT_STATE.md`
     - `docs/HANDOFF.md`
     - relevant code/files
   - Normal task full path:
     - `AGENTS.md`
     - `docs/CURRENT_STATE.md`
     - `docs/HANDOFF.md`
     - `docs/ARCHITECTURE.md`
     - `docs/INTERFACES.md`
     - `docs/TASKS.md`
     - `docs/MASTER_PLAN.md`
     - `docs/LEARNINGS.md`
     - `docs/FIELD_GUIDE.md`
     - `docs/PLAN.md`
     - relevant Firestorm/reference docs
     - relevant code

2. **Identify the owning crate(s).**
   Name them explicitly before writing the plan.
   If ownership is unclear, check `ARCHITECTURE.md`, `INTERFACES.md`, and `FIELD_GUIDE.md` before asking the user.

3. **Research before asking.**
   Do not ask questions that are answerable from repo docs or code.
   Ask only when the issue is a real tradeoff, product intent question, or architecture fork.

4. **Check the task against current state.**
   Plans must align with:
   - current repo truth in `CURRENT_STATE.md`
   - latest next step in `HANDOFF.md`
   - active priorities in this file
   - current rendering roadmap state, if the task is on the rendering track

5. **Write the plan to `docs/plans/`.**
   A good plan is decision-complete and implementation-ready.
   The implementer should not have to invent scope, boundaries, or validation.

6. **Review the plan before implementation.**
   Store plan reviews in `docs/reviews/`.

7. **Get user sign-off before implementation.**
   Do not begin meaningful implementation on an unapproved plan.

---

## TASK PLAN FORMAT

Use this structure:

```md
# Plan: <title>

## Objective
## Scope
## Current known state
## Files and components touched
## Boundary check
## Step sequence
## Validation plan
## Risks and open questions
## Completion criteria
