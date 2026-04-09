# Review: Plan Mesh Fetch Cookie-State Parity Hardening (2026-04-03)

## Verdict
- Approved.

## Architecture / Boundary Fit
- Constrained to `viewer_net` HTTP transport path and tests.
- No cross-crate ownership drift.

## Correctness Concerns
- Cookie carry-over must not regress existing candidate fallback behavior.
- Should remain deterministic when no `Set-Cookie` is present.

## Modularity / Maintainability
- Keep cookie parsing/merge in small helper functions.
- Preserve existing generic fetch helper for non-mesh paths.

## Validation Adequacy
- Targeted unit tests + bounded live run are adequate for this slice.

## Risks / Open Questions
- Even perfect cookie carry-over may not bypass CDN authorization policy.

## Learnings Delta Verdict
- none.

## Required Revisions
- None.
