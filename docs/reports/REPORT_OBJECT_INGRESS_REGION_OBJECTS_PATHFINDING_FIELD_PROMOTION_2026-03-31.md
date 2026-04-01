# Report: Object Ingress RegionObjects Pathfinding Field Promotion (2026-03-31)

## Summary of implemented work
- Extended `RegionObjectsInspection` with a bounded typed `RegionObjectsPathfindingSummary` per surfaced UUID-keyed child map.
- Populated named pathfinding/object fields from the proven `RegionObjects` lane:
  - `name`
  - `description`
  - `owner`
  - `owner_is_group`
  - `position` when present
  - `landimpact`
  - `modifiable`
  - `navmesh_category`
  - `can_be_volume`
  - `is_scripted`
  - `phantom`
  - walkability coefficients `A-D`
  - derived `linkset_use`
- Updated the app-side relay/log summary so live runs now emit a compact `child_typed=...` lane alongside the existing semantic summary.

## Files changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_OBJECT_INGRESS_REGION_OBJECTS_PATHFINDING_VARIANT_DISCRIMINATION_2026-03-31.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_REGION_OBJECTS_PATHFINDING_VARIANT_DISCRIMINATION_2026-03-31.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/LEARNINGS.md`

## Validation run
- `cargo fmt --all`: PASSED
- `cargo check -p viewer_net -p viewer_app`: PASSED
- `cargo test -p viewer_net`: PASSED
- `cargo test -p viewer_app`: PASSED
- bounded connected run:
  - `VIEWER_APP_LIVE_STARTUP=on VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_region_objects_pathfinding_field_promotion_2026-03-31.jsonl cargo run -p viewer_app`
  - result: PASSED for evidence capture; process intentionally stopped after 35 seconds
  - artifacts:
    - `artifacts/logs/live_region_objects_pathfinding_field_promotion_2026-03-31.out.log`
    - `artifacts/logs/live_region_objects_pathfinding_field_promotion_2026-03-31.err.log`
    - `artifacts/logs/network_debug_region_objects_pathfinding_field_promotion_2026-03-31.jsonl`

## Result status
- The `RegionObjects` opening is now a named-field lane, not just a semantic guess.
- The bounded live run surfaced typed records such as:
  - `profile=pathfinding_linkset`
  - `linkset_use=dynamic_phantom`
  - `walkability=100/100/100/100`
  - concrete `name=...`
  - concrete `owner=...`
  - `description=(No Description)` for some objects
- LLUDP object ingress still remained blocked during the same run:
  - `RegionHandshake = none`
  - `RegionHandshakeReply = none`
  - `ObjectUpdate* = none`
  - `update_messages=0`

## Evidence-backed conclusion
- The repo now has a real typed foothold into simulator object-related data on the `RegionObjects` path.
- The next best branch is not more transport work; it is discriminating between the live pathfinding payload variants, because some records present a normal description while others surface coordinate-like comma data in the `description` slot and do not expose a separate `position` in the bounded window.
- The next selected branch is:
  - `docs/plans/PLAN_OBJECT_INGRESS_REGION_OBJECTS_PATHFINDING_VARIANT_DISCRIMINATION_2026-03-31.md`

## Risks or follow-up items
- The first bounded objects do not appear fully uniform; not every entry is a clean textbook `LLPathfindingObject` projection.
- `position` may be absent in the early live sample set even when Firestorm’s pathfinding object model supports it.

## Learnings delta
- `added`
- Added L46 to `docs/LEARNINGS.md`.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/LEARNINGS.md`
