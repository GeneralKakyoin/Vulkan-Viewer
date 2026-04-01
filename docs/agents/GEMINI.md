# Gemini Setup Notes

Gemini is a second-eye review tool for this repo.

This doc is the canonical Gemini CLI workflow reference for this repository.

## Default Gemini responsibilities

- perform repo scans and summarize current state
- review existing plans for correctness, modularity, architecture fit, and milestone alignment
- review implementations for boundary fit and validation adequacy
- provide bounded second-opinion risk analysis when necessary

## Gemini must not be the primary planner or decision-maker

- Codex and the saved repo artifacts remain the primary planning and decision path
- Gemini should not author the canonical task plan by default
- Gemini should not make final scope, architecture, or branch-direction decisions
- Gemini may challenge a proposal, surface risks, or suggest alternatives, but the accepted plan and final decisions must still be recorded through the normal repo workflow

## Gemini may write code when explicitly asked

This is allowed, but it is not the default operating mode.
When writing code, Gemini must follow the same repository law in `AGENTS.md`.

## Local CLI expectation

- `gemini` should be available on `PATH`
- local auth must be configured outside the repo
- never commit Gemini auth, settings, tokens, or session artifacts into the repo

Minimal smoke command:

```powershell
gemini -p "Reply with exactly the single word OK."
```

Use that command to verify installed-and-authenticated behavior before relying on Gemini for second-eye review work.

## Gemini startup behavior

1. Read `AGENTS.md`
2. Read required continuity docs based on task size
3. Read existing relevant artifacts in `docs/plans/`, `docs/reviews/`, and `docs/reports/`
4. Prefer narrowed artifact context before doing a full repo rescan
5. Use strict markdown output templates from `docs/agents/prompts/`

## Recommended CLI usage

Prefer headless prompts for bounded work:

```powershell
gemini -p "<prompt>"
```

Useful flags:

- `--model <name>` to select the model explicitly
- `--approval-mode plan` for read-only second-eye review work
- `--output-format text` for simple terminal use
- `--output-format json` or `stream-json` when another tool needs structured output
- `--include-directories <path>` when Gemini needs extra workspace context outside the repo root

Prefer concise, review-oriented prompts that cite the relevant repo artifacts and requested output file.

Examples:

```powershell
gemini --approval-mode plan -p "Read AGENTS.md, docs/CURRENT_STATE.md, docs/HANDOFF.md, and docs/plans/PLAN_X.md. Produce a second-eye plan review with verdict, boundary fit, correctness concerns, validation adequacy, and learnings delta."
```

```powershell
gemini --approval-mode plan -p "Read AGENTS.md and the current continuity docs, then provide a second-eye risk summary of the current blocker and whether the proposed next step looks safe."
```

## When to use Gemini from Codex

Use Gemini when Codex would benefit from:

- a plan review before implementation
- an implementation review focused on risks and validation gaps
- a second-opinion repo scan or bounded architecture/risk summary

Do not use Gemini as a way to bypass:

- required written plans
- required reviews
- repo boundary checks
- user approval for meaningful implementation
- continuity/reporting updates
- Codex ownership of planning and final decisions

## Recommended usage

Use Gemini Flash for fast second-eye scans and summarization.
Use Gemini 3.1 for review, risk analysis, and architecture-sensitive second opinions.
