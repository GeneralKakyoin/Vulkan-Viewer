# Plan: Test Automation Flags (Auto Camera + Screenshot Capture)

## Objective
Add two new deterministic, flag-driven runtime test modes to `viewer_app` so models/agents can validate rendering changes with repeatable camera motion and saved frame evidence.

## Scope
In scope:
- Add an automated camera movement test mode, enabled by `STRESS_TEST` (string/alias value).
- Add an automated screenshot capture test mode, enabled by `STRESS_TEST` (string/alias value).
- Keep both modes bounded and deterministic with env-based tuning knobs.
- Expose usage in documentation with concrete run commands.

Out of scope:
- Golden-image comparison and pixel-diff pass/fail logic.
- Multi-camera scripted path files.
- CI pipeline integration for artifact collection.

## Current known state
- `viewer_app` currently supports `STRESS_TEST=1` and `STRESS_TEST=2`.
- Camera control currently comes from input (`InputState`) and optional stress-object animation.
- `viewer_render` renders to surface but does not currently provide screenshot readback.

## Files and components touched
- `crates/viewer_app/src/main.rs`
  - Test mode parsing
  - Auto-camera update path
  - Screenshot capture scheduling and file output wiring
- `crates/viewer_render/src/lib.rs`
  - Optional frame readback API for screenshot capture
- `crates/viewer_render/Cargo.toml`
  - `image` dependency for PNG encoding
- `README.md` and continuity docs
  - clear usage documentation for new modes

## Boundary check
- `viewer_app` remains orchestration/policy owner for test mode selection and scheduling.
- `viewer_render` owns GPU readback internals and capture bytes generation.
- No `viewer_net` or `viewer_grid` behavior changes.
- No crate-boundary collapse.

## Step sequence
1. Add a small internal test-mode parser for `STRESS_TEST` values (legacy values preserved).
2. Introduce auto-camera config parsing from env with safe defaults.
3. Apply auto-camera transform before rendering when test mode is active.
4. Add renderer frame-capture API to return RGBA readback bytes.
5. Add screenshot scheduler (every N frames, max frames, output directory).
6. Add/adjust tests for parser/config behavior.
7. Update docs with concrete commands for both modes.

## Validation plan
- `cargo fmt`
- `cargo check`
- `cargo test -p viewer_app`
- `cargo run -p viewer_app` smoke runs:
  - auto-camera mode
  - screenshot mode (verify PNG outputs emitted)

## Risks and open questions
Risks:
- GPU readback can be sensitive to row padding and mapping behavior.
- Screenshot cadence that is too aggressive can impact frame time.

Open questions:
- None blocking for this scoped implementation.

## Deferred-too-early candidates captured
- Add pass/fail baseline image diff harness (deferred to a future `U` testing-focused milestone).
- Add waypoint file-based camera scripting (deferred until broader automation requirements are defined).

## Completion criteria
- Two new flag-driven test modes are available and documented:
  - automated camera movement
  - automated screenshot capture
- Existing stress modes remain functional.
- Validation commands run and results documented in a report.
