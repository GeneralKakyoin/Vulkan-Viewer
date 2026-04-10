# REPORT: Render Truncated Feed Retention and Texture Bind Truthfulness (2026-04-10)

## Summary of implemented work
- Updated `viewer_core` object-feed seam application to always retain against current exported object IDs, even when export is truncated.
  - This prevents stale world-object-feed proxies from accumulating unboundedly across truncated windows.
- Updated `viewer_app` texture coverage relay to classify renderer-bound texture IDs (`renderer.has_texture(id)`) as `ready` before consulting transient live-result maps.
  - This aligns diagnostics with actual render bind state.
- Updated core test fixture expectations for deterministic proxy retention under truncated exports.

## Files changed
- `crates/viewer_core/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_RENDER_TRUNCATED_FEED_RETENTION_AND_TEXTURE_BIND_TRUTHFULNESS_2026-04-10.md`
- `docs/reviews/REVIEW_PLAN_RENDER_TRUNCATED_FEED_RETENTION_AND_TEXTURE_BIND_TRUTHFULNESS_2026-04-10.md`
- `docs/reviews/REVIEW_IMPL_RENDER_TRUNCATED_FEED_RETENTION_AND_TEXTURE_BIND_TRUTHFULNESS_2026-04-10.md`

## Validation run
- `cargo fmt --all` -> passed
- `cargo check -p viewer_core -p viewer_app` -> passed
- `cargo test -p viewer_core scene_world_object_feed_proxies_create_and_remove_deterministically -- --nocapture` -> passed
- `cargo test -p viewer_app test_extract_decoded_object_feed_texture_ids_is_deterministic_and_capped -- --nocapture` -> passed
- bounded live run (`cargo run -p viewer_app`, timeout-bounded) with:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_APP_RENDER_PROOF=on`
  - `VIEWER_APP_RENDER_PROOF_LOG_PATH=artifacts/logs/render_live_proof_2026-04-10_truncfix1.jsonl`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_render_live_proof_2026-04-10_truncfix1.jsonl`
  - stdout/stderr logs:
    - `artifacts/logs/live_truncfix1_stdout.log`
    - `artifacts/logs/live_truncfix1_stderr.log`

## Result status
- Render proof verdict: `PASS` in `artifacts/logs/render_live_proof_2026-04-10_truncfix1.jsonl`.
- Texture coverage now reports non-zero ready counts after decode/bind (example: `ready=20`, later `ready=11`) instead of stuck unresolved-only output.
- Truncated export windows remain root/child balanced (`root_objects=64 parented_objects=64`) while avoiding stale proxy accumulation from previous snapshots.

## Risks or follow-up items
- Export cap is still 128; dense regions can still omit desired objects from the active set.
- A subset of texture IDs still unresolved due fetch/capability failures (403-class), requiring separate transport/capability handling if full parity is required.

## Learnings delta
- `none` — no new durable learning identified; changes operationalize existing learnings around bounded live verification and object-feed stability.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md` with latest notable changes for this slice.
- Updated `docs/HANDOFF.md` with exact current state, validation, and next step.
