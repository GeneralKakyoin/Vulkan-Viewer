# Plan: SLURL Auto-Teleport Capture Control (2026-03-31)

## Objective
Add a bounded runtime knob that automatically queues the existing reconnect-based SLURL teleport after login, so the teleport capture branch can be executed from the terminal without manual UI interaction.

## Scope
- add a small `viewer_app` env-configured auto-teleport control
- reuse the existing `TeleportViaSlurl` worker command and normalization path
- log when the auto-teleport is armed and when it fires
- document the new knob in testing/docs continuity

Out of scope:
- new teleport semantics
- in-session teleport parity
- broader object-ingress decode changes

## Current known state
- The `Network Debug` window already supports reconnect-based SLURL teleport.
- The worker already normalizes supported SLURLs and reconnects using the new start location.
- The next planned evidence step is a teleport/region-change capture, but terminal-only execution still cannot trigger the UI button.

## Files and components touched
- `crates/viewer_app/src/main.rs`
- `docs/TESTING_REFERENCE.md`
- continuity/report artifacts

## Boundary check
- Keep this entirely in `viewer_app` orchestration.
- Do not move teleport semantics into `viewer_net`.
- Do not widen UI scope; this knob is for capture automation only.

## Step sequence
1. Add optional env parsing for an auto-teleport SLURL target and a bounded delay.
2. Track whether the auto-teleport has already fired in the current worker session.
3. After the worker reaches connected steady state, queue the existing `TeleportViaSlurl` command once.
4. Emit clear relay lines for armed/fired/rejected auto-teleport events.
5. Update testing/docs so the next teleport capture can be run headlessly from the terminal.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_app`
- `cargo test -p viewer_app`
- bounded offline smoke:
  - `VIEWER_APP_LIVE_STARTUP=off cargo run -p viewer_app`
- optional bounded startup smoke with the knob set in offline mode to confirm it does not crash

## Risks and open questions
- If the delay is too short, the reconnect may fire before the user can inspect the first region; keep it configurable and bounded.
- Auto-teleport should fire only once per worker session to avoid reconnect loops.

## Deferred-too-early candidates captured
- none

## Learnings pre-check
- L06: keep start-location/reconnect policy in `viewer_app`.
- L07: no secrets in the snapshot/debug path.
- L51: reuse the normalized `uri:Region&x&y&z` path instead of inventing a second teleport format.

## Completion criteria
- A terminal-only run can arm a one-shot reconnect teleport using an env var.
- The existing teleport path is reused rather than duplicated.
- The repo docs show how to run the next teleport capture without manual UI interaction.
