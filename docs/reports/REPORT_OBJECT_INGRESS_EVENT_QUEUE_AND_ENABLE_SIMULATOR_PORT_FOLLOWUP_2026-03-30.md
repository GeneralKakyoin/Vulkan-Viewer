# Report: Object Ingress EventQueue And EnableSimulator Port Follow-Up (2026-03-30)

## Summary of implemented work
- Completed the persistent EventQueue behavior slice by moving seed-cap fetch early and keeping one active `EventQueueGet` consumer alive during startup and early steady state.
- Extended EventQueue parsing so nested LLSD/JSON body values are preserved instead of dropping non-scalar submaps.
- Added bounded extraction/relay for:
  - `EnableSimulator`
  - `EstablishAgentCommunication`
  - `ParcelProperties`
- Added a bounded `viewer_net` helper to send `UseCircuitCode` on the active social socket to explicit simulator ports on the current simulator host.
- Updated the live worker to de-duplicate EventQueue-delivered `EnableSimulator` ports and send one-shot `UseCircuitCode` follow-up to each newly seen port.

## Files changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_OBJECT_INGRESS_EVENT_QUEUE_CONTROL_CONSUMPTION_2026-03-30.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_EVENT_QUEUE_CONTROL_CONSUMPTION_2026-03-30.md`
- `docs/plans/PLAN_OBJECT_INGRESS_ENABLE_SIMULATOR_PORT_FOLLOWUP_2026-03-30.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_ENABLE_SIMULATOR_PORT_FOLLOWUP_2026-03-30.md`
- `docs/plans/PLAN_OBJECT_INGRESS_POST_ENABLE_SIMULATOR_SEEDCAP_2026-03-30.md`

## Validation run
- `cargo fmt --all`: PASSED
- `cargo check -p viewer_net -p viewer_app`: PASSED
- `cargo test -p viewer_net`: PASSED
- `cargo test -p viewer_app`: PASSED
- bounded connected run with EventQueue nested-field relay:
  - `artifacts/logs/live_event_queue_control_consumption_2026-03-30_bounded.out.log`: PASSED
- bounded connected run with `EnableSimulator` port follow-up:
  - `artifacts/logs/live_enable_simulator_port_followup_2026-03-30.out.log`: PASSED

## Result status
- The repo now ingests structured simulator/world data from the simulator-host EventQueue lane.
- Confirmed live ingress now includes:
  - repeated `AgentGroupDataUpdate`
  - repeated `AgentStateUpdate`
  - nested `ParcelProperties` content
  - port-only `EnableSimulator` details
- The bounded EventQueue relay now shows concrete followed simulator ports:
  - `13013`
  - `13000`
  - `13001`
- The worker now sends one-shot `UseCircuitCode` follow-up to those ports on the current simulator host.
- Even after that follow-up, LLUDP world-object ingress remains blocked:
  - `RegionHandshake = none`
  - `RegionHandshakeReply = none`
  - `ObjectUpdate* = none`
  - `update_messages=0 total_objects=0`

## Evidence-backed conclusion
- A real simulator data opening now exists, but it is currently on the simulator-host EventQueue/world-control lane rather than the LLUDP object-update lane.
- The newly proven facts are:
  - nested EventQueue body preservation matters
  - `EnableSimulator` on this path is not empty; it currently exposes port-only `SimulatorInfo`
  - one-shot `UseCircuitCode` to those ports is not sufficient by itself to unlock object updates
- The next selected branch is:
  - `docs/plans/PLAN_OBJECT_INGRESS_POST_ENABLE_SIMULATOR_SEEDCAP_2026-03-30.md`

## Risks or follow-up items
- The remaining gate is likely per-region seed-cap / simulator-host follow-up rather than another standalone first-region LLUDP packet guess.
- `EstablishAgentCommunication` may still be absent, truncated, or otherwise unsurfaced on the current bounded path.

## Learnings delta
- `added`
- Added L41 to `docs/LEARNINGS.md`.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/LEARNINGS.md`
