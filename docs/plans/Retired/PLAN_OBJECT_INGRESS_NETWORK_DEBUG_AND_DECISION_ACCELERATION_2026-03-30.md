# Plan: Object Ingress Network Debug And Decision Acceleration (2026-03-30)

## Objective
Add a bounded in-app network debug window plus persistent network-debug log output, then use that observability to drive one fast, evidence-led branch through the remaining object-ingress blocker instead of lingering in repeated protocol guesses.

## Scope
- Add a dedicated `Network Debug` window in the app UI.
- Persist the same network-debug information to a structured log file.
- Expose the currently relevant object-ingress networking data in one place:
  - first-simulator LLUDP startup/steady-state counters
  - EventQueue lifecycle and event mix
  - capability-family inventory and current URLs classified by family
  - `EnableSimulator` targets/follow-up attempts
  - object-ingress counters (`RegionHandshake`, `RegionHandshakeReply`, `ObjectUpdate*`)
  - reconnect/failure state and recent network-relay events
- Use the new surface to run the next bounded decision cycle quickly.

Out of scope:
- full packet capture UI
- raw packet hex dump browser
- replacing existing runtime relay window
- broad transport redesign before the debug surface proves a concrete next branch

## Current known state
- The repo now has a real simulator/world-data opening through the simulator-host EventQueue lane.
- Nested EventQueue parsing is live and surfaces:
  - `ParcelProperties`
  - port-only `EnableSimulator` details
- One-shot `UseCircuitCode` follow-up to EventQueue `EnableSimulator` ports is implemented and validated.
- Even after that follow-up:
  - `RegionHandshake = none`
  - `RegionHandshakeReply = none`
  - `ObjectUpdate* = none`
  - `update_messages=0 total_objects=0`
- The current observability is spread across:
  - runtime relay lines
  - bounded live logs
  - several specialized summaries
- That is enough to investigate, but not yet fast enough to support repeated bounded runs without friction.

## Files and components touched
- `crates/viewer_core/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `crates/viewer_ui/src/lib.rs`
- optionally `docs/TESTING_REFERENCE.md` if new env/config knobs are added
- follow-up continuity/report artifacts under `docs/reports/`, `docs/HANDOFF.md`, and `docs/LEARNINGS.md`

## Boundary check
- `viewer_net` remains owner of transport/protocol mechanics and reusable summaries.
- `viewer_app` owns orchestration, network-debug aggregation, and log-file writing.
- `viewer_core` owns any typed state that the UI consumes.
- `viewer_ui` owns presentation of the new `Network Debug` window and nothing else.
- The plan must not collapse `viewer_net` diagnostics directly into `viewer_ui`.

## Step sequence
1. Define a bounded typed `NetworkDebugState` surface in `viewer_core` for UI consumption.
2. Aggregate existing network diagnostics in `viewer_app` into that state without inventing new protocol semantics.
3. Add a `Network Debug` window in `viewer_ui` with compact sections:
   - session/circuit
   - EventQueue/caps
   - LLUDP ingress summary
   - `EnableSimulator` follow-up history
   - recent network relay tail
4. Write the same network-debug events to a structured log file from `viewer_app`.
   - preferred shape: JSONL or another append-only structured format
   - include timestamps and stable event categories
5. Use the new surface for one bounded evidence run focused on the current next branch:
   - `EstablishAgentCommunication`
   - per-region seed-cap updates
   - or another simulator-host prerequisite if the window/log proves that more directly
6. Add explicit decision gates so we do not stay here too long:
   - if the window/log surfaces `EstablishAgentCommunication`, the next implementation is seed-cap follow-up
   - if it proves `EstablishAgentCommunication` is absent but another simulator-host call is missing, implement that one call next
   - if after one more bounded branch `ObjectUpdate*` is still zero, stop adding guesses and write a short decision report naming the exact remaining unknown

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_core -p viewer_app -p viewer_ui -p viewer_net`
- `cargo test -p viewer_core -p viewer_app -p viewer_ui -p viewer_net`
- bounded `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`
- manual verification:
  - confirm `Network Debug` window shows live values
  - confirm structured log file is created and appended to
  - review at least one produced log file sample

## Risks and open questions
- The biggest risk is over-designing the debug surface instead of keeping it focused on the current object-ingress blocker.
- A network debug window can become a second runtime relay if not scoped carefully.
- Log-file persistence must stay bounded and append-only; do not add a heavy telemetry subsystem.
- The current repo already has relay/log concepts, so the plan should reuse those patterns where possible instead of inventing a parallel diagnostics architecture.

## Deferred-too-early candidates captured
- Full raw-packet browser / hex inspector is deferred.
- Packet capture import/pcap visualization inside the app is deferred.
- Long-term generalized telemetry dashboarding is deferred.

## Learnings pre-check
- L23: `EnableSimulator` retarget/follow-up alone is not sufficient.
- L33: any simulator traffic is not the same as object ingress.
- L40: EventQueue/capability evidence must drive the next branch before more LLUDP guessing.
- L41: persistent EventQueue plus one-shot `EnableSimulator` port follow-up can still leave `ObjectUpdate*` at zero.

## Completion criteria
- The repo has an approved plan that adds a dedicated network debug window and persistent log output.
- The plan names the concrete data that must appear in that window/log.
- The plan includes explicit decision gates so the next object-ingress branch is selected quickly instead of extending the current stall.
