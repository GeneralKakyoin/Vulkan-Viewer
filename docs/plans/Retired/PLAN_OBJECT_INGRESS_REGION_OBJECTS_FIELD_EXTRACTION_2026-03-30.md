# Plan: Object Ingress RegionObjects Field Extraction (2026-03-30)

## Scope
Take the now-proven `RegionObjects` capability opening and extract a bounded set of stable per-object fields from the returned UUID-keyed maps.

Out of scope:
- rendering or scene integration
- full pathfinding/linkset schema modeling
- broad LLUDP parity work

## Current known state
- The primary simulator `RegionObjects` capability now returns a UUID-keyed top-level map on the main simulator-host `:12043` lane.
- Each surfaced top-level UUID currently classifies as a nested `map`, proving the response contains structured per-object payloads rather than an empty shell.
- This satisfies the immediate "ingest any object data from the simulator" goal, but the current summary does not yet expose stable fields from inside those per-object maps.

## Files and components expected
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- continuity artifacts and execution report

## Boundary check
- `viewer_net` owns response parsing and bounded field extraction.
- `viewer_app` owns one-shot probe timing and relay/log surfacing.
- No UI or renderer changes are required.

## Step sequence
1. Extend the `RegionObjects` inspection path to inspect one or a few first UUID-keyed child maps.
2. Surface a bounded set of stable inner keys and values, such as names, owners, local IDs, or category/type hints if present.
3. Re-run one bounded live probe and confirm the app now logs concrete object-field data, not just UUID-keyed maps.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- bounded connected run: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`

## Risks and open questions
- The inner field names may be highly pathfinding-specific or sparse.
- The response may contain many large maps, so extraction must stay tightly bounded.

## Deferred-too-early candidates captured
- Full `RegionObjects` response modeling remains deferred.
- Any attempt to merge `RegionObjects` data into the world seam remains deferred until the field shape is understood.

## Learnings pre-check
- L42: widening the seed-cap request alone did not restore LLUDP object ingress.
- L43: `RegionObjects` can already provide object-related simulator data even while LLUDP `ObjectUpdate*` stays absent.

## Exact completion criteria
- The repo logs at least one bounded stable field/value summary from inside the `RegionObjects` per-object maps.
- The next step after that is chosen from evidence rather than assumption.
