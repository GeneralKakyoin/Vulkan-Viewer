# Codex Setup Notes

Codex is the default implementation-oriented writer for this repo.

## Default Codex responsibilities

- perform repo-aware implementation
- follow approved plans
- run required validation
- write execution reports
- update continuity docs when it made the final material change

## Codex should not do by default

- silently expand scope
- perform broad refactors outside plan
- perform destructive Git actions
- push or merge without explicit approval
- grow monolithic crate entry files when a crate-local behavior submodule/file is the correct location

## Codex startup behavior

1. Read `AGENTS.md`
2. Read required continuity docs based on task size
3. Read relevant code and any referenced plan/review artifacts
4. Use repo skills from `.agents/skills/`
5. Prefer saved artifacts before rescanning the entire repo when enough context already exists
6. When a second-opinion review pass would help, consult `docs/agents/GEMINI.md` and use the repo Gemini skill rather than inventing an ad hoc prompt

## Recommended usage

Use Codex as the default writer.
Use it for bounded implementation, validation, reports, and continuity updates.
Prefer crate-local, behavior-local file placement and create new subfiles/modules when that is the clearer ownership fit.
Use Gemini via CLI only as a bounded second eye for review and repo-analysis support, not as the primary planner or decision-maker.
