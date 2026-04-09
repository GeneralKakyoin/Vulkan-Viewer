# Plan: Mesh Fetch Cookie-State Parity Hardening (2026-04-03)

## Scope
- Harden mesh HTTP fetch path to preserve capability cookie state across staged probe/follow-up requests.
- Keep scope limited to `viewer_net` fetch path and bounded live validation.

## Current Known State
- Mesh URL shape and range/user-agent parity are already implemented.
- Live mesh fetch on Fidelis still returns `403 AccessDenied`.
- Existing fetch helper recreates HTTP client per attempt phase, discarding any `Set-Cookie` state between requests.

## Files / Components Touched
- `crates/viewer_net/src/lib.rs`
- `docs/reviews/REVIEW_IMPL_MESH_FETCH_COOKIE_PARITY_2026-04-03.md`
- `docs/reports/REPORT_MESH_FETCH_COOKIE_PARITY_2026-04-03.md`
- continuity docs (`docs/CURRENT_STATE.md`, `docs/HANDOFF.md`)

## Boundary Check
- Transport-only changes in `viewer_net`.
- No architecture or crate-boundary changes.

## Step Sequence
1. Add cookie extraction/merge helpers for `Set-Cookie -> Cookie` forwarding.
2. Add mesh-specific staged fetch helper that carries cookie state across range probe/full fetch fallbacks.
3. Keep generic asset fetch behavior unchanged for non-mesh callers.
4. Add focused tests for cookie merge and staged mesh cookie reuse.
5. Run bounded live Fidelis fixture run and inspect mesh outcome.

## Validation Plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net fetch_mesh_asset_bytes_reuses_set_cookie_between_probe_and_followup -- --nocapture`
- `cargo test -p viewer_net merge_cookie_header_prefers_new_values_and_preserves_existing -- --nocapture`
- bounded live run with fixture mesh:
  - `cargo run -p viewer_app` (time-boxed) + log inspection

## Risks / Open Questions
- Asset-CDN may deny by session policy unrelated to request cookie carry-over.
- Mesh fixture UUID may be invalid/denied regardless of request parity.

## Learnings Pre-Check
- L10: protocol/message constants must be template-sourced.
- Existing mesh-ingress learnings indicate route-specific access behavior can dominate parity fixes.

## Completion Criteria
- Mesh staged fetch path preserves cookie state across probe/follow-up in tests.
- Bounded live run establishes whether cookie parity changes runtime `403` behavior.
- Continuity docs updated with result and next blocker.
