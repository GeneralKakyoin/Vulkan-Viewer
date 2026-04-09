# Review: Implementation Mesh Fetch Cookie-State Parity Hardening (2026-04-03)

## Verdict
- Approved; runtime blocker remains.

## Architecture / Boundary Fit
- Change remains in `viewer_net` mesh fetch path.
- No boundary violations.

## Correctness Concerns
- Mesh staged fetch now preserves cookie state across:
  - range probe (`bytes=0-4095`)
  - full fetch fallback
  - full-range fallback (`bytes=0-`)
- Non-mesh asset fetch path still uses existing behavior.

## Modularity / Maintainability
- Added focused helpers:
  - `extract_cookie_header_from_set_cookie(...)`
  - `merge_cookie_header(...)`
  - `fetch_bytes_from_candidate_urls_with_cookie_state(...)`
- Added targeted tests for staged cookie reuse and cookie-merge behavior.

## Validation Adequacy
- `cargo fmt --all` PASS
- `cargo check -p viewer_net -p viewer_app` PASS
- `cargo test -p viewer_net fetch_mesh_asset_bytes_reuses_set_cookie_between_probe_and_followup -- --nocapture` PASS
- `cargo test -p viewer_net merge_cookie_header_prefers_new_values_and_preserves_existing -- --nocapture` PASS
- bounded live run executed with fixture mesh and reviewed logs.

## Risks / Open Questions
- Live Fidelis fixture run still returns `403 AccessDenied` on mesh fetch after this patch.
- Indicates blocker likely outside cookie carry-over (authorization/session scope or request identity constraints).

## Learnings Delta Verdict
- none.

## Approval Status
- Implementation accepted for parity hardening; does not fully unblock live mesh bytes.
