# Plan: Workspace Parity Repair (N11-R16)

## Summary
Repair cross-crate API drift so `viewer_app` and supporting crates compile and run together, then verify code-level presence of the last five milestone features (`N11`, `R12`, `A13`, `U14`, `R16`).

## Objective
- Restore a compileable workspace and runnable `viewer_app` on the current branch.
- Reconcile missing interfaces across `viewer_core`, `viewer_grid`, `viewer_net`, `viewer_render`, `viewer_ui`, and `viewer_asset`.
- Produce code-backed verification for the last five milestones rather than doc-only claims.

## Why now
- Current branch has API mismatches that prevent `cargo check --workspace` and block runtime verification.
- User explicitly requested repair and code-level milestone verification.

## In scope
- Add missing typed contracts referenced by `viewer_app`:
  - recovery action/result types
  - probe result typing/classification
  - transition visual cue/state typing
- Add missing network probe execution method and grid classification helper.
- Align renderer and UI interfaces with current `viewer_app` callsites.
- Add missing texture-cache failure-reset method used by recovery controls.
- Validate build/tests/runtime smoke and document milestone code evidence.

## Out of scope
- Broad feature expansion beyond the already claimed milestone surface.
- Architecture restructuring or crate-boundary changes.
- Protocol behavior changes not required for restoring existing intended interfaces.

## Current known state
- `viewer_app/src/main.rs` references symbols absent in sibling crates.
- `cargo check --workspace` fails with unresolved symbols and interface mismatches.
- Last five milestones are represented in docs but not fully coherent in code at branch tip.

## Files and components touched
- `crates/viewer_core/src/lib.rs`
- `crates/viewer_grid/src/continuity.rs`
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_render/src/lib.rs`
- `crates/viewer_ui/src/lib.rs`
- `crates/viewer_asset/src/texture_fixture.rs`
- verification/continuity docs as needed for this repair slice

## Boundary check
- Keep transport execution in `viewer_net`, semantic mapping in `viewer_grid`.
- Keep cue/recovery contracts in `viewer_core`.
- Keep GPU implementation in `viewer_render`.
- Keep action emission/presentation in `viewer_ui`.
- Keep cache internals in `viewer_asset`.

## Step sequence
1. Add missing `viewer_core` enums/structs and required defaults/sanitization.
2. Add `viewer_grid::continuity::classify_probe_outcome(...)`.
3. Add `viewer_net::Connection::execute_continuity_probe(...)` with bounded capability probe.
4. Add `FixtureTextureCache::clear_failures()` for U14 recovery path.
5. Align `viewer_render::render_frame(...)` signature/uniform packing with transition cue input.
6. Align `viewer_ui::{UiActions, RenderInput}` with current app usage and wire recovery controls.
7. Run `cargo fmt`, `cargo check --workspace`, targeted tests, and `cargo run -p viewer_app` smoke.
8. Perform code-level milestone verification checklist for `N11/R12/A13/U14/R16`.

## Validation plan
- `cargo fmt --all`
- `cargo check --workspace`
- `cargo test -p viewer_core -p viewer_grid -p viewer_net -p viewer_render -p viewer_ui -p viewer_asset -p viewer_app`
- `cargo run -p viewer_app` (bounded smoke)

## Risks and open questions
- Risk: adding continuity fields may require widespread struct-literal updates.
- Risk: renderer uniform layout drift if host/WGSL structs are not updated in lockstep.
- Risk: live continuity probe behavior may depend on capability availability in-session.

## Deferred-too-early candidates captured
- None for this repair-only slice.

## Learnings pre-check
- `L06`: preserve `viewer_net` vs `viewer_grid` boundary.
- `L07`: avoid sensitive data expansion in snapshot contracts.
- `L08`/`L09`: no per-frame GPU resource churn/pipeline creation drift.
- `L10`: no guessed protocol IDs.
- `L22`: screenshot verification requires manual review when used.

## Completion criteria
- Workspace compiles and app starts in smoke run.
- Missing interface errors are resolved without boundary violations.
- Code-level verification report confirms presence of `N11/R12/A13/U14/R16` features.
