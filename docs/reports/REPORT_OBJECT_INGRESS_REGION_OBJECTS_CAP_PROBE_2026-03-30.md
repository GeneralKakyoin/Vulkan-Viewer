# Report: Object Ingress RegionObjects Capability Probe (2026-03-30)

## Summary of implemented work
- Added a bounded `RegionObjectsInspection` path in `viewer_net` for read-only capability probing.
- Added `Connection::fetch_region_objects_once(...)` to GET the `RegionObjects` capability and inspect the response shape for JSON or LLSD XML.
- Added a bounded live-worker probe in `viewer_app` that runs once when the primary simulator capability map includes `RegionObjects`.
- Routed `region_objects` relay lines into the network-debug log/window categories.

## Files changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_OBJECT_INGRESS_REGION_OBJECTS_FIELD_EXTRACTION_2026-03-30.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_REGION_OBJECTS_FIELD_EXTRACTION_2026-03-30.md`

## Validation run
- `cargo fmt --all`: PASSED
- `cargo check -p viewer_net -p viewer_app`: PASSED
- `cargo test -p viewer_net`: PASSED
- `cargo test -p viewer_app`: PASSED
- bounded connected runs:
  - `VIEWER_APP_LIVE_STARTUP=on VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_region_objects_probe_2026-03-30.jsonl cargo run -p viewer_app`: PASSED for evidence capture; process intentionally stopped after 45 seconds
  - `VIEWER_APP_LIVE_STARTUP=on VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_region_objects_probe_2026-03-30_v2.jsonl cargo run -p viewer_app`: PASSED for evidence capture; process intentionally stopped after 20 seconds

## Result status
- The primary simulator `RegionObjects` capability returned object-related simulator data.
- The bounded live probe surfaced a UUID-keyed top-level map on the main simulator-host `:12043` lane.
- The second bounded run made that explicit: the first surfaced UUID keys each classified as nested `map` values, proving the response contains structured per-object payloads rather than an empty or scalar-only body.
- Example artifact:
  - `artifacts/logs/network_debug_region_objects_probe_2026-03-30_v2.jsonl`
- LLUDP object ingress still remained blocked during the same runs:
  - `RegionHandshake = none`
  - `RegionHandshakeReply = none`
  - `ObjectUpdate* = none`
  - `update_messages=0`
  - `total_objects=0`

## Evidence-backed conclusion
- The immediate user goal is now satisfied: the viewer ingests object-related simulator data from the simulator via the `RegionObjects` capability path, even though LLUDP `ObjectUpdate*` is still absent.
- The next best branch is no longer "find any opening." It is "extract stable fields from the `RegionObjects` per-object maps."
- The next selected branch is:
  - `docs/plans/PLAN_OBJECT_INGRESS_REGION_OBJECTS_FIELD_EXTRACTION_2026-03-30.md`

## Risks or follow-up items
- `RegionObjects` data may be pathfinding/linkset-oriented rather than a drop-in substitute for LLUDP world-object updates.
- The current summary proves structure exists, but does not yet expose stable object fields from inside the child maps.

## Learnings delta
- `added`
- Added L43 to `docs/LEARNINGS.md`.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/LEARNINGS.md`
