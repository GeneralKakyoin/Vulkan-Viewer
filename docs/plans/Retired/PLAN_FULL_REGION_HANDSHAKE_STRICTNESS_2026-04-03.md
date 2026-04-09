# Plan: Full Region Handshake Strictness + Recovery Nudge (2026-04-03)

## Scope
- Move LLUDP region handshake behavior to strict mode: send `RegionHandshakeReply` only after inbound `RegionHandshake` observation.
- Add bounded startup handshake nudge (re-prime interest messages) to improve handshake arrival reliability.
- Keep changes within `viewer_net` + `viewer_app` startup loop.

## Current Known State
- LLUDP object ingress can reach PASS without observed `RegionHandshake`.
- Fallback handshake reply currently sends even when handshake not observed.
- This creates partial-startup behavior and masks full handshake status.

## Files / Components Touched
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/reviews/REVIEW_IMPL_FULL_REGION_HANDSHAKE_STRICTNESS_2026-04-03.md`
- `docs/reports/REPORT_FULL_REGION_HANDSHAKE_STRICTNESS_2026-04-03.md`
- continuity docs (`docs/CURRENT_STATE.md`, `docs/HANDOFF.md`)

## Boundary Check
- No architecture changes; transport + app orchestration only.
- No grid semantic ownership moved.

## Step Sequence
1. Remove fallback send path in `send_pending_region_handshake_reply(...)`; require observed handshake flags.
2. Add `Connection` helper for `RegionHandshake` observed-state query.
3. In `viewer_app` worker loop, add bounded startup re-prime when handshake not yet observed.
4. Update/add tests for strict handshake-reply behavior and re-prime gating where applicable.
5. Run fmt/check/targeted tests + bounded live run to verify handshake observation and reply sequencing.

## Validation Plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- targeted tests in `viewer_net` for pending-handshake-reply behavior
- bounded live run and log inspection for:
  - `region_handshake_observed` != none
  - `region_handshake_reply_send` after observed handshake

## Risks / Open Questions
- Some regions may still not emit LLUDP `RegionHandshake`; strict mode may reduce legacy fallback ingress behavior.
- Re-prime cadence must be bounded to avoid packet spam.

## Learnings Pre-Check
- L10 applies for message-number correctness.
- Existing LLUDP startup learnings require explicit evidence lines rather than inferred readiness.

## Completion Criteria
- No fallback `RegionHandshakeReply` without observed `RegionHandshake`.
- Bounded startup nudge exists and is rate-limited.
- Live run evidence clearly indicates whether full handshake sequence is achieved.
