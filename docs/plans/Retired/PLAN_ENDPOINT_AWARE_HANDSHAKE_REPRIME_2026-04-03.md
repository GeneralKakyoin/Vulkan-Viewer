# Plan: Endpoint-Aware LLUDP Handshake Re-prime + Reply Targeting (2026-04-03)

## Scope
- Investigate and fix likely OpenSim-aligned handshake blockers in our startup LLUDP path.
- Implement endpoint-aware `RegionHandshakeReply` targeting.
- Add bounded handshake re-prime bundle (`UseCircuitCode` + `CompleteAgentMovement`) for active/observed endpoints.
- Keep scope to `viewer_net` + `viewer_app` startup orchestration and diagnostics.

## Current Known State
- OpenSim sends `RegionHandshake` after accepted `UseCircuitCode`.
- Our runtime can send one fallback `RegionHandshakeReply` without observed inbound handshake.
- Current sends are one-shot per endpoint action; no dedicated bounded handshake resend bundle exists.
- Recent Firestorm Fidelis capture shows `RegionHandshake=0`, `RegionHandshakeReply=1` outbound.

## Files / Components Touched
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/reviews/REVIEW_PLAN_ENDPOINT_AWARE_HANDSHAKE_REPRIME_2026-04-03.md`
- `docs/reviews/REVIEW_IMPL_ENDPOINT_AWARE_HANDSHAKE_REPRIME_2026-04-03.md`
- `docs/reports/REPORT_ENDPOINT_AWARE_HANDSHAKE_REPRIME_2026-04-03.md`
- continuity docs (`docs/CURRENT_STATE.md`, `docs/HANDOFF.md`, `docs/LEARNINGS.md` delta check)

## Boundary Check
- No crate-boundary expansion.
- No architecture rewrite.
- Firestorm/OpenSim used as protocol behavior references only.

## Step Sequence
1. Add sender-endpoint tracking for inbound first-simulator packets in `viewer_net`.
2. Route `RegionHandshakeReply` to best-known handshake endpoint (region-handshake sender, then bootstrap sender, then circuit target).
3. Add bounded handshake re-prime API in `viewer_net` that sends `UseCircuitCode` + `CompleteAgentMovement` across deduped candidate endpoints.
4. Wire strict-mode reprime loop in `viewer_app` to use the new handshake re-prime bundle before interest re-prime.
5. Add/adjust targeted tests for reply-target selection and re-prime behavior.
6. Run fmt/check/tests.
7. Write report + continuity updates.

## Validation Plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net send_pending_region_handshake_reply -- --nocapture`
- `cargo test -p viewer_net send_handshake_reprime_bundle -- --nocapture`
- `cargo test -p viewer_app -- --nocapture`

## Risks / Open Questions
- Re-prime traffic must stay bounded and deterministic.
- Endpoint override must not regress regions that only accept root first-sim endpoint.
- Strict-mode behavior may still vary by region route.

## Deferred Too-Early Candidates
- none.

## Learnings Pre-Check
- L68/L69/L71 directly constrain this plan (endpoint/handshake progression evidence, viewer flag semantics, avoid speculative broad packet spray).

## Completion Criteria
- `RegionHandshakeReply` can target observed sender endpoint instead of only initial target.
- Bounded handshake re-prime bundle is implemented and callable from `viewer_app` strict-mode loop.
- Targeted tests pass for new behavior.
- Continuity and report artifacts updated with validation outcomes.
