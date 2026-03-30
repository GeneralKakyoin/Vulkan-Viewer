# Report: Object Ingress ACK-Trailer Parity (2026-03-29)

## Summary of Implemented Work
- Added first-simulator LLUDP packet metadata parsing in `viewer_net` so decode paths can see flags, packet IDs, and body bounds excluding ACK trailers.
- Queued reliable inbound first-simulator packet IDs for later ACK emission, excluding inbound `PacketAck`.
- Appended bounded Firestorm-style ACK trailers to outbound first-simulator datagrams sent through the existing socket send helper.
- Updated first-simulator body decoders to ignore appended ACK trailers.
- Added targeted tests for ACK-trailer parsing, reliable ACK queuing, ACK-trailer attachment on outbound send, and decode correctness with ACK trailers present.

## Files Changed
- `crates/viewer_net/src/lib.rs`
- `docs/reviews/REVIEW_IMPL_OBJECT_INGRESS_ACK_TRAILER_PARITY_2026-03-29.md`
- `docs/reports/REPORT_OBJECT_INGRESS_ACK_TRAILER_PARITY_2026-03-29.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/LEARNINGS.md`

## Validation Run
- `cargo fmt --all` — PASSED
- `cargo check -p viewer_net -p viewer_app` — PASSED
- `cargo test -p viewer_net` — PASSED
- `cargo test -p viewer_app` — PASSED
- `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app` — PASSED as a bounded connected run

## Result Status
- Implementation status: complete for the approved ACK-trailer parity scope.
- Connected outcome: object ingress remains blocked.
- Live stdout summary from the bounded run remained:
  - `update_messages=0`
  - `total_objects=0`
  - `handshake_complete=true`
  - `region_handshake_updates=0`
  - startup kinds unchanged at `AgentDataUpdate`, `AgentMovementComplete`, `HealthMessage`, `OnlineNotification`, `PacketAck`, `TestMessage`, and `ViewerEffect`

## Risks or Follow-up Items
- ACK-trailer parity was a real missing transport behavior, but it was not the missing behavior that restores `RegionHandshake` or `ObjectUpdate*`.
- The next bounded slice should not revisit `AgentUpdate` cadence or socket continuity; both are already validated as insufficient on their own.
- Standalone `PacketAck` scheduling and ping reply behavior remain deferred because earlier naive approximations regressed startup.

## Learnings Delta
- added: this slice produced a new durable learning that ACK-trailer parity alone does not unblock object ingress once the retained-socket and startup-interest baseline is already present.

## Continuity Updates Performed
- Added implementation review: `docs/reviews/REVIEW_IMPL_OBJECT_INGRESS_ACK_TRAILER_PARITY_2026-03-29.md`
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/LEARNINGS.md`
