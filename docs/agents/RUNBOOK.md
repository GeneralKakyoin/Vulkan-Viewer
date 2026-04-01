# Agent Runbook

This file explains how to run the role-based workflow for this repo.
It complements `AGENTS.md`.

## Default operating loop

1. Repo scan / context collection
2. Planner writes a plan in `docs/plans/`
3. Planner captures any "too early" scoped-out features in `docs/plans/DEFERRED_FEATURES.md`
4. Plan reviewer writes a review in `docs/reviews/`
5. Implementer executes the approved plan
6. Implementation reviewer writes a review in `docs/reviews/`
7. Implementer or integrator writes `docs/reports/REPORT_<slug>.md`
8. Final actor updates continuity docs and `docs/HANDOFF.md`
9. use FIELD_GUIDE.md when locating non-obvious ownership points, hidden boundaries, or hard-to-grep logic


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
- Deferred features: `docs/plans/DEFERRED_FEATURES.md`
- Reviews: `docs/reviews/`
- Reports: `docs/reports/`
- Tool setup: `docs/agents/`

## Local Gemini CLI usage

- Canonical reference: `docs/agents/GEMINI.md`
- Use Gemini CLI only as a second eye for plan review, implementation review, and bounded repo scans
- Do not use Gemini CLI as the primary planner or final decision-maker
- Prefer headless prompts for saved-artifact work:

```powershell
gemini --approval-mode plan -p "<prompt>"
```

- Verify local availability/auth with:

```powershell
gemini -p "Reply with exactly the single word OK."
```

## Push and merge policy

- Branch creation allowed
- Local commits allowed
- Push requires explicit user approval
- Merge requires explicit user approval
- Any remote-affecting action requires explicit prompt to the user
