# Report: Object Ingress Network Debug And Decision Acceleration (2026-03-30)

## Summary of implemented work
- Added a typed `NetworkDebugState` surface in `viewer_core` so the app and UI can share bounded network-debug sections without leaking transport internals into presentation code.
- Added a dedicated `Network Debug` egui window in `viewer_ui` that shows bounded session, capability/EventQueue, LLUDP, follow-up, and recent-network-event sections.
- Extended `viewer_app` to:
  - aggregate existing relay categories into the new network-debug sections
  - keep a bounded recent-event tail for the window
  - append network-focused relay events to a dedicated JSONL log file
  - refresh a session snapshot section every frame so the window reflects current startup/object-ingress state
- Reused the new surface in one bounded live run with an explicit log path override to confirm that the network-debug feed is populated during a real login session.

## Files changed
- `crates/viewer_core/src/lib.rs`
- `crates/viewer_ui/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/TESTING_REFERENCE.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`

## Validation run
- `cargo fmt --all`: PASSED
- `cargo check -p viewer_core -p viewer_app -p viewer_ui -p viewer_net`: PASSED
- `cargo test -p viewer_core`: PASSED
- `cargo test -p viewer_ui`: PASSED
- `cargo test -p viewer_net`: PASSED
- `cargo test -p viewer_app`: PASSED
- bounded connected run with explicit network-debug log override:
  - command: `VIEWER_APP_LIVE_STARTUP=on VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_2026-03-30.jsonl cargo run -p viewer_app`
  - result: PASSED for evidence capture; process intentionally stopped after 45 seconds
  - artifacts:
    - `artifacts/logs/live_network_debug_window_2026-03-30.out.log`
    - `artifacts/logs/live_network_debug_window_2026-03-30.err.log`
    - `artifacts/logs/network_debug_2026-03-30.jsonl`

## Result status
- The repo now has a bounded in-app network-debug surface backed by typed state instead of ad hoc UI wiring.
- The repo now persists network-debug relay events to JSONL through `VIEWER_NETWORK_DEBUG_LOG_PATH` or the default `logs/network_debug.jsonl`.
- The bounded live run produced `39` network-debug log entries covering:
  - retained first-simulator socket continuity
  - startup and steady-state LLUDP forensics
  - simulator-host capability inventory on `:12043`
  - `EventQueueGet` startup and repeated EventQueue responses
  - repeated `EnableSimulator` details and follow-up sends
- The new surface did not change the object-ingress truth:
  - `RegionHandshake = none`
  - `RegionHandshakeReply = none`
  - `ObjectUpdate* = none`
  - `update_messages=0`
  - `total_objects=0`

## Evidence-backed conclusion
- The new debug surface is working as an accelerator, not as a fix.
- In the bounded live run, the dedicated log clearly showed repeated EventQueue completions (`ack_out` reached `4`) and repeated `EnableSimulator` details/follow-up while LLUDP object ingress stayed at zero.
- The next branch should stay on `docs/plans/PLAN_OBJECT_INGRESS_POST_ENABLE_SIMULATOR_SEEDCAP_2026-03-30.md`, using the new window/log to prove whether per-region seed-cap or `EstablishAgentCommunication` evidence is still missing.

## Risks or follow-up items
- Direct human visual inspection of the new `Network Debug` window was not completed from the terminal-only workflow; runtime launch and the backing data/log feed were validated instead.
- Do not broaden this surface into packet inspection or generalized telemetry before it drives the next concrete object-ingress decision.

## Learnings delta
- `none`
- No durable protocol learning was identified; this slice added observability and operator speed rather than a new simulator-behavior fact.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/TESTING_REFERENCE.md`
