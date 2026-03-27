# Plan: U04 Patch 01 (Docs Alignment + Warning Fix)

## Scope
Small follow-up patch to U04 to:
- remove a single `cargo check` warning in `viewer_ui`
- align U04 continuity docs to the actually implemented UX session-status labels

No feature additions, no crate-boundary changes, no protocol changes.

## Current known state
- `cargo check` emits `unused_mut` in `crates/viewer_ui/src/lib.rs` in the runtime relay filtering block.
- `docs/CURRENT_STATE.md` and related continuity docs describe UX session states that do not match the implemented `viewer_core::SessionUxStatus` variants/labels.

## Files touched
- `crates/viewer_ui/src/lib.rs` (warning-only fix)
- `docs/CURRENT_STATE.md` (wording alignment)
- `docs/HANDOFF.md` (wording alignment if needed)
- `docs/reports/REPORT_u04.md` (wording alignment if needed)

## Boundary check
- No new cross-crate dependencies.
- `viewer_ui` remains presentation-only.
- No changes to `viewer_core` types or `viewer_app` mapping logic.

## Step sequence
1. Remove `unused_mut` in runtime relay filtering.
2. Update U04 docs to use the actual session status labels:
   - `disabled`, `starting`, `connected`, `reconnecting`, `failed` (and reason keys where applicable).
3. Re-run validations.
4. Update `docs/HANDOFF.md` and `docs/reports/REPORT_u04.md` if their claims are now inaccurate.

## Validation plan
- `cargo fmt`
- `cargo check` (must be 0 warnings)
- `cargo test -p viewer_core -p viewer_ui`

## Risks / open questions
- None expected (pure docs + warning cleanup).

## Completion criteria
- `cargo check` emits 0 warnings.
- Continuity docs describe the same session status labels the code renders.
