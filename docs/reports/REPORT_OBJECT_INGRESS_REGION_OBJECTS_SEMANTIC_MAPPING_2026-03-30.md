# Report: Object Ingress RegionObjects Semantic Mapping (2026-03-30)

## Summary of implemented work
- Extended `RegionObjectsInspection` with bounded semantic/profile summaries for UUID-keyed child maps.
- Mapped the previously opaque `A/B/C/D`, `can_be_volume`, and related fields to Firestorm pathfinding linkset semantics using:
  - `reference/firestorm/indra/newview/llpathfindinglinkset.cpp`
  - `reference/firestorm/indra/newview/llpathfindingobject.cpp`
  - `reference/firestorm/indra/newview/llviewerregion.cpp`
- Updated the app-side `RegionObjects` relay so live output now surfaces `child_profiles` and `child_semantics`, not just raw child keys and scalar snippets.
- Normalized bool-like `0/1` payload fields before deriving `linkset_use`, so live summaries match Firestorm’s pathfinding rules.

## Files changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_OBJECT_INGRESS_REGION_OBJECTS_PATHFINDING_FIELD_PROMOTION_2026-03-30.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_REGION_OBJECTS_PATHFINDING_FIELD_PROMOTION_2026-03-30.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/LEARNINGS.md`

## Validation run
- `cargo fmt --all`: PASSED
- `cargo check -p viewer_net -p viewer_app`: PASSED
- `cargo test -p viewer_net`: PASSED
- `cargo test -p viewer_app`: PASSED
- bounded connected run:
  - `VIEWER_APP_LIVE_STARTUP=on VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_region_objects_semantic_mapping_2026-03-30.jsonl cargo run -p viewer_app`
  - result: PASSED for evidence capture; process intentionally stopped after 35 seconds
  - artifacts:
    - `artifacts/logs/live_region_objects_semantic_mapping_2026-03-30.out.log`
    - `artifacts/logs/live_region_objects_semantic_mapping_2026-03-30.err.log`
    - `artifacts/logs/network_debug_region_objects_semantic_mapping_2026-03-30.jsonl`

## Result status
- The repo can now explain the first surfaced `RegionObjects` child-map family as Firestorm-style `pathfinding_linkset` payloads rather than opaque nested maps.
- The bounded live run surfaced evidence-backed semantics such as:
  - `linkset_use=dynamic_phantom`
  - `walkability=A:100|B:100|C:100|D:100`
  - normalized `can_be_volume=false|true`
  - normalized `phantom=true`
  - concrete object names like `DSS Candlier Frame` and `Trance  Chair: Rope Bondage`
- LLUDP object ingress still remained blocked during the same run:
  - `RegionHandshake = none`
  - `RegionHandshakeReply = none`
  - `ObjectUpdate* = none`
  - `update_messages=0`

## Evidence-backed conclusion
- The current `RegionObjects` opening is not a generic object dump; it is a usable pathfinding-linkset/object lane.
- The next best branch is to promote a small typed subset of those pathfinding fields (`name`, `description`, `owner`, `landimpact`, `navmesh_category`, `position`, normalized booleans) instead of relying on summary strings alone.
- The next selected branch is:
  - `docs/plans/PLAN_OBJECT_INGRESS_REGION_OBJECTS_PATHFINDING_FIELD_PROMOTION_2026-03-30.md`

## Risks or follow-up items
- The current relay still truncates semantic values for boundedness, so some fields such as `description` and `owner` may not appear in every top-level summary line.
- This semantic path is still separate from LLUDP `ObjectUpdate*`; it should not be treated as proof of LLUDP parity.

## Learnings delta
- `added`
- Added L45 to `docs/LEARNINGS.md`.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/LEARNINGS.md`
