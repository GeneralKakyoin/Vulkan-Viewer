# REPORT_RENDER_TEXTURE_THROUGHPUT_AND_FETCH_PARITY_2026-04-10

## Summary of implemented work
- Increased app-side live texture streaming throughput:
  - `LIVE_TEXTURE_FETCH_MAX_INFLIGHT` raised from `16` to `48`.
  - `tick_scene_textures(...)` now polls fixture/live texture queue with `A10_REQUESTS_PER_TICK_CAP` (64) instead of fixed `16`.
- Hardened `viewer_net` texture fetch request shape:
  - `fetch_texture_asset_bytes(...)` now uses Firestorm user-agent and cookie-aware candidate traversal via `fetch_bytes_from_candidate_urls_with_cookie_state(...)`.
- Added/updated texture fetch regression tests in `viewer_net`:
  - assert candidate-first texture fetch still succeeds.
  - assert Set-Cookie from first failed candidate is reused on subsequent candidate.

## Files changed
- `crates/viewer_app/src/main.rs`
- `crates/viewer_net/src/asset_fetch.rs`
- `crates/viewer_net/src/lib.rs`
- `docs/plans/PLAN_RENDER_TEXTURE_THROUGHPUT_AND_FETCH_PARITY_2026-04-10.md`
- `docs/reviews/REVIEW_PLAN_RENDER_TEXTURE_THROUGHPUT_AND_FETCH_PARITY_2026-04-10.md`

## Validation run
- `cargo fmt --all` -> PASS
- `cargo check -p viewer_net -p viewer_app` -> PASS
- `cargo test -p viewer_net fetch_texture_asset_bytes_uses_firestorm_style_candidate_first_with_accept_header -- --nocapture` -> PASS
- `cargo test -p viewer_net fetch_texture_asset_bytes_reuses_set_cookie_between_attempts -- --nocapture` -> PASS
- bounded live run (`cargo run -p viewer_app`, timeout-bounded 90s) with:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_APP_RENDER_PROOF=on`
  - `VIEWER_APP_RENDER_PROOF_WINDOW_SECS=25`
  - `VIEWER_APP_RENDER_PROOF_LOG_PATH=artifacts/logs/render_live_proof_2026-04-10_texburst1.jsonl`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_render_live_proof_2026-04-10_texburst1.jsonl`
  - `VIEWER_OBJECT_FEED_EXPORT_MAX=512`
  - Result: `PASS`

## Result status
- PASS with live evidence of improved texture readiness:
  - prior recent bounded runs: ready ~`22/100` to `36/108`
  - this run: `texture_bind coverage ... exported_texture_ids=98 ready=86 unresolved=12`
- Texture 403 failures still occur for a subset of IDs and remain an active runtime limitation.

## Risks or follow-up items
- Some unresolved IDs are still capability-denied (`403`) and may require capability-policy classification/fallback strategy.
- Dense scenes still rely on export truncation policy (`VIEWER_OBJECT_FEED_EXPORT_MAX`) for manageable ingest windows.

## Learnings delta
- none - no new durable learning identified; this task applied already-known bottleneck patterns and validated expected improvements.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md` latest notable changes section.
- Updated `docs/HANDOFF.md` with exact next steps and current blockers.
