# Report: Object Ingress ACK Flush Timing (2026-03-30)

## Summary of Implemented Work
- Added an explicit `PacketAck` encoder in `viewer_net`.
- Added a bounded `flush_pending_ack_ids_on_circuit(...)` helper for the active first-simulator `SocialCircuit`.
- Triggered that helper from `viewer_app` at the approved checkpoints:
  - after startup social-prime work
  - before the first steady-state forensic summary
- Added a targeted `viewer_net` test proving explicit `PacketAck` send shape and queue drain behavior.

## Files Changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_OBJECT_INGRESS_POST_ACK_UNKNOWN_CLASSIFICATION_2026-03-30.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_POST_ACK_UNKNOWN_CLASSIFICATION_2026-03-30.md`
- `docs/reports/REPORT_OBJECT_INGRESS_ACK_FLUSH_TIMING_2026-03-30.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/LEARNINGS.md`

## Validation Run
- `cargo fmt --all`: PASSED
- `cargo check -p viewer_net -p viewer_app`: PASSED
- `cargo test -p viewer_net`: PASSED
- `cargo test -p viewer_app`: PASSED
- bounded connected run with captured log: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`

## Result Status
- The ACK flush timing change succeeded mechanically:
  - startup pending ACK queue reached `0`
  - first steady-state window flushed `3` pending ACK IDs
  - first steady-state summary ended with `pending=0`
  - explicit outbound `PacketAck` was observed on-wire
- Object ingress remained blocked:
  - `update_messages=0`
  - `total_objects=0`
  - `region_handshake_updates=0`
- New post-ACK-flush clue surfaced:
  - previously unclassified `0x00000016`
  - previously unclassified `0xffff0105`
- Firestorm message-template lookup identifies them as:
  - `CameraConstraint` (High 22)
  - `GenericMessage` (Low 261)

## Risks or Follow-up Items
- Explicit ACK flush timing is not sufficient by itself to restore object ingress.
- The next active plan is `docs/plans/PLAN_OBJECT_INGRESS_POST_ACK_UNKNOWN_CLASSIFICATION_2026-03-30.md`.
- `GenericMessage` may require payload-level inspection after basic classification.

## Learnings Delta
- added
  - explicit ACK flush timing can drain the queue without restoring object ingress

## Continuity Updates Performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/LEARNINGS.md`
- Added this report
