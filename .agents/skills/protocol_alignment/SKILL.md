---
name: protocol-alignment
description: Align login, bootstrap, or protocol behavior with Second Life / OpenSim expectations using Firestorm as behavior reference only. Use for request fields, response meaning, payload shape, redirect behavior, or compatibility debugging.
---

## Read first
- AGENTS.md
- docs/CURRENT_STATE.md
- docs/HANDOFF.md
- docs/FIRESTORM_REFERENCE.md
- relevant docs/RESEARCH/*
- relevant viewer_net and viewer_grid code

## Goal
Find the smallest compatibility correction without changing project architecture.

## Do
1. Identify whether the mismatch is:
   - transport
   - codec
   - request shape
   - response interpretation
   - sequencing
2. Use Firestorm and documented SL behavior as behavioral reference only.
3. Propose the smallest likely-correct fix.
4. Implement only if the change is narrow and low-risk.
5. Add or update focused tests.
6. Update docs/RESEARCH if understanding changed.

## Output
Return:
- mismatch identified
- evidence used
- smallest correction made or proposed
- tests added/updated
- what remains uncertain

## Rules
- Never copy Firestorm code.
- Never collapse viewer_net and viewer_grid together.
- Do not start simulator work unless the current phase explicitly allows it.
