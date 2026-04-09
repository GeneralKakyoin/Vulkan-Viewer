# Plan: Wave 5 Viewer App Runtime Relay Helper Extraction (2026-04-04)

## Objective
Reduce `viewer_app/src/main.rs` size by extracting runtime relay/network-debug helper functions into a crate-local module, with no intended behavior change.

## Scope
- In scope:
  - Move relay/logging helper cluster from `crates/viewer_app/src/main.rs` into `crates/viewer_app/src/runtime_relay_utils.rs`.
  - Keep existing callsites unchanged via module import.
- Out of scope:
  - Relay format/content changes.
  - Runtime behavior or protocol changes.
  - Cross-crate ownership changes.

## Current known state
- `crates/viewer_app/src/main.rs` is ~10k lines and remains a monolith hotspot.
- Runtime relay/network-debug helper functions are cohesive and have local ownership in `viewer_app`.

## Files and components touched
- `crates/viewer_app/src/main.rs`
- `crates/viewer_app/src/runtime_relay_utils.rs` (new)
- Wave plan/review/report + continuity updates.

## Boundary check
- `viewer_app` remains orchestration owner and relay emitter.
- No changes to `viewer_core`, `viewer_net`, `viewer_grid`, `viewer_render`, or `viewer_ui` boundaries.

## Step sequence
1. Create `runtime_relay_utils.rs`.
2. Move relay/network-debug helper functions into the new module with `pub(super)` visibility.
3. Wire `mod runtime_relay_utils;` and `use runtime_relay_utils::*;` in `main.rs`.
4. Run formatter/check/tests for `viewer_app`.
5. Update review/report and continuity docs.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_app`
- `cargo test -p viewer_app`

## Risks and open questions
- Risk: missing imports/symbol visibility after extraction.
  - Mitigation: module uses `super::*` and `pub(super)` helpers.
- Open question: next `viewer_app` helper cluster after relay utilities (startup/config vs mesh/texture scheduling helpers).

## Deferred-too-early candidates captured
- None in this structural slice.

## Learnings pre-check
- Applicable:
  - L06 (boundary discipline under pressure)
  - L70 (structured progression over ad hoc churn)
  - L82 (crate-local behavior-local placement)

## Completion criteria
- New helper module compiles and is wired.
- `viewer_app` full tests pass.
- Continuity and wave artifacts updated.
