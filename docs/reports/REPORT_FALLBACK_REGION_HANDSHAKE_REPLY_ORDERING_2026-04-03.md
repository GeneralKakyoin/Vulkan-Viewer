# Report: Fallback RegionHandshakeReply Ordering Parity (2026-04-03)

## Summary
Implemented fallback-only startup ordering adjustment for `RegionHandshakeReply` so it is no longer sent in the startup prime prelude or before first social receive iteration.

## Files Changed
- `crates/viewer_app/src/main.rs`
- `crates/viewer_net/src/lib.rs`
- `docs/plans/PLAN_FALLBACK_REGION_HANDSHAKE_REPLY_ORDERING_2026-04-03.md`
- `docs/reviews/REVIEW_PLAN_FALLBACK_REGION_HANDSHAKE_REPLY_ORDERING_2026-04-03.md`
- `docs/reviews/REVIEW_IMPL_FALLBACK_REGION_HANDSHAKE_REPLY_ORDERING_2026-04-03.md`
- `docs/reports/REPORT_FALLBACK_REGION_HANDSHAKE_REPLY_ORDERING_2026-04-03.md`

## Implementation Details
1. `viewer_app` startup prime:
- removed pre-send call to `send_pending_region_handshake_reply(...)` in `prime_startup_social_circuit(...)`.

2. `viewer_net` social polling:
- removed pre-receive `send_pending_region_handshake_reply(...)` call at top of `poll_social_events(...)`.
- kept existing post-receive evaluation so fallback reply can still send once inbound packets are observed.

## Validation Run
- `cargo fmt --all` -> PASS
- `cargo check -p viewer_net -p viewer_app` -> PASS
- `cargo test -p viewer_net send_pending_region_handshake_reply -- --nocapture` -> PASS
- `cargo test -p viewer_app -- --nocapture` -> PASS

## Live / Diff Evidence
- Fresh non-strict run:
  - `artifacts/logs/live_startup_parity_non_strict_post_reply_order_2026-04-03_174328.log`
  - `artifacts/logs/network_debug_startup_parity_non_strict_post_reply_order_2026-04-03_174328.jsonl`
- Updated first-divergence artifact:
  - `artifacts/logs/startup_first_divergence_diff_post_reply_order_2026-04-03_174328.json`

Observed outcome:
- First divergence moved from `RegionHandshakeReply` at index 3 to `AgentThrottle` at index 3.
- `RegionHandshakeReply` now appears later (`#12`) after inbound packets are observed.
- LLUDP object gate still remained `FAIL` in this bounded Fidelis window.

## Result Status
- Ordering correction: complete and verified in startup transcript.
- Object ingress unblock: not achieved in this bounded run.

## Risks / Follow-up
- Next likely parity target is startup control ordering around `AgentThrottle`/`ViewerEffect`/`PacketAck` relative to Firestorm’s early sequence.

## Learnings Delta
- none.
Reason: one route/window shows corrected ordering but no unlock; needs further parity slices before declaring a durable rule.
