# Plan: Wave 2/3 Viewer App + Core Helper Extraction (2026-04-04)

## Objective
Apply the same bounded helper-extraction pattern used in `viewer_net` to the next two major crates: `viewer_app` and `viewer_core`, with no intended behavior changes.

## Scope
- In scope:
  - `viewer_app`: extract start-location / SLURL normalization helpers from `main.rs` into a crate-local subfile.
  - `viewer_core`: extract math/matrix/quaternion helpers from `lib.rs` into a crate-local subfile with re-exports.
  - Keep existing callsites/API surface stable.
- Out of scope:
  - Runtime behavior changes.
  - Cross-crate ownership changes.
  - Broad cleanup unrelated to extracted helper clusters.

## Current known state
- `viewer_app/src/main.rs` and `viewer_core/src/lib.rs` remain monolithic hotspots.
- Extraction pattern is already validated in `viewer_net` and can be reused safely.

## Files and components touched
- `crates/viewer_app/src/main.rs`
- `crates/viewer_app/src/start_location_utils.rs` (new)
- `crates/viewer_core/src/lib.rs`
- `crates/viewer_core/src/math_utils.rs` (new)
- Wave plan/review/report + continuity updates.

## Boundary check
- `viewer_app` stays orchestration-only; helper move does not alter ownership.
- `viewer_core` remains domain/math owner; math helpers stay in-crate and are re-exported.
- No changes to `viewer_net`/`viewer_grid` boundary.

## Step sequence
1. Extract `viewer_app` start-location helper cluster into `start_location_utils.rs`.
2. Extract `viewer_core` math helper cluster into `math_utils.rs`.
3. Wire `mod` + `use`/`pub use` in parent files.
4. Run targeted tests for moved helper lanes.
5. Run full `viewer_app` and `viewer_core` tests.
6. Update reports and continuity docs.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_app -p viewer_core`
- `cargo test -p viewer_app parse_start_location_maps_home_last_and_uri -- --nocapture`
- `cargo test -p viewer_app parse_start_location_normalizes_supported_slurls -- --nocapture`
- `cargo test -p viewer_core compute_profile_freshness_logic -- --nocapture`
- `cargo test -p viewer_app`
- `cargo test -p viewer_core`

## Risks and open questions
- Risk: moved private helper visibility can break parent-module access.
  - Mitigation: `pub(super)` for app helpers and explicit re-export for core math helpers.
- Open question: next crate sequence after this slice (`viewer_ui` vs `viewer_render` first).

## Deferred-too-early candidates captured
- None in this structural slice.

## Learnings pre-check
- Applicable:
  - L06 (boundary discipline under pressure)
  - L70 (structured progression over ad hoc churn)
  - L82 (crate-local behavior-local placement)

## Completion criteria
- New helper modules compile and are wired.
- Existing targeted/full tests pass for both crates.
- Continuity docs and report reflect extracted state.
