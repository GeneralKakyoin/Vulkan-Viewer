# Report: Wave 1 Viewer Net Helper Extraction (2026-04-04)

## Summary of implemented work
- Expanded Wave 1 `viewer_net` extraction beyond the initial micro-slice.
- Added new helper modules:
  - `crates/viewer_net/src/cookie_utils.rs`
  - `crates/viewer_net/src/login_codec_utils.rs`
  - `crates/viewer_net/src/asset_fetch.rs`
  - `crates/viewer_net/src/object_decode_utils.rs`
- Updated `crates/viewer_net/src/lib.rs` to:
  - declare and wire helper modules
  - re-export asset fetch API functions from `asset_fetch`
  - import helper utilities from `object_decode_utils`
  - remove relocated helper implementations from the monolithic file
- No intended runtime/protocol behavior changes.

## Files changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_net/src/cookie_utils.rs` (new)
- `crates/viewer_net/src/login_codec_utils.rs` (new)
- `crates/viewer_net/src/asset_fetch.rs` (new)
- `crates/viewer_net/src/object_decode_utils.rs` (new)
- `docs/plans/PLAN_WAVE1_VIEWER_NET_HELPER_EXTRACTION_2026-04-04.md` (new)
- `docs/reviews/REVIEW_PLAN_WAVE1_VIEWER_NET_HELPER_EXTRACTION_2026-04-04.md` (new)
- `docs/reviews/REVIEW_IMPL_WAVE1_VIEWER_NET_HELPER_EXTRACTION_2026-04-04.md` (new)
- `docs/reports/REPORT_WAVE1_VIEWER_NET_HELPER_EXTRACTION_2026-04-04.md` (new)
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`

## Validation run
- `cargo fmt --all` -> PASS
- `cargo check -p viewer_net` -> PASS
- `cargo test -p viewer_net merge_cookie_header_prefers_new_values_and_preserves_existing -- --nocapture` -> PASS
- `cargo test -p viewer_net llsd_codec_decodes_simple_login_response -- --nocapture` -> PASS
- `cargo test -p viewer_net decode_object_update_compressed_extracts_mesh_id_from_extra_params -- --nocapture` -> PASS
- `cargo test -p viewer_net fetch_mesh_asset_bytes_reuses_set_cookie_between_probe_and_followup -- --nocapture` -> PASS
- `cargo test -p viewer_net` -> PASS

## Result status
- Complete for this bounded Wave 1 helper extraction slice.
- `viewer_net/src/lib.rs` reduced from ~715 KB to ~651 KB in this slice.

## Risks or follow-up items
- `viewer_net/src/lib.rs` remains large; additional extraction slices still required.
- Next slice should target another tightly scoped helper cluster (recommended: LLUDP encode utility group).

## Learnings delta
- `none` — no new durable learning identified; existing locality/boundary learnings covered this slice.

## Continuity updates performed
- Updated `CURRENT_STATE.md` latest notable changes.
- Replaced `HANDOFF.md` with current extraction state and next step.
