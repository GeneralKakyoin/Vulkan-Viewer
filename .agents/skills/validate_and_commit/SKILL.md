---
name: validate-and-commit
description: Validate a finished change and close the loop with commit + continuity updates. Use after implementation is complete.
---

## Read first
- AGENTS.md
- docs/CURRENT_STATE.md
- docs/HANDOFF.md
- the summary of the completed change

## Goal
Ensure the change is validated, recorded, and recoverable.

## Do
1. Run:
   - cargo check
   - cargo test
2. If the task has a manual run path, note whether it was exercised.
3. Confirm continuity docs are updated if needed.
4. Stage changes.
5. Propose a clear commit message.

## Output
Return:
- validation status
- docs updated or not
- recommended commit message
- any remaining risk

## Rules
- Never skip validation.
- Never skip continuity updates if state changed.
- Do not commit unrelated files.
