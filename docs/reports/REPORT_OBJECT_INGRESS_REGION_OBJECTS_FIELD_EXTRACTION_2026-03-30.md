# Report: Object Ingress RegionObjects Field Extraction (2026-03-30)

## Summary of implemented work
- Extended `RegionObjectsInspection` to capture bounded inner-map keys and scalar values from the first UUID-keyed child maps.
- Updated the app-side `RegionObjects` summary so runtime relay and network-debug logs now show those child fields directly.
- Preserved the probe as a single read-only capability fetch on the primary simulator-host `:12043` lane.

## Files changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_OBJECT_INGRESS_REGION_OBJECTS_SEMANTIC_MAPPING_2026-03-30.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_REGION_OBJECTS_SEMANTIC_MAPPING_2026-03-30.md`

## Validation run
- `cargo fmt --all`: PASSED
- `cargo check -p viewer_net -p viewer_app`: PASSED
- `cargo test -p viewer_net`: PASSED
- `cargo test -p viewer_app`: PASSED
- bounded connected run:
  - `VIEWER_APP_LIVE_STARTUP=on VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_region_objects_field_extraction_2026-03-30_v2.jsonl cargo run -p viewer_app`
  - result: PASSED for evidence capture; process intentionally stopped after 35 seconds
  - artifacts:
    - `artifacts/logs/live_region_objects_field_extraction_2026-03-30_v2.out.log`
    - `artifacts/logs/live_region_objects_field_extraction_2026-03-30_v2.err.log`
    - `artifacts/logs/network_debug_region_objects_field_extraction_2026-03-30_v2.jsonl`

## Result status
- The `RegionObjects` opening is now concretely usable, not just structurally proven.
- The first bounded child-map extraction surfaced repeatable inner keys:
  - `A`
  - `B`
  - `C`
  - `D`
  - `can_be_volume`
  - `description`
- The first bounded child-map scalar values surfaced:
  - `A=100`
  - `B=100`
  - `C=100`
  - `D=100`
- LLUDP object ingress still remained blocked during the same run:
  - `RegionHandshake = none`
  - `RegionHandshakeReply = none`
  - `ObjectUpdate* = none`

## Evidence-backed conclusion
- The viewer now ingests object-related simulator data and bounded inner object fields from the simulator through `RegionObjects`.
- The next best branch is no longer about opening the path; it is about understanding what these `RegionObjects` child fields mean.
- The next selected branch is:
  - `docs/plans/PLAN_OBJECT_INGRESS_REGION_OBJECTS_SEMANTIC_MAPPING_2026-03-30.md`

## Risks or follow-up items
- The current child fields look pathfinding/linkset-oriented and may not correspond to LLUDP object semantics.
- Field letters `A-D` still need source-backed interpretation before they should be treated as named domain values.

## Learnings delta
- `added`
- Added L44 to `docs/LEARNINGS.md`.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/LEARNINGS.md`
