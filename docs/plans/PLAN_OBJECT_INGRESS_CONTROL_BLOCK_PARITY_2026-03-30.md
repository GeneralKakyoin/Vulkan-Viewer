# Plan: Object Ingress Control-Block Parity (2026-03-30)

## Objective
Investigate the next post-startup object-ingress blocker by adding the Firestorm control block incrementally rather than mirroring the whole pre-burst sequence at once.

## Scope
- Use the newer Firestorm `Test.pcapng` evidence to define the working control block immediately before the first `ObjectUpdateCached` burst.
- Promote only the earliest newly confirmed missing control message in the first implementation slice.
- Keep later control messages explicitly staged behind fresh validation checkpoints.
- Do not broaden into terrain rendering, full avatar motion parity, or speculative standalone reliability scheduling.

## Current Known State
- The one-port retained-socket path is already proven in live validation.
- The viewer already sends:
  - `AgentThrottle`
  - `AgentUpdate`
  - `MuteListRequest`
  - `MoneyBalanceRequest`
  - `AgentDataUpdateRequest`
- That narrowed startup-request subset did **not** restore object ingress.
- `Test.pcapng` refines the working Firestorm sequence on `192.168.1.194:61221 <-> 16.144.39.130:13001` to:
  1. `UseCircuitCode`
  2. `CompleteAgentMovement`
  3. inbound `AgentDataUpdate`
  4. inbound `TestMessage`
  5. inbound `RegionHandshake`
  6. inbound `AgentMovementComplete`
  7. inbound `PacketAck`
  8. inbound `ViewerEffect`
  9. inbound `HealthMessage`
  10. outbound `RegionHandshakeReply`
  11. outbound `PacketAck`
  12. outbound `AgentThrottle`
  13. outbound `AgentHeightWidth`
  14. outbound `AgentUpdate`
  15. outbound `AgentAnimation`
  16. outbound `SetAlwaysRun`
  17. outbound `PacketAck`
  18. outbound `MuteListRequest`
  19. outbound `MoneyBalanceRequest`
  20. outbound `AgentDataUpdateRequest`
  21. inbound `PacketAck`
  22. inbound `ObjectUpdateCached` burst
- The new concrete packet surfaced by this capture is `AgentHeightWidth` (`Low 83`), which appears between `AgentThrottle` and `AgentUpdate` on the working Firestorm path.

## Files and Components Touched
- Planning/review artifacts under `docs/plans/` and `docs/reviews/`
- Continuity docs under `docs/CURRENT_STATE.md`, `docs/HANDOFF.md`, and `docs/CLUES.md`
- Later implementation, when promoted, is expected to touch:
  - `crates/viewer_net/src/lib.rs`
  - `crates/viewer_app/src/main.rs`

## Boundary Check
- LLUDP control-message construction and message classification belong in `viewer_net`.
- Startup orchestration belongs in `viewer_app`.
- Firestorm is used only as protocol evidence.
- No render, UI, asset-fetch, or grid-boundary changes are authorized by this plan.

## Step Sequence
1. Record the new Firestorm control block evidence from `Test.pcapng` in continuity docs so the next work does not drift back to the already-ruled-out startup subset.
2. First promotion candidate:
   - add `AgentHeightWidth` parity only
   - place it after `AgentThrottle` and before `AgentUpdate`
   - keep the existing retained-socket startup path unchanged otherwise
3. Validate live after `AgentHeightWidth` alone.
4. Second promotion candidate after that validation:
   - add `SetAlwaysRun` parity only
   - place it after `AgentUpdate`
   - keep `AgentAnimation` and ACK-timing behavior unchanged
5. Third promotion candidate after that validation:
   - add `AgentAnimation` parity only
   - place it after `AgentUpdate` and before `SetAlwaysRun`
   - use the working Firestorm startup packet shape from `Test.pcapng`
   - keep ACK-timing behavior unchanged
6. Only if object ingress still remains zero after `AgentAnimation`, revise the plan and promote the next smallest justified step from the remaining block:
   - bounded ACK-timing investigation around that same control cluster
7. After each promotion:
   - stop
   - run bounded connected validation
   - write the actual result before adding another packet

## Validation Plan
- For the planning-only step: no code validation required
- For the first implementation slice that this plan authorizes later:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_app`
  - bounded connected run: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`
- Success must still be measured by:
  - non-zero `ObjectUpdate*` / object-feed counters
  - not merely by “more packets were sent”

## Risks and Open Questions
- `AgentHeightWidth` may still be correlated rather than causal.
- Firestorm’s ACK timing around packet IDs `6..10` may matter as much as the messages themselves.
- `SetAlwaysRun` may still be correlated rather than causal, even though it is later in the working Firestorm block.
- `AgentAnimation` may also be correlated rather than causal, even when matched to the working startup packet.

## Deferred-too-Early Candidates Captured
- Full promotion of the remaining control block (any broader animation-state parity beyond the one observed startup packet, plus ACK-timing behavior) stays deferred until the `AgentAnimation` slice is validated.
- Full control-block mirroring in one step remains explicitly out of scope.

## Learnings Pre-check
- L27: do not approximate reliability behavior casually.
- L32: once one-port continuity is proven, stop spending slices on socket continuity.
- L33: receiving real simulator traffic is not the same as receiving object ingress.
- L34: the startup request subset (`MuteListRequest`, `MoneyBalanceRequest`, `AgentDataUpdateRequest`) is not sufficient on the current path.

## Completion Criteria
- The repo has a staged plan for the Firestorm control block instead of a broad “mirror more packets” idea.
- The first promotion candidate is explicit: `AgentHeightWidth` only.
- Later candidates remain deferred until fresh live validation justifies promoting them.
- The currently approved next promotion is `AgentAnimation` only.
