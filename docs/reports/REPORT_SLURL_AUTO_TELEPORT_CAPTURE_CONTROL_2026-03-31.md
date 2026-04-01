# Report: SLURL Auto-Teleport Capture Control (2026-03-31)

## Summary of implemented work
- Added env-configured one-shot auto-teleport control to `viewer_app` so terminal-driven runs can trigger the existing reconnect-based SLURL teleport path without manual UI interaction.
- Reused the current SLURL normalization/reconnect behavior instead of introducing a second teleport implementation.
- Added relay visibility for auto-teleport armed/firing/rejected events.
- Documented the new env knobs and a bounded example command in `docs/TESTING_REFERENCE.md`.

## Files changed
- `crates/viewer_app/src/main.rs`
- `docs/TESTING_REFERENCE.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/reports/REPORT_SLURL_AUTO_TELEPORT_CAPTURE_CONTROL_2026-03-31.md`

## Validation run
- `cargo fmt --all` — PASSED
- `cargo check -p viewer_app` — PASSED
- `cargo test -p viewer_app` — PASSED
- bounded direct-binary smoke with env-configured auto-teleport target and dummy login endpoint — PASSED for startup/no-crash smoke and intentionally stopped after ~6 seconds
  - command shape:
    - `VIEWER_APP_LIVE_STARTUP=on`
    - `VIEWER_LOGIN_ENDPOINT=https://example.invalid/login`
    - `VIEWER_LOGIN_USERNAME=user`
    - `VIEWER_LOGIN_PASSWORD=pass`
    - `VIEWER_LOGIN_WIRE_FORMAT=xmlrpc`
    - `VIEWER_APP_AUTO_TELEPORT_SLURL=secondlife://Ahern/50/60/70`
    - `VIEWER_APP_AUTO_TELEPORT_DELAY_TICKS=5`
    - `target/debug/viewer_app.exe`
  - artifacts:
    - `artifacts/logs/live_slurl_auto_teleport_capture_control_binary_2026-03-31.out.log`
    - `artifacts/logs/live_slurl_auto_teleport_capture_control_binary_2026-03-31.err.log`

## Result status
- Complete for the bounded automation goal.
- Terminal-only teleport capture setup is now available for the next live evidence run.
- Real logged-in teleport/reconnect behavior remains unvalidated until a credentialed live run is performed.

## Risks or follow-up items
- This remains reconnect-based teleport, not in-session teleport parity.
- Auto-teleport is intentionally one-shot per worker run; if a future slice needs repeated scripted hops, that should be a separate planned change.
- The next intended use is `docs/plans/PLAN_OBJECT_INGRESS_REGION_OBJECTS_TUPLE_TELEPORT_CAPTURE_2026-03-31.md`.

## Learnings delta
- none
- No durable learning identified; this slice reused the existing teleport pattern and added a bounded orchestration knob rather than revealing a new protocol or architectural constraint.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/TESTING_REFERENCE.md`
