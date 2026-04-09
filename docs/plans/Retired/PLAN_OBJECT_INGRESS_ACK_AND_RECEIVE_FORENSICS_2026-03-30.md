# Plan: Object Ingress ACK And Receive Forensics (2026-03-30)

## Objective
Resolve the remaining object-ingress blocker by addressing the four strongest unresolved concerns as one staged investigation surface:
- ACK/control-reply behavior
- receive-path observability gaps
- `RegionHandshake` / startup-state progression
- ordering nuance in the startup control exchange

## Scope
- Add bounded observability and comparison tools for the first-simulator ACK/control exchange on the already-proven one-port path.
- Make receive-path differences visible enough to tell whether packets are absent on the wire, present but not classified, or present but not reaching the current summary path.
- Explicitly track whether `RegionHandshake` appears, when it appears, and whether we can prove it is missing versus merely unobserved.
- Preserve staged execution: this plan identifies the ordered work, but implementation should still promote one smallest justified behavior change at a time after each evidence pass.
- Do not broaden into terrain rendering, full asset HTTP parity, or broad refactors.

## Current Known State
- The one-port retained-socket path is proven.
- The viewer now reproduces the observed standalone Firestorm startup control messages:
  - `AgentHeightWidth`
  - `SetAlwaysRun`
  - the observed startup `AgentAnimation`
  - plus the earlier startup subset `MuteListRequest`, `MoneyBalanceRequest`, and `AgentDataUpdateRequest`
- None of those standalone message promotions restored object ingress.
- Live runs still typically show:
  - `update_messages=0`
  - `total_objects=0`
  - `region_handshake_updates=0`
- Firestorm `Test.pcapng` shows:
  - a working control block
  - inbound ACK behavior around that block
  - `ObjectUpdateCached` beginning only after that exchange settles
- The remaining blocker is now more likely in ACK/control-reply behavior or a receive-path observation gap than in one more missing standalone startup message.

## Files and Components Touched
- Planning/review artifacts under `docs/plans/` and `docs/reviews/`
- Continuity docs under `docs/CURRENT_STATE.md`, `docs/HANDOFF.md`, `docs/CLUES.md`, and `docs/LEARNINGS.md`
- Later implementation is expected to touch:
  - `crates/viewer_net/src/lib.rs`
  - `crates/viewer_app/src/main.rs`

## Boundary Check
- LLUDP ACK parsing/queueing, receive classification, and protocol observability belong in `viewer_net`.
- Live-worker relay summaries belong in `viewer_app`.
- Firestorm remains protocol evidence only.
- No renderer, asset transport, or UI architecture changes are authorized by this plan.

## Concern Map

### Concern 1: ACK / Control-Reply Behavior
- Question:
  - Are we sending the right ACK/control replies at the right time for the control block that precedes object ingress?
- Current evidence:
  - Firestorm shows inbound/outbound `PacketAck` activity around the working control block immediately before `ObjectUpdateCached`.
  - Earlier naive reliability work regressed startup, so we must not guess here.
- Planned response:
  - instrument ACK queue state, outbound ACK trailers, and any explicit `PacketAck` observations around the startup control window
  - compare packet IDs and timing against the Firestorm working sequence
  - only then promote the smallest justified ACK/control-reply change

### Concern 2: Receive-Path Observability Gap
- Question:
  - Are packets arriving but failing to show up in our live summaries?
- Current evidence:
  - pcaps have shown broader traffic categories (`LayerData`, `CoarseLocationUpdate`, `AttachedSound`) that do not always appear consistently in bounded live summaries
  - this leaves open the possibility that some important packets are reaching the socket but not the current reporting path
- Planned response:
  - add bounded receive-window summaries that separately report:
    - raw packet message numbers observed
    - typed classifications observed
    - packets seen but left unclassified
  - distinguish “not received” from “received but not surfaced”

### Concern 3: `RegionHandshake` / Startup State Progression
- Question:
  - Is `RegionHandshake` truly absent on the blocked path, or are we simply not observing it consistently?
- Current evidence:
  - Firestorm working path clearly receives `RegionHandshake`
  - our live runs often end with `region_handshake_updates=0`
- Planned response:
  - add a bounded startup-state timeline that records exact first observation indices for:
    - `RegionHandshake`
    - `AgentMovementComplete`
    - `PacketAck`
    - object-update family packets
  - use that timeline to prove whether `RegionHandshake` is absent or just outside the current summary window

### Concern 4: Ordering Nuance In The Control Exchange
- Question:
  - Even if we send roughly the right packet family, are we sending them in the wrong relative order?
- Current evidence:
  - Firestorm’s working control block has a clear relative order
  - we now send most of that block, but object ingress still does not begin
- Planned response:
  - log a compact ordered startup send/receive transcript on our path
  - compare the order against the Firestorm capture
  - only promote reorder changes after evidence shows a concrete mismatch

## Step Sequence
1. Add bounded receive/ACK/state observability in `viewer_net`:
   - ACK queue state and outbound ACK-trailer usage summaries
   - ordered first-simulator send/receive transcript slices
   - raw message-number counters for packets that remain unclassified
   - first-observation indices for `RegionHandshake`, `PacketAck`, and object-update family packets
2. Surface those summaries in concise relay lines from `viewer_app` during:
   - startup prime completion
   - first steady-state window
3. Run a bounded connected validation with the new observability only.
4. Compare the resulting transcript against the preserved Firestorm working sequence, especially around:
   - control-block packet IDs
   - inbound/outbound ACK timing
   - first `RegionHandshake`
   - first `ObjectUpdateCached`
5. Use that comparison to choose exactly one next behavior-change slice:
   - ACK/control-reply adjustment
   - receive-path classification fix
   - or ordering correction
6. Stop and write a revised implementation plan for that one chosen behavior change.

## Validation Plan
- For the observability implementation slice this plan is intended to authorize next:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_app`
  - bounded connected run: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`
- Success criteria for this slice are observational, not ingress-restorative:
  - produce a reliable transcript of ACK/control/receive state
  - clearly narrow the next behavior-change target

## Risks and Open Questions
- Over-instrumentation can create noise, so summaries must stay bounded and startup-focused.
- We must not let this turn into another speculative reliability change without fresh evidence.
- The correct next fix may still be small but non-obvious; this plan is about making that visible, not guessing it.

## Deferred-too-Early Candidates Captured
- Broad ACK/control mirroring without the new observability remains deferred.
- Any asset-HTTP or terrain-rendering work remains deferred; this plan stays on the simulator control/receive path.

## Learnings Pre-check
- L27: do not approximate reliability behavior casually.
- L32: socket continuity is no longer the primary blocker.
- L33: receiving simulator traffic is not the same as receiving object ingress.
- L34: the startup request subset is not sufficient.
- L35: `AgentHeightWidth` alone is not sufficient.
- L36: `SetAlwaysRun` alone is not sufficient.
- L37: the observed startup `AgentAnimation` packet alone is not sufficient.

## Completion Criteria
- The repo has one explicit plan that addresses all four remaining concerns rather than treating them as separate unsorted suspicions.
- The next implementation step is narrowed to observability for ACK/control/receive behavior on the same one-port path.
- The plan preserves staged follow-up behavior changes rather than authorizing another broad parity sweep.
