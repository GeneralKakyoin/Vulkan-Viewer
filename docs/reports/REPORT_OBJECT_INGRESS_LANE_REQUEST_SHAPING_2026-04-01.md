# Report: Object Ingress Lane Request Shaping (2026-04-01)

## Summary of implemented work
- Added Firestorm-aligned capability request-shape model in `viewer_grid`:
  - typed probe methods (`GET`, `POST`, `DEL`)
  - shaped probe descriptors for `SimulatorFeatures`, `InterestList`, `UntrustedSimulatorMessage`, and `ViewerAsset`
  - `ViewerAsset` query-key URL builders for `texture_id`, `mesh_id`, `material_id`, `animatn_id`, `sound_id`
- Added generic one-shot shaped probe executor in `viewer_net`:
  - accepts method/url-candidates/optional headers/body/range
  - returns enriched metadata: status/content_type/body_bytes + selected_url/method/decode/body_preview_hash/response_class
- Replaced startup non-baseline generic GET probing in `viewer_app` with shaped matrix behavior:
  - immediate `asset-cdn` `ViewerAsset` query-key probes
  - EventQueue-gated `simhost` probes (`SimulatorFeatures`, `InterestList`, `UntrustedSimulatorMessage`)
  - new explicit protocol markers: `LaneProbeShape:start|ok|err`
- Added deterministic asset-id input policy for shaped viewer-asset probes:
  - `VIEWER_APP_LANE_PROBE_ASSET_IDS` (optional)
  - bounded synthetic fallback IDs when unset
- Updated testing reference with the new env knob.

## Firestorm references used
- `reference/firestorm/indra/newview/llviewerassetstorage.cpp` (`/?<type>_id=...` capability URL shaping)
- `reference/firestorm/indra/newview/lltexturefetch.cpp` (`/?texture_id=...` texture fetch URL shape)
- `reference/firestorm/indra/newview/llmeshrepository.cpp` (`/?mesh_id=...` URL shaping + range-fetch behavior context)
- `reference/firestorm/indra/newview/llviewerregion.cpp` (`SimulatorFeatures` GET, `InterestList` POST)

## Files changed
- `crates/viewer_grid/src/lib.rs`
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/TESTING_REFERENCE.md`
- `docs/plans/PLAN_OBJECT_INGRESS_LANE_REQUEST_SHAPING_2026-04-01.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_LANE_REQUEST_SHAPING_2026-04-01.md`
- `docs/reviews/REVIEW_IMPL_OBJECT_INGRESS_LANE_REQUEST_SHAPING_2026-04-01.md`
- `docs/reports/REPORT_OBJECT_INGRESS_LANE_REQUEST_SHAPING_2026-04-01.md`

## Validation run
- `cargo fmt --all`: PASSED
- `cargo check -p viewer_grid -p viewer_net -p viewer_app`: PASSED
- `cargo test -p viewer_grid -p viewer_net -p viewer_app`: PASSED
- bounded live run:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_APP_CAPABILITY_PROBES_REQUIRE_EVENT_QUEUE_OK=true`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_lane_request_shaping_2026-04-01.jsonl`
  - `cargo run -p viewer_app` (timeout-bounded capture)

## Result status
- Acceptance checks met for this slice:
  - shaped probe lines present for both host families
  - per-shape outcomes are reproducible and include method/query key/url variant/decode/body bytes
  - no crate-boundary violations introduced
- Example live outcomes from the capture:
  - `asset-cdn` `ViewerAsset` shaped requests:
    - `texture_id` => `200 image/x-j2c` (binary/unparsed)
    - `mesh_id` => `403 application/vnd.ll.mesh` (`xml_error`)
    - `material_id` => `403 application/vnd.ll.material` (`xml_error`)
    - `animatn_id` => `403 application/vnd.ll.animation` (`xml_error`)
    - `sound_id` => `200 application/ogg` (binary/unparsed)
  - EventQueue gate opened, then simhost shaped probes:
    - `SimulatorFeatures` GET => `200 application/llsd+xml` (`llsd_map`)
    - `InterestList` POST => `200 application/llsd+xml` (`llsd_map`)
    - `UntrustedSimulatorMessage` GET => `405 text/plain` (`plain_text`)
- LLUDP gate remained `FAIL` in the same run (`ObjectUpdate*` absent), unchanged by design.

## Risks or follow-up items
- Next step should decide whether to add bounded method fallback to the shaped untrusted lane matrix (currently GET-only in matrix, readiness path already has fallback).
- A later branch can add deeper semantic decoders for currently `unparsed` binary probe bodies when needed.

## Learnings delta
- `added` (L64)

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/LEARNINGS.md`
- Updated `docs/TESTING_REFERENCE.md`
