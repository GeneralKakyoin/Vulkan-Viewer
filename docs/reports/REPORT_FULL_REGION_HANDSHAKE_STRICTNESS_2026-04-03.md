# Report: Full Region Handshake Strictness + Recovery Nudge (2026-04-03)

## Summary
Implemented a strict-handshake option that only sends `RegionHandshakeReply` after observing inbound `RegionHandshake`, while preserving legacy fallback behavior by default. Added bounded startup re-prime nudges when strict mode is enabled.

## Files Changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_FULL_REGION_HANDSHAKE_STRICTNESS_2026-04-03.md`
- `docs/reviews/REVIEW_PLAN_FULL_REGION_HANDSHAKE_STRICTNESS_2026-04-03.md`
- `docs/reviews/REVIEW_IMPL_FULL_REGION_HANDSHAKE_STRICTNESS_2026-04-03.md`
- `docs/reports/REPORT_FULL_REGION_HANDSHAKE_STRICTNESS_2026-04-03.md`
- `docs/TESTING_REFERENCE.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`

## Implementation Details
1. `viewer_net` handshake policy control:
- added `require_observed_region_handshake_for_reply` connection policy flag (default `false`).
- added `set_require_observed_region_handshake_for_reply(...)`.
- added `has_observed_region_handshake(...)` helper.
- `send_pending_region_handshake_reply(...)` now:
  - strict mode (`true`): sends only when handshake flags have been observed
  - default mode (`false`): preserves fallback behavior.

2. `viewer_app` strict-mode orchestration:
- added env-backed config: `VIEWER_APP_REQUIRE_REGION_HANDSHAKE_REPLY` (default `false`).
- worker applies strict policy to `Connection`.
- when strict mode is enabled and handshake remains absent, app issues bounded startup re-prime (`send_startup_interest_messages`) up to 3 attempts at fixed interval.

3. Tests updated:
- `viewer_net` handshake reply tests updated for strict behavior path.
- `viewer_app` config parsing tests updated to include new env knob.

## Validation Run
- `cargo fmt --all` -> PASS
- `cargo check -p viewer_net -p viewer_app` -> PASS
- `cargo test -p viewer_net send_pending_region_handshake_reply -- --nocapture` -> PASS
- `cargo test -p viewer_app -- --nocapture` -> PASS

## Live Validation
### Default mode (fallback enabled)
- artifacts:
  - `artifacts/logs/live_region_handshake_default_2026-04-03_021321.log`
  - `artifacts/logs/network_debug_region_handshake_default_2026-04-03_021321.jsonl`
- observed:
  - `region_handshake_observed index=none`
  - `region_handshake_reply_send count=1` (fallback path)
  - `lludp_object_gate: PASS` after steady-state window.

### Strict mode (`VIEWER_APP_REQUIRE_REGION_HANDSHAKE_REPLY=true`)
- artifacts:
  - `artifacts/logs/live_region_handshake_strict_2026-04-03_021506.log`
  - `artifacts/logs/network_debug_region_handshake_strict_2026-04-03_021506.jsonl`
- observed:
  - `region_handshake_observed index=none`
  - `region_handshake_reply_send count=0`
  - bounded `region_handshake_reprime` attempts 1..3
  - `lludp_object_gate` remained `FAIL` in-window.

## Result Status
- **Strict full-handshake enforcement is implemented and live-verifiable.**
- **Full inbound `RegionHandshake` is still not observed on this Fidelis route in bounded runs.**

## Risks / Follow-up
- Route/session-specific simulator behavior still blocks full observed handshake sequence.
- Next step: cross-route A/B strict-mode capture and Firestorm side-by-side evidence on the same route/time window to isolate server-path variance.

## Learnings Delta
- `none` (no new durable invariant yet).

## Continuity Updates Performed
- Added plan/review/report for this slice.
- Updated testing reference for new env knob.
- Updated current state and handoff with live A/B outcomes.
