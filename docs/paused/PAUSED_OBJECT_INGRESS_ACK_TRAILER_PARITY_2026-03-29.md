# Paused Work: Object Ingress ACK-Trailer Parity (2026-03-29)

## Why This Exists
- Work was intentionally stopped before transport code changes at the user’s request.
- This note captures exactly what the next implementation slice was going to do so it can be resumed cleanly later.

## Status At Stop
- No transport/runtime code was changed for this slice.
- Planning artifacts for the slice were created:
  - `docs/plans/PLAN_OBJECT_INGRESS_ACK_TRAILER_PARITY_2026-03-29.md`
  - `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_ACK_TRAILER_PARITY_2026-03-29.md`
- `docs/plans/DEFERRED_FEATURES.md` was updated to defer standalone `PacketAck` flush scheduling and `StartPingCheck` / `CompletePingCheck` parity until stronger evidence exists.

## Problem Snapshot
- Current best live state still reaches `handshake_complete=true` but never receives `RegionHandshake` or `ObjectUpdate*`.
- Firestorm’s capture shows:
  - `agent_movement_complete`
  - `agent_throttle_send`
  - reliable `agent_update_send`
  - immediate `recv_object_update` burst
- The earlier naive standalone reliability-reply experiment regressed startup and was reverted, so the next slice was narrowed to a safer transport-only parity target.

## What I Was Going To Implement
1. Add bounded LLUDP packet-metadata parsing in `crates/viewer_net/src/lib.rs` for:
   - packet flags
   - packet ID
   - message number
   - effective body end when an ACK trailer is present
2. Queue reliable inbound first-simulator packet IDs for ACK when packets arrive on the retained active circuit.
3. Append queued ACK IDs to subsequent outbound first-simulator datagrams in Firestorm-compatible trailer form:
   - set `LL_ACK_FLAG`
   - append network-order packet IDs
   - append the trailing ACK-count byte
4. Route existing first-simulator sends through that helper so startup/social-circuit messages can carry ACKs automatically.
5. Update body-slicing decode helpers so inbound ACK trailers do not corrupt object or social payload decoding once broader traffic starts flowing.
6. Add targeted tests for:
   - ACK-trailer parsing
   - reliable inbound ACK queuing
   - outbound ACK-trailer attachment
   - body decode when ACK trailers are present

## Files I Expected To Touch
- `crates/viewer_net/src/lib.rs`
- continuity docs after implementation/validation:
  - `docs/reports/`
  - `docs/CURRENT_STATE.md`
  - `docs/HANDOFF.md`
  - `docs/LEARNINGS.md`

## Why This Slice Looked Better Than The Old One
- It stays inside `viewer_net` transport mechanics.
- It avoids immediately retrying standalone `PacketAck` / ping replies, which already caused a startup regression.
- It matches Firestorm’s send-path behavior more closely: collect reliable inbound ACKs, then piggyback them onto outgoing traffic.

## What Was Explicitly Deferred
- Standalone `PacketAck` flush scheduling.
- `StartPingCheck` / `CompletePingCheck` reply parity.
- Any wider resend-window or duplicate-suppression system.

## Resume Checklist
- Re-read:
  - `docs/HANDOFF.md`
  - `docs/plans/PLAN_OBJECT_INGRESS_ACK_TRAILER_PARITY_2026-03-29.md`
  - `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_ACK_TRAILER_PARITY_2026-03-29.md`
  - `docs/LEARNINGS.md` entries `L10`, `L23`, `L24`, `L25`, `L27`, `L29`
- Implement only the ACK-trailer parity slice.
- Validate with:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_app`
  - bounded connected capture: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`

## Expected Success Signal
- Connected capture broadens beyond the current seven startup kinds and begins showing `RegionHandshake` and/or `ObjectUpdate*`.
- If not, the remaining blocker is still transport/protocol parity, but this particular ACK path can be ruled in or out with evidence.
