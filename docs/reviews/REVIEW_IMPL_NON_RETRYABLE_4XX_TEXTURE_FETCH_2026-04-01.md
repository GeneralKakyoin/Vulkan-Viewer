# Review: Implementation Non-Retryable 4xx Texture Fetch (2026-04-01)

## Verdict
Approved.

## Architecture And Boundary Fit
- Change is confined to `viewer_app` failure classification and retry behavior through existing scheduler hooks.
- No crate-boundary ownership changes.

## Correctness Concerns
- Addressed: deterministic `401/403/404` now classified as non-retryable (`MissingCapability`) and no longer backoff-looped.
- Retry path remains bounded for transport/timeouts.

## Modularity And Maintainability Concerns
- Added explicit helper `non_retryable_texture_http_status(...)` and targeted unit test.

## Validation Adequacy
- `cargo fmt --all` passed.
- `cargo check -p viewer_app` passed.
- `cargo test -p viewer_app` passed.
- Bounded live run confirmed `failed attempt=1` without retries for HTTP `403` texture requests.

## Risks And Open Questions
- `401/403/404` are grouped under `MissingCapability`; if future diagnostics need finer operator messaging, add a richer failure enum variant later.

## Learnings Delta Verdict
- `add` (L66): deterministic denied/not-found HTTP texture responses should be non-retryable in the scheduler path.

## Required Revisions Or Approval Status
- Approval status: approved.
