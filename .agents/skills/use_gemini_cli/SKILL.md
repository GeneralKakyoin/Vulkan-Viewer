# Skill: use-gemini-cli

Use this skill when Codex should invoke the local Gemini CLI as a second eye for plan review, implementation review, or a bounded repo-risk scan.

Rules:
- read `docs/agents/GEMINI.md` first for the canonical workflow
- keep Gemini prompts bounded and artifact-driven
- prefer headless usage: `gemini --approval-mode plan -p "<prompt>"`
- direct Gemini to read `AGENTS.md`, required continuity docs, and the specific plan/review/report files it needs
- use Gemini to support repo law, not bypass it
- do not use Gemini as the primary planner or final decision-maker
- keep implementation with Codex unless the user explicitly wants Gemini writing code
- if Gemini output changes repo truth, still update the required repo artifacts in `docs/plans/`, `docs/reviews/`, `docs/reports/`, `docs/CURRENT_STATE.md`, and `docs/HANDOFF.md`
check docs/FIELD_GUIDE.md when locating non-obvious functions, data paths, or ownership points
