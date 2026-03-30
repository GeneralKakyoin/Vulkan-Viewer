# Plan: LLUDP Reliability Replies (2026-03-29)

## Objective
Implement the minimum LLUDP control replies that a stable first-simulator circuit expects, so the simulator does not stall after the initial reliable/control packet burst.

## Scope
- Add bounded handling for reliable inbound packet acknowledgements on the active social circuit.
- Add `StartPingCheck` classification and automatic `CompletePingCheck` replies.
- Keep the change inside `viewer_net` receive/send helpers that already own the social-circuit socket.
- Preserve the existing socket-continuity, startup-parity, and single-social-socket fixes.

## Current Known State
- After fixing socket continuity and removing nearby-chat socket churn, live startup still reports exactly seven inbound packets and then no world-object ingress.
- The startup kind summary in `artifacts/logs/live_single_social_socket_2026-03-29_222138.log` is: `AgentDataUpdate`, `AgentMovementComplete`, `HealthMessage`, `OnlineNotification`, `PacketAck`, `TestMessage`, `ViewerEffect`.
- Firestorm’s message layer (`reference/firestorm/indra/llmessage/message.cpp`) automatically:
  - collects reliable inbound packet IDs for ack,
  - sends packet acknowledgements,
  - replies to `StartPingCheck` with `CompletePingCheck`.
- Our current transport observes these packets but does not send corresponding LLUDP control replies.

## Files and Components Touched
- `crates/viewer_net/src/lib.rs`
- continuity artifacts under `docs/reviews/`, `docs/reports/`, `docs/CURRENT_STATE.md`, `docs/HANDOFF.md`, and `docs/LEARNINGS.md`

## Boundary Check
- This is transport/session mechanics and belongs in `viewer_net`.
- No grid semantics, UI, renderer, or asset ownership changes are allowed.

## Step Sequence
1. Add packet-header helpers for reliable/ack flags and inbound packet ID extraction.
2. Add bounded outbound helpers for `PacketAck` and `CompletePingCheck`.
3. Extend the active social-circuit receive path to automatically ack reliable inbound packets and answer `StartPingCheck`.
4. Add targeted tests for ping reply and reliable-packet acknowledgement behavior.
5. Re-run live verification to see whether the simulator proceeds past the initial seven control packets.

## Validation Plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- connected capture: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app` with bounded log collection

## Risks and Open Questions
- If object ingress still remains zero, the next blocker is likely another specific first-simulator control message rather than generic transport reliability.
- Acknowledgement handling should stay minimal and socket-local; this plan does not attempt to recreate Firestorm’s full resend window logic.

## Deferred-too-Early Candidates Captured
- None. Full LLUDP resend/duplicate suppression machinery remains out of scope.

## Learnings Pre-check
- L10: derive LLUDP control IDs/flags from authoritative source, not memory.
- L23: judge success by actual object ingress.
- L24: preserve first-simulator socket continuity across all receive/send lanes.
- L25: follow live packet evidence after each bounded fix rather than widening blindly.

## Completion Criteria
- Reliable inbound packets on the social circuit are acknowledged.
- `StartPingCheck` is answered with `CompletePingCheck`.
- Live verification either shows broader packet flow/object ingress or leaves a tighter, evidence-backed remaining blocker.
