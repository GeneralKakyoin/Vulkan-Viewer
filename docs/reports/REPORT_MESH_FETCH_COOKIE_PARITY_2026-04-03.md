# Report: Mesh Fetch Cookie-State Parity Hardening (2026-04-03)

## Summary
Implemented mesh-fetch cookie-state carry-over between staged probe/follow-up HTTP requests so `Set-Cookie` responses can be reused as `Cookie` request headers across fallback phases.

## Files Changed
- `crates/viewer_net/src/lib.rs`
- `docs/plans/PLAN_MESH_FETCH_COOKIE_PARITY_2026-04-03.md`
- `docs/reviews/REVIEW_PLAN_MESH_FETCH_COOKIE_PARITY_2026-04-03.md`
- `docs/reviews/REVIEW_IMPL_MESH_FETCH_COOKIE_PARITY_2026-04-03.md`
- `docs/reports/REPORT_MESH_FETCH_COOKIE_PARITY_2026-04-03.md`

## Implementation Details
1. Added mesh staged-fetch cookie state handling:
- new internal helper `fetch_bytes_from_candidate_urls_with_cookie_state(...)` returns both fetch result and merged cookie header state.
- `fetch_mesh_asset_bytes(...)` now threads cookie state across range probe and fallbacks.

2. Added cookie helpers:
- `extract_cookie_header_from_set_cookie(...)` parses response `Set-Cookie` headers into request-compatible `Cookie` pairs.
- `merge_cookie_header(...)` merges and deduplicates cookie key/value pairs deterministically.

3. Existing generic fetch API preserved:
- `fetch_bytes_from_candidate_urls(...)` remains existing public behavior for non-mesh callers.

## Validation Run
- `cargo fmt --all` -> PASS
- `cargo check -p viewer_net -p viewer_app` -> PASS
- `cargo test -p viewer_net fetch_mesh_asset_bytes_reuses_set_cookie_between_probe_and_followup -- --nocapture` -> PASS
- `cargo test -p viewer_net merge_cookie_header_prefers_new_values_and_preserves_existing -- --nocapture` -> PASS

## Live Validation
- bounded run artifacts:
  - `artifacts/logs/live_mesh_cookie_parity_2026-04-03_015824.log`
  - `artifacts/logs/network_debug_mesh_cookie_parity_2026-04-03_015824.jsonl`

Observed:
- LLUDP ingress remained healthy and cache-miss recovery remained active (`RequestMultipleObjects` observed).
- Mesh fixture request still failed with:
  - `403 Forbidden`
  - XML body `Code=AccessDenied`

## Result Status
- **Cookie-state parity hardening implemented and tested.**
- **Live mesh fetch remains blocked by `AccessDenied` in this route/session.**

## Risks / Follow-up
- Next likely blocker is capability authorization identity/parity beyond cookie state (header/session binding or lane policy).
- Next investigation should capture Firestorm mesh success on same mesh ID/session and diff request metadata at proxy/trace level if possible.

## Learnings Delta
- `none` (no new durable invariant established yet).

## Continuity Updates Performed
- Added plan/review/report artifacts for this slice.
- Updated `docs/CURRENT_STATE.md` and `docs/HANDOFF.md`.
