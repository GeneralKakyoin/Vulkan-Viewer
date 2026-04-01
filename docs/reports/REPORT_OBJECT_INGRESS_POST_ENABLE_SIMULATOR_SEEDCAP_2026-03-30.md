# Report: Object Ingress Post-EnableSimulator Seed-Cap Follow-Up (2026-03-30)

## Summary of implemented work
- Added an approved review artifact for the active post-`EnableSimulator` seed-cap plan.
- Expanded the default seed-cap request in `viewer_net` to include the Firestorm-evidenced region capability names:
  - `UntrustedSimulatorMessage`
  - `InterestList`
  - `RegionObjects`
- Added a reusable `fetch_seed_capabilities_from_url(...)` helper in `viewer_net` so the app can follow explicit seed-cap URLs without duplicating request/parse logic.
- Updated the live worker in `viewer_app` to perform bounded one-shot follow-up for EventQueue-delivered seed-cap URLs and to relay the returned capability inventory plus a compact summary of important caps.

## Files changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_POST_ENABLE_SIMULATOR_SEEDCAP_2026-03-30.md`
- `docs/plans/PLAN_OBJECT_INGRESS_REGION_OBJECTS_CAP_PROBE_2026-03-30.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_REGION_OBJECTS_CAP_PROBE_2026-03-30.md`

## Validation run
- `cargo fmt --all`: PASSED
- `cargo check -p viewer_net -p viewer_app`: PASSED
- `cargo test -p viewer_net`: PASSED
- `cargo test -p viewer_app`: PASSED
- bounded connected run with network-debug capture:
  - command: `VIEWER_APP_LIVE_STARTUP=on VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_post_enable_seedcap_2026-03-30.jsonl cargo run -p viewer_app`
  - result: PASSED for evidence capture; process intentionally stopped after 45 seconds
  - artifacts:
    - `artifacts/logs/live_post_enable_seedcap_2026-03-30.out.log`
    - `artifacts/logs/live_post_enable_seedcap_2026-03-30.err.log`
    - `artifacts/logs/network_debug_post_enable_seedcap_2026-03-30.jsonl`

## Result status
- The primary simulator seed-cap inventory now explicitly returns additional simulator-host capabilities on `:12043`:
  - `InterestList`
  - `RegionObjects`
  - `UntrustedSimulatorMessage`
- The bounded live run still did not surface `EstablishAgentCommunication`.
- The EventQueue mix remained:
  - `AgentGroupDataUpdate`
  - `AgentStateUpdate`
  - `EnableSimulator`
  - `ParcelProperties`
- LLUDP object ingress still remained blocked:
  - `RegionHandshake = none`
  - `RegionHandshakeReply = none`
  - `ObjectUpdate* = none`
  - `update_messages=0`
  - `total_objects=0`

## Evidence-backed conclusion
- The missing gate is no longer “we never asked for the primary simulator’s broader region caps.” The simulator now returns those caps and the app surfaces them.
- The current best opening for the user’s stated goal is now the explicit `RegionObjects` capability, because it is object-related data already proven present on the primary simulator capability map.
- The next selected branch is:
  - `docs/plans/PLAN_OBJECT_INGRESS_REGION_OBJECTS_CAP_PROBE_2026-03-30.md`

## Risks or follow-up items
- `RegionObjects` may expose pathfinding/linkset-oriented object data rather than the same stream as LLUDP world-object updates.
- `EstablishAgentCommunication` may still be absent because another simulator-host precondition remains unmet.

## Learnings delta
- `added`
- Added L42 to `docs/LEARNINGS.md`.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/LEARNINGS.md`
