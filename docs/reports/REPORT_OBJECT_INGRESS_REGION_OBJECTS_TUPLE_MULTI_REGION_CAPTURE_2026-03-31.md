# Report: Object Ingress RegionObjects Tuple Multi-Region Capture (2026-03-31)

## Summary of implemented work
- Executed a longer bounded live capture using the existing tuple-analysis instrumentation.
- Assessed whether additional runtime duration, without a region change, broadened the tuple family beyond the current same-name sample.

## Files changed
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/LEARNINGS.md`
- `docs/OBJECT_INGRESS_STATUS.md`
- `docs/plans/PLAN_OBJECT_INGRESS_REGION_OBJECTS_TUPLE_TELEPORT_CAPTURE_2026-03-31.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_REGION_OBJECTS_TUPLE_TELEPORT_CAPTURE_2026-03-31.md`

## Validation run
- bounded connected run:
  - `VIEWER_APP_LIVE_STARTUP=on VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_region_objects_tuple_multi_region_capture_2026-03-31.jsonl cargo run -p viewer_app`
  - result: PASSED for evidence capture; process intentionally stopped after 110 seconds
  - artifacts:
    - `artifacts/logs/live_region_objects_tuple_multi_region_capture_2026-03-31.out.log`
    - `artifacts/logs/live_region_objects_tuple_multi_region_capture_2026-03-31.err.log`
    - `artifacts/logs/network_debug_region_objects_tuple_multi_region_capture_2026-03-31.jsonl`

## Result status
- The longer same-region capture did **not** broaden the tuple family.
- The latest longer live result still shows:
  - `samples=2`
  - `names=DSS Candlier Frame`
  - `s0=const:0`
  - `s1=const:10.000000`
  - `s2=const:30`
  - `s3=const:0`
  - `s4=var:2|3`
  - `s5=const:0`
- The tuple family stayed unchanged across the longer window.
- LLUDP object ingress remains unchanged:
  - `RegionHandshake = none`
  - `RegionHandshakeReply = none`
  - `ObjectUpdate* = none`
  - `update_messages=0`
  - `total_objects=0`

## Evidence-backed conclusion
- Time alone is not enough to broaden the tuple family in the current region.
- The next evidence step should change region, not just extend the same-region capture window.
- If a region-change/teleport capture still does not broaden the tuple family, the current tuple pattern should be treated as likely object-local content rather than a generally decodable structure.

## Risks or follow-up items
- A region-change capture may surface multiple unrelated tuple families.
- Manual teleporting will add operator timing variance to the next evidence pass.

## Learnings delta
- added
- Added L50 to `docs/LEARNINGS.md`.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/LEARNINGS.md`
- Updated `docs/OBJECT_INGRESS_STATUS.md`
- Added the next-step plan/review for teleport capture
