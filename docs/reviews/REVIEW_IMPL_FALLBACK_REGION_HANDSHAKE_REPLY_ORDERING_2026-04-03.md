# Review: Implementation Fallback RegionHandshakeReply Ordering Parity (2026-04-03)

## Verdict
- Approved.

## Correctness Findings
- Removed startup-prime pre-send path for pending `RegionHandshakeReply`.
- Removed pre-receive send in `poll_social_events`; reply evaluation now occurs after inbound packet observation.
- Strict-mode gating logic in `viewer_net` was not changed.

## Validation Adequacy
- `cargo fmt --all` passed.
- `cargo check -p viewer_net -p viewer_app` passed.
- `cargo test -p viewer_net send_pending_region_handshake_reply -- --nocapture` passed.
- `cargo test -p viewer_app -- --nocapture` passed.
- Fresh bounded non-strict run + first-divergence diff rerun completed.

## Risks / Open Questions
- Object ingress can remain blocked even with improved startup ordering.

## Learnings Delta Verdict
- none.
Reason: this is a targeted ordering correction and needs broader route confirmation before becoming a durable invariant.
