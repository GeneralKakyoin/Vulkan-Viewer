# Report: Endpoint-Aware LLUDP Handshake Re-prime + Reply Targeting (2026-04-03)

## Summary
Implemented endpoint-aware handshake progression hardening for LLUDP startup:
- `RegionHandshakeReply` can now target observed handshake/bootstrap sender endpoints instead of always using the initial circuit target.
- Added a bounded handshake reprime bundle API (`UseCircuitCode` + `CompleteAgentMovement`) over deduped candidate endpoints.
- Wired strict-mode reprime in `viewer_app` to use the handshake reprime bundle before startup-interest reprime.

## Files Changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_ENDPOINT_AWARE_HANDSHAKE_REPRIME_2026-04-03.md`
- `docs/reviews/REVIEW_PLAN_ENDPOINT_AWARE_HANDSHAKE_REPRIME_2026-04-03.md`
- `docs/reviews/REVIEW_IMPL_ENDPOINT_AWARE_HANDSHAKE_REPRIME_2026-04-03.md`
- `docs/reports/REPORT_ENDPOINT_AWARE_HANDSHAKE_REPRIME_2026-04-03.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`

## Implementation Details
1. Endpoint-aware sender tracking in `viewer_net`:
- Added connection state fields for `last_bootstrap_sender_endpoint` and `last_region_handshake_sender_endpoint`.
- Added sender tracking helper and wired it into first-simulator receive loops with sender visibility.

2. Reply targeting hardening:
- `send_pending_region_handshake_reply(...)` now resolves a target endpoint priority:
  - observed `RegionHandshake` sender
  - observed bootstrap sender
  - fallback `circuit.target`
- Reply still remains one-shot guarded by `region_handshake_reply_sent`.

3. Bounded handshake reprime bundle:
- Added `send_handshake_reprime_bundle(&SocialCircuit) -> Result<usize, ConnectionError>`.
- Sends deduped `UseCircuitCode` + `CompleteAgentMovement` across prioritized endpoints.

4. `viewer_app` strict-mode loop integration:
- In strict mode reprime branch, app now calls handshake reprime bundle first, then startup-interest reprime.
- Added bounded diagnostic line with sent datagram count.

5. Tests added:
- `send_pending_region_handshake_reply_prefers_observed_bootstrap_sender_endpoint`
- `send_handshake_reprime_bundle_sends_to_observed_and_primary_endpoints`

## Validation Run
- `cargo fmt --all` -> PASS
- `cargo check -p viewer_net -p viewer_app` -> PASS (pre-existing warnings only)
- `cargo test -p viewer_net send_pending_region_handshake_reply -- --nocapture` -> PASS
- `cargo test -p viewer_net send_handshake_reprime_bundle -- --nocapture` -> PASS
- `cargo test -p viewer_app -- --nocapture` -> PASS

## Result Status
- Endpoint-aware reply targeting: complete.
- Bounded handshake reprime bundle: complete.
- Strict-mode integration in app loop: complete.

## Risks / Follow-up
- Live route-specific behavior still needs bounded runtime verification to confirm inbound `RegionHandshake` appearance on current route(s).
- Recommended next run: strict-mode bounded live capture on Fidelis and one alternate route to compare handshake index and object gate progression.

## Learnings Delta
- `none`.
Reason: this slice adds mitigation behavior and deterministic tests, but does not establish a new durable, route-independent protocol invariant yet.

## Continuity Updates Performed
- Added plan/reviews/report artifacts for this slice.
- Updated `docs/CURRENT_STATE.md` and `docs/HANDOFF.md` with current implementation status and next steps.
