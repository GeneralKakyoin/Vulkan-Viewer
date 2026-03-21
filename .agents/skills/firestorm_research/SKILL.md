---
name: firestorm-research
description: Extract behavior from Firestorm safely and translate it into clean Rust-facing guidance. Use for login, bootstrap, capability, and compatibility questions.
---

## Read first
- AGENTS.md
- docs/FIRESTORM_REFERENCE.md
- docs/MASTER_PLAN.md
- docs/CURRENT_STATE.md
- existing relevant docs/RESEARCH/*

## Goal
Use Firestorm to understand behavior, not to borrow architecture.

## Do
1. Define the exact question being answered.
2. Identify the smallest relevant Firestorm file set.
3. Extract:
   - sequence
   - field expectations
   - state transitions
   - special cases
4. Write or update a note in docs/RESEARCH.
5. Map findings into this project’s crate boundaries.

## Output
Return:
- question investigated
- relevant Firestorm files
- behavior summary
- crate-boundary mapping
- research note updated

## Rules
- Do not copy code.
- Do not mirror Firestorm architecture.
- Translate behavior into clean Rust abstractions.
