# Plan: N15 Continuity Probe Command Wiring and Recovery Signaling

## Summary
Implement the next bounded network-continuity slice after `U14`: wire the deferred continuity-probe command path from UI/app orchestration into live `viewer_net` probe execution, with explicit cooldown/result semantics and typed diagnostics propagation.

## Objective
- Close the deferred U14 gap where `RetryContinuityProbe` is currently disabled/unavailable.
- Preserve `viewer_net` transport ownership and `viewer_grid` semantic ownership while exposing a safe operator-triggered probe.
- Keep the feature bounded to deterministic command dispatch and status reporting, not broad teleport/session parity.

## Why now
- `N11` already hardened transition-state diagnostics and monotonic phase behavior.
- `U14` added bounded operator recovery controls but intentionally deferred live probe execution wiring.
- The next practical step is to connect existing recovery UX to an actual bounded probe command path.

## In scope
- Add/extend typed recovery command(s) from `viewer_ui` -> `viewer_app` -> worker command channel.
- Add bounded continuity-probe execution entrypoint in `viewer_net` for the existing live session.
- Thread probe outcome back through `LiveVisualSnapshot`/continuity diagnostics fields.
- Enforce deterministic cooldown and single-flight guard semantics for probe retries.
- Add targeted tests for command dispatch, cooldown gating, and result mapping.

## Out of scope
- Teleport destination session orchestration parity.
- Multi-hop region transition prediction.
- Broad LLUDP/protocol expansion beyond what is needed for probe command execution.
- UI redesign beyond surfacing command state/outcome.

## Current known state
- `N11` continuity hardening and diagnostics are present (`docs/reports/REPORT_N11.md`).
- `U14` explicitly defers runtime probe execution and marks control as unavailable.
- `viewer_app` already owns orchestration and worker lifecycle; `viewer_ui` emits bounded actions.
- `viewer_net` already has first-simulator probe-related mechanics that can be reused through owned APIs.

## Files and components touched
- `crates/viewer_core/src/lib.rs`
  - Recovery action/result typing if additional explicit result codes are needed.
- `crates/viewer_ui/src/lib.rs`
  - Recovery control wiring and state messaging for enabled probe retry path.
- `crates/viewer_app/src/main.rs`
  - Action dispatch to worker command lane.
  - Cooldown/single-flight guard and result mapping into snapshot-facing diagnostics.
- `crates/viewer_net/src/lib.rs` (and worker modules as needed)
  - Bounded probe execution API for live session.
  - Typed probe result payload suitable for app mapping.
- `docs/plans/DEFERRED_FEATURES.md`
  - Promote/update deferred item once N15 is implemented.

## Boundary check
- `viewer_ui` emits operator intent only.
- `viewer_app` orchestrates and applies cooldown policy; it does not own transport internals.
- `viewer_net` executes probe transport mechanics and returns typed outcomes.
- `viewer_grid` remains owner of semantic interpretation where applicable.
- No boundary collapse between `viewer_net` and `viewer_grid`.

## Step sequence
1. Confirm command/result contract in `viewer_core` for probe retry action and status mapping.
2. Add `viewer_ui` action emission path for probe retry (enabled state controlled by app-provided status).
3. Add `viewer_app` dispatch path from UI action to worker command channel with cooldown/single-flight guards.
4. Add `viewer_net` worker command handler and bounded probe execution method.
5. Map probe result into `LiveVisualSnapshot` continuity fields and diagnostics-safe display values.
6. Add targeted tests across `viewer_app`/`viewer_net` for command routing, guard behavior, and result mapping.
7. Validate with bounded runtime smoke and continuity diagnostics inspection.

## Validation plan
- `cargo fmt --all`
- `cargo check --workspace`
- `cargo test -p viewer_core -p viewer_ui -p viewer_app -p viewer_net`
- `cargo test --workspace` (if cross-crate behavior changes materially)
- Runtime smoke:
  - `VIEWER_APP_LIVE_STARTUP=off cargo run -p viewer_app` (control visibility baseline)
  - Connected smoke with credentials (when available) to verify probe command path and diagnostics state transitions

## Risks and open questions
- Risk: accidental overlap between probe transport and broader reconnect/session logic.
- Risk: insufficient cooldown guarding could create repeated probe bursts.
- Open question: whether probe retry should be allowed during `Confirming` or only degraded/stalled states.

## Deferred-too-early candidates captured
- Teleport destination-session orchestration parity (already deferred in `docs/plans/DEFERRED_FEATURES.md`).
- Multi-hop region transition prediction (already deferred in `docs/plans/DEFERRED_FEATURES.md`).

## Learnings pre-check
- `L05`: preserve Unknown traffic as diagnostic signal; do not hide classification gaps during probe work.
- `L06`: keep `viewer_net` transport vs `viewer_grid` meaning boundary explicit.
- `L07`: do not expose sensitive session data through `LiveVisualSnapshot`.
- `L10`: source packet/message IDs from canonical templates when touching LLUDP classification.
- `L22`: if screenshot smoke is used, include manual image review in evidence.

## Completion criteria
- Recovery probe action is enabled and wired end-to-end in live mode.
- Probe execution is bounded (cooldown + single-flight) and emits typed outcomes.
- Continuity diagnostics reflect probe command outcomes deterministically.
- Validation commands pass or blockers are explicitly documented.

