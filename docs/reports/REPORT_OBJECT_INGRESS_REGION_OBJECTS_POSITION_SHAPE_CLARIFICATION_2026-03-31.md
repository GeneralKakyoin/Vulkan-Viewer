# Report: Object Ingress RegionObjects Position Shape Clarification (2026-03-31)

## Summary of implemented work
- Added typed `position` inspection for `RegionObjects` child maps in `viewer_net`.
- The typed pathfinding summary now records:
  - whether the `position` key was present
  - the parsed `position` summary when available
  - the raw `position_shape`
- Updated the variant labeling so it distinguishes:
  - `...with_position`
  - `...position_unparsed`
  - `...no_position_key`
- Updated the app relay so bounded `child_typed=...` output now surfaces `position`, `position_key=present`, and `position_shape=...` early enough to remain visible in the live logs.

## Files changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/LEARNINGS.md`
- `docs/OBJECT_INGRESS_STATUS.md`
- `docs/plans/PLAN_OBJECT_INGRESS_REGION_OBJECTS_TUPLE_CONTENT_INTERPRETATION_2026-03-31.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_REGION_OBJECTS_TUPLE_CONTENT_INTERPRETATION_2026-03-31.md`
- `docs/reports/REPORT_OBJECT_INGRESS_REGION_OBJECTS_POSITION_SHAPE_CLARIFICATION_2026-03-31.md`

## Validation run
- `cargo fmt --all`: PASSED
- `cargo check -p viewer_net -p viewer_app`: PASSED
- `cargo test -p viewer_net`: PASSED
- `cargo test -p viewer_app`: PASSED
- bounded connected run:
  - `VIEWER_APP_LIVE_STARTUP=on VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_region_objects_position_shape_clarification_2026-03-31.jsonl cargo run -p viewer_app`
  - result: PASSED for evidence capture; process intentionally stopped after 20 seconds
  - artifacts:
    - `artifacts/logs/live_region_objects_position_shape_clarification_2026-03-31.out.log`
    - `artifacts/logs/live_region_objects_position_shape_clarification_2026-03-31.err.log`
    - `artifacts/logs/network_debug_region_objects_position_shape_clarification_2026-03-31.jsonl`

## Result status
- The current live `RegionObjects` linkset records do include parsed `position` values.
- The currently surfaced live shape is:
  - `position_key=present`
  - `position_shape=llsd_array_len3`
- Example bounded live records now show:
  - `position=39.21141815185546875|68.02368927001953125|2999.260009765625`
  - `position=61.998996734619140625|87.66783905029296875|2962.088134765625`
- The tuple-like description records are now correctly labeled as:
  - `variant=pathfinding_tuple_description_with_position`
- The placeholder description records are now correctly labeled as:
  - `variant=pathfinding_placeholder_description_with_position`
- LLUDP object ingress remains unchanged:
  - `RegionHandshake = none`
  - `RegionHandshakeReply = none`
  - `ObjectUpdate* = none`
  - `update_messages=0`
  - `total_objects=0`

## Evidence-backed conclusion
- The earlier “no position” conclusion was not correct for the current live `RegionObjects` pathfinding-linkset lane.
- The live records are coming through as LLSD arrays of length 3 for `position`, and the relay now surfaces them.
- The next best branch is no longer “find the missing position”; it is to decide whether the tuple-like description strings are just raw object description content or a repeatable content pattern worth a bounded hint.

## Risks or follow-up items
- The tuple-like description values are still unexplained semantically.
- Mixed numeric formatting is visible in the live positions, so any future normalization should remain bounded and additive.

## Learnings delta
- added
- Added L47 to `docs/LEARNINGS.md`.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/LEARNINGS.md`
- Updated `docs/OBJECT_INGRESS_STATUS.md`
- Added next-step plan and review for tuple-content interpretation
