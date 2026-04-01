# Plan: Object Ingress EnableSimulator Port Follow-Up (2026-03-30)

## Scope
Use the newly surfaced port-only `EnableSimulator` EventQueue details to send bounded `UseCircuitCode` follow-up datagrams to additional simulator ports on the current simulator host, using the existing active social socket.

This slice is bounded to:
- port-only `EnableSimulator` follow-up on the current simulator host IP
- one-shot `UseCircuitCode` send per newly seen port
- concise runtime relays for attempted targets and observed outcomes

Out of scope:
- full child-region session lifecycle
- automatic `CompleteAgentMovement` on child regions
- long-lived multi-region state ownership
- teleport/handoff parity work beyond this port-follow proof

## Current known state
- Persistent EventQueue is working with early seed-cap fetch.
- Nested EventQueue body parsing now surfaces:
  - `ParcelProperties` nested world data
  - port-only `EnableSimulator` fields like `SimulatorInfo[0].Port=13013`, `13000`, `13001`
- Firestorm reference says `EnableSimulator` is actionable control traffic that enables a simulator circuit and sends `UseCircuitCode`.
- Our worker currently receives those messages but does not follow them with any transport action.
- LLUDP `ObjectUpdate*` remains absent on the current first-region path.

## Files and components touched
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- continuity artifacts under `docs/reports/`, `docs/CURRENT_STATE.md`, `docs/HANDOFF.md`, and `docs/LEARNINGS.md`

## Boundary check
- `viewer_net` owns the reusable transport helper that sends `UseCircuitCode` to an explicit simulator target on an existing circuit.
- `viewer_app` owns bounded runtime policy:
  - extract new ports from EventQueue
  - de-duplicate sends
  - emit relays
- No new crate boundary crossings are introduced.

## Step sequence
1. Add a `viewer_net` helper to send `UseCircuitCode` on an existing `SocialCircuit` to an explicit `SocketAddr`.
2. In the live worker, extract `EnableSimulator` ports from EventQueue details.
3. For each newly seen port:
   - combine it with the current simulator host IP
   - send one bounded `UseCircuitCode` follow-up on the active circuit socket
   - log the attempted target
4. Re-run one bounded connected validation and compare:
   - whether new simulator traffic appears after the follow-up
   - whether `RegionHandshake`, `RegionHandshakeReply`, or `ObjectUpdate*` appears
   - whether startup regresses

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- bounded connected run: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`

## Risks and open questions
- Port-only `EnableSimulator` may still refer to neighbors that do not produce first useful object traffic on this path.
- Sending `UseCircuitCode` may be necessary but not sufficient if additional capability or region setup is required.
- This slice must remain one-shot and de-duplicated; do not let it grow into an unmanaged retry storm.

## Deferred-too-early candidates captured
- Full child-region handshake/session management remains deferred until a one-shot port-follow attempt proves value.
- Automatic per-port `CompleteAgentMovement` remains deferred until `UseCircuitCode` alone is tested.

## Learnings pre-check
- L23: `EnableSimulator` retarget alone is not a sufficient fix; this change must be validated against actual ingress outcome.
- L33: receiving simulator traffic is not the same as receiving object ingress.
- L40: once EventQueue is live, consume that lane before returning to more standalone UDP guesses.

## Exact completion criteria
- Newly seen `EnableSimulator` ports trigger one-shot `UseCircuitCode` follow-up on the existing socket.
- The bounded live run clearly records which targets were attempted.
- The result clearly answers whether this follow-up changes object-ingress behavior.
