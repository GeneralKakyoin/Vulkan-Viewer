# Plan: Object Ingress Post-EnableSimulator Seed-Cap Follow-Up (2026-03-30)

## Scope
Investigate and implement the smallest next behavior after port-only `EnableSimulator` follow-up proved insufficient: determine whether the missing gate is per-region seed-cap installation / `EstablishAgentCommunication` handling or another simulator-host capability step on the followed simulator ports.

Out of scope:
- broad multi-region orchestration
- teleport/session parity
- speculative LLUDP startup packet additions unrelated to the EventQueue evidence

## Current known state
- The viewer now ingests structured simulator/world EventQueue data, including nested `ParcelProperties`.
- `EnableSimulator` currently surfaces port-only targets (`13013`, `13000`, `13001`) on the current simulator host.
- One-shot `UseCircuitCode` follow-up to those ports now occurs and is visible in live logs.
- Even after that follow-up:
  - `RegionHandshake = none`
  - `RegionHandshakeReply = none`
  - `ObjectUpdate* = none`
  - `update_messages=0 total_objects=0`
- Firestorm source still indicates that region graph expansion also depends on `EstablishAgentCommunication` / per-region seed-cap installation.

## Files and components expected
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/reports/`
- continuity artifacts

## Boundary check
- `viewer_net` owns EventQueue extraction and capability/transport helpers.
- `viewer_app` owns bounded runtime follow-up policy and relay output.
- No new crate boundary changes are authorized.

## Step sequence
1. Determine whether `EstablishAgentCommunication` is present but currently absent from bounded summaries or truly absent on our path.
2. If present, surface the seed-cap / sim target details explicitly and test bounded follow-up.
3. If absent, compare the current simulator-host request sequence against Firestorm’s per-region capability startup and choose the smallest justified missing capability step.
4. Re-run bounded connected validation and check again for `RegionHandshake` / `ObjectUpdate*`.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- bounded connected run: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`

## Risks and open questions
- The missing gate may still be capability-driven rather than a further LLUDP send.
- EventQueue-delivered port data may identify neighboring sims without granting enough state to open full region ingress.

## Deferred-too-early candidates captured
- Full child-region lifecycle management remains deferred until per-region seed-cap evidence is explicit.

## Learnings pre-check
- L23, L33, L40 constrain this branch.
- L41 additionally constrains it once recorded: port-only `EnableSimulator` follow-up alone is not sufficient.

## Exact completion criteria
- The repo has one selected post-`EnableSimulator` branch.
- The next behavior change is justified by explicit simulator-host evidence rather than another generic LLUDP guess.
