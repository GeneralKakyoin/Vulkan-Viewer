# Plan: Object Ingress Persistent EventQueueGet Adoption (2026-03-30)

## Scope
Implement a bounded persistent `EventQueueGet` consumer on the active simulator-host capability URL so the viewer no longer relies on sparse one-shot polling during startup and early steady state.

Out of scope:
- new simulator-host capability families beyond `EventQueueGet`
- new LLUDP startup message promotions
- teleport-crossing capability refresh logic outside what is required to keep the active poll alive

## Current known state
- The completed parallel-protocol investigation showed:
  - the viewer successfully fetches seed caps
  - the fetched capability inventory is dominated by simulator-host `:12043` URLs for `EventQueueGet`, `AgentProfile`, `GetDisplayNames`, `SimulatorFeatures`, and `MapLayer`
  - the viewer currently emits only a one-shot `EventQueueGet:start ack=0 ...` during the bounded run
  - no `EventQueueGet` completion surfaced in the bounded window
  - `RegionHandshake` and `ObjectUpdate*` remained absent
- Firestorm source shows:
  - `LLViewerRegion::setCapability("EventQueueGet", ...)` creates `LLEventPoll`
  - `LLEventPoll` is the sole persistent EventQueue consumer
  - the long-poll path is treated as simulator-host HTTPS traffic on `:12043`

## Files and components touched
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- continuity artifacts under `docs/reports/`, `docs/CURRENT_STATE.md`, `docs/HANDOFF.md`, and `docs/LEARNINGS.md`

## Boundary check
- `viewer_net` owns any reusable EventQueue transport/state helper.
- `viewer_app` owns worker scheduling and relay surfacing.
- This slice does not authorize unrelated capability clients or broad HTTP orchestration refactors.

## Step sequence
1. Replace the current sparse one-shot EventQueue scheduling with one active persistent long-poll loop on the fetched `EventQueueGet` URL.
2. Preserve bounded diagnostics so startup/steady-state relays show:
   - poll start
   - response/timeout/failure
   - ack advancement
   - event count and named event mix
3. Keep reconnect/error behavior conservative:
   - one active poll at a time
   - bounded retry on timeout-like responses
   - no broad reconnect-policy rewrite
4. Re-run the bounded connected validation and compare:
   - whether EventQueue responses now appear
   - whether `RegionHandshake` or `ObjectUpdate*` starts after the persistent poll is active
   - whether startup regresses

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- bounded connected run: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`

## Risks and open questions
- Persistent `EventQueueGet` may be necessary but still not sufficient if another simulator-host capability is also missing.
- The poll loop must stay bounded enough that failure handling does not recreate the reverted broad reliability experiment.

## Deferred-too-early candidates captured
- New simulator-host capability clients remain deferred until persistent `EventQueueGet` is validated.
- Teleport-specific refresh logic remains deferred until the base persistent poll behavior is proven on first region entry.

## Learnings pre-check
- L32, L33, L38, L39 constrain this plan.
- L40 specifically motivates this branch: once the investigation proves seed caps land on simulator-host `:12043` and only a one-shot EventQueue start appears, prioritize persistent EventQueue behavior over more UDP guessing.

## Exact completion criteria
- The viewer runs one active persistent `EventQueueGet` consumer on the fetched simulator-host URL.
- The bounded live run surfaces EventQueue response timing and ack progression explicitly.
- The result clearly answers whether persistent EventQueue behavior changes object-ingress outcome on the current path.
