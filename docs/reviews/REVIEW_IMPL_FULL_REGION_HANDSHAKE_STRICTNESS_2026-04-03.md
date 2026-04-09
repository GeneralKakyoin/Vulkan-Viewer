# Review: Implementation Full Region Handshake Strictness + Recovery Nudge (2026-04-03)

## Verdict
- Partially approved: implementation is correct and bounded, but route-level full handshake objective remains unmet in live evidence.

## Architecture / Boundary Fit
- `viewer_net`: handshake-reply policy control and handshake-observed helper.
- `viewer_app`: optional strict-mode startup nudge orchestration.
- No crate boundary violations.

## Correctness Concerns
- Strict mode now enforces: no `RegionHandshakeReply` without observed inbound `RegionHandshake`.
- Default behavior remains unchanged unless strict mode env is enabled.
- Bounded re-prime logic prevents unbounded startup packet spam.

## Modularity / Maintainability
- Added explicit API:
  - `Connection::has_observed_region_handshake()`
  - `Connection::set_require_observed_region_handshake_for_reply(bool)`
- Added explicit config knob in app for strict behavior.

## Validation Adequacy
- `cargo fmt --all` PASS
- `cargo check -p viewer_net -p viewer_app` PASS
- `cargo test -p viewer_net send_pending_region_handshake_reply -- --nocapture` PASS
- `cargo test -p viewer_app -- --nocapture` PASS
- bounded live A/B runs executed (default vs strict).

## Risks / Open Questions
- Strict mode on Fidelis still shows no inbound `RegionHandshake`; object ingress can stall in strict mode.
- Indicates upstream simulator route behavior remains the blocker, not local send/reply policy correctness.

## Learnings Delta Verdict
- none.

## Approval Status
- Code changes accepted; objective "full region handshake observed live" remains open.
