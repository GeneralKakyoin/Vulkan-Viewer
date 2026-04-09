# Review: Implementation Startup Receive-First + Immediate ACK Flush Parity Slice (2026-04-03)

## Verdict
- Approved.

## Correctness Findings
- Startup prime now performs a bounded prelude social receive drain before sending startup-interest bundle.
- `poll_social_events` now flushes pending LLUDP ACK IDs immediately after each inbound payload observation.
- Live bounded non-strict run confirms LLUDP object ingress gate now reaches PASS in first steady-state window.

## Validation Adequacy
- `cargo fmt --all` PASS
- `cargo check -p viewer_net -p viewer_app` PASS
- `cargo test -p viewer_net send_pending_region_handshake_reply -- --nocapture` PASS
- `cargo test -p viewer_app -- --nocapture` PASS
- bounded live run PASS with object gate evidence

## Risks / Open Questions
- Startup send-order parity with Firestorm remains imperfect (first divergence still at index 3), but functional object-ingress objective is met in this bounded window.

## Learnings Delta Verdict
- add.
Reason: receive-first + immediate ACK flush materially changed outcome from object gate FAIL to PASS in bounded Fidelis run.
