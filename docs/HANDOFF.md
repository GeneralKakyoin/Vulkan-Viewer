# HANDOFF: Object Ingress AgentAnimation Complete

## What Changed
- Added `AgentAnimation` (`High 5`) startup parity in `viewer_net`.
- Placed it after `AgentUpdate` and before `SetAlwaysRun` in the startup interest flow.
- Retained the earlier `AgentHeightWidth` and `SetAlwaysRun` startup parity.
- Added targeted `viewer_net` assertions for the ordered startup quintet:
  - `AgentThrottle`
  - `AgentHeightWidth`
  - `AgentUpdate`
  - `AgentAnimation`
  - `SetAlwaysRun`

## Validation Run
- `cargo fmt --all`: PASSED
- `cargo check -p viewer_net -p viewer_app`: PASSED
- `cargo test -p viewer_net`: PASSED
- `cargo test -p viewer_app`: PASSED
- bounded connected run `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`: LOG CAPTURED; tool timeout hit before self-exit, then spawned `viewer_app` process was stopped explicitly

## Exact Current State
- The current live runtime path still did **not** split across multiple local UDP ports.
- The bounded March 30, 2026 `AgentAnimation` run used one shared local port `59553` at:
  - `after_probe`
  - `after_open_social_circuit`
  - `after_startup_social_prime`
  - `after_first_steady_state_window`
- All of those summaries reported `split=false`.
- The startup control flow now includes:
  - `AgentHeightWidth`
  - `AgentUpdate`
  - `AgentAnimation`
  - `SetAlwaysRun`
- Object ingress is still blocked on that one-port path:
  - `update_messages=0`
  - `total_objects=0`
  - `region_handshake_updates=0`
- The startup receive summary remained:
  - `AgentDataUpdate`
  - `AgentMovementComplete`
  - `AttachedSound`
  - `HealthMessage`
  - `OnlineNotification`
  - `PacketAck`
  - `TestMessage`
- No `ObjectUpdate*` appeared.

## Exact Next Step
- Implement `docs/plans/PLAN_OBJECT_INGRESS_ACK_AND_RECEIVE_FORENSICS_2026-03-30.md`.
- The next slice should add bounded observability for:
  - ACK/control timing
  - receive-path surfacing
  - `RegionHandshake` first-observation tracking
  - ordered startup transcript comparison
- Do not add more standalone startup messages before that observability slice is complete.

## Blockers / Risks
- Do not spend another slice on socket continuity alone unless new live evidence contradicts the new diagnostics.
- The narrowed startup-request subset is ruled out as a sufficient fix.
- `AgentHeightWidth` is now also ruled out as a sufficient fix.
- `SetAlwaysRun` is now also ruled out as a sufficient fix.
- `AgentAnimation` is now also ruled out as a sufficient fix.
- The remaining blocker is now more likely in ACK/control-reply behavior than in another missing standalone startup message.
