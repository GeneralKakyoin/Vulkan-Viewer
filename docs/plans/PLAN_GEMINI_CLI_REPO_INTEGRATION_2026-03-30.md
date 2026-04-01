# Plan: Gemini CLI Repo Integration 2026-03-30

## Objective
Document the newly available Gemini CLI as a first-class repo tool and add a repo-local Codex skill so future agents can use it consistently for planning, review, and secondary analysis work.

## Scope
In scope:
- Update repo agent docs under `docs/agents/` so Gemini CLI usage, expectations, and safe invocation patterns are explicit.
- Add a repo-local skill under `.agents/skills/` that teaches Codex when and how to use Gemini CLI in this repository.
- Update continuity artifacts to reflect that Gemini CLI is now available and documented.

Out of scope:
- Changing product code or runtime behavior.
- Adding Gemini-specific automation scripts, wrappers, or CI integration.
- Introducing a new multi-agent workflow outside the current repo law in `AGENTS.md`.

## Current known state
- `docs/agents/GEMINI.md` already describes Gemini conceptually as the default planning/review tool, but it does not yet document the local CLI workflow.
- The repo already exposes local repo skills via `.agents/skills/`, and `.codex/config.toml` has `use_repo_skills = true`.
- The `gemini` CLI is installed on this machine and now responds successfully in headless mode.
- There is no repo-local skill that tells Codex when to invoke Gemini CLI or what command shape to prefer.

## Files and components touched
- `docs/agents/GEMINI.md`
- `docs/agents/CODEX.md`
- `docs/agents/RUNBOOK.md`
- `.agents/skills/<new gemini skill>/SKILL.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/reports/REPORT_gemini_cli_repo_integration_2026-03-30.md`
- `docs/reviews/REVIEW_PLAN_GEMINI_CLI_REPO_INTEGRATION_2026-03-30.md`

## Boundary check
- Docs/skill-only change; no crate or protocol boundaries change.
- Repo law remains in `AGENTS.md`; Gemini CLI documentation must reinforce that it follows the same repo contract rather than creating a parallel process.

## Step sequence
1. Capture a docs-only plan and plan review for this repo integration task.
2. Update `docs/agents/GEMINI.md` with concrete CLI usage, recommended modes, and safe repo workflow expectations.
3. Update adjacent agent docs (`CODEX.md`, `RUNBOOK.md`) so Codex can discover when Gemini CLI is appropriate.
4. Add one repo-local skill that triggers when Codex needs Gemini CLI for planning, review, or second-opinion repo analysis.
5. Validate the new docs and skill by running the Gemini CLI smoke command used in the documentation.
6. Write a report and update continuity docs.

## Validation plan
- Run:
  - `gemini -p "Reply with exactly the single word OK."`
- Docs-only change, so no cargo validation is required for the scoped task.

## Risks and open questions
- Risk: duplicating instructions between `docs/agents/` and the skill. Mitigation: keep workflow detail centralized in `docs/agents/GEMINI.md` and keep the skill concise.
- Risk: over-authorizing Gemini usage. Mitigation: document it as a planning/review/analysis tool by default, not as a scope-expanding bypass.

## Deferred-too-early candidates captured
- A wrapper script for standardized Gemini prompts is useful but too early for this docs-only integration.
- Automatic sync between repo skills and Gemini CLI skill-install/link commands is useful but out of scope.

## Learnings pre-check
- No applicable learnings. This task is a docs-and-skill integration slice and does not touch the protocol/debugging constraints in existing entries.

## Completion criteria
- Repo agent docs clearly describe how Gemini CLI should be used in this repository.
- A repo-local Codex skill exists for Gemini CLI usage.
- Continuity artifacts record that Gemini CLI is available and documented.
- The documented Gemini smoke command is run successfully during validation.
