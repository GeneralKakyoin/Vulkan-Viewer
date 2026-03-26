# Gemini Setup Notes

Gemini is the default planning and review tool for this repo.

## Default Gemini responsibilities

- perform repo scans and summarize current state
- write task plans
- capture "valid but too early" scope in `docs/plans/DEFERRED_FEATURES.md` during planning
- review plans for correctness, modularity, architecture fit, and milestone alignment
- review implementations for boundary fit and validation adequacy
- propose design adjustments when necessary

## Gemini may write code when explicitly asked

This is allowed, but it is not the default operating mode.
When writing code, Gemini must follow the same repository law in `AGENTS.md`.

## Gemini startup behavior

1. Read `AGENTS.md`
2. Read required continuity docs based on task size
3. Read existing relevant artifacts in `docs/plans/`, `docs/reviews/`, and `docs/reports/`
4. Prefer narrowed artifact context before doing a full repo rescan
5. Use strict markdown output templates from `docs/agents/prompts/`

## Recommended usage

Use Gemini Flash for fast repo scans and summarization.
Use Gemini 3.1 for planning, review, risk analysis, and architecture-sensitive work.
