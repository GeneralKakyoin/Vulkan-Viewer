# Plan: Object Ingress ACK-Trailer Parity (2026-03-29)

## Objective
Restore the missing reliable-transport parity most likely gating first-simulator object ingress by collecting reliable inbound packet IDs and appending ACK trailers to subsequent outbound first-simulator datagrams on the active social circuit.

## Scope
- Add bounded LLUDP packet-flag/trailer parsing for first-simulator traffic in `viewer_net`.
- Queue reliable inbound packet IDs observed on the first-simulator path.
- Append queued ACK IDs to subsequent outbound first-simulator datagrams in Firestorm-compatible trailer form.
- Strip inbound ACK trailers from message-body decode slices so object/world decode remains correct once the simulator starts appending ACKs.
- Keep the change inside `viewer_net`; do not broaden into app/UI/render changes.

## Current Known State
- The retained first-simulator socket, startup receive activation, single-social-socket discipline, startup `AgentThrottle`/reliable `AgentUpdate`, and recurring non-reliable `AgentUpdate` cadence are already in place.
- The latest connected capture in `artifacts/logs/live_agent_update_cadence_2026-03-29_231942.log` still shows `update_messages=0 total_objects=0 handshake_complete=true traffic_obs=7 region_handshake_updates=0`.
- Firestorm source appends queued ACK IDs onto outgoing packets (`reference/firestorm/indra/llmessage/message.cpp`, send path around ACK packing) and collects reliable inbound packet IDs for later ACK emission.
- Firestorm object-ingress capture shows `agent_throttle_send` and a reliable `agent_update_send` immediately before a large `recv_object_update` burst in `artifacts/logs/firestorm_agvproto_capture_2026-03-29_211128.log`.
- A prior naive experiment that added standalone `PacketAck` / `CompletePingCheck` replies regressed startup handshake completion and was reverted.

## Files and Components Touched
- `crates/viewer_net/src/lib.rs`
- `docs/plans/DEFERRED_FEATURES.md`
- continuity artifacts under `docs/reviews/`, `docs/reports/`, `docs/CURRENT_STATE.md`, `docs/HANDOFF.md`, and `docs/LEARNINGS.md`

## Boundary Check
- LLUDP flag/trailer handling and reliable ACK propagation are transport/session mechanics and belong in `viewer_net`.
- No grid semantics move into `viewer_net`.
- No changes to crate ownership or app/render/UI boundaries.

## Step Sequence
1. Add a bounded LLUDP packet-metadata helper that exposes:
   - packet flags
   - packet ID
   - decoded message number
   - effective body end excluding ACK trailer bytes when present
2. Update first-simulator inbound observation so valid reliable inbound packets queue their packet IDs for ACK, while explicitly excluding `PacketAck` packets from re-ack collection.
3. Add bounded outbound helper logic that appends queued ACK IDs to outgoing first-simulator datagrams using Firestorm-compatible trailer layout:
   - set `LL_ACK_FLAG`
   - append network-order packet IDs
   - append trailing ACK-count byte
4. Route first-simulator sends that already use `send_first_simulator_handshake_datagram_with_socket(...)` through that ACK-trailer helper so startup/social-circuit messages can carry queued ACKs without adding a separate scheduler.
5. Update first-simulator decode helpers to respect the effective body end so appended inbound ACK trailers do not corrupt object, region, or social decoders once broader traffic starts flowing.
6. Add targeted tests for:
   - ACK-trailer parsing
   - reliable inbound ACK queuing
   - outbound ACK-trailer attachment
   - body decode with ACK trailers present
7. Run a bounded connected capture to verify whether first-simulator packet flow broadens into `RegionHandshake` and/or `ObjectUpdate*`.

## Validation Plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- connected capture: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app` with bounded log collection

## Risks and Open Questions
- If object ingress still remains zero after ACK-trailer parity, the remaining gap is likely a different specific post-`AgentMovementComplete` startup message or control behavior rather than socket continuity.
- Reliable ACK collection must stay bounded; this slice does not add full resend windows, duplicate suppression, or generalized circuit scheduling.
- Standalone `PacketAck` flushing and `StartPingCheck`/`CompletePingCheck` parity are intentionally excluded here because the earlier naive attempt regressed startup and Firestorm’s first object burst appears before ping traffic becomes relevant.

## Deferred-too-Early Candidates Captured
- Standalone `PacketAck` flush scheduling and `StartPingCheck` / `CompletePingCheck` reply parity are deferred in `docs/plans/DEFERRED_FEATURES.md` until ACK-trailer parity is proven insufficient or live evidence shows ping traffic on the blocked path.

## Learnings Pre-check
- L10: use authoritative LLUDP flag/message references, not memory.
- L23: judge success by actual object ingress, not by “more protocol code exists.”
- L24: preserve the first retained socket across all first-simulator sends/receives.
- L25: a completed handshake alone is not enough; verify packet-flow change.
- L27: do not retry naive standalone reliability replies casually.
- L29: do not spend another slice on `AgentUpdate` cadence alone.

## Completion Criteria
- Reliable inbound packet IDs observed on the first-simulator path are queued and attached to subsequent outbound first-simulator datagrams in Firestorm-compatible ACK-trailer form.
- First-simulator decode helpers tolerate inbound ACK trailers without corrupting body parsing.
- Connected validation either restores broader first-simulator flow (`RegionHandshake` and/or `ObjectUpdate*`) or leaves a tighter, evidence-backed remaining blocker.
