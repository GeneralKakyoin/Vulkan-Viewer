# Report: Object Ingress AgentUpdate Cadence (2026-03-29)

## Summary of Implemented Work
- Added a reusable `viewer_net::Connection::send_agent_update_on_circuit(...)` helper so `AgentUpdate` can be sent on an existing first-simulator `SocialCircuit` outside the startup-only helper.
- Refactored `send_startup_interest_messages(...)` to reuse the dedicated throttle and `AgentUpdate` send helpers.
- Added bounded `viewer_app` live-worker scheduling for recurring non-reliable `AgentUpdate` keepalive sends on the active social circuit.
- Re-armed the keepalive cadence when the social circuit is re-opened and startup interest is re-primed.
- Added targeted tests covering:
  - non-reliable `AgentUpdate` send behavior in `viewer_net`
  - deterministic keepalive interval/scheduling behavior in `viewer_app`

## Files Changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_OBJECT_INGRESS_AGENT_UPDATE_CADENCE_2026-03-29.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_AGENT_UPDATE_CADENCE_2026-03-29.md`
- `docs/reviews/REVIEW_IMPL_OBJECT_INGRESS_AGENT_UPDATE_CADENCE_2026-03-29.md`
- `docs/reports/REPORT_OBJECT_INGRESS_AGENT_UPDATE_CADENCE_2026-03-29.md`
- `docs/plans/DEFERRED_FEATURES.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/LEARNINGS.md`

## Validation Run
- `cargo fmt --all`
  - Passed.
- `cargo check -p viewer_net -p viewer_app`
  - Passed.
- `cargo test -p viewer_net`
  - Passed.
- `cargo test -p viewer_app`
  - Passed.
- Connected capture:
  - Command: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`
  - Runtime: bounded 60-second capture via local process control
  - Stdout log: `artifacts/logs/live_agent_update_cadence_2026-03-29_231942.log`
  - Stderr log: `artifacts/logs/live_agent_update_cadence_2026-03-29_231942.err.log`
  - Result: process exited cleanly after bounded capture

## Result Status
- Code implementation and targeted validation: completed.
- Connected protocol result: still blocked.
- Exact live evidence from the new capture:
  - startup summary remained `update_messages=0 total_objects=0 handshake_complete=true traffic_obs=7 region_handshake_updates=0`
  - startup kinds remained exactly:
    - `AgentDataUpdate:1`
    - `AgentMovementComplete:1`
    - `HealthMessage:1`
    - `OnlineNotification:1`
    - `PacketAck:1`
    - `TestMessage:1`
    - `ViewerEffect:1`
  - recurring tick summaries remained `update_messages=0 total_objects=0 handshake_complete=true`

## Risks or Follow-up Items
- Recurring `AgentUpdate` cadence on the active circuit is not sufficient by itself to restore object ingress.
- The next slice should target protocol-accurate first-simulator control/reliability behavior with stronger Firestorm-source grounding, not more socket or `AgentUpdate` cadence tuning.
- Live texture work remains downstream-blocked because real object ingress and texture-ID export are still absent.

## Learnings Delta
- added: `L29` because recurring `AgentUpdate` cadence did not change the live packet mix or unblock object ingress after retained-socket and startup-parity repairs were already in place.

## Continuity Updates Performed
- Added implementation review and execution report artifacts for this slice.
- Updated `docs/plans/DEFERRED_FEATURES.md` with the too-early full camera/control-parity follow-up.
- Updated `docs/CURRENT_STATE.md`.
- Replaced `docs/HANDOFF.md` with the latest exact state and next step.
- Updated `docs/LEARNINGS.md`.
