# Plan: Object Ingress Capability Readiness Invocation Investigation (2026-04-01)

## Scope
- Add bounded capability-readiness instrumentation for object-ingress-relevant simulator-host caps.
- Add bounded one-shot invocation probes for `InterestList` and `UntrustedSimulatorMessage`.
- Add bounded EventQueue `cap not found` reconnect handling.

Out of scope:
- New LLUDP startup packet/control changes
- Renderer/UI feature work
- Broad capability client implementations beyond one-shot readiness probes

## Current known state
- LLUDP startup parity-bundle run gate failed:
  - `object_update=none`
  - `update_messages=0`
  - `total_objects=0`
  - `local_ids=none`
- Seed capabilities include `EventQueueGet`, `RegionObjects`, `InterestList`, `UntrustedSimulatorMessage`.
- Current runtime invokes `EventQueueGet` and `RegionObjects`, but does not perform bounded probes on `InterestList` / `UntrustedSimulatorMessage`.
- Repeated EventQueue `404 cap not found` failures occur in long bounded runs.

## Files and components touched
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/TESTING_REFERENCE.md`
- continuity/report artifacts under `docs/reports/`, `docs/CURRENT_STATE.md`, `docs/HANDOFF.md`, `docs/OBJECT_INGRESS_STATUS.md`, `docs/LEARNINGS.md`

## Boundary check
- `viewer_net` owns capability HTTP fetch transport helpers.
- `viewer_app` owns orchestration, readiness matrix state, and relay output.
- No crate boundary changes.

## Step sequence
1. Add reusable `viewer_net` one-shot simulator capability inspection helper and expose bounded methods for:
   - `fetch_interest_list_once`
   - `fetch_untrusted_simulator_message_once`
2. In `viewer_app`, add capability-readiness matrix tracking (`available`, `invoked`, `ok`, `err`, `last`).
3. Execute bounded startup probes for available `InterestList` and `UntrustedSimulatorMessage` URLs and log outcomes.
4. Add explicit readiness matrix relay line in periodic parallel-protocol summary output.
5. Add EventQueue `cap not found` bounded reconnect trigger with config knob.
6. Add/update targeted tests and env docs.
7. Run validation and capture outcome.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- bounded live run with capture log:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_APP_LLUDP_STARTUP_PARITY_BUNDLE=on`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_capability_readiness_2026-04-01.jsonl`
  - `cargo run -p viewer_app`

## Risks and open questions
- Some capability probes may return non-LLSD or non-success responses by design; treat this as readiness evidence, not immediate breakage.
- Reconnect-on-cap-not-found can increase reconnect churn if threshold is too low; keep bounded and configurable.

## Deferred-too-early candidates captured
- Full capability client parity for `InterestList`/`UntrustedSimulatorMessage` remains deferred.

## Learnings pre-check
- L33: ingress success must be judged by actual object updates, not generic traffic.
- L40/L41/L42: capability-path evidence must drive branching.
- L58: parity-bundle gate fail requires capability-readiness branch, not more LLUDP guessing.

## Completion criteria
- Readiness matrix relay appears with explicit per-cap status.
- `InterestList` and `UntrustedSimulatorMessage` one-shot invocation probes run when available and log outcome.
- EventQueue cap-not-found reconnect policy is implemented and bounded.
- Required validation passes and continuity artifacts are updated.
