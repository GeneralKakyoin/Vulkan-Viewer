# Agent Runbook

This file explains how to run the role-based workflow for this repo.
It complements `AGENTS.md`.

## Default operating loop

1. Repo scan / context collection
2. Planner writes a plan in `docs/plans/`
3. Plan reviewer writes a review in `docs/reviews/`
4. Implementer executes the approved plan
5. Implementation reviewer writes a review in `docs/reviews/`
6. Implementer or integrator writes `docs/reports/REPORT_<slug>.md`
7. Final actor updates continuity docs and `docs/HANDOFF.md`
8. use FIELD_GUIDE.md when locating non-obvious ownership points, hidden boundaries, or hard-to-grep logic


## Default role mapping

- Fast scan / repo summarization: fast model
- Planning: deeper review-oriented model
- Plan review: deeper review-oriented model
- Implementation: implementation-oriented model
- Implementation review: deeper review-oriented model by default

## Escalation ladder

- Small, local, low-risk task: implementation-oriented model may handle directly after reading required docs
- Medium multi-file task: planner -> plan review -> implement -> review
- Architectural or protocol-sensitive task: deep planning required, explicit boundary check required, Firestorm evidence required when relevant

## Artifact locations

- Plans: `docs/plans/`
- Reviews: `docs/reviews/`
- Reports: `docs/reports/`
- Tool setup: `docs/agents/`

## Push and merge policy

- Branch creation allowed
- Local commits allowed
- Push requires explicit user approval
- Merge requires explicit user approval
- Any remote-affecting action requires explicit prompt to the user
