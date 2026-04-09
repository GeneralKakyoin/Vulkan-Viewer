# Report: Object Ingress Seed-Cap Broadening And Host-Lane Discovery (2026-04-01)

## Summary of implemented work
- Expanded `viewer_net` seed-capability request payload to a Firestorm-aligned broad set for capability-lane discovery.
- Added `viewer_app` host-family summary logging for discovered non-baseline capabilities at:
  - startup seed-cap success
  - follow-up region-seed capability fetches
- Added bounded baseline-cap exclusion logic so host-lane summaries focus on non-baseline capability names.
- Extended tests for:
  - broadened seed request body membership (`GetMesh2`, `ReadOfflineMsgs`, `ViewerStats`)
  - host-lane summary baseline exclusion behavior.

## Files changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_OBJECT_INGRESS_SEED_CAP_BROADENING_HOST_LANE_DISCOVERY_2026-04-01.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_SEED_CAP_BROADENING_HOST_LANE_DISCOVERY_2026-04-01.md`
- `docs/reviews/REVIEW_IMPL_OBJECT_INGRESS_SEED_CAP_BROADENING_HOST_LANE_DISCOVERY_2026-04-01.md`

## Validation run
- `cargo fmt --all`: PASSED
- `cargo check -p viewer_net -p viewer_app`: PASSED
- `cargo test -p viewer_net`: PASSED
- `cargo test -p viewer_app`: PASSED
- bounded live run:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_seed_cap_broadening_host_lane_discovery_2026-04-01.jsonl`
  - `cargo run -p viewer_app` (bounded capture)
  - observed evidence includes:
    - `seed non-baseline capability names by host: asset-cdn:count=4 names=GetMesh,GetMesh2,GetTexture,ViewerAsset; simhost-...:count=102 ...`
    - startup inventory with broad capability discovery across simhost and asset-cdn lanes.

## Result status
- Capability discovery is now materially broader and route-aware.
- Alternate HTTP lanes are now explicitly visible in startup evidence (`asset-cdn` + `simhost` grouped names).
- LLUDP object ingress remains blocked in the same run (`ObjectUpdate*` still absent on this path).

## Risks or follow-up items
- Lane discovery alone does not establish semantic use for object ingress.
- Next branch should probe/interpret candidate non-baseline lanes with strict acceptance criteria.

## Learnings delta
- added
- Added L62 to `docs/LEARNINGS.md` capturing that narrow seed-cap requests can mask alternate host lanes; broadening reveals lanes but does not, by itself, unblock LLUDP object ingress.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/OBJECT_INGRESS_STATUS.md`
- Updated `docs/LEARNINGS.md`
