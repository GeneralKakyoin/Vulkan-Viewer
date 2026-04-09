# Report: LLUDP Object Mesh Discovery + Network Debug Declutter 2026-04-03

## Summary of implemented work
- Updated live mesh request scheduling to stop using generic visible-scene mesh IDs (proxy/guess-prone) as default discovery input.
- Kept explicit fixture mesh IDs for deterministic testing and retained decoded object-feed mesh IDs (from real LLUDP object updates) as primary live discovery input.
- Decluttered Network Debug presentation:
  - focused Session summary on object-ingress signals
  - section default-open behavior now prioritizes `Session`, `LLUDP`, and `RegionObjects`
  - per-section line cap now shows recent tail with count
  - `Recent Network Events` starts collapsed

## Files changed
- `crates/viewer_app/src/main.rs`
- `crates/viewer_ui/src/lib.rs`
- `docs/plans/PLAN_LLUDP_OBJECT_MESH_DISCOVERY_AND_DEBUG_DECLUTTER_2026-04-03.md`
- `docs/reviews/REVIEW_PLAN_LLUDP_OBJECT_MESH_DISCOVERY_AND_DEBUG_DECLUTTER_2026-04-03.md`
- `docs/reviews/REVIEW_IMPL_LLUDP_OBJECT_MESH_DISCOVERY_AND_DEBUG_DECLUTTER_2026-04-03.md`

## Validation run
- `cargo fmt --all` -> PASS
- `cargo check -p viewer_app -p viewer_ui -p viewer_net` -> PASS
- `cargo test -p viewer_app test_extract_decoded_object_feed_mesh_ids_is_deterministic_and_capped -- --nocapture` -> PASS
- `cargo test -p viewer_ui -- --nocapture` -> PASS
- `cargo run -p viewer_app` remains intermittently blocked by local linker lock (`LNK1104` on `target/debug/deps/viewer_app.exe`) in this environment.
- Runtime smoke executed via existing binary:
  - `VIEWER_APP_LIVE_STARTUP=off STRESS_TEST=screenshot VIEWER_TEST_SCREENSHOT_DIR=artifacts/screenshots_mesh_discovery_debug_declutter_2026-04-03 VIEWER_TEST_SCREENSHOT_EVERY_N_FRAMES=1 VIEWER_TEST_SCREENSHOT_MAX_FRAMES=1 .\target\debug\viewer_app.exe` -> PASS
  - screenshot reviewed: `artifacts/screenshots_mesh_discovery_debug_declutter_2026-04-03/viewer_test_0001.png` -> visual verdict PASS (expected scene and no obvious render corruption).
- Bounded live validation (decoded-only mesh discovery mode):
  - `VIEWER_APP_LIVE_STARTUP=on VIEWER_FIXTURE_MESHES=0 VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_mesh_discovery_decoded_only_2026-04-03.jsonl .\target\debug\viewer_app.exe` -> PASS (bounded run)
  - evidence:
    - `artifacts/logs/live_mesh_discovery_decoded_only_2026-04-03.log`
    - `artifacts/logs/network_debug_mesh_discovery_decoded_only_2026-04-03.jsonl`
    - object feed progressed in-window (`update_messages` rose to `855`, `total_objects=128`)
    - no `mesh_fetch` events were emitted with fixtures disabled in this bounded window.

## Result status
- Code changes complete and compile/test checks pass for touched crates.
- Runtime smoke validated via direct binary execution.
- Live bounded run confirms decoded-only mesh discovery path is active (no fixture-driven mesh requests when fixtures are disabled).

## Risks or follow-up items
- Local linker lock condition can still block fresh `cargo run` rebuilds in this environment.
- Next protocol follow-up: capture/verify decoded LLUDP object-feed entries with non-empty `mesh_id` to trigger real mesh fetches in decoded-only mode.

## Learnings delta
- none: no new durable learning beyond applying existing guidance to source mesh IDs from decoded LLUDP object updates.

## Continuity updates performed
- Added plan/review/report artifacts for this slice.
- Updated `docs/CURRENT_STATE.md` and `docs/HANDOFF.md`.
