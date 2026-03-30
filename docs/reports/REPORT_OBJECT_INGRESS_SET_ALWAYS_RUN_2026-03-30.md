# Report: Object Ingress SetAlwaysRun (2026-03-30)

## Summary of Implemented Work
- Added `SetAlwaysRun` (`Low 88`) startup parity in `viewer_net`.
- Placed it in the startup control flow after `AgentUpdate`, keeping the rest of the startup path unchanged.
- Used a bounded startup value for this slice:
  - `AlwaysRun = false`
- Expanded targeted `viewer_net` coverage so startup interest tests now lock:
  - `AgentThrottle`
  - `AgentHeightWidth`
  - `AgentUpdate`
  - `SetAlwaysRun`
  ordering and the `AlwaysRun` field shape.

## Files Changed
- `crates/viewer_net/src/lib.rs`
- `docs/reports/REPORT_OBJECT_INGRESS_SET_ALWAYS_RUN_2026-03-30.md`
- `docs/reviews/REVIEW_IMPL_OBJECT_INGRESS_SET_ALWAYS_RUN_2026-03-30.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/LEARNINGS.md`
- `docs/CLUES.md`

## Validation Run
- `cargo fmt --all`: PASSED
- `cargo check -p viewer_net -p viewer_app`: PASSED
- `cargo test -p viewer_net`: PASSED
- `cargo test -p viewer_app`: PASSED
- bounded connected run `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`: PASSED

## Result Status
- The one-port retained-socket path remained intact:
  - local port `49475`
  - `split=false`
- The startup send tail now explicitly includes:
  - `m=0xffff0058` (`SetAlwaysRun`)
- Object ingress remained blocked:
  - `update_messages=0`
  - `total_objects=0`
  - `region_handshake_updates=0`
- The startup receive summary on this run was:
  - `AgentDataUpdate`
  - `AgentMovementComplete`
  - `HealthMessage`
  - `OnlineNotification`
  - `PacketAck`
  - `SimulatorViewerTimeMessage`
  - `TestMessage`
- The first steady-state window still did not show `ObjectUpdate*`; it observed more control/broader traffic including another `AgentMovementComplete`, `HealthMessage`, `OnlineNotification`, and `CoarseLocationUpdate`.

## Risks or Follow-up Items
- `SetAlwaysRun` alone is not sufficient on the current path.
- The next remaining staged candidates are:
  - `AgentAnimation`
  - a tighter ACK-timing investigation around the same control block
- The next slice should still promote only one of those directions.

## Learnings Delta
- `added`: `L36` documenting that `SetAlwaysRun` alone does not restore object ingress on the current one-port path.

## Continuity Updates Performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/LEARNINGS.md`
- Updated `docs/CLUES.md`
