# Plan: RegionHandshake Unblock via OpenSimulator Endpoint/Handshake Parity (2026-04-01)

## Objective
Use the new OpenSimulator reference evidence to unblock `RegionHandshake` with the smallest bounded patch set, without widening scope into unrelated LLUDP parity work.

## Scope
- Add endpoint-shape observability for EventQueue simulator-target messages.
- Parse endpoint fields robustly (including binary-IP shapes) and target follow-up sends to authoritative `ip:port`.
- Add explicit handshake progression evidence (`RegionHandshake` -> `RegionHandshakeReply` send attempt/result).
- Re-run bounded live capture and evaluate handshake gate outcome.

## Current known state
- Follow-up sends for `EnableSimulator` are present, but current captures are mostly `port`-only details and still show:
  - `region_handshake=none`
  - `region_handshake_reply=none`
  - `lludp_object_gate=FAIL`
- OpenSimulator reference indicates:
  - `EnableSimulator` can encode endpoint as `SimulatorInfo.IP` (binary) + `Port`.
  - `EstablishAgentCommunication` can include `sim-ip-and-port`.
  - initial-data/object stream progression depends on successful handshake-reply progression.

## Files and components touched
- `crates/viewer_net/src/lib.rs`
  - EventQueue target extraction helpers.
  - Endpoint parsing utilities for simulator target fields.
- `crates/viewer_app/src/main.rs`
  - EventQueue diagnostics/relay details.
  - EnableSimulator follow-up targeting logic.
  - RegionHandshakeReply send-attempt/result relay lines.
- Docs and continuity artifacts for this slice.

## Boundary check
- `viewer_net`: parsing/extraction + transport-target helpers only.
- `viewer_app`: orchestration and diagnostics emission only.
- No `viewer_grid` behavior changes.
- No architecture or crate ownership changes.

## Step sequence
1. Add bounded raw-field diagnostics for `EnableSimulator` and `EstablishAgentCommunication` events.
   - Include flattened key/value preview and explicit marker when endpoint is unresolved.
2. Extend simulator-target endpoint extraction in `viewer_net` to support:
   - `sim-ip-and-port` string.
   - `SimulatorInfo.IP` (binary-decoded IPv4/IPv6) + `SimulatorInfo.Port`.
   - existing string `IP` + `Port`.
3. Update `viewer_app` follow-up routing:
   - prefer authoritative extracted `ip:port`;
   - retain bounded fallback path when only port is available;
   - emit endpoint source label in relay (`sim_ip_and_port`, `simulatorinfo_binary_ip_port`, `ip_port_fields`, `port_only_fallback`).
4. Add explicit handshake progression relay lines:
   - `region_handshake_observed ...`
   - `region_handshake_reply_send:ok|err ...`
   - include endpoint + packet-id evidence where available.
5. Execute one bounded live run for handshake decision:
   - validate whether `RegionHandshake` and `RegionHandshakeReply` first-seen indices move from `none`.
6. If handshake still absent, stop and write a decision-complete report naming the remaining unknown (no broad startup packet expansion in this slice).

## Validation plan
1. `cargo fmt --all`
2. `cargo check -p viewer_net -p viewer_app`
3. `cargo test -p viewer_net -p viewer_app`
4. bounded live capture:
   - `VIEWER_APP_LIVE_STARTUP=on`
   - `VIEWER_APP_CAPABILITY_PROBES_REQUIRE_EVENT_QUEUE_OK=true`
   - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_region_handshake_opensim_endpoint_parity_2026-04-01.jsonl`
   - `cargo run -p viewer_app` (timeout-bounded)

## Risks and open questions
- EventQueue payload variants may still omit usable endpoint data in this route.
- Even with correct endpoint targeting, upstream simulator policy/session state could still suppress handshake.
- Handshake progression may fail due to strict field validation (`AgentID`/`SessionID`/`CircuitCode`) outside endpoint parsing; this slice adds evidence for that branch but does not yet alter credentials/session shaping.

## Deferred-too-early candidates captured
- None for this bounded slice.

## Learnings pre-check
- Applied:
  - L58, L60, L67, L68
  - Existing LLUDP hard-gate policy (`ObjectUpdate*` with local-id evidence) remains unchanged.

## Completion criteria
- EventQueue simulator-target diagnostics explicitly show endpoint-source resolution outcomes.
- Follow-up send relays include endpoint source and resolved `ip:port` when available.
- Handshake progression lines show concrete `RegionHandshake`/`RegionHandshakeReply` state transitions (or explicit persistent absence with richer endpoint evidence).
- A single bounded live run produces decision-quality evidence for next branch selection.
