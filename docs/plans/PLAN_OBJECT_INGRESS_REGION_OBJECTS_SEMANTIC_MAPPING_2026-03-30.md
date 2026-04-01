# Plan: Object Ingress RegionObjects Semantic Mapping (2026-03-30)

## Scope
Take the now-extracted `RegionObjects` child-map fields and determine what the first surfaced field families mean, using bounded repo/Firestorm evidence.

Out of scope:
- renderer/world integration
- full pathfinding schema implementation
- LLUDP object-update parity work

## Current known state
- `RegionObjects` is now a proven object-related simulator data path.
- The first bounded child-map extraction surfaced repeatable inner keys:
  - `A`
  - `B`
  - `C`
  - `D`
  - `can_be_volume`
  - `description`
- The first bounded child-map scalar values include repeated `A=100`, `B=100`, `C=100`, `D=100`.

## Files and components expected
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- Firestorm pathfinding/linkset references
- continuity artifacts and execution report

## Boundary check
- `viewer_net` owns bounded parsing improvements.
- `viewer_app` owns relay/log summarization.
- No UI/render boundary changes are authorized.

## Step sequence
1. Compare the extracted child-field names against Firestorm pathfinding/linkset code and any related data model.
2. If a small semantic mapping is justified, rename or summarize the surfaced fields more helpfully in the relay.
3. Re-run one bounded live probe and confirm the `RegionObjects` output is now more interpretable.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- bounded connected run: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`

## Risks and open questions
- The fields may remain opaque without broader schema work.
- Firestorm may use internal pathfinding model names rather than simple wire-field labels.

## Deferred-too-early candidates captured
- Full response-to-domain modeling remains deferred.

## Learnings pre-check
- L43: `RegionObjects` is a valid object-related opening even without LLUDP object updates.
- L44: bounded child-field extraction can already surface stable inner keys and values.

## Exact completion criteria
- The repo can explain at least one of the first surfaced `RegionObjects` child-field families more clearly than raw letters.
- The next step stays evidence-led.
