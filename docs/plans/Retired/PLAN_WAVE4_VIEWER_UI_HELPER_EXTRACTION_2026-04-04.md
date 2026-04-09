# Plan: Wave 4 Viewer UI Helper Extraction (2026-04-04)

## Objective
Apply the same bounded crate-local helper extraction pattern to `viewer_ui`, with no intended runtime behavior change.

## Scope
- In scope:
  - Extract a UI status/helper function cluster from `crates/viewer_ui/src/lib.rs` into a new crate-local module.
  - Keep existing callsites and behavior stable via local module wiring.
- Out of scope:
  - UI behavior changes.
  - Cross-crate moves or ownership changes.
  - Broad cleanup outside the extracted helper cluster.

## Current known state
- `crates/viewer_ui/src/lib.rs` remains a large monolithic file.
- Prior waves validated the extraction approach in `viewer_net`, `viewer_app`, and `viewer_core`.

## Files and components touched
- `crates/viewer_ui/src/lib.rs`
- `crates/viewer_ui/src/ui_status_helpers.rs` (new)
- Wave plan/review/report + continuity updates.

## Boundary check
- `viewer_ui` remains presentation-only and read-only over app/core state contracts.
- Helper extraction is crate-local and does not alter app/core/net/render boundaries.

## Step sequence
1. Move a bounded status/helper cluster from `viewer_ui/src/lib.rs` into `ui_status_helpers.rs`.
2. Wire `mod` + local imports in `viewer_ui/src/lib.rs`.
3. Ensure helper visibility is limited (`pub(super)`).
4. Run crate validation (fmt/check/tests).
5. Update report/reviews and continuity docs.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_ui`
- `cargo test -p viewer_ui`

## Risks and open questions
- Risk: helper visibility/import drift after extraction.
  - Mitigation: use `pub(super)` and preserve existing call paths.
- Open question: next helper cluster target inside `viewer_render` for Wave 5.

## Deferred-too-early candidates captured
- None in this structural slice.

## Learnings pre-check
- Applicable:
  - L18 (egui window-state mutation pattern constraints)
  - L82 (crate-local behavior-local placement)
- No additional learnings required before this bounded extraction.

## Completion criteria
- New `viewer_ui` helper module compiles and is wired.
- Full `viewer_ui` tests pass.
- Continuity/report artifacts updated with this wave result.
