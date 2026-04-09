# Report: Startup Receive-First + Immediate ACK Flush Parity Slice (2026-04-03)

## Summary
Implemented a bounded startup timing change to improve LLUDP object ingress:
- process an initial social receive checkpoint before sending startup-interest bundle
- flush pending LLUDP ACK IDs immediately after each social receive

## Files Changed
- `crates/viewer_app/src/main.rs`
- `crates/viewer_net/src/lib.rs`
- `docs/plans/PLAN_STARTUP_RECEIVE_FIRST_ACK_FLUSH_PARITY_2026-04-03.md`
- `docs/reviews/REVIEW_PLAN_STARTUP_RECEIVE_FIRST_ACK_FLUSH_PARITY_2026-04-03.md`
- `docs/reviews/REVIEW_IMPL_STARTUP_RECEIVE_FIRST_ACK_FLUSH_PARITY_2026-04-03.md`
- `docs/reports/REPORT_STARTUP_RECEIVE_FIRST_ACK_FLUSH_PARITY_2026-04-03.md`
- `docs/LEARNINGS.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`

## Implementation Details
1. `viewer_app` startup prime ordering:
- `prime_startup_social_circuit(...)` now performs a bounded prelude `drain_startup_social_circuit(...)` before startup-interest/startup-request bundle sends.

2. `viewer_net` social receive timing:
- `poll_social_events(...)` now calls `flush_pending_ack_ids_on_circuit(...)` immediately after each inbound payload observation and fallback-reply check.

## Validation Run
- `cargo fmt --all` -> PASS
- `cargo check -p viewer_net -p viewer_app` -> PASS
- `cargo test -p viewer_net send_pending_region_handshake_reply -- --nocapture` -> PASS
- `cargo test -p viewer_app -- --nocapture` -> PASS

## Live Validation
- bounded non-strict run:
  - `artifacts/logs/live_startup_parity_receivefirst_ackflush_2026-04-03_175048.log`
  - `artifacts/logs/network_debug_startup_parity_receivefirst_ackflush_2026-04-03_175048.jsonl`

Observed:
- startup transcript now includes early fallback reply + explicit packet-ack cadence.
- first steady-state receive includes `ObjectUpdate` entries.
- `after_first_steady_state_window lludp_object_gate: verdict=PASS`.

## Firestorm Diff Snapshot
- regenerated artifact:
  - `artifacts/logs/startup_first_divergence_diff_receivefirst_ackflush_2026-04-03_175048.json`
- first divergence remains at index 3 (`ViewerEffect` vs viewer `RegionHandshakeReply`), but object ingress objective improved to PASS in bounded window.

## Result Status
- Functional objective met in bounded live evidence: LLUDP object gate now PASS.
- Startup send-order parity is still not identical to Firestorm.

## Risks / Follow-up
- Need one repeat bounded run (same route) and one alternate route run to confirm repeatability.

## Learnings Delta
- `added`: L77.

## Continuity Updates Performed
- Added this report and corresponding plan/reviews.
- Updated `docs/CURRENT_STATE.md`, `docs/HANDOFF.md`, and `docs/LEARNINGS.md`.
