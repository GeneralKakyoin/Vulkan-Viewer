# Plan: Object Ingress UntrustedSimulatorMessage POST Probe Fallback (2026-04-01)

## Objective
Get `UntrustedSimulatorMessage` capability readiness probing to use the Firestorm-aligned invocation method by adding a bounded POST fallback when GET returns `405 Method Not Allowed`.

## Scope
- `viewer_net` capability probe behavior for `fetch_untrusted_simulator_message_once(...)` only.
- Keep `viewer_app` readiness wiring unchanged.
- Add targeted tests for GET->POST fallback behavior.

## Current known state
- EventQueue-gated probes now show `InterestList` succeeds post-gate.
- `UntrustedSimulatorMessage` still returns `405 Method Not Allowed` when probed with current GET helper.
- Firestorm source shows untrusted simulator messages are sent via POST with LLSD payload containing `message` and `body`.

## Files and components touched
- `crates/viewer_net/src/lib.rs`
- continuity docs/reports after validation

## Boundary check
- Change remains in transport/probe mechanics (`viewer_net` ownership).
- No grid semantics move into `viewer_app`.
- No architecture or crate-boundary change.

## Step sequence
1. Add POST capability-bytes helper for LLSD payloads.
2. Update `fetch_untrusted_simulator_message_once(...)`:
   - try existing GET path
   - on 405 fallback to POST LLSD envelope (`message`, `body`)
3. Parse success response into existing inspection shape; preserve explicit HTTP failure on non-success statuses.
4. Add unit test proving GET 405 then POST success path.
5. Run fmt/check/tests and bounded live run to capture updated readiness output.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- bounded live run:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_APP_LLUDP_STARTUP_PARITY_BUNDLE=on`
  - `VIEWER_APP_EVENT_QUEUE_CAP_NOT_FOUND_BEFORE_RECONNECT=3`
  - `VIEWER_APP_CAPABILITY_PROBES_REQUIRE_EVENT_QUEUE_OK=true`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_untrusted_probe_post_fallback_2026-04-01.jsonl`
  - `cargo run -p viewer_app` (timeout-bounded)

## Risks and open questions
- POST method may still return non-success status if message/body shape is insufficient.
- Even if this probe succeeds, LLUDP object ingress may remain blocked.

## Deferred-too-early candidates captured
- Full simulator-message parity over this capability is deferred; this slice is probe readiness only.
- No `DEFERRED_FEATURES.md` update required.

## Learnings pre-check
- L59: treat capability outcomes as first-class readiness evidence.
- L60: ordering matters for capability probes; keep EventQueue gating in place.

## Completion criteria
- `UntrustedSimulatorMessage` probe no longer hard-fails on GET 405 path when POST fallback is available.
- Logs show concrete updated readiness outcome for untrusted probe.
- Continuity docs reflect result and next decision.
