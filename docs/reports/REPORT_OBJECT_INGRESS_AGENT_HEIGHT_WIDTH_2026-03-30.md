# Report: Object Ingress AgentHeightWidth (2026-03-30)

## Summary of Implemented Work
- Added `AgentHeightWidth` (`Low 83`) startup parity in `viewer_net`.
- Placed it in the startup control block after `AgentThrottle` and before `AgentUpdate`, matching the current staged plan.
- Used a bounded startup default payload for this slice:
  - `GenCounter = 0`
  - `Height = 720`
  - `Width = 1280`
- Expanded targeted `viewer_net` coverage so startup interest tests now lock:
  - `AgentThrottle`
  - `AgentHeightWidth`
  - `AgentUpdate`
  ordering and core field layout.

## Files Changed
- `crates/viewer_net/src/lib.rs`
- `docs/reports/REPORT_OBJECT_INGRESS_AGENT_HEIGHT_WIDTH_2026-03-30.md`
- `docs/reviews/REVIEW_IMPL_OBJECT_INGRESS_AGENT_HEIGHT_WIDTH_2026-03-30.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/LEARNINGS.md`
- `docs/CLUES.md`

## Validation Run
- `cargo fmt --all`: PASSED
- `cargo check -p viewer_net -p viewer_app`: PASSED
- `cargo test -p viewer_net`: PASSED
- `cargo test -p viewer_app`: PASSED
- bounded connected run `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`: BEHAVIOR OBSERVED, but the command hit the tool timeout before self-exit; log evidence was collected and the launched `viewer_app` process was then stopped explicitly

## Result Status
- The one-port retained-socket path remained intact:
  - local port `63178`
  - `split=false`
- Startup behavior changed only modestly:
  - startup send count increased from `8` to `9`
  - startup receive summary now included `CoarseLocationUpdate:1` on this run
- Object ingress remained blocked:
  - `update_messages=0`
  - `total_objects=0`
  - `region_handshake_updates=0`
- No `ObjectUpdate*` appeared in the startup or first steady-state summaries.

## Risks or Follow-up Items
- `AgentHeightWidth` alone is not sufficient on the current path.
- The remaining control-block candidates are still:
  - `AgentAnimation`
  - `SetAlwaysRun`
  - adjacent ACK-timing behavior
- The next slice should still promote only one additional control behavior at a time.

## Learnings Delta
- `added`: `L35` documenting that `AgentHeightWidth` alone does not restore object ingress on the current one-port path.

## Continuity Updates Performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/LEARNINGS.md`
- Updated `docs/CLUES.md`
