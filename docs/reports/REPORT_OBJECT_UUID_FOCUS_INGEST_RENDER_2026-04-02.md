# REPORT_OBJECT_UUID_FOCUS_INGEST_RENDER_2026-04-02

## Summary of Implemented Work
- Added LLUDP object-feed UUID retention in `viewer_net` by persisting `ObjectUpdate` `FullID` bytes per `local_id` and exporting optional `object_id` in decoded object-feed entries.
- Extended core snapshot object-feed contract with additive `object_id` field.
- Added `viewer_app` runtime focus filter env var:
  - `VIEWER_APP_OBJECT_UUID_FOCUS=<canonical uuid>`
  - when set, only matching object-feed entries are forwarded into `LiveVisualSnapshot` for rendering ingestion.
- Added tests for UUID retention/export and focus-filter behavior.
- Added testing-reference documentation for the new focus env var.

## Files Changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_core/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/TESTING_REFERENCE.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/LEARNINGS.md`

## Validation Run
- `cargo fmt --all` -> PASS
- `cargo check -p viewer_net -p viewer_core -p viewer_app` -> PASS
- `cargo test -p viewer_net object_feed_export_includes_object_id_when_known -- --nocapture` -> PASS
- `cargo test -p viewer_app focus_filter_keeps_only_matching_object_id -- --nocapture` -> PASS
- `cargo test -p viewer_app canonical_uuid_like_validator_is_strict -- --nocapture` -> PASS
- `cargo test -p viewer_app in_process_config_parses_defaults_and_overrides -- --nocapture` -> PASS
- bounded live run:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_ASSET_SOURCE_MODE=live`
  - `VIEWER_LOGIN_START=secondlife://Puppy/140/178/24`
  - `VIEWER_APP_OBJECT_UUID_FOCUS=10930d3b-1821-c584-a0c7-28a34999800d`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_object_uuid_focus_puppy_2026-04-02.jsonl`
  - `cargo run -p viewer_app` (timeout-bounded)

## Result Status
- Implementation status: COMPLETE (code + tests + docs).
- Runtime objective status: PARTIAL.
  - Focus filter path is active (object feed remains filtered and bounded summaries report `total_objects=0` under focus mode).
  - In this bounded capture, no observed LLUDP `ObjectUpdate` `FullID` matched the target UUID.

## Runtime Evidence
- Target UUID present in RegionObjects typed sample:
  - `artifacts/logs/live_object_uuid_focus_puppy_2026-04-02.log:39`
- LLUDP object ingress healthy in same run (`ObjectUpdate` observed, gate PASS):
  - `artifacts/logs/live_object_uuid_focus_puppy_2026-04-02.log:75`
- Focused object-feed summaries stayed empty (no match in bounded window):
  - `artifacts/logs/live_object_uuid_focus_puppy_2026-04-02.log:65`
  - `artifacts/logs/live_object_uuid_focus_puppy_2026-04-02.log:84`
  - `artifacts/logs/live_object_uuid_focus_puppy_2026-04-02.log:101`
- Network debug artifact:
  - `artifacts/logs/network_debug_object_uuid_focus_puppy_2026-04-02.jsonl`

## Risks / Follow-up
- Some objects may not emit a matching full `ObjectUpdate` for the same UUID within a short bounded session window.
- Single next probe to close evidence gap:
  - repeat run with same focus UUID and a longer bounded capture window (or active camera movement around object) until at least one matching `FullID` appears in the decoded object feed.

## Learnings Delta
- Added new learning entry `L75` in `docs/LEARNINGS.md`.

## Continuity Updates Performed
- `docs/CURRENT_STATE.md` updated.
- `docs/HANDOFF.md` replaced with latest handoff.
- `docs/LEARNINGS.md` updated.
