# Report: Non-Retryable 4xx Texture Fetch Classification (2026-04-01)

## Summary Of Implemented Work
- Updated `viewer_app` live texture failure classification so HTTP `401/403/404` are treated as non-retryable (`MissingCapability`) instead of retryable `Transport`.
- Kept scheduler retry policy unchanged (`Timeout`/`Transport` only) while preventing deterministic denied/not-found loops.
- Added targeted unit test for bounded non-retryable HTTP status classification.

## Files Changed
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_NON_RETRYABLE_4XX_TEXTURE_FETCH_2026-04-01.md`
- `docs/reviews/REVIEW_PLAN_NON_RETRYABLE_4XX_TEXTURE_FETCH_2026-04-01.md`
- `docs/reviews/REVIEW_IMPL_NON_RETRYABLE_4XX_TEXTURE_FETCH_2026-04-01.md`
- `docs/reports/REPORT_NON_RETRYABLE_4XX_TEXTURE_FETCH_2026-04-01.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/LEARNINGS.md`

## Validation Run
- `cargo fmt --all` -> PASSED
- `cargo check -p viewer_app` -> PASSED
- `cargo test -p viewer_app` -> PASSED
- bounded live run:
  - env: `VIEWER_APP_LIVE_STARTUP=on`, `VIEWER_FIXTURE_TEXTURES=1`, `VIEWER_ASSET_SOURCE_MODE=live`
  - command: `cargo run -p viewer_app > artifacts/logs/live_texture_scheduler_runtime_4xx_nonretry_2026-04-01.log`
  - observed scheduler lines:
    - queued three texture IDs
    - terminal `failed ... reason=MissingCapability attempt=1 detail=http status 403 Forbidden`
    - no retry lines for those IDs

## Result Status
- Complete for this bounded classification fix.

## Risks Or Follow-Up Items
- For richer diagnostics, consider future split of capability/auth-denied vs not-found reasons (currently both map to `MissingCapability`).

## Learnings Delta
- `added`: `L66`.

## Continuity Updates Performed
- Updated `docs/CURRENT_STATE.md` and replaced `docs/HANDOFF.md` with the latest state/next step.
- Added durable learning entry `L66` to `docs/LEARNINGS.md`.
