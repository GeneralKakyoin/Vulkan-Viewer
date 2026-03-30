# Plan: Object Ingress Runtime Socket Forensics (2026-03-30)

## Objective
Determine whether the remaining object-ingress blocker is still a runtime local-port/socket-lifecycle defect on the first-simulator path, and only then decide whether additional Firestorm pre-burst messages are necessary.

## Scope
- Add bounded runtime diagnostics for first-simulator socket creation, retention, reuse, reopen, and receive/send activity.
- Verify which local UDP port(s) are actually active on the blocked viewer path during a live run.
- Compare the instrumented runtime result against the preserved Firestorm and app pcap artifacts.
- Do not broaden into speculative implementation of every extra Firestorm startup request in this slice.

## Current Known State
- Retained-socket continuity, startup receive activation, single-social-socket discipline, startup `AgentThrottle` / reliable `AgentUpdate`, recurring `AgentUpdate`, and ACK-trailer parity are all already implemented.
- The latest connected validation still reports `update_messages=0 total_objects=0 handshake_complete=true region_handshake_updates=0`.
- Preserved pcap artifacts now exist:
  - Firestorm working reference: `artifacts/pcaps/firestorm_object_ingress_reference_2026-03-30_firee.pcapng`
  - blocked app reference: `artifacts/pcaps/app_object_ingress_reference_2026-03-30_App.pcapng`
- The Firestorm pcap shows a single coherent simulator conversation on `16.144.39.130:13001` with pre-burst sends `AgentUpdate`, `AgentAnimation`, `SetAlwaysRun`, `PacketAck`, `MuteListRequest`, `MoneyBalanceRequest`, and `AgentDataUpdateRequest` before the first `ObjectUpdateCached` burst.
- The app pcap reaches the same simulator endpoint but shows:
  - one handshake/control flow on local port `51485` with no object burst
  - a second local port `61225` receiving simulator traffic during the capture window

## Files and Components Touched
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/reviews/`
- `docs/reports/`
- continuity docs under `docs/CURRENT_STATE.md`, `docs/HANDOFF.md`, and `docs/LEARNINGS.md`

## Boundary Check
- Socket lifecycle, local-port diagnostics, and first-simulator send/receive tracing belong in `viewer_net`.
- Worker-facing relay of bounded diagnostic summaries belongs in `viewer_app`.
- No render, UI, asset, or grid-semantics ownership changes are needed.

## Step Sequence
1. Add a bounded first-simulator socket diagnostic model in `viewer_net` that records:
   - socket creation reason
   - local bind address/port
   - remote simulator target
   - retain/reuse/reopen events
   - send/receive events annotated with local port
2. Thread those diagnostics through the existing retained-probe and `SocialCircuit` paths so live runs can prove whether the same local port is actually reused end-to-end.
3. Emit concise worker relay lines in `viewer_app` that summarize first-simulator local-port lifecycle during startup and the first steady-state polling window.
4. Add targeted tests that lock:
   - retained probe socket local-port reuse
   - no fresh-socket bind in steady-state nearby-chat polling on the active circuit
   - diagnostic summaries for retain/reuse/reopen transitions
5. Run a bounded connected validation and compare the runtime local-port trace against:
   - Firestorm pcap `63490 <-> 13001`
   - app pcap evidence showing `51485` plus `61225`
6. Only if runtime single-port continuity is proven on the live blocked path, draft the next separate message-parity slice for the extra Firestorm pre-burst requests.

## Validation Plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- bounded connected run: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`
- compare connected runtime port trace against:
  - `artifacts/pcaps/firestorm_object_ingress_reference_2026-03-30_firee.pcapng`
  - `artifacts/pcaps/app_object_ingress_reference_2026-03-30_App.pcapng`

## Risks and Open Questions
- `App.pcapng` may have started after an earlier local-port association already existed, so the new diagnostics must disambiguate “capture window artifact” from “actual current runtime socket churn.”
- The extra Firestorm pre-burst requests may still matter later, but they are not yet proven to be the gating factor.
- Over-instrumentation could add noise; diagnostics should stay bounded and startup-focused.

## Deferred-too-Early Candidates Captured
- Blind mirroring of Firestorm pre-burst housekeeping/control requests (`AgentAnimation`, `SetAlwaysRun`, `MuteListRequest`, `MoneyBalanceRequest`, `AgentDataUpdateRequest`) is deferred until runtime single-port continuity is either proven or falsified by the new diagnostics.

## Learnings Pre-check
- L23: success must be judged by actual object ingress, not by apparently plausible protocol changes.
- L24: startup object traffic can be lost if socket continuity breaks across the first-simulator boundary.
- L25: code-level socket continuity fixes are not enough; current packet mix must still be compared against runtime evidence.
- L26: steady-state first-simulator traffic must reuse the active circuit.
- L27: avoid speculative reliability behavior.
- L29: do not spend another slice on `AgentUpdate` cadence alone.
- L30: ACK-trailer parity alone is insufficient once the baseline path is in place.

## Completion Criteria
- A bounded live run clearly reports which local UDP port(s) are used for first-simulator startup and steady-state traffic.
- The repo can answer whether the blocked viewer path still has a runtime split-port/local-port lifecycle defect.
- The next implementation decision is narrowed to one of:
  - fix runtime socket continuity in practice
  - or move to a separate message-parity plan with the extra Firestorm pre-burst requests
