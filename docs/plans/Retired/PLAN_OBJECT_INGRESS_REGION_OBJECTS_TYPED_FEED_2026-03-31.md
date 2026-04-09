# Plan: Object Ingress RegionObjects Typed Feed (2026-03-31)

## Scope
- promote the existing `RegionObjects` opening into a small typed object-sample feed
- surface a cleaner current-region sample in the existing network debug/log output
- keep the slice bounded to the already-proven `RegionObjects` lane

Out of scope:
- LLUDP `ObjectUpdate*` work
- in-session teleport/handoff work
- wider UI redesign

## Current known state
- Teleport capture proved the reconnect-based SLURL path works and broadens the region/object sample.
- `RegionObjects` is returning stable cross-region typed fields such as `name`, `owner`, `position`, `profile`, `linkset_use`, and `walkability`.
- The tuple-style description no longer looks like a universal schema blocker; it appears to be region/object-local content.

## Files and components touched
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- continuity/report artifacts

## Boundary check
- Keep protocol parsing and typed sample derivation in `viewer_net`.
- Keep relay/log formatting and operator-facing summary text in `viewer_app`.
- Do not move this data into `viewer_core` unless a later slice needs persistent cross-crate UI state.

## Step sequence
1. Add a bounded typed-object sample representation to the `RegionObjectsInspection` result.
2. Populate it from the existing pathfinding summaries using only fields already proven stable across regions.
3. Update the app-side `RegionObjects` summarizer to emit a compact current-region object sample summary.
4. Add/adjust targeted tests for the typed sample extraction and summary formatting.
5. Update status/report continuity to reflect that `RegionObjects` is now a dependable typed object-data lane.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`

## Risks and open questions
- The sample must stay bounded and avoid turning the debug line into an unreadable dump.
- Some fields remain variant-sensitive; only stable proven fields should be promoted in this slice.

## Deferred-too-early candidates captured
- none

## Learnings pre-check
- L43: `RegionObjects` is already a valid object-data opening even without LLUDP parity.
- L45: treat the field family as Firestorm-style pathfinding/linkset payloads.
- L49 and L50: stop over-investing in tuple decoding once region-change evidence shows broader stable object fields.

## Exact completion criteria
- `RegionObjectsInspection` contains a bounded typed sample list using trusted fields only.
- The app logs a clean current-region object sample summary from that list.
- Validation passes for touched crates.
