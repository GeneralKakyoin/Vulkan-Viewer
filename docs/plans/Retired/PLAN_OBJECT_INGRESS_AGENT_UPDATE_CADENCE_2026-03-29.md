# Plan: Object Ingress AgentUpdate Cadence (2026-03-29)

## Objective
Restore first-region object ingress by closing the remaining interest-state send gap on the active first-simulator social circuit, without reopening socket-lifecycle or speculative LLUDP-reliability work.

## Scope
- Add a bounded helper path for sending `AgentUpdate` on the already-active `SocialCircuit`.
- Extend the live worker so first-simulator `AgentUpdate` is not startup-only: send a forced reliable update at startup activation and then a bounded recurring update cadence while connected.
- Keep the existing retained-socket, startup-drain, and one-social-socket discipline intact.
- Add targeted diagnostics/tests proving the cadence uses the active circuit and does not create fresh handshaked sockets.

Out of scope:
- LLUDP `PacketAck` / `CompletePingCheck` reply work.
- Full movement/control-flag parity with the interactive app camera.
- Texture-ID export or live texture bridge fixes.
- Broad protocol expansion beyond first-simulator `AgentUpdate` cadence on the active circuit.

## Current Known State
- Socket continuity, early startup receive activation, and single-social-socket discipline are already in place, but live object ingress remains zero.
- Current best live captures still report `update_messages=0 total_objects=0 handshake_complete=true traffic_obs=7 region_handshake_updates=0`.
- The startup packet mix on the retained-socket path is now explicit and contains `AgentDataUpdate`, `AgentMovementComplete`, `HealthMessage`, `OnlineNotification`, `PacketAck`, `TestMessage`, and `ViewerEffect`, but no `ObjectUpdate*` or `RegionHandshake`.
- Firestorm behavior evidence shows:
  - `process_agent_movement_complete(...)` sends throttle and then `send_agent_update(true, true)`.
  - the main loop continues calling `send_agent_update(false)`.
- Our viewer currently sends only a bounded startup `AgentUpdate` via `send_startup_interest_messages(...)`; the steady-state worker loop does not maintain an `AgentUpdate` cadence.
- Prior naive LLUDP reliability-reply work regressed startup handshake completion and was reverted; this slice must not guess at ack/ping behavior.

## Files and Components Touched
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/plans/DEFERRED_FEATURES.md`
- review/report artifacts under `docs/reviews/` and `docs/reports/`

## Boundary Check
- `viewer_net` owns LLUDP encode/send helpers and first-simulator circuit mechanics.
- `viewer_app` owns live-worker scheduling and when the active circuit is used.
- No `viewer_grid`, renderer, UI, or asset ownership changes are allowed in this slice.
- Do not move app-camera ownership into `viewer_net`; use bounded worker-owned interest-state values only.

## Step Sequence
1. Refactor `viewer_net` first-simulator send helpers so `AgentUpdate` can be sent independently on an existing `SocialCircuit` without reopening sockets.
2. Preserve the existing startup throttle send, but make the startup `AgentUpdate` helper explicit so it can be reused by the live worker after activation/reopen.
3. Add bounded `AgentUpdate` cadence state to the live worker in `viewer_app`:
   - send one forced reliable update immediately after startup social-circuit priming
   - send recurring non-reliable updates on the active circuit at a deterministic interval while connected
   - reuse the same bounded camera basis/far-distance values already used for startup parity
4. Ensure reopen/re-prime paths re-arm the cadence without introducing fresh-socket churn.
5. Add targeted tests for the new `viewer_net` helper and `viewer_app` cadence scheduling decisions.
6. Run connected verification and compare the resulting packet mix/object-feed counters against the existing retained-socket baseline.

## Validation Plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- connected capture:
  - `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`
  - bounded log collection under `artifacts/logs/`

## Risks and Open Questions
- If object ingress still remains zero after recurring `AgentUpdate` is present, the next blocker is likely another specific first-simulator control/reliability behavior rather than listening/socket activation.
- Recurring `AgentUpdate` cadence must stay bounded and deterministic; this slice must not turn into a full motion/input subsystem.
- The worker does not currently own authoritative live camera vectors from the render loop; this slice should keep using bounded worker-side interest-state values rather than inventing a new cross-thread camera contract.

## Deferred-too-Early Candidates Captured
- Full app-camera and control-flag parity for live `AgentUpdate` messages is deferred as too early for this object-ingress slice and is recorded in `docs/plans/DEFERRED_FEATURES.md`.

## Learnings Pre-check
- L05: keep unknown traffic visible; do not hide unexplained packet mix changes.
- L10: use message-template-backed LLUDP IDs, not memory.
- L23: success must be judged by non-zero object ingress, not handshake symptoms alone.
- L25: socket continuity alone is insufficient; the next step must follow observed packet-flow evidence.
- L26: continuous first-simulator traffic must reuse the active social circuit.
- L27: do not mix speculative LLUDP reliability replies into this slice.

## Completion Criteria
- The live worker sends `AgentUpdate` on the active first-simulator social circuit at startup and on a bounded recurring cadence while connected.
- No fresh handshaked UDP sockets are introduced by the cadence path.
- Targeted tests cover the new helper/cadence behavior.
- Connected verification shows either non-zero `object_feed` updates or a tighter, evidence-backed remaining blocker after the send-gap repair.
