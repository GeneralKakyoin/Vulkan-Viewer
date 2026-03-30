# Plan: Single Social Socket Discipline (2026-03-29)

## Objective
Stop `viewer_app`/`viewer_net` from repeatedly re-targeting the first simulator onto fresh UDP sockets during normal social and nearby-chat polling, so the retained startup circuit can actually receive the region/object feed it negotiated.

## Scope
- Replace fresh-socket nearby chat polling/sending on the live worker path with reuse of the active `SocialCircuit`.
- Keep the existing retained-socket startup activation and startup parity sends in place.
- Preserve bounded behavior: no broad chat subsystem redesign, no crate-boundary changes beyond `viewer_net` helpers and `viewer_app` orchestration.

## Current Known State
- Live capture after the startup-parity patch still shows `update_messages=0 total_objects=0`.
- The new startup diagnostics in `artifacts/logs/live_startup_protocol_parity_2026-03-29_221717.log` show only `AgentDataUpdate`, `AgentMovementComplete`, `HealthMessage`, `OnlineNotification`, `PacketAck`, `TestMessage`, and `ViewerEffect` on the startup receive path.
- `viewer_app` currently calls `connection.poll_nearby_chat_udp(...)` every worker tick.
- `viewer_net::poll_nearby_chat_udp(...)` calls `prepare_chat_socket(...)`, which binds a new socket and re-sends `UseCircuitCode` + `CompleteAgentMovement`.
- `viewer_net::send_nearby_chat(...)` also uses `prepare_chat_socket(...)`, so chat sends can create the same re-targeting side effect.

## Files and Components Touched
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- continuity artifacts under `docs/reviews/`, `docs/reports/`, `docs/CURRENT_STATE.md`, `docs/HANDOFF.md`, and `docs/LEARNINGS.md`

## Boundary Check
- `viewer_net` owns the socket-level mechanics for nearby/social LLUDP traffic.
- `viewer_app` owns which circuit is used by the live worker loop.
- No `viewer_grid`, renderer, UI, or asset ownership changes are allowed.

## Step Sequence
1. Add `viewer_net` helpers that poll/send nearby chat on an existing `SocialCircuit` without creating a fresh handshaked socket.
2. Update `viewer_app` live worker paths to use the active social circuit for nearby chat polling and nearby chat sends.
3. Leave legacy/fallback helper paths in place only where they are not part of the continuous live worker loop.
4. Re-run targeted tests and bounded live capture to confirm the simulator is no longer being re-targeted away from the retained startup socket.

## Validation Plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- connected capture: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app` with bounded log collection under `artifacts/logs/`

## Risks and Open Questions
- If object ingress still remains zero after single-socket discipline is restored, the remaining gap is likely another protocol control reply rather than socket churn.
- Nearby chat send/poll behavior must still remain functional when the social circuit is absent or being reopened.

## Deferred-too-Early Candidates Captured
- None. This is a correctness repair for the existing live worker path, not a new feature.

## Learnings Pre-check
- L05: keep unknown traffic visible while changing receive paths.
- L10: use message-template-backed LLUDP behavior, not memory.
- L23: success is non-zero live object ingress, not just a completed handshake.
- L24: preserve startup socket continuity across every first-simulator receive path, not just probe -> social open.
- L25: once startup continuity is repaired, compare actual packet flow and follow the evidence.

## Completion Criteria
- The live worker no longer creates fresh handshaked nearby-chat sockets during normal connected operation.
- Nearby chat poll/send in the live worker reuse the active `SocialCircuit`.
- Connected validation either shows object-feed ingress improving or produces a tighter remaining blocker after socket churn is removed.
