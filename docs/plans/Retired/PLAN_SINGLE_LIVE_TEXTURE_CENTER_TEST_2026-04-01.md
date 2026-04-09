# Plan: Single Live Texture Center Test Mode (2026-04-01)

## Scope
- Add a bounded runtime mode that places one textured object at scene center for live asset-ingest verification.
- Ensure the mode can be activated without broad scene noise.

## Current Known State
- Live texture ingest is working when requests are emitted (`texture_fetch: ready` evidence exists).
- Current stress modes either spawn large scenes or do not guarantee a single center-target texture object.

## Files and Components Touched
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_SINGLE_LIVE_TEXTURE_CENTER_TEST_2026-04-01.md`
- `docs/reviews/REVIEW_PLAN_SINGLE_LIVE_TEXTURE_CENTER_TEST_2026-04-01.md`
- `docs/reports/REPORT_SINGLE_LIVE_TEXTURE_CENTER_TEST_2026-04-01.md`
- continuity docs (`docs/CURRENT_STATE.md`, `docs/HANDOFF.md`, `docs/LEARNINGS.md`)

## Boundary Check
- Keep changes in `viewer_app` orchestration/test-mode only.
- No transport/protocol changes in `viewer_net`.
- No capability semantics changes in `viewer_grid`.

## Step Sequence
1. Add a new `StressTestMode` variant for a single live texture center object (env alias in `STRESS_TEST` parsing).
2. Implement `spawn_single_live_texture_center_test()` that inserts one center cube and assigns the first fixture texture ID.
3. Wire mode into app startup mode dispatch.
4. Add unit test for mode parsing.
5. Validate with:
   - `cargo fmt --all`
   - `cargo check -p viewer_app`
   - targeted `viewer_app` test for new mode parsing
   - bounded live run proving `texture_fetch: queued` and `texture_fetch: ready` for the center-test ID.

## Risks and Open Questions
- If no fixture/live texture ID is provided, the center object will remain untextured.
- This mode is for ingest verification, not final visual polish.

## Deferred-Too-Early Candidates
- Automated screenshot pixel assertion for center-object texture application (defer to tooling/parity test harness milestone).

## Learnings Pre-Check
- L65, L66, L71.

## Exact Completion Criteria
- `STRESS_TEST` includes a dedicated single-live-texture-center mode.
- Bounded run shows live fetch queue and ready evidence for one texture ID.
- Continuity artifacts updated with exact evidence path/lines.
