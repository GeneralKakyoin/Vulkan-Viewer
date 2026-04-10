# HANDOFF: Texture Throughput + Fetch Parity Hardening (2026-04-10)

## What Changed
- `viewer_app` texture streaming throughput increased:
  - `LIVE_TEXTURE_FETCH_MAX_INFLIGHT` raised `16 -> 48`
  - `tick_scene_textures(...)` now polls texture cache with `A10_REQUESTS_PER_TICK_CAP` (64) instead of fixed `16`
- `viewer_net` texture fetch path now uses Firestorm-style user-agent and cookie-aware candidate traversal for texture asset requests.
- Added texture fetch regression tests for:
  - candidate-first fetch success
  - Set-Cookie reuse between candidate attempts

## Validation Run
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net fetch_texture_asset_bytes_uses_firestorm_style_candidate_first_with_accept_header -- --nocapture`
- `cargo test -p viewer_net fetch_texture_asset_bytes_reuses_set_cookie_between_attempts -- --nocapture`
- bounded live run (`cargo run -p viewer_app`, timeout-bounded) with:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_APP_RENDER_PROOF=on`
  - `VIEWER_APP_RENDER_PROOF_WINDOW_SECS=25`
  - `VIEWER_APP_RENDER_PROOF_LOG_PATH=artifacts/logs/render_live_proof_2026-04-10_texburst1.jsonl`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_render_live_proof_2026-04-10_texburst1.jsonl`
  - `VIEWER_OBJECT_FEED_EXPORT_MAX=512`

## Exact Current State
- Live proof artifact: `artifacts/logs/render_live_proof_2026-04-10_texburst1.jsonl` => `PASS`.
- In-window texture coverage reached `exported_texture_ids=98 ready=86 unresolved=12`.
- Prior bounded runs were significantly lower (`ready=22/100` to `ready=36/108`).
- Some texture IDs still fail with capability `403` and remain unresolved.

## Exact Next Step
1. Add focused texture-failure bucket diagnostics for 403 bodies (auth expired vs missing key) on texture fetches, mirroring existing mesh bucket clarity.
2. Run another bounded live capture with the same proof window and compare unresolved sample churn over 2 windows.
3. If unresolved remains high, add bounded retry policy for specific retryable texture error buckets only (not blanket retries).

## Blockers / Risks
- Capability-denied texture IDs (`403`) may be truly inaccessible and can still leave surfaces fallback-colored.
- Object-feed export truncation still limits total active coverage in very dense regions unless `VIEWER_OBJECT_FEED_EXPORT_MAX` is raised.
