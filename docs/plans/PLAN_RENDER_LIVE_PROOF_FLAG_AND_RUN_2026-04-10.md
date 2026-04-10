# PLAN_RENDER_LIVE_PROOF_FLAG_AND_RUN_2026-04-10.md

## Scope
- Add a bounded live-verification mode in `viewer_app` that proves:
  - no-position object updates keep stable world placement (no drift)
  - live texture decode/application path is active (decoded texture evidence)
- Run a real live session with the new mode and produce a machine-readable proof artifact.

## Current Known State
- Placement logic and texture diagnostics were improved, but user reports live confidence is still insufficient.
- Existing logs are verbose but do not provide a single pass/fail signal tied directly to placement stability under no-position updates.

## Files / Components Touched
- `crates/viewer_app/src/main.rs`
  - add render-proof config/state
  - add env flag parsing
  - add per-frame proof sampling and final summary emission
  - wire texture decode/failure counters into proof state
- `docs/reports/REPORT_RENDER_LIVE_PROOF_FLAG_AND_RUN_2026-04-10.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`

## Boundary Check
- No crate-boundary changes.
- No protocol-shape changes.
- No dependency changes.

## Step Sequence
1. Implement `VIEWER_APP_RENDER_PROOF` config and proof state in `viewer_app`.
2. Sample live scene per frame:
   - identify object-feed local IDs whose current snapshot lacks position payload
   - verify corresponding scene proxy positions remain stable vs last observed frame for same local ID
3. Count texture decode success/failure events from live texture updates.
4. Emit one JSON summary line to artifact path and relay pass/fail once proof window elapses.
5. Run live viewer session with proof mode and collect artifact.

## Validation Plan
- `cargo fmt --all`
- `cargo check -p viewer_app`
- targeted tests for render-proof config parsing / summary evaluation
- live run with:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_APP_RENDER_PROOF=on`
  - `VIEWER_APP_RENDER_PROOF_LOG_PATH=<artifact>`
  - timeout-bounded `cargo run -p viewer_app`

## Risks / Open Questions
- Live result depends on environment credentials/capability health.
- If live login fails, artifact should still clearly report no-proof / insufficient evidence state.

## Deferred-too-early Candidates
- None for this bounded verification slice.

## Learnings Pre-check
- L04: keep dirty-only scene apply semantics intact.
- L16: preserve renderer fallback texture behavior.
- L22: include explicit manual/runtime artifact evidence, not command success alone.
- L82: keep edits crate-local and behavior-local.

## Completion Criteria
- New flag exists and produces deterministic proof JSON summary in live runs.
- I run the live version and provide artifact path + verdict from this run.
