# Report: Object Ingress AgentAnimation (2026-03-30)

## Summary of Implemented Work
- Added `AgentAnimation` (`High 5`) startup parity in `viewer_net`.
- Placed it in the startup control flow after `AgentUpdate` and before `SetAlwaysRun`.
- Used the working Firestorm startup packet shape from `Test.pcapng` for this bounded slice:
  - one `AnimationList` entry
  - `AnimID = ANIM_AGENT_DO_NOT_DISTURB`
  - `StartAnim = false`
  - one empty `PhysicalAvatarEventList` entry
- Expanded targeted `viewer_net` coverage so startup interest tests now lock:
  - `AgentThrottle`
  - `AgentHeightWidth`
  - `AgentUpdate`
  - `AgentAnimation`
  - `SetAlwaysRun`

## Files Changed
- `crates/viewer_net/src/lib.rs`
- `docs/reports/REPORT_OBJECT_INGRESS_AGENT_ANIMATION_2026-03-30.md`
- `docs/reviews/REVIEW_IMPL_OBJECT_INGRESS_AGENT_ANIMATION_2026-03-30.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/LEARNINGS.md`
- `docs/CLUES.md`

## Validation Run
- `cargo fmt --all`: PASSED
- `cargo check -p viewer_net -p viewer_app`: PASSED
- `cargo test -p viewer_net`: PASSED
- `cargo test -p viewer_app`: PASSED
- bounded connected run `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`: LOG CAPTURED; tool timeout hit before self-exit, then spawned `viewer_app` process was stopped explicitly

## Result Status
- The one-port retained-socket path remained intact:
  - local port `59553`
  - `split=false`
- Startup send count increased again:
  - `sends=11`
- Object ingress remained blocked:
  - `update_messages=0`
  - `total_objects=0`
  - `region_handshake_updates=0`
- The startup receive summary on this run was:
  - `AgentDataUpdate`
  - `AgentMovementComplete`
  - `AttachedSound`
  - `HealthMessage`
  - `OnlineNotification`
  - `PacketAck`
  - `TestMessage`
- The first steady-state window still did not show `ObjectUpdate*`.

## Risks or Follow-up Items
- `AgentAnimation` alone is not sufficient on the current path.
- The staged control-block message promotions are now exhausted:
  - `AgentHeightWidth`
  - `SetAlwaysRun`
  - `AgentAnimation`
- The next remaining direction should be a tighter ACK-timing / control-reply investigation rather than another single message addition.

## Learnings Delta
- `added`: `L37` documenting that the observed startup `AgentAnimation` packet alone does not restore object ingress on the current one-port path.

## Continuity Updates Performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/LEARNINGS.md`
- Updated `docs/CLUES.md`
