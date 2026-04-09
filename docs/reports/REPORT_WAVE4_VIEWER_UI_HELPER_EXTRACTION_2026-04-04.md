# Report: Wave 4 Viewer UI Helper Extraction (2026-04-04)

## Summary of implemented work
- Applied the helper-extraction pattern to the next crate in sequence: `viewer_ui`.
- Added new crate-local module: `crates/viewer_ui/src/ui_status_helpers.rs`.
- Moved a bounded status/helper cluster from `crates/viewer_ui/src/lib.rs` into the new module:
  - `should_submit_on_enter`
  - connection/send/session/status labels and chips
  - `sorted_friend_ids_for_filter`
- Wired parent module imports:
  - `mod ui_status_helpers;`
  - `use ui_status_helpers::*;`
- No intended runtime behavior changes.

## Files changed
- `crates/viewer_ui/src/lib.rs`
- `crates/viewer_ui/src/ui_status_helpers.rs` (new)
- `docs/plans/PLAN_WAVE4_VIEWER_UI_HELPER_EXTRACTION_2026-04-04.md` (new)
- `docs/reviews/REVIEW_PLAN_WAVE4_VIEWER_UI_HELPER_EXTRACTION_2026-04-04.md` (new)
- `docs/reviews/REVIEW_IMPL_WAVE4_VIEWER_UI_HELPER_EXTRACTION_2026-04-04.md` (new)
- `docs/reports/REPORT_WAVE4_VIEWER_UI_HELPER_EXTRACTION_2026-04-04.md` (new)
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`

## Validation run
- `cargo fmt --all` -> PASS
- `cargo check -p viewer_ui` -> PASS
- `cargo test -p viewer_ui` -> PASS

## Result status
- Complete for this bounded Wave 4 `viewer_ui` helper extraction slice.

## Risks or follow-up items
- Remaining planned extraction crates:
  - `viewer_render`
  - `viewer_asset`
  - `viewer_grid`
- `viewer_ui/src/lib.rs` remains large and should continue through bounded helper-cluster slices.

## Learnings delta
- `none` — no new durable learning identified; existing L18/L82 guidance covered this slice.

## Continuity updates performed
- Updated `CURRENT_STATE.md` with Wave 4 status and validation.
- Updated `HANDOFF.md` with exact current state and next crate sequence.
