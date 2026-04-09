# Plan: Wave 6 Viewer App Multi-Cluster Helper Extraction (2026-04-04)

## Objective
Substantially reduce `crates/viewer_app/src/main.rs` by extracting multiple cohesive helper clusters into crate-local modules while preserving behavior.

## Scope
- In scope:
  - Extract object-feed diagnostics helpers into `object_feed_diagnostics_utils.rs`.
  - Extract first-simulator diagnostics helpers into `first_sim_diagnostics_utils.rs`.
  - Extract capability/protocol diagnostics helpers into `capability_diagnostics_utils.rs`.
  - Wire `mod` + `use` in `main.rs`.
- Out of scope:
  - Runtime behavior changes.
  - Cross-crate ownership changes.
  - Protocol logic redesign.

## Current known state
- `viewer_app/src/main.rs` remained a major monolith hotspot after prior extraction waves.
- These helper clusters are behavior-local and called from many points, making them good extraction candidates.

## Files and components touched
- `crates/viewer_app/src/main.rs`
- `crates/viewer_app/src/object_feed_diagnostics_utils.rs` (new)
- `crates/viewer_app/src/first_sim_diagnostics_utils.rs` (new)
- `crates/viewer_app/src/capability_diagnostics_utils.rs` (new)
- Wave plan/review/report + continuity updates.

## Boundary check
- All moved code remains in `viewer_app`.
- No changes to crate boundaries (`viewer_core`, `viewer_net`, `viewer_grid`, `viewer_render`, `viewer_ui`).

## Step sequence
1. Identify discrete helper clusters in `main.rs`.
2. Create three crate-local utility modules and move helper functions.
3. Keep callsites stable via module imports.
4. Run formatting/check/tests.
5. Update continuity and wave artifacts.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_app`
- `cargo test -p viewer_app`

## Risks and open questions
- Risk: high fan-out helpers can break via visibility/import drift.
  - Mitigation: `pub(super)` visibility where needed and full crate tests.
- Open question: next `viewer_app` split candidate after this wave (startup/config helpers vs avatar/profile merge helpers).

## Deferred-too-early candidates captured
- None in this structural slice.

## Learnings pre-check
- Applicable:
  - L06 (boundary discipline under pressure)
  - L70 (structured progression over ad hoc churn)
  - L82 (crate-local behavior-local placement)

## Completion criteria
- New modules compile and all `viewer_app` tests pass.
- `main.rs` reduced materially with behavior preserved.
- Continuity artifacts updated.
