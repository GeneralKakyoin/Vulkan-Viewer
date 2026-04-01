# Report: Object Ingress RegionObjects Typed Feed Broader SLURL Capture (2026-04-01)

## Summary of implemented work
- Ran a bounded live reconnect capture using a different SLURL target (`secondlife://Morris/128/128/25`).
- Confirmed the pre-teleport phase still surfaces `typed_sample=...` correctly.
- Captured a post-teleport phase against a different simhost where `RegionObjects` startup probe returned `<root>` with `typed_sample=none`.

## Files changed
- `docs/plans/PLAN_OBJECT_INGRESS_REGION_OBJECTS_TYPED_FEED_BROADER_SLURL_CAPTURE_2026-04-01.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_REGION_OBJECTS_TYPED_FEED_BROADER_SLURL_CAPTURE_2026-04-01.md`
- `docs/reports/REPORT_OBJECT_INGRESS_REGION_OBJECTS_TYPED_FEED_BROADER_SLURL_CAPTURE_2026-04-01.md`
- `docs/plans/PLAN_OBJECT_INGRESS_REGION_OBJECTS_POST_RECONNECT_REPROBE_TIMING_2026-04-01.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_REGION_OBJECTS_POST_RECONNECT_REPROBE_TIMING_2026-04-01.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/OBJECT_INGRESS_STATUS.md`
- `docs/LEARNINGS.md`

## Validation run
- bounded direct-binary live run:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_APP_AUTO_TELEPORT_SLURL=secondlife://Morris/128/128/25`
  - `VIEWER_APP_AUTO_TELEPORT_DELAY_TICKS=40`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_region_objects_typed_feed_broader_slurl_capture_2026-04-01.jsonl`
  - `target/debug/viewer_app.exe`
- artifact outputs:
  - `artifacts/logs/live_region_objects_typed_feed_broader_slurl_capture_2026-04-01.out.log`
  - `artifacts/logs/live_region_objects_typed_feed_broader_slurl_capture_2026-04-01.err.log`
  - `artifacts/logs/network_debug_region_objects_typed_feed_broader_slurl_capture_2026-04-01.jsonl`
- static checks:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`

## Result status
- `typed_sample=...` relay remains valid and readable pre-teleport.
- Post-teleport reconnect reached a different simhost (`simhost-0a962ce03cdb50c3e...`) and active startup flow, but the bounded `RegionObjects` startup probe showed:
  - `keys=<root>`
  - `child_profiles=none`
  - `typed_sample=none`
- LLUDP status unchanged:
  - `RegionHandshake = none`
  - `RegionHandshakeReply = none`
  - `ObjectUpdate* = none`
  - `update_messages=0`
  - `total_objects=0`

## Risks or follow-up items
- This result suggests a post-reconnect timing window where `RegionObjects` can be empty in the first bounded probe.
- Next branch should test bounded delayed re-probe timing before treating post-reconnect emptiness as schema/region invariance.

## Learnings delta
- added
- Added L53: post-reconnect `RegionObjects` emptiness in the first bounded probe window should be treated as a timing question first.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/OBJECT_INGRESS_STATUS.md`
- Updated `docs/LEARNINGS.md`
