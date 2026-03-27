# HANDOFF: Deferred Features Promotion (Testing Automation Slice)

## What Changed
- **Promoted deferred feature: baseline image diff harness**
  - Added new binary: `cargo run -p viewer_app --bin screenshot_diff -- <baseline_dir> <candidate_dir> [max_mean_abs_error]`.
  - Compares PNG files by filename and computes mean absolute RGBA error; exits non-zero on failure.
- **Promoted deferred feature: waypoint/script camera paths**
  - Added `VIEWER_TEST_CAMERA_PATH_FILE` support in `viewer_app` auto-camera config.
  - Script format: JSON array of waypoints with `time_sec`, `position` (`[x,y,z]`), and `look_at` (`[x,y,z]`).
  - When valid and present, scripted camera path overrides orbit camera behavior for `STRESS_TEST=camera|screenshot`.
- **Tests and docs**
  - Added/updated `viewer_app` tests for script parsing/loading.
  - Updated `docs/TESTING_REFERENCE.md` with the new binary command and env var.
  - Updated `docs/plans/DEFERRED_FEATURES.md` statuses for the two promoted items.

## Validation Run
- `cargo fmt --all`: PASSED
- `cargo check --workspace`: PASSED
- `cargo test -p viewer_app`: PASSED

## Exact Current State
- Deterministic screenshot capture now has an in-repo baseline comparison harness.
- Deterministic camera verification now supports file-driven waypoint scripts in addition to env-based orbit tuning.
- Deferred list reflects these two items as promoted.

## Exact Next Step
- Create a small baseline set under `artifacts/` and run `screenshot_diff` in CI-like smoke flow to establish first comparison threshold policy.

## Blockers / Risks
- The screenshot diff harness currently compares only common filename PNGs in two directories; recursive matching and richer reporting are not yet included.
- Camera waypoint script validation is intentionally strict (array format, finite values, increasing `time_sec`), and invalid files fall back to orbit mode with a warning.

