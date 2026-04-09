# Plan: Fallback RegionHandshakeReply Ordering Parity (2026-04-03)

## Scope
- Adjust fallback-only `RegionHandshakeReply` scheduling so it is no longer sent in the startup prime prelude.
- Keep strict-mode behavior unchanged.
- Validate impact with bounded non-strict startup capture and Firestorm first-divergence diff rerun.

## Current Known State
- Firestorm-vs-viewer diff showed first divergence at send index 3: Firestorm `ViewerEffect`, viewer `RegionHandshakeReply`.
- Early fallback reply was emitted from startup prime path before startup interest bundle completed.

## Files / Components Touched
- `crates/viewer_app/src/main.rs`
- `crates/viewer_net/src/lib.rs`
- `docs/reviews/REVIEW_PLAN_FALLBACK_REGION_HANDSHAKE_REPLY_ORDERING_2026-04-03.md`
- `docs/reviews/REVIEW_IMPL_FALLBACK_REGION_HANDSHAKE_REPLY_ORDERING_2026-04-03.md`
- `docs/reports/REPORT_FALLBACK_REGION_HANDSHAKE_REPLY_ORDERING_2026-04-03.md`

## Boundary Check
- No architecture changes.
- Startup orchestration / transport call-order only.

## Step Sequence
1. Remove startup-prime pre-send of pending handshake reply.
2. Remove pre-receive send in social poll loop; keep post-receive evaluation.
3. Run fmt/check/tests.
4. Run bounded non-strict startup capture and recompute Firestorm first-divergence.

## Validation Plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net send_pending_region_handshake_reply -- --nocapture`
- `cargo test -p viewer_app -- --nocapture`
- bounded non-strict run + diff artifact

## Risks / Open Questions
- Delaying fallback reply may reduce early unlock on routes that expect immediate fallback.
- Ordering parity gain may still not unlock object ingress.

## Learnings Pre-Check
- L38/L39/L71 apply (evidence-led change, fallback semantics, avoid broad speculative edits).

## Completion Criteria
- `RegionHandshakeReply` is no longer emitted during startup prime prelude.
- First-divergence artifact no longer shows `RegionHandshakeReply` at index 3.
- Validation commands pass.
