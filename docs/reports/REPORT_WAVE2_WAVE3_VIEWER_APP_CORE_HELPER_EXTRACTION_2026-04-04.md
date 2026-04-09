# Report: Wave 2/3 Viewer App + Core Helper Extraction (2026-04-04)

## Summary of implemented work
- Applied the same helper-extraction pattern used in `viewer_net` to the next two major crates.
- `viewer_app`:
  - added `crates/viewer_app/src/start_location_utils.rs`
  - moved start-location/SLURL normalization helper cluster out of `main.rs`
  - wired `mod start_location_utils;` and local imports in `main.rs`
- `viewer_core`:
  - added `crates/viewer_core/src/math_utils.rs`
  - moved math/matrix/quaternion helper cluster out of `lib.rs`
  - wired `mod math_utils;` and re-exported helpers from `lib.rs`
- No intended runtime behavior changes.

## Files changed
- `crates/viewer_app/src/main.rs`
- `crates/viewer_app/src/start_location_utils.rs` (new)
- `crates/viewer_core/src/lib.rs`
- `crates/viewer_core/src/math_utils.rs` (new)
- `docs/plans/PLAN_WAVE2_WAVE3_VIEWER_APP_CORE_HELPER_EXTRACTION_2026-04-04.md` (new)
- `docs/reviews/REVIEW_PLAN_WAVE2_WAVE3_VIEWER_APP_CORE_HELPER_EXTRACTION_2026-04-04.md` (new)
- `docs/reviews/REVIEW_IMPL_WAVE2_WAVE3_VIEWER_APP_CORE_HELPER_EXTRACTION_2026-04-04.md` (new)
- `docs/reports/REPORT_WAVE2_WAVE3_VIEWER_APP_CORE_HELPER_EXTRACTION_2026-04-04.md` (new)
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`

## Validation run
- `cargo fmt --all` -> PASS
- `cargo check -p viewer_app -p viewer_core` -> PASS
- `cargo test -p viewer_app parse_start_location_maps_home_last_and_uri -- --nocapture` -> PASS
- `cargo test -p viewer_app parse_start_location_normalizes_supported_slurls -- --nocapture` -> PASS
- `cargo test -p viewer_core compute_profile_freshness_logic -- --nocapture` -> PASS
- `cargo test -p viewer_app` -> PASS
- `cargo test -p viewer_core` -> PASS

## Result status
- Complete for this bounded Wave 2/3 helper extraction slice.

## Risks or follow-up items
- Additional extraction work remains for `viewer_ui`, `viewer_render`, `viewer_asset`, and `viewer_grid`.
- `viewer_app/src/main.rs` and `viewer_core/src/lib.rs` are reduced incrementally but remain large.

## Learnings delta
- `none` — no new durable learning identified; existing L82/L06 guidance covered this slice.

## Continuity updates performed
- Updated `CURRENT_STATE.md` with latest wave status.
- Updated `HANDOFF.md` with current state and next extraction step.
