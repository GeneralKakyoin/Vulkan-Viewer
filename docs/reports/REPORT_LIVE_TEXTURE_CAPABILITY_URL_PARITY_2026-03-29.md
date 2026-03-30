# REPORT_LIVE_TEXTURE_CAPABILITY_URL_PARITY_2026-03-29

## Summary of implemented work
- Added ordered texture capability URL generation in `viewer_grid::AssetCapabilityPolicy`, with Firestorm-style `/?texture_id=` tried first for both `GetTexture` and `ViewerAsset`.
- Added a shared bounded texture HTTP fetch helper in `viewer_net` so live scene textures and profile images now reuse the same candidate retry behavior.
- Updated the live texture request path in `viewer_app` to use candidate URLs instead of a single exact URL.
- Added targeted tests for URL ordering, fallback-to-second-candidate behavior, and profile-image reuse of the shared helper.

## Files changed
- `crates/viewer_grid/src/lib.rs`
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_LIVE_TEXTURE_CAPABILITY_URL_PARITY_2026-03-29.md`
- `docs/reviews/REVIEW_PLAN_LIVE_TEXTURE_CAPABILITY_URL_PARITY_2026-03-29.md`
- `docs/reviews/REVIEW_IMPL_LIVE_TEXTURE_CAPABILITY_URL_PARITY_2026-03-29.md`
- `docs/reports/REPORT_LIVE_TEXTURE_CAPABILITY_URL_PARITY_2026-03-29.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/LEARNINGS.md`

## Validation run
- `cargo fmt --all` -> PASSED
- `cargo check -p viewer_grid -p viewer_net -p viewer_app` -> PASSED
- `cargo test -p viewer_grid -p viewer_net -p viewer_app` -> PASSED

## Result status
- Implemented and targeted validation passed.
- Non-blocking warning remains during `check`/`test`: `viewer_render` `DEBUG_CLIP_SPACE_TRIANGLE` dead code warning.
- Connected live validation was not run in this pass.

## Risks or follow-up items
- The provided `C:\\Users\\matti\\Desktop\\fire.pcapng` did not include transfer-level texture payloads, so it could not prove the request shape end to end.
- If live textures still do not appear after this fix, the next investigation should confirm whether real texture IDs are ever reaching the request queue from live object ingress.

## Learnings delta
- `added` - `L28` documenting that texture capability URLs must be treated as ordered candidates through one shared fetch helper.

## Continuity updates performed
- Added plan artifact for this slice.
- Added plan review and implementation review artifacts.
- Updated `docs/CURRENT_STATE.md`.
- Updated `docs/HANDOFF.md`.
- Updated `docs/LEARNINGS.md`.
