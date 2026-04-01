# Report: Object Ingress RegionObjects Pathfinding Variant Discrimination (2026-03-31)

## Summary of implemented work
- Added bounded pathfinding variant discrimination to the typed `RegionObjects` lane.
- The typed summary now surfaces:
  - `variant`
  - `description_shape`
  - `description_tuple` when the live payload contains a six-value comma-separated numeric tuple
- Preserved the existing typed pathfinding fields (`profile`, `linkset_use`, `walkability`, `name`, `description`, `owner`) and kept the change bounded to transport extraction plus relay clarity.
- Added a dedicated continuity tracker at `docs/OBJECT_INGRESS_STATUS.md` that separates what is working, what is not working, and the current open questions.

## Files changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/OBJECT_INGRESS_STATUS.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/LEARNINGS.md`

## Validation run
- `cargo fmt --all`: PASSED
- `cargo check -p viewer_net -p viewer_app`: PASSED
- `cargo test -p viewer_net`: PASSED
- `cargo test -p viewer_app`: PASSED earlier in the slice and unaffected by the final test-only correction
- bounded connected run:
  - `VIEWER_APP_LIVE_STARTUP=on VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_region_objects_pathfinding_field_promotion_2026-03-31.jsonl cargo run -p viewer_app`
  - result: PASSED for evidence capture; process intentionally stopped after 35 seconds
  - artifacts:
    - `artifacts/logs/live_region_objects_pathfinding_field_promotion_2026-03-31.out.log`
    - `artifacts/logs/live_region_objects_pathfinding_field_promotion_2026-03-31.err.log`
    - `artifacts/logs/network_debug_region_objects_pathfinding_field_promotion_2026-03-31.jsonl`

## Result status
- The repo can now explain one live variant difference clearly:
  - some records are `variant=pathfinding_placeholder_description_no_position`
  - some records are `variant=pathfinding_tuple_description_no_position`
- The tuple-like records now surface as:
  - `description_shape=comma_numeric_tuple6`
  - `description_tuple=0|10.000000|30|0|2|0` or similar
- The placeholder records surface as:
  - `description_shape=placeholder_text`
  - `description=(No Description)`
- LLUDP object ingress still remained blocked during the same run:
  - `RegionHandshake = none`
  - `RegionHandshakeReply = none`
  - `ObjectUpdate* = none`
  - `update_messages=0`

## Evidence-backed conclusion
- The typed `RegionObjects` pathfinding lane is working and the current live sample set is not uniform.
- The next best branch is to determine what the tuple-style records actually represent and whether they indicate another pathfinding subtype or an overloaded `description` field.
- The new `docs/OBJECT_INGRESS_STATUS.md` file should now be kept current as the quick audit surface for this investigation.

## Risks or follow-up items
- The tuple-like description payload is still unexplained; it is now observable and classified, but not yet semantically decoded.
- A separate `position` field still did not appear in the bounded first-object sample.

## Learnings delta
- `added`
- Added L46 to `docs/LEARNINGS.md`.

## Continuity updates performed
- Added `docs/OBJECT_INGRESS_STATUS.md`
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/LEARNINGS.md`
