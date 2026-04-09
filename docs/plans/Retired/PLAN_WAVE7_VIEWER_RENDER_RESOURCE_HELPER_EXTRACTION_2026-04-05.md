# Plan: Wave 7 Viewer Render Resource Helper Extraction (2026-04-05)

## Objective
Apply the crate-local helper extraction pattern to `viewer_render` by moving low-level render/resource utility helpers out of `lib.rs` into a behavior-local module with no intended behavior change.

## Scope
- In scope:
  - Extract render utility functions from `crates/viewer_render/src/lib.rs` into a new module.
  - Preserve callsites via local module import.
  - Keep existing tests and behavior unchanged.
- Out of scope:
  - Shader behavior changes.
  - Render pipeline logic changes.
  - Cross-crate ownership changes.

## Current known state
- `viewer_render/src/lib.rs` is a monolithic owner file and suitable for bounded extraction.
- A cohesive utility cluster exists for alignment, fallback resources, clear-color blending, and mesh buffer creation.

## Files and components touched
- `crates/viewer_render/src/lib.rs`
- `crates/viewer_render/src/render_resource_utils.rs` (new)
- Wave plan/review/report + continuity updates.

## Boundary check
- All changes remain in `viewer_render`.
- No boundary changes to `viewer_app`, `viewer_core`, `viewer_asset`, `viewer_net`, `viewer_grid`, or `viewer_ui`.

## Step sequence
1. Create `render_resource_utils.rs`.
2. Move utility helper cluster from `lib.rs` into the new module.
3. Wire `mod render_resource_utils;` and `use render_resource_utils::*;`.
4. Run formatter/check/tests for `viewer_render`.
5. Update continuity artifacts.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_render`
- `cargo test -p viewer_render`

## Risks and open questions
- Risk: helper visibility/import drift.
  - Mitigation: `pub(super)` helpers and full crate test pass.
- Open question: next bounded extraction cluster inside `viewer_render` after this wave.

## Deferred-too-early candidates captured
- None in this structural slice.

## Learnings pre-check
- Applicable:
  - L08 (depth resource lifecycle discipline)
  - L09 (render startup ownership discipline)
  - L16 (fallback texture semantics)
  - L82 (crate-local behavior-local placement)

## Completion criteria
- New utility module compiles and is wired.
- `viewer_render` tests pass.
- Continuity docs and wave artifacts updated.
