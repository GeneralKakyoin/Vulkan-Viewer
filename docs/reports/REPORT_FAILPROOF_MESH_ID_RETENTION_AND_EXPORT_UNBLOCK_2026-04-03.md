# Report: Failproof Mesh-ID Retention and Export Unblock 2026-04-03

## Summary of implemented work
- Added object-feed mesh-retention counters in `viewer_net` for:
  - state/export object counts
  - state/export mesh-object counts
  - per-family mesh hits for `ObjectUpdate`, `ObjectUpdateCompressed`, and `ObjectExtraParams`
- Fixed object-feed export semantics so:
  - `object_feed_total_objects` remains the full transport-side map size
  - export truncation deterministically prioritizes mesh-bearing objects
  - known mesh IDs are not cleared by later non-mesh updates
- Added replay-grade `viewer_net` tests from a real `FirestormsFidelis` `ObjectUpdate` payload showing:
  - embedded `ExtraParams` yields a real mesh ID
  - later cached and terse updates do not erase it
  - truncation still exports mesh-bearing objects
- Updated `viewer_app` so relay summaries now report both state and export views explicitly and preserve transport totals across the net-to-app bridge.
- Added app regression coverage for count preservation and relay formatting.

## Files changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_FAILPROOF_MESH_ID_RETENTION_AND_EXPORT_UNBLOCK_2026-04-03.md`
- `docs/reviews/REVIEW_PLAN_FAILPROOF_MESH_ID_RETENTION_AND_EXPORT_UNBLOCK_2026-04-03.md`
- `docs/reviews/REVIEW_IMPL_FAILPROOF_MESH_ID_RETENTION_AND_EXPORT_UNBLOCK_2026-04-03.md`
- `docs/reports/REPORT_FAILPROOF_MESH_ID_RETENTION_AND_EXPORT_UNBLOCK_2026-04-03.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/LEARNINGS.md`

## Validation run
- `cargo fmt --all` -> PASS
- `cargo check -p viewer_net -p viewer_app` -> PASS
- `cargo test -p viewer_net` -> PASS
- `cargo test -p viewer_app` -> PASS
- `cargo build -p viewer_app` -> PASS
- bounded live decoded-only verification with rebuilt executable -> PASS for evidence capture
  - command environment:
    - `VIEWER_APP_LIVE_STARTUP=on`
    - `VIEWER_FIXTURE_MESHES=0`
    - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_live_mesh_retention_decoded_only_2026-04-03_rerun.jsonl`
    - `STRESS_TEST=` (cleared to avoid screenshot-mode auto-exit)
  - executable:
    - `.\target\debug\viewer_app.exe`
  - artifacts:
    - `artifacts/logs/live_mesh_retention_decoded_only_2026-04-03_rerun.out.log`
    - `artifacts/logs/live_mesh_retention_decoded_only_2026-04-03_rerun.err.log`
    - `artifacts/logs/network_debug_live_mesh_retention_decoded_only_2026-04-03_rerun.jsonl`

## Result status
- Complete for the planned slice.
- Live bounded evidence shows the discovery/export/scheduler path is working on the current route/window:
  - `tick summary: update_messages=130 state_total_objects=674 export_objects=128 state_mesh_objects=305 export_mesh_objects=128 export_truncated=true mesh_id_count=101 ... mesh_hits=ObjectUpdate:265,ObjectUpdateCompressed:54,ObjectExtraParams:0`
  - multiple real `mesh_fetch` attempts and completions succeeded in-window, for example:
    - `mesh_fetch: attempt ... cap=ViewerAsset ... status=206`
    - `mesh_fetch: attempt ... cap=ViewerAsset ... status=200`
    - `mesh_fetch: ready ... attempt=1`

## Risks or follow-up items
- Earlier April 3 conclusions that live mesh discovery still yielded `mesh_id_count=0` are superseded; those runs used an older executable and should not guide next steps.
- The next slice should move downstream from discovery/export into mesh-byte consumption, caching, or render integration if user-visible mesh display is still incomplete.

## Learnings delta
- added: L78 for the embedded-vs-standalone `ExtraParams` layout trap that hid live mesh IDs until real Firestorm payloads were replayed.

## Continuity updates performed
- Added plan, plan review, implementation review, and execution report for this slice.
- Updated `docs/CURRENT_STATE.md`.
- Replaced `docs/HANDOFF.md` with the latest state.
- Added L78 to `docs/LEARNINGS.md`.
