# Report: Startup Protocol And Social Socket Discipline (2026-03-29)

## Summary of Implemented Work
- Added bounded startup parity sends on the active first-simulator social circuit:
  - `RegionHandshakeReply`
  - `AgentThrottle`
  - one-shot `AgentUpdate`
- Corrected `RegionHandshake` decode to handle zero-coded payloads and stage a pending reply.
- Changed the live worker to use the active `SocialCircuit` for nearby-chat polling/sending instead of opening fresh handshaked UDP sockets in the worker loop.
- Added startup receive-kind relay output so live captures show the exact first-simulator packet mix.
- Attempted a follow-up LLUDP reliability-reply patch (`PacketAck`/`CompletePingCheck` handling), observed a handshake regression in live runs, and reverted that code before completion.

## Files Changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_STARTUP_PROTOCOL_PARITY_2026-03-29.md`
- `docs/reviews/REVIEW_PLAN_STARTUP_PROTOCOL_PARITY_2026-03-29.md`
- `docs/plans/PLAN_SINGLE_SOCIAL_SOCKET_DISCIPLINE_2026-03-29.md`
- `docs/reviews/REVIEW_PLAN_SINGLE_SOCIAL_SOCKET_DISCIPLINE_2026-03-29.md`
- `docs/plans/PLAN_LLUDP_RELIABILITY_REPLIES_2026-03-29.md`
- `docs/reviews/REVIEW_PLAN_LLUDP_RELIABILITY_REPLIES_2026-03-29.md`

## Validation Run
- `cargo fmt --all`: PASSED
- `cargo check -p viewer_net -p viewer_app`: PASSED
- `cargo test -p viewer_net`: PASSED
- `cargo test -p viewer_app`: PASSED
- Connected captures:
  - `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app` -> `artifacts/logs/live_startup_protocol_parity_2026-03-29_221717.log`
  - `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app` -> `artifacts/logs/live_single_social_socket_2026-03-29_222138.log`
  - reliability-reply experiment (reverted after regression):
    - `artifacts/logs/live_lludp_reliability_2026-03-29_222955.log`
    - `artifacts/logs/live_lludp_reliability_2026-03-29_223058.log`

## Result Status
- Partial improvement in code correctness:
  - startup parity sends are present
  - nearby-chat polling no longer re-handshakes fresh sockets in the live worker
  - startup kind summaries are now explicit
- Live world-object ingress is still blocked on the restored best state:
  - `update_messages=0`
  - `total_objects=0`
  - `region_handshake_updates=0`
  - startup kinds on best non-regressed run:
    - `AgentDataUpdate:1`
    - `AgentMovementComplete:1`
    - `HealthMessage:1`
    - `OnlineNotification:1`
    - `PacketAck:1`
    - `TestMessage:1`
    - `ViewerEffect:1`
- Reliability-reply experiment was not kept because it regressed startup to `handshake_complete=false`.

## Risks or Follow-up Items
- The remaining blocker is now narrower: we still need the exact missing first-simulator control reply/parity after the initial seven-packet burst.
- `TestMessage` is now a concrete live clue and should be compared against Firestorm or SL upstream handling before any further transport change.
- The reverted reliability patch should not be reintroduced without a protocol-accurate design.

## Learnings Delta
- added
  - nearby chat/live worker traffic must reuse the active social circuit, not re-handshake new sockets
  - naive LLUDP reliability-reply approximation can regress the startup handshake and must not be guessed

## Continuity Updates Performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/LEARNINGS.md`
- Added this report
