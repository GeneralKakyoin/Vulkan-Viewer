# Plan: Non-Retryable 4xx Texture Fetch Classification (2026-04-01)

## Objective
Stop retrying deterministic denied/not-found live texture fetch responses by classifying HTTP `401/403/404` as non-retryable asset failures.

## Scope
- Update `viewer_app` asset fetch failure classification.
- Keep scheduler retry policy unchanged (`Timeout`/`Transport` only); change the reason mapping so 401/403/404 do not map to retryable `Transport`.
- Add targeted tests for non-retryable status classification helpers.

## Current Known State
- Live scheduler currently retries `403 AccessDenied` up to max attempts because those responses are mapped to `Transport`.

## Files And Components Touched
- `crates/viewer_app/src/main.rs`
- `docs/reviews/REVIEW_PLAN_NON_RETRYABLE_4XX_TEXTURE_FETCH_2026-04-01.md`
- `docs/reviews/REVIEW_IMPL_NON_RETRYABLE_4XX_TEXTURE_FETCH_2026-04-01.md`
- `docs/reports/REPORT_NON_RETRYABLE_4XX_TEXTURE_FETCH_2026-04-01.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`

## Boundary Check
- `viewer_app` orchestration/failure classification only.
- No `viewer_net`/`viewer_grid` boundary changes.

## Step Sequence
1. Add helper for non-retryable texture HTTP statuses (`401/403/404`).
2. Update `classify_asset_fetch_failure_reason(...)` to map those statuses to non-retryable `MissingCapability`.
3. Add targeted unit tests for helper behavior.
4. Validate with fmt/check/test and bounded live rerun to confirm no retry loop on 403.

## Validation Plan
1. `cargo fmt --all`
2. `cargo check -p viewer_app`
3. `cargo test -p viewer_app`
4. bounded live run with `VIEWER_APP_LIVE_STARTUP=on`, `VIEWER_ASSET_SOURCE_MODE=live`, `VIEWER_FIXTURE_TEXTURES=1` and inspect `texture_fetch` relay lines.

## Risks And Open Questions
- Mapping `401/403/404` to `MissingCapability` is coarse but intentionally non-retryable and bounded.

## Deferred-Too-Early Candidates Captured
- none

## Learnings Pre-Check
- Applies `L64`, `L65`.
- No additional durable learning expected for this narrow classification fix.

## Completion Criteria
- `403` texture fetches terminate without retry backoff loop.
- Targeted validation passes and live relay evidence confirms behavior.
