# Plan: Live Mesh Blocker Isolation (2026-04-03)

## Scope
- Add additive diagnostics that make decoded mesh-ID discovery visible in `viewer_net` and `viewer_app`.
- Harden mesh capability request selection so mesh fetch can fall through across `ViewerAsset`, `GetMesh2`, and `GetMesh`.
- Add focused tests for the new diagnostics and fallback behavior.
- Run bounded validation, including live captures when `VIEWER_LOGIN_*` env is available.

## Current Known State
- Decoded-only live runs have previously shown healthy LLUDP object ingress without any in-window `mesh_fetch` relay lines.
- Fixture-driven mesh requests do execute, but prior live evidence showed `ViewerAsset?mesh_id=...` returning `403 AccessDenied`.
- Existing mesh URL selection preferred the first populated cap family and did not expose the full ordered candidate set to diagnostics.

## Files / Components Touched
- `crates/viewer_grid/src/lib.rs`
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/reviews/REVIEW_PLAN_LIVE_MESH_BLOCKER_ISOLATION_2026-04-03.md`
- `docs/reviews/REVIEW_IMPL_LIVE_MESH_BLOCKER_ISOLATION_2026-04-03.md`
- `docs/reports/REPORT_LIVE_MESH_BLOCKER_ISOLATION_2026-04-03.md`
- continuity docs (`docs/CURRENT_STATE.md`, `docs/HANDOFF.md`)

## Boundary Check
- `viewer_grid`: capability meaning and ordered mesh candidate shaping.
- `viewer_net`: transport-side object-feed counters and HTTP fetch attempt capture.
- `viewer_app`: relay formatting and worker-side mesh fetch observability.
- No architecture changes and no new crate-boundary crossings.

## Step Sequence
1. Extend transport decode summaries with bounded object-feed family counters needed for mesh discovery triage.
2. Add app-side relay formatting for decoded mesh-ID counts, mesh-ID samples, and LLUDP contributing family counts.
3. Change mesh capability policy to surface all available mesh caps in priority order with cap metadata.
4. Add mesh HTTP attempt diagnostics so each candidate attempt records cap name, URL variant, status, and 403 bucket when present.
5. Add/adjust focused tests in `viewer_grid`, `viewer_net`, and `viewer_app`.
6. Run `fmt`, `check`, touched-crate tests, and bounded runtime validation.
7. Run bounded live captures when login env is available; otherwise record the exact environment blocker.

## Validation Plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_grid -p viewer_app`
- `cargo test -p viewer_grid`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- bounded runtime smoke:
  - `VIEWER_APP_LIVE_STARTUP=off STRESS_TEST=screenshot ... .\\target\\debug\\viewer_app.exe`
- bounded live captures when configured:
  - decoded-only with `VIEWER_FIXTURE_MESHES=0`
  - fixture mesh with one known UUID and dedicated `VIEWER_NETWORK_DEBUG_LOG_PATH`

## Risks / Open Questions
- If decoded mesh IDs still do not appear in live runs, the primary blocker remains upstream discovery rather than HTTP transport.
- If later mesh caps still return authorization failures, the blocker is likely external route/session policy rather than first-cap-only request selection.
- This shell may not have the required `VIEWER_LOGIN_*` env needed for live verification.

## Deferred-Too-Early Candidates
- None.

## Learnings Pre-Check
- L06: preserve the `viewer_grid` vs `viewer_net` meaning/transport boundary explicitly.
- L73: deterministic mesh-lane verification needs a forced mesh UUID rather than opportunistic scene discovery.
- L74: `RegionObjects` mesh candidates are observability only and cannot be treated as guaranteed mesh-ID discovery.

## Completion Criteria
- Mesh discovery relays explicitly report decoded object count, mesh-ID count, sample mesh IDs, and contributing LLUDP family counts.
- Mesh fetch relays record per-attempt cap metadata and HTTP outcome details.
- Mesh capability ordering falls through across all available mesh caps in tests.
- Validation results are recorded exactly, including whether live capture ran or was blocked by missing env.
