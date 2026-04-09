# Review: Plan Non-Retryable 4xx Texture Fetch (2026-04-01)

## Verdict
Approved.

## Architecture And Boundary Fit
- Fits `viewer_app` responsibility; no crate-boundary drift.

## Correctness Concerns
- Keep retry policy narrow and deterministic.
- Ensure tests cover status-to-retryability mapping.

## Modularity And Maintainability Concerns
- Prefer a dedicated helper for HTTP-status retryability.

## Validation Adequacy
- Proposed fmt/check/test + bounded live evidence is adequate.

## Risks And Open Questions
- Coarse `MissingCapability` mapping for all 401/403/404 is acceptable for this bounded fix.

## Learnings Delta Verdict
- none (plan stage).

## Required Revisions Or Approval Status
- Approval status: approved.
