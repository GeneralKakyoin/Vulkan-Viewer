# Report: Object Ingress RegionObjects Tuple Sample Widening (2026-03-31)

## Summary of implemented work
- Widened the bounded `RegionObjects` child-map capture limits in `viewer_net`.
- Expanded the tuple-analysis summary to include bounded distinct object names in addition to slot-level const/var analysis.
- Re-ran bounded live capture to test whether the broader code-side sample window actually surfaces more tuple records in the current region.

## Files changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/LEARNINGS.md`
- `docs/OBJECT_INGRESS_STATUS.md`
- `docs/plans/PLAN_OBJECT_INGRESS_REGION_OBJECTS_TUPLE_MULTI_REGION_CAPTURE_2026-03-31.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_REGION_OBJECTS_TUPLE_MULTI_REGION_CAPTURE_2026-03-31.md`

## Validation run
- `cargo fmt --all`: PASSED
- `cargo check -p viewer_net -p viewer_app`: PASSED
- `cargo test -p viewer_net`: PASSED
- bounded connected run:
  - `VIEWER_APP_LIVE_STARTUP=on VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_region_objects_tuple_sample_widening_2026-03-31.jsonl cargo run -p viewer_app`
  - result: PASSED for evidence capture; process intentionally stopped after 35 seconds
  - artifacts:
    - `artifacts/logs/live_region_objects_tuple_sample_widening_2026-03-31.out.log`
    - `artifacts/logs/live_region_objects_tuple_sample_widening_2026-03-31.err.log`
    - `artifacts/logs/network_debug_region_objects_tuple_sample_widening_2026-03-31.jsonl`

## Result status
- The wider code-side capture window did **not** broaden the tuple sample in the current region.
- The latest bounded live result still shows:
  - `samples=2`
  - `names=DSS Candlier Frame`
  - `s0=const:0`
  - `s1=const:10.000000`
  - `s2=const:30`
  - `s3=const:0`
  - `s4=var:2|3`
  - `s5=const:0`
- This means the current bottleneck is no longer the sample cap inside the code; it is the narrowness of the live region/window content.
- LLUDP object ingress remains unchanged:
  - `RegionHandshake = none`
  - `RegionHandshakeReply = none`
  - `ObjectUpdate* = none`
  - `update_messages=0`
  - `total_objects=0`

## Evidence-backed conclusion
- Code-side tuple widening succeeded mechanically, but it did not produce a broader tuple family in the current live environment.
- The next best step is not more logging in the same place. It is a broader evidence capture:
  - longer window
  - or different region / teleport
- Until that broader sample exists, assigning semantics to slot `4` would still be overfitting.

## Risks or follow-up items
- The current tuple pattern may be object-local rather than region-wide.
- A broader multi-region run may expose multiple tuple families that need to be treated separately.

## Learnings delta
- added
- Added L49 to `docs/LEARNINGS.md`.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/LEARNINGS.md`
- Updated `docs/OBJECT_INGRESS_STATUS.md`
- Added the next-step plan/review for broader tuple capture
