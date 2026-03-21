---
name: implement-feature
description: Implement a bounded feature or fix while preserving crate boundaries and current project phase. Use for narrow coding tasks with a clear goal.
---

## Read first
- AGENTS.md
- docs/CURRENT_STATE.md
- docs/HANDOFF.md
- docs/ARCHITECTURE.md
- docs/INTERFACES.md
- docs/TASKS.md if relevant

## Goal
Implement a small, reviewable change in the correct crate with minimal collateral changes.

## Do
1. Confirm the owning crate.
2. Confirm what must not change.
3. Implement the smallest working change.
4. Add or update focused tests if appropriate.
5. Validate with:
   - cargo check
   - cargo test
6. If the project state changed materially, update:
   - docs/CURRENT_STATE.md
   - docs/HANDOFF.md

## Output
Return:
- what changed
- files changed
- validation performed
- any assumptions
- whether continuity docs were updated

## Rules
- No broad refactors.
- No silent boundary changes.
- No unrelated cleanup.
- Keep diffs narrow and reversible.
