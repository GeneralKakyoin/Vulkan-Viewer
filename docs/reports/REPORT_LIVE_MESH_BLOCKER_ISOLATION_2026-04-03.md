# Report: Live Mesh Blocker Isolation (2026-04-03)

## Summary of Implemented Work
- Added decoded object-feed mesh-discovery diagnostics in `viewer_app` backed by expanded decode counters in `viewer_net`.
- Changed mesh capability selection to expose all ordered mesh-cap candidates with cap metadata instead of stopping at the first populated cap family.
- Added per-attempt mesh HTTP diagnostics so relays can show cap name, URL variant, status, and 403 bucket information.
- Added focused tests locking app-side summary formatting and transport fallback behavior.

## Files Changed
- `crates/viewer_grid/src/lib.rs`
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_LIVE_MESH_BLOCKER_ISOLATION_2026-04-03.md`
- `docs/reviews/REVIEW_PLAN_LIVE_MESH_BLOCKER_ISOLATION_2026-04-03.md`
- `docs/reviews/REVIEW_IMPL_LIVE_MESH_BLOCKER_ISOLATION_2026-04-03.md`
- `docs/reports/REPORT_LIVE_MESH_BLOCKER_ISOLATION_2026-04-03.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`

## Validation Run
- `cargo fmt --all` -> PASS
- `cargo check -p viewer_net -p viewer_grid -p viewer_app` -> PASS
- `cargo test -p viewer_grid` -> PASS
- `cargo test -p viewer_net` -> PASS
- `cargo test -p viewer_app` -> PASS
- `VIEWER_APP_LIVE_STARTUP=off STRESS_TEST=screenshot VIEWER_TEST_SCREENSHOT_DIR=artifacts/screenshots_mesh_blocker_offline_smoke_2026-04-03 VIEWER_TEST_SCREENSHOT_EVERY_N_FRAMES=1 VIEWER_TEST_SCREENSHOT_MAX_FRAMES=1 .\\target\\debug\\viewer_app.exe` -> PASS as bounded runtime smoke; process was time-boxed, screenshot captured and manually reviewed
- `cargo build -p viewer_app` -> PASS
- bounded decoded-only live capture with `.env` loaded into process env:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_FIXTURE_MESHES=0`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_live_mesh_blocker_decoded_only_2026-04-03.jsonl`
  - `.\\target\\debug\\viewer_app.exe`
  - bounded 45s run -> PASS for evidence capture
- bounded fixture-mesh live capture with `.env` loaded into process env:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_FIXTURE_MESHES=947d4505-eb76-2ef5-c049-e7882881d689`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_live_mesh_blocker_fixture_2026-04-03.jsonl`
  - `.\\target\\debug\\viewer_app.exe`
  - bounded 45s run -> PASS for evidence capture

## Result Status
- Code implementation is complete for the planned diagnostics and ordered mesh-cap fallback work.
- Offline runtime validation passed.
- Fresh live captures completed successfully by loading `.env` into the bounded run process.

## Live Validation Findings
- Decoded-only capture artifacts:
  - `artifacts/logs/live_mesh_blocker_decoded_only_2026-04-03.out.log`
  - `artifacts/logs/live_mesh_blocker_decoded_only_2026-04-03.err.log`
  - `artifacts/logs/network_debug_live_mesh_blocker_decoded_only_2026-04-03.jsonl`
- Fixture-mesh capture artifacts:
  - `artifacts/logs/live_mesh_blocker_fixture_2026-04-03.out.log`
  - `artifacts/logs/live_mesh_blocker_fixture_2026-04-03.err.log`
  - `artifacts/logs/network_debug_live_mesh_blocker_fixture_2026-04-03.jsonl`

Observed decoded-only result:
- LLUDP object ingress was healthy in-window (`lludp_object_gate: PASS`).
- Object-feed activity was substantial:
  - final bounded summary reached `update_messages=793 total_objects=128`
  - contributing families included `ObjectUpdate:459`, `ObjectUpdateCompressed:95`, and `ImprovedTerseObjectUpdate:225`
- Despite that, every bounded object-feed summary reported:
  - `mesh_id_count=0`
  - `sample_mesh_ids=none`
  - `ObjectExtraParams:0`

Observed fixture-mesh result:
- Mesh command lane executed immediately for the forced UUID.
- Every attempted fetch candidate in-window was `ViewerAsset` only and returned:
  - `status=403`
  - `bucket=AccessDenied`
- Final relay remained:
  - `reason=MissingCapability`
  - `detail=http status 403 Forbidden ... bucket=AccessDenied`

## Root-Cause Conclusion
- **Primary blocker:** upstream live mesh-ID discovery on this route/window.
  - Decision-rule match: decoded-only live run reached healthy object ingress but still showed `mesh_id_count=0` throughout the bounded capture.
- **Secondary blocker:** fixture-driven mesh HTTP fetch still ends in `ViewerAsset` `403 AccessDenied`.
  - In this live session the worker only emitted two mesh URL candidates (the `ViewerAsset` `v1`/`v2` shapes), so later-cap fallback did not occur in practice despite being implemented and covered by unit tests.

## Risks or Follow-Up Items
- Next branch should target why decoded LLUDP object-feed export never observes mesh IDs even with frequent `ObjectUpdate` / `ObjectUpdateCompressed` traffic and healthy object ingress.
- Secondary follow-up should inspect why the live mesh worker only sees `ViewerAsset` candidates at request time even though startup seed-cap diagnostics list `GetMesh` and `GetMesh2`.

## Learnings Delta
- `none` because this slice added observability and fallback coverage but did not yet produce a new durable invariant from fresh live evidence.

## Continuity Updates Performed
- Added plan/review/report artifacts for this slice.
- Updated `docs/CURRENT_STATE.md`.
- Updated `docs/HANDOFF.md`.
