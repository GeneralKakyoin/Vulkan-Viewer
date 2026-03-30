# HANDOFF: Object Ingress Parallel Protocol Startup Investigation Complete

## What Changed
- Added bounded cross-protocol diagnostics for object-ingress startup investigation.
- `viewer_net` now classifies/surfaces `RegionHandshakeReply`, `CameraConstraint`, and `GenericMessage`, and exposes URL-family summaries for seed-capability inventories.
- `viewer_app` now relays seed-cap fetches, capability-family inventory, and `EventQueueGet` scheduling alongside the existing LLUDP first-simulator summaries.
- Captured a bounded connected run plus a `tshark` extraction over `C:\\Users\\matti\\Desktop\\Firestorms.pcapng`.
- Added the next active implementation plan `docs/plans/PLAN_OBJECT_INGRESS_PERSISTENT_EVENT_QUEUE_2026-03-30.md`.

## Validation Run
- `cargo fmt --all`: PASSED
- `cargo check -p viewer_net -p viewer_app`: PASSED
- `cargo test -p viewer_net`: PASSED
- `cargo test -p viewer_app`: PASSED
- bounded connected run `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`: PASSED
- `tshark` extraction against `C:\\Users\\matti\\Desktop\\Firestorms.pcapng`: PASSED

## Exact Current State
- The current live runtime path remains a proven one-port first-simulator path.
- The viewer already has the currently confirmed startup control/request subset on-wire, including:
  - `AgentHeightWidth`
  - `AgentUpdate`
  - `AgentAnimation`
  - `SetAlwaysRun`
  - `MuteListRequest`
  - `MoneyBalanceRequest`
  - `AgentDataUpdateRequest`
- Object ingress remains blocked on that path:
  - `update_messages=0`
  - `total_objects=0`
  - `region_handshake_updates=0`
- The new cross-protocol transcript shows:
  - seed caps fetched successfully
  - capability family mix `asset-cdn:2, simhost:12043:5`
  - `EventQueueGet`, `AgentProfile`, `GetDisplayNames`, `SimulatorFeatures`, and `MapLayer` all on simulator-host `:12043`
  - one surfaced `EventQueueGet:start ack=0 ...` during the bounded run
  - no surfaced `EventQueueGet` completion, timeout, or failure before shutdown
- LLUDP object-ingress outcome is still unchanged:
  - `RegionHandshake` absent
  - `RegionHandshakeReply` absent
  - `ObjectUpdate*` absent
  - `CameraConstraint` and `GenericMessage` appear in the first steady-state window
- Firestorm source and pcap evidence now point at persistent simulator-host capability polling as the next most justified gap.

## Exact Next Step
- Implement `docs/plans/PLAN_OBJECT_INGRESS_PERSISTENT_EVENT_QUEUE_2026-03-30.md`.
- Replace the current sparse one-shot `EventQueueGet` behavior with one active persistent long-poll consumer on the fetched simulator-host URL.
- Keep the next slice bounded to `EventQueueGet` adoption and diagnostics; do not widen into new capability families yet.

## Blockers / Risks
- Do not reintroduce the earlier broad reliability-reply experiment.
- Do not stack more standalone UDP startup message guesses on top of this investigation result.
- Do not widen the next slice into broad simulator-host capability work before persistent `EventQueueGet` is validated.
- The narrowed startup-request subset is ruled out as a sufficient fix.
- `AgentHeightWidth`, `SetAlwaysRun`, and startup `AgentAnimation` are each ruled out as sufficient standalone fixes.
- Explicit ACK flush timing is ruled out as a sufficient standalone fix.
