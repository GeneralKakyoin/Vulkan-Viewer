# Review: Implementation Object Ingress UntrustedSimulatorMessage POST Probe Fallback (2026-04-01)

## Verdict
approved

## Architecture and boundary fit
- Change is confined to `viewer_net` capability transport probe logic.
- `viewer_app` orchestration boundaries remain unchanged.
- No crate-boundary drift.

## Correctness concerns
- GET->POST fallback is correctly limited to GET `405 Method Not Allowed`.
- Non-success POST outcomes still surface `ConnectionError::HttpStatus`.
- Successful but non-LLSD POST responses now map to bounded synthetic inspection instead of false error.

## Modularity and maintainability concerns
- Added focused helpers for POST LLSD capability fetch and untrusted probe payload synthesis.
- Added targeted tests for both parsed and transport-only success responses.

## Validation adequacy
- `cargo fmt --all`: pass
- `cargo check -p viewer_net -p viewer_app`: pass
- `cargo test -p viewer_net`: pass (includes new untrusted fallback tests)
- `cargo test -p viewer_app`: pass
- bounded live run: pass for evidence capture

## Risks and open questions
- Probe now confirms transport invocation path, but body remains unparsed (`probe_decode=unparsed_body`), so semantic capability parity is still open.
- LLUDP object ingress remains blocked and unchanged.

## Learnings delta verdict
- add

## Required revisions or approval status
- Approved; proceed with continuity updates and next branch decision.
