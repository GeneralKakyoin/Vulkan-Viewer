# Plan: Object Ingress Capability Probe EventQueue Gating (2026-04-01)

## Objective
Test whether simulator-host capability probe ordering is a startup prerequisite by gating `InterestList` and `UntrustedSimulatorMessage` one-shot probes behind the first successful `EventQueueGet` result.

## Scope
- `viewer_app` only.
- Defer startup `InterestList`/`UntrustedSimulatorMessage` probes until `EventQueueGet:ok` is observed.
- Emit explicit protocol-event lines for deferred and executed probe states.
- Keep LLUDP startup behavior and transport code unchanged.

## Current known state
- `InterestList` currently returns `404 Agent not found` during startup-cap probe window.
- `UntrustedSimulatorMessage` currently returns `405 Method Not Allowed` during startup-cap probe window.
- First successful `EventQueueGet` arrives after those probes in current logs.
- LLUDP gate remains `FAIL` in the same runs.

## Files and components touched
- `crates/viewer_app/src/main.rs`
- `docs/TESTING_REFERENCE.md`
- continuity/reporting docs after implementation validation

## Boundary check
- Keep capability transport in `viewer_net` untouched.
- Limit sequencing logic to orchestration in `viewer_app`.
- No architecture or crate-boundary changes.

## Step sequence
1. Add a startup config flag to control probe ordering gate (default on for this branch’s diagnostic objective).
2. Replace immediate startup probe calls with pending probe URLs.
3. Trigger pending probes only after first `EventQueueGet:ok` and emit explicit relay/protocol lines.
4. Add/adjust tests for config parsing and probe-gating helper behavior.
5. Run formatting, targeted checks/tests, and bounded live run with log capture.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_app -p viewer_net`
- `cargo test -p viewer_app`
- `cargo test -p viewer_net`
- bounded live run:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_APP_LLUDP_STARTUP_PARITY_BUNDLE=on`
  - `VIEWER_APP_EVENT_QUEUE_CAP_NOT_FOUND_BEFORE_RECONNECT=3`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_capability_probe_event_queue_gating_2026-04-01.jsonl`
  - `cargo run -p viewer_app` (timeout-bounded)

## Risks and open questions
- EventQueue may fail before first `ok`, leaving probes deferred and unexecuted in a short bounded run.
- `404/405` may persist after gating; if so, ordering is likely not the primary blocker.

## Deferred-too-early candidates captured
- Full write/read semantic client behavior for `InterestList` and `UntrustedSimulatorMessage` remains deferred.
- No `docs/plans/DEFERRED_FEATURES.md` update required for this bounded sequencing probe.

## Learnings pre-check
- L38: move from receive-surfacing guesses toward control/ordering evidence.
- L59: capability outcomes must be interpreted as readiness evidence before new LLUDP startup guesses.

## Completion criteria
- Logs clearly show deferred probes waiting for `EventQueueGet:ok` and then executing.
- New run establishes whether `InterestList`/`UntrustedSimulatorMessage` outcomes change post-gate.
- Continuity docs/report updated with evidence and next branch decision.
