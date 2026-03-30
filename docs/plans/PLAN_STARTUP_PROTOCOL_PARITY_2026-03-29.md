# Plan: Startup Protocol Parity Repair (2026-03-29)

## Objective
Restore first-region object ingress by adding the missing startup protocol actions that occur after initial simulator handshake, using repo evidence plus Firestorm behavioral reference.

## Scope
- Add bounded support for `RegionHandshakeReply`, `AgentThrottle`, and `AgentUpdate` on the first-simulator social circuit.
- Trigger those startup messages early in the live worker after the retained social circuit is active.
- Preserve existing socket continuity and startup-drain behavior.
- Keep the change within `viewer_net` transport helpers and `viewer_app` startup orchestration.

## Current Known State
- `docs/CLUES.md` correctly identified the original probe-to-social-socket handoff bug; that bug is now fixed.
- Live validation after the socket fix still shows `handshake_complete=true` with `update_messages=0 total_objects=0` and `region_handshake_updates=0`.
- Firestorm capture at `artifacts/logs/firestorm_agvproto_capture_2026-03-29_211128.log` shows `agent_throttle_send` and `agent_update_send` immediately around `agent_movement_complete`, followed by a large `recv_object_update` burst.
- The authoritative local message template at `reference/firestorm/scripts/messages/message_template.msg` states:
  - `RegionHandshakeReply` is viewer -> sim and “After the simulator receives this, it will start sending data about objects.”
  - `AgentUpdate` informs the simulator of camera / interest-list state.
- Current repo code classifies inbound `RegionHandshake` but does not send `RegionHandshakeReply`, `AgentThrottle`, or `AgentUpdate`.

## Files and Components Touched
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- continuity artifacts under `docs/reviews/`, `docs/reports/`, `docs/CURRENT_STATE.md`, `docs/HANDOFF.md`, and `docs/LEARNINGS.md`

## Boundary Check
- `viewer_net` owns LLUDP packet encoding/sending and first-simulator circuit behavior.
- `viewer_app` owns startup sequencing and worker orchestration.
- No `viewer_grid`, renderer, UI, or asset boundary changes are permitted.

## Step Sequence
1. Add `viewer_net` encoders/senders for `RegionHandshakeReply`, `AgentThrottle`, and a bounded `AgentUpdate`.
2. Add a bounded startup helper in `viewer_net` or `viewer_app` usage path that sends throttle/update once the social circuit is active.
3. Extend startup receive handling so an observed `RegionHandshake` can trigger `RegionHandshakeReply` on the active circuit without adding a second receive path.
4. Keep later worker polling and diagnostics on the existing social-circuit path.
5. Add targeted tests for the new packet encoders/startup send behavior.
6. Run connected validation and confirm whether object-feed counters become non-zero.

## Validation Plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- connected capture: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app` with bounded log collection under `artifacts/logs/`

## Risks and Open Questions
- `RegionHandshake` may still not arrive on the current path; if so, sending `AgentThrottle`/`AgentUpdate` first may still be necessary to elicit it.
- `AgentUpdate` cadence in the full viewer is continuous; this slice only adds a bounded startup send and should not pretend to implement full movement parity.
- `AgentThrottle` payload values should stay conservative and Firestorm-like enough to avoid inventing new behavior.

## Deferred-too-Early Candidates Captured
- None. Full continuous `AgentUpdate` movement loop parity remains outside this bounded startup repair.

## Learnings Pre-check
- L05: keep unknown traffic visible while adjusting startup message flow.
- L10: source LLUDP message IDs from the message template, not memory.
- L23: retarget/socket fixes are insufficient unless live object ingress actually appears.
- L24: preserve startup socket continuity across probe and long-lived receive.
- L25: once socket continuity is fixed, compare current post-AMC parity against Firestorm rather than guessing broadly.

## Completion Criteria
- The live worker sends the missing bounded startup parity messages on the active first-simulator circuit.
- `viewer_net` has tests covering the new message encoders/startup send path.
- Connected verification either shows non-zero `object_feed` counters or produces tighter protocol evidence about what remains missing.
