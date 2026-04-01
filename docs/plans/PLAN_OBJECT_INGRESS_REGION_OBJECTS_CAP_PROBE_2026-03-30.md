# Plan: Object Ingress RegionObjects Capability Probe (2026-03-30)

## Scope
Use the newly surfaced primary simulator `RegionObjects` capability as a bounded probe for ingesting any object-related simulator data, without waiting for LLUDP `ObjectUpdate*` to unblock first.

Out of scope:
- full pathfinding UI or editing support
- broad capability dashboarding
- child-region orchestration
- replacing the LLUDP object-ingress investigation

## Current known state
- The primary simulator seed-cap request now returns:
  - `EventQueueGet`
  - `InterestList`
  - `RegionObjects`
  - `UntrustedSimulatorMessage`
  - plus the previously used caps
- `EstablishAgentCommunication` still did not appear in the bounded live run.
- EventQueue still reports only:
  - `AgentGroupDataUpdate`
  - `AgentStateUpdate`
  - `EnableSimulator`
  - `ParcelProperties`
- LLUDP object ingress remains blocked:
  - `RegionHandshake = none`
  - `RegionHandshakeReply = none`
  - `ObjectUpdate* = none`
- The user goal is explicitly to ingest any simulator object data, regardless of whether it renders.

## Files and components expected
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- continuity artifacts and execution report

## Boundary check
- `viewer_net` owns the bounded capability fetch and generic response inspection/parsing.
- `viewer_app` owns when to invoke the probe and how to relay/summarize the result.
- No UI or renderer changes are required for this slice.

## Step sequence
1. Add a bounded `viewer_net` helper to GET the `RegionObjects` capability and inspect the returned top-level structure.
2. Surface a compact summary in `viewer_app` when the primary seed-cap inventory includes `RegionObjects`.
3. If the response contains object/linkset entries, surface a few stable identifiers or counts in the runtime relay/network debug log.
4. Re-run one bounded connected validation and determine whether this produces the first object-related simulator payload on our path.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- bounded connected run: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`

## Risks and open questions
- `RegionObjects` may expose pathfinding/linkset data rather than the same object model as LLUDP object updates.
- The capability may require a specific request shape or permissions to return useful data.
- A successful `RegionObjects` probe would satisfy the immediate “any object data” goal, but it would not prove LLUDP object ingress is fixed.

## Deferred-too-early candidates captured
- Full `RegionObjects` response modeling is deferred until a bounded probe proves the response is useful.
- `InterestList` write-side behavior is deferred until read-side object evidence is understood.

## Learnings pre-check
- L40: persistent EventQueue mattered before additional capability work.
- L41: EventQueue plus port follow-up alone is not enough.
- L42: primary-region special-cap parity can expose additional caps without restoring LLUDP object ingress.

## Exact completion criteria
- The repo performs one bounded `RegionObjects` capability probe when that cap is available.
- The run records whether object-related capability data is present and what minimal structure it has.
- The result clearly states whether this path satisfies the “any object data” goal or whether the next branch must move elsewhere.
