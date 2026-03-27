# Plan: Testing Reference Compendium (Flags, Modes, and Commands)

## Objective
Create a single, canonical, model-friendly reference that enumerates **all** test-related commands, flags, and environment variables currently supported by this repo, so agents can reliably choose the right validation and runtime verification for the change they are making.

## Scope
In scope:
- Add a new docs page that consolidates:
  - `cargo` validation commands used in this repo (`fmt`, `check`, targeted `test`, runtime `run` smoke).
  - Runtime verification modes (notably `STRESS_TEST=...`) and their tuning knobs.
  - All `VIEWER_*` environment variables that affect validation, test harnesses, or deterministic runtime verification (including offline/fixture modes).
  - “Manual/diagnostic entry points” used as tests (e.g. `viewer_net` examples, `viewer_app` relay bin), including required env vars.
- Update existing docs to point to this new reference where appropriate (keep changes minimal and non-disruptive).

Out of scope:
- Adding new test modes or changing code behavior.
- Adding CI integration or golden-image/pixel-diff tooling.
- Refactoring existing docs broadly; only add links/short pointers.

## Current known state
- The repo already uses deterministic runtime verification via:
  - `STRESS_TEST` modes in `viewer_app` (including `camera` and `screenshot`).
  - fixture-backed texture acquisition (`VIEWER_FIXTURE_TEXTURES` + `test_assets/*`).
- Many environment variables exist for live startup orchestration, in-process worker probing, and network diagnostics (`VIEWER_LOGIN_*`, `VIEWER_FIRST_SIM_*`, etc.).
- There is no single canonical doc page that lists all supported flags and “test entry points”.

## Files and components touched
Add:
- `docs/TESTING_REFERENCE.md` (new canonical compendium)

Update (links/pointers only):
- `README.md`
- `docs/TASKS.md` (planning/validation guidance)
- `docs/CURRENT_STATE.md` (mention new reference exists)
- `docs/HANDOFF.md` (latest handoff reflects new doc and validation)
- Optionally `docs/FIELD_GUIDE.md` (non-obvious “where is testing reference” pointer)

## Boundary check
- Docs-only change: no crate boundaries, architecture, or code behavior changes.
- No protocol semantics are invented or altered.

## Step sequence
1. Inventory current test-related controls from repo evidence:
   - enumerate `VIEWER_*` env vars used in code
   - enumerate `STRESS_TEST` modes + tuning env vars
   - enumerate fixture/test-asset hooks
   - enumerate manual diagnostic entry points (`cargo run ... --example`, relay bin)
2. Author `docs/TESTING_REFERENCE.md` with:
   - a “choose the right validation” decision guide
   - canonical command snippets
   - env var tables grouped by subsystem
   - explicit “not implemented / retired / doc-only” section for historical flags
3. Update minimal pointers in existing docs so agents can find the compendium fast.
4. Validate with required repo checks.
5. Write an execution report and update continuity docs.

## Validation plan
- Always run:
  - `cargo fmt`
  - `cargo check`
- No targeted `cargo test` expected (docs-only change), unless formatting/check reveals an existing failure.

## Risks and open questions
Risks:
- The compendium could drift if new flags are added but not documented; mitigate by positioning it as the canonical place to update when adding env vars/test modes.

Open questions:
- None blocking (the goal is purely documentary).

## Deferred-too-early candidates captured
- Add a command that auto-generates the env var table from code literals (useful, but out of scope for this docs-only task).
- Add CI automation for screenshot capture artifacts (already explicitly out of scope in prior testing plan).

## Completion criteria
- A single canonical doc exists that consolidates test commands, flags, and env vars used by this repo.
- README and task workflow docs point to it.
- Validation commands run and results recorded in a report.

