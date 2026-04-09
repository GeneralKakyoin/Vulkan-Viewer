# Plan: First-Simulator Socket Continuity Fix (2026-03-29)

## Objective
Preserve first-simulator UDP socket continuity from the initial handshake probe into the first long-lived social circuit so the simulator's startup object burst reaches the active receive path.

## Scope
- Update `viewer_net` to retain the probe socket after a successful first-simulator handshake probe.
- Reuse that retained socket for the first `open_social_circuit()` call instead of binding a second ephemeral port.
- Avoid redundant `UseCircuitCode` / `CompleteAgentMovement` sends when the retained socket already completed the handshake.
- Add targeted `viewer_net` tests covering retained-socket reuse and fallback behavior.
- Leave broader object decode, `EnableSimulator`, and worker-policy changes out of scope for this slice.

## Current Known State
- `docs/CLUES.md` identifies a two-socket startup path: `probe_first_simulator_handshake_window_with_policy()` binds one UDP socket, then `open_social_circuit()` later binds another.
- Firestorm capture evidence in `artifacts/logs/firestorm_agvproto_capture_2026-03-29_211128.log` shows dense `ObjectUpdate*` traffic immediately after `AgentMovementComplete`.
- Viewer logs show non-object traffic arriving while `object_updates=0`, consistent with the initial burst being lost before the long-lived receive path takes over.
- Current `viewer_net::prepare_chat_socket()` always creates a new socket and always re-sends handshake datagrams.

## Files and Components Touched
- `crates/viewer_net/src/lib.rs`
- `docs/plans/PLAN_FIRST_SIM_SOCKET_CONTINUITY_2026-03-29.md`
- `docs/reviews/REVIEW_PLAN_FIRST_SIM_SOCKET_CONTINUITY_2026-03-29.md`
- follow-up report/review/continuity artifacts after implementation

## Boundary Check
- `viewer_net` owns UDP transport, first-simulator handshake mechanics, and socket lifecycle for this flow.
- `viewer_app` orchestration remains unchanged unless implementation reveals a wiring gap that cannot be handled inside `viewer_net`.
- No `viewer_grid`, renderer, UI, or asset boundary changes are allowed in this slice.

## Step Sequence
1. Add a retained first-simulator socket field owned by `Connection` and clear it on login reset/disconnect.
2. Update `probe_first_simulator_handshake_window_with_policy()` to stash the probe socket after the receive loop only when the handshake reached `AgentMovementComplete`.
3. Refactor `prepare_chat_socket()` / `open_social_circuit()` so the first social circuit takes the retained socket when present; otherwise it keeps the existing bind-and-handshake path.
4. Skip duplicate handshake sends when a retained probe socket is reused, while preserving the existing re-handshake behavior for freshly bound social sockets.
5. Add targeted `viewer_net` tests proving socket reuse (same local sender port, no extra handshake datagrams) and fallback to a fresh socket when no retained probe exists.
6. Validate with required formatting/check/test commands.

## Validation Plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`

## Risks and Open Questions
- If the retained socket should also be reused by nearby-chat polling later, that is a separate design choice and is out of scope here.
- If live object ingress still remains zero after this fix, the next blocker is likely in downstream receive/drain policy rather than startup socket continuity.
- Assumption: only the first long-lived social circuit needs the retained probe socket; later reopen paths may legitimately bind a new socket.

## Deferred-too-Early Candidates Captured
- None. No additional deferred feature was identified beyond this bounded transport fix.

## Learnings Pre-check
- L05: keep unknown packet diagnostics intact while touching first-simulator traffic paths.
- L10: do not change LLUDP message IDs without template evidence.
- L23: success for this slice must be judged by non-zero object ingress evidence, not by retarget or handshake diagnostics alone.

## Completion Criteria
- `viewer_net` retains and reuses the probe socket for the first social circuit after a successful probe.
- Redundant handshake sends are skipped on the reused socket path.
- Targeted tests cover the retained-socket and fresh-socket behaviors.
- Validation commands complete successfully and continuity/report artifacts record the exact outcome.
