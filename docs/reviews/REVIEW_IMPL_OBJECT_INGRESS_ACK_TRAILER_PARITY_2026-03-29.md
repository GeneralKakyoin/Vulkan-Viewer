# Review: Object Ingress ACK-Trailer Parity (2026-03-29)

## Verdict
Approved as implemented.

## Architecture and Boundary Fit
- The change stays inside `viewer_net` transport mechanics.
- ACK-trailer parsing, reliable inbound ACK collection, and outbound ACK attachment are correctly kept out of `viewer_app`, `viewer_core`, and render/UI crates.

## Correctness Concerns
- Inbound first-simulator packet parsing now exposes flags, packet ID, and an effective body range that excludes appended ACK trailers.
- Reliable inbound packets queue ACK IDs, while inbound `PacketAck` packets are excluded from re-ack collection.
- Outbound first-simulator datagrams append bounded ACK trailers in Firestorm-compatible layout and drain only the ACK IDs that were actually attached.
- The initial failing test exposed an important sequencing detail: handshake packets sent during `open_social_circuit()` legitimately consume queued ACKs before a later `AgentUpdate`. The test was corrected to seed pending ACKs after circuit open.

## Modularity and Maintainability Concerns
- The shared `FirstSimulatorPacketHeader` helper keeps trailer math centralized instead of scattering body-slice logic across decoders.
- Routing ACK attachment through `send_first_simulator_handshake_datagram_with_socket(...)` keeps later first-simulator sends on one transport path.

## Validation Adequacy
- `cargo fmt --all`: passed
- `cargo check -p viewer_net -p viewer_app`: passed
- `cargo test -p viewer_net`: passed
- `cargo test -p viewer_app`: passed
- bounded connected run `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`: passed as a command and reproduced the same blocked object-ingress state

## Risks and Open Questions
- Connected behavior is unchanged after ACK-trailer parity: `update_messages=0`, `total_objects=0`, `region_handshake_updates=0`, `handshake_complete=true`.
- The remaining blocker is therefore deeper than missing ACK trailers on existing outbound packets.
- Standalone `PacketAck` scheduling and ping parity remain deferred until stronger live evidence points there.

## Learnings Delta Verdict
- add: ACK-trailer parity alone did not restore object ingress once startup socket continuity and startup interest sends were already in place.

## Required Revisions or Approval Status
- No implementation revisions required for this slice.
- Next slice should target a different, evidenced post-`AgentMovementComplete` control/parity gap.
