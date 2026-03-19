---
name: debug-trace-analysis
description: Analyze LoginTrace or similar debug output to identify whether a failure is transport, codec, request-shape, or response-interpretation related. Use for login failures and protocol debugging.
---

## Read first
- AGENTS.md
- docs/CURRENT_STATE.md
- docs/HANDOFF.md
- relevant docs/RESEARCH/*
- the provided trace or debug output

## Goal
Turn raw trace output into the smallest justified next action.

## Do
1. Inspect:
   - initial request
   - redirect steps
   - final response
   - final interpreted result
2. Classify the failure:
   - transport
   - codec
   - request semantics
   - protocol sequencing
   - server-side auth/result
3. State confidence level.
4. Recommend the smallest next fix or investigation step.

## Output
Return:
- failure classification
- evidence
- confidence level
- smallest next step
- files likely involved

## Rules
- Prefer evidence over guesswork.
- Do not propose broad changes from one trace.
- Keep recommendations narrow.
