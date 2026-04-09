# REPORT: RegionObjects Mesh Candidate Extraction (2026-04-02)

## Summary of Implemented Work
- Extended `viewer_net::RegionObjectsInspection` with:
  - `candidate_mesh_asset_ids: Vec<String>`
- Added bounded candidate extraction in `viewer_net`:
  - scans child-map scalar fields in both JSON and LLSD inspection paths
  - requires mesh-related key hints (`mesh|sculpt|model|shape|asset`)
  - requires UUID-like value format
  - deduplicates and caps candidate list size
- Extended `viewer_app` RegionObjects summary output:
  - now includes `mesh_candidates=...` in relay/protocol lines

## Files Changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_REGION_OBJECTS_MESH_CANDIDATE_EXTRACTION_2026-04-02.md`
- `docs/reviews/REVIEW_PLAN_REGION_OBJECTS_MESH_CANDIDATE_EXTRACTION_2026-04-02.md`

## Validation Run
- `cargo fmt --all` -> PASS
- `cargo check -p viewer_net -p viewer_app` -> PASS
- `cargo test -p viewer_net region_objects_mesh_candidate_extraction_filters_by_key_hint_and_uuid -- --nocapture` -> PASS
- `cargo test -p viewer_app summarize_region_objects_inspection_includes_typed_sample_summary -- --nocapture` -> PASS
- bounded live run:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_ASSET_SOURCE_MODE=live`
  - `VIEWER_FIXTURE_MESHES=947d4505-eb76-2ef5-c049-e7882881d689`
  - `VIEWER_LOGIN_START=secondlife://Ahern/8/10/41`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_region_objects_mesh_candidates_2026-04-02.jsonl`
  - `cargo run -p viewer_app` (timeout-bounded capture)

## Runtime Evidence
- RegionObjects summary now includes the new field:
  - `artifacts/logs/live_region_objects_mesh_candidates_2026-04-02.log:39`
  - `... tuple_analysis=none mesh_candidates=none ...`
- Network debug artifact also includes the same field:
  - `artifacts/logs/network_debug_region_objects_mesh_candidates_2026-04-02.jsonl:12`
- Mesh ingest probe remained observable in same run:
  - `artifacts/logs/live_region_objects_mesh_candidates_2026-04-02.log:60`
    - `mesh_fetch: queued id=947d4505-eb76-2ef5-c049-e7882881d689 ...`
  - `artifacts/logs/live_region_objects_mesh_candidates_2026-04-02.log:61`
    - `mesh_fetch: failed ... 403 Forbidden ...`

## Result Status
- Diagnostic feature implemented and validated.
- Current Ahern payload sample did not expose mesh candidate IDs under current bounded inspection (`mesh_candidates=none`).

## Risks / Follow-up
- RegionObjects pathfinding-shaped payloads may not include mesh/sculpt asset fields; candidate extraction remains best-effort.
- Next probe to close gap:
  - add one additional bounded extractor from decoded LLUDP object-update payloads (if available) and emit a second candidate source line.

## Learnings Delta
- `added` (L74): RegionObjects capability samples on this route can omit mesh-asset identifiers even when object ingress is healthy; mesh candidate extraction must remain source-tagged and best-effort.

## Continuity Updates Performed
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/LEARNINGS.md`
