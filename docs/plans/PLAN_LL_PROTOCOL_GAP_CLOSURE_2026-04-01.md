# Plan: LL Protocol Gap Closure (2026-04-01)

## Scope
Close the highest-impact protocol gaps identified in `REPORT_LL_PROTOCOL_FULL_STACK_EXTRACTION_2026-04-01.md` with bounded, evidence-first patches that preserve crate boundaries.

## Current known state
- Login/auth fallback and bootstrap extraction are operational (`agent_id/session_id/circuit_code/seed_capability`).
- Capability shaping across core lanes is operational and EventQueue-gated ordering is in place.
- EventQueue endpoint extraction now handles `sim-ip-and-port` and binary `SimulatorInfo.IP` forms, and child follow-up sends are happening.
- LLUDP ingress gate still fails because `RegionHandshake` and `RegionHandshakeReply` are absent in bounded live runs.

Evidence:
- `crates/viewer_net/src/lib.rs:2359-2376`
- `crates/viewer_grid/src/lib.rs:451-509`
- `crates/viewer_net/src/lib.rs:8411-8450`
- `artifacts/logs/network_debug_region_handshake_opensim_endpoint_parity_2026-04-01.jsonl:21-25`
- `artifacts/logs/network_debug_region_handshake_opensim_endpoint_parity_2026-04-01.jsonl:56-61`

## Files and components touched
- `crates/viewer_net/src/lib.rs`
  - child-endpoint handshake eligibility diagnostics
  - bounded A/B child send-mode flag path (if needed)
  - payload identity hash diagnostics for child send verification
- `crates/viewer_app/src/main.rs`
  - relay formatting for new diagnostics
  - bounded config/env wiring for A/B mode
- `docs/reports/REPORT_LL_PROTOCOL_FULL_STACK_EXTRACTION_2026-04-01.md`
- `docs/reviews/REVIEW_PLAN_LL_PROTOCOL_GAP_CLOSURE_2026-04-01.md`
- continuity docs (`CURRENT_STATE`, `HANDOFF`, `LEARNINGS`)

## Boundary check
- `viewer_net`: transport/protocol mechanics and decode/send diagnostics only.
- `viewer_app`: orchestration and runtime relay output only.
- `viewer_grid`: no new semantics unless required by direct evidence.
- No Firestorm/OpenSim architecture copying; behavior reference only.

## Step sequence
1. Add child endpoint handshake eligibility diagnostics in `viewer_net`.
   - Record endpoint, packet IDs, first inbound low-frequency message (if any), and timeout classification.
2. Surface the same diagnostics in `viewer_app` protocol relay with one-line correlation records.
3. Run one bounded live capture and classify each child endpoint (`accepted`/`silent`/`rejected`).
4. If all child endpoints remain `silent`, add a bounded A/B child send mode (alternate socket policy) behind env flag.
5. Run paired bounded captures (A baseline, B alternate mode).
6. If either mode yields handshake, verify immediate progression (`RegionHandshakeReply` send and `lludp_object_gate` transition).
7. If neither mode yields handshake, add payload identity hash diagnostics for child `UseCircuitCode` and `CompleteAgentMovement` and rerun once.
8. Stop after decision-quality evidence is reached; do not widen into unrelated refactors.

## Validation plan
1. `cargo fmt --all`
2. `cargo check -p viewer_net -p viewer_app`
3. `cargo test -p viewer_net -p viewer_app`
4. bounded runtime captures (`cargo run -p viewer_app`, timeout-bounded) with explicit artifact paths:
   - baseline handshake-eligibility run
   - optional A/B child send mode run
5. Evidence extraction with line-anchored `rg -n` on the produced artifact(s).

## Risks and open questions
- Child endpoint silence may be caused by server policy outside viewer control.
- A/B send-mode may introduce ambiguous results if both remain silent; payload hash diagnostics are the fallback discriminator.
- EventQueue degradation can confound long captures; keep runs bounded and repeatable.

## Deferred-too-early candidates captured
- Deferred: full packet-capture-based LLUDP replayer/harness for complete parity simulation.
- Why deferred: too broad for the next bounded unblock slice; requires separate tooling scope.
- Synced in `docs/plans/DEFERRED_FEATURES.md` in this update.

## Learnings pre-check
Applied constraints from:
- L64 (lane request shape exactness)
- L68 (handshake-gated initial data progression)
- L69 (endpoint parsing parity may still leave handshake absent)

## Exact completion criteria
- New diagnostics can classify each child endpoint as `accepted`, `silent`, or `rejected` in one bounded run.
- At least one bounded run provides a decisive branch:
  - handshake observed and object-gate progression measurable, or
  - explicit evidence that current send mode and payload identity are not sufficient.
- Report includes exact artifact paths and `rg -n` line evidence for every decision.
