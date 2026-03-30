# Plan: Object Ingress Post-AMC Request Parity (2026-03-30)

## Objective
Restore the missing first-region object ingress by adding a bounded, evidence-backed subset of Firestorm's post-`AgentMovementComplete` startup requests on the now-confirmed one-port simulator path, while also making currently received `LayerData` traffic explicit in local diagnostics.

## Scope
- Add a small, startup-bounded set of post-`AgentMovementComplete` viewer->simulator requests that Firestorm sends during startup and that our viewer currently lacks.
- Extend first-simulator inbound classification/summary so live runs explicitly report `LayerData` instead of hiding it in raw packet-level evidence.
- Keep this slice limited to request parity and observability.
- Do not broaden into full avatar animation parity, broad movement-state mirroring, terrain rendering, or speculative standalone reliability scheduling.

## Current Known State
- Runtime socket forensics are complete and the latest bounded connected run showed one shared local UDP port end to end with `split=false`.
- `improved.pcapng` confirms that the viewer app does receive same-port simulator traffic on `16.144.39.130:13001`, but that traffic is mainly:
  - `LayerData`
  - `CoarseLocationUpdate`
  - `AttachedSound`
  - `ViewerEffect`
  - `TestMessage`
  - `SimulatorViewerTimeMessage`
  - `ChatFromSimulator`
  - `AgentDataUpdate`
  - `AgentMovementComplete`
- `improved.pcapng` does **not** show:
  - `ObjectUpdate`
  - `ObjectUpdateCompressed`
  - `ObjectUpdateCached`
  - `ImprovedTerseObjectUpdate`
  - `KillObject`
- The preserved Firestorm reference capture shows this pre-burst outbound sequence before the first `ObjectUpdateCached` burst:
  - `AgentUpdate`
  - `AgentAnimation`
  - `SetAlwaysRun`
  - `PacketAck`
  - `MuteListRequest`
  - `MoneyBalanceRequest`
  - `AgentDataUpdateRequest`
- Firestorm source evidence shows that startup itself explicitly issues:
  - `MuteListRequest` in `reference/firestorm/indra/newview/llstartup.cpp`
  - `MoneyBalanceRequest` in `reference/firestorm/indra/newview/llstartup.cpp`
  - `AgentDataUpdateRequest` in `reference/firestorm/indra/newview/llstartup.cpp`
- Firestorm source places `SetAlwaysRun` and `AgentAnimation` in broader agent-state paths, not the same tight startup block.

## Files and Components Touched
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/plans/DEFERRED_FEATURES.md`
- `docs/reviews/`
- `docs/reports/`
- continuity docs under `docs/CURRENT_STATE.md`, `docs/HANDOFF.md`, and `docs/LEARNINGS.md`

## Boundary Check
- LLUDP message construction, first-simulator request helpers, and inbound classification belong in `viewer_net`.
- Startup orchestration and bounded worker scheduling belong in `viewer_app`.
- No renderer, UI, asset-fetch, or grid-semantics boundary changes are required.
- Firestorm is used only as protocol/behavior evidence, not as an architecture template.

## Step Sequence
1. Add a typed `LayerData` first-simulator inbound classification branch in `viewer_net` so live runs explicitly distinguish terrain/layer traffic from object-feed traffic.
2. Add bounded helper(s) in `viewer_net` for these startup requests, matching Firestorm message-template IDs and field shapes:
   - `MuteListRequest`
   - `MoneyBalanceRequest`
   - `AgentDataUpdateRequest`
3. Wire those helpers into the existing first-simulator startup path in `viewer_app` on the active retained `SocialCircuit`, after the current `CompleteAgentMovement` / startup-prime path and without introducing a new socket or independent scheduler.
4. Add targeted tests that lock:
   - correct outbound message IDs and core field blocks for the three new requests
   - `LayerData` inbound classification
   - startup orchestration ordering remaining bounded and one-shot
5. Run a bounded connected validation and compare the new live packet mix against:
   - the preserved Firestorm pre-burst reference
   - the prior `improved.pcapng` app result that showed `LayerData` but no `ObjectUpdate*`
6. If object ingress is still zero after this slice, stop and write the next separate plan rather than broadening in-place.

## Validation Plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- bounded connected run: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`
- inspect resulting live log for:
  - explicit `LayerData` classification
  - presence or absence of `ObjectUpdate*`
  - whether object-feed counters change from zero

## Risks and Open Questions
- `MuteListRequest` and `MoneyBalanceRequest` may be startup housekeeping rather than the actual gating trigger for object ingress.
- `AgentDataUpdateRequest` is the strongest candidate of the three, but that still requires live validation.
- Adding `LayerData` classification improves observability, but it does not itself decode or render terrain.
- If ingress still stays at zero, the next blocker may be a narrower control/reply path such as `SetAlwaysRun`, `AgentAnimation`, or a still-missing inbound decode surface.

## Deferred-too-Early Candidates Captured
- Full startup mirroring of `AgentAnimation` remains deferred because it belongs to broader avatar/motion state, not the smallest justified startup slice.
- `SetAlwaysRun` parity remains deferred until this tighter startup-request subset is validated, because it lives in Firestorm's general walk/run state path rather than the same startup block as the three targeted requests.
- Full `LayerData` decode/render remains deferred; this slice only makes it visible in diagnostics.

## Learnings Pre-check
- L23: success must be measured by actual object ingress, not by plausible-looking startup parity.
- L27: avoid speculative reliability behavior while changing LLUDP startup.
- L29: do not spend another slice on `AgentUpdate` cadence alone.
- L30: ACK-trailer parity alone is insufficient.
- L32: once one-port continuity is proven, shift effort toward message/control parity or inbound classification.

## Completion Criteria
- `LayerData` appears as an explicit first-simulator inbound kind in local diagnostics.
- The viewer sends bounded startup `MuteListRequest`, `MoneyBalanceRequest`, and `AgentDataUpdateRequest` on the active first-simulator circuit.
- A bounded connected run answers whether this narrowed request-parity slice changes object ingress from zero.
- The repo has a clean yes/no result for this specific request subset without silently broadening into the rest of Firestorm's pre-burst behavior.
