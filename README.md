# SL Viewer (Rust Rewrite)

A modern rewrite of a Second Life / OpenSim viewer using Rust, `wgpu`, and ECS.

See:
- `docs/PROJECT_BRIEF.md`
- `docs/plans/PLAN.md`

## Flag-Driven Runtime Test Modes

The viewer supports stress/test modes via `STRESS_TEST`.

- `STRESS_TEST=1` - scene stress test (legacy moving proxy set)
- `STRESS_TEST=2` - geometry torture scene (procedural + sculpt + mesh placeholders)
- `STRESS_TEST=camera` (alias: `auto_camera`, `5`) - deterministic orbiting auto-camera test
- `STRESS_TEST=screenshot` (alias: `screenshots`, `6`) - auto-camera + periodic PNG capture test

For the canonical list of validation commands, test knobs, and supported `VIEWER_*` env vars, see `docs/TESTING_REFERENCE.md`.

### Auto-camera tuning env vars
- `VIEWER_TEST_CAMERA_CENTER` (`x,y,z`, default `0,0,0`)
- `VIEWER_TEST_CAMERA_RADIUS` (default `22`)
- `VIEWER_TEST_CAMERA_HEIGHT` (default `10`)
- `VIEWER_TEST_CAMERA_LOOK_HEIGHT` (default `2`)
- `VIEWER_TEST_CAMERA_SPEED` (radians/sec, default `0.5`)
- `VIEWER_TEST_CAMERA_PHASE` (radians, default `0`)

### Screenshot tuning env vars
- `VIEWER_TEST_SCREENSHOT_DIR` (default `artifacts/screenshots`)
- `VIEWER_TEST_SCREENSHOT_EVERY_N_FRAMES` (default `30`)
- `VIEWER_TEST_SCREENSHOT_MAX_FRAMES` (default `12`)

### Example
```powershell
$env:VIEWER_APP_LIVE_STARTUP='off'
$env:STRESS_TEST='screenshot'
$env:VIEWER_TEST_SCREENSHOT_DIR='artifacts/screenshots_smoke'
$env:VIEWER_TEST_SCREENSHOT_EVERY_N_FRAMES='10'
$env:VIEWER_TEST_SCREENSHOT_MAX_FRAMES='3'
cargo run -p viewer_app
```

