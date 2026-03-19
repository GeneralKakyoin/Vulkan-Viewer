---
name: plan-next-step
description: Determine the smallest correct next step for this viewer rewrite. Use when starting work, resuming work, or when the next milestone is unclear. Do not use for direct implementation.
---

## Read first
- AGENTS.md
- docs/MASTER_PLAN.md
- docs/CURRENT_STATE.md
- docs/HANDOFF.md
- docs/ARCHITECTURE.md
- docs/INTERFACES.md
- relevant docs/RESEARCH/*

## Goal
Figure out the current phase, blocker, and smallest next useful step without changing code.

## Do
1. Identify the current phase.
2. Identify what already works.
3. Identify the current blocker.
4. Identify the smallest next task that improves the project without widening scope.
5. List the files likely involved.
6. State what must not change.

## Output
Return exactly:
- current phase
- blocker
- smallest next step
- why this is next
- files likely involved
- boundaries to preserve

## Rules
- Do not implement.
- Do not broaden scope.
- Prefer the smallest viable next step.
- If the next step appears large, split it into a smaller one.
