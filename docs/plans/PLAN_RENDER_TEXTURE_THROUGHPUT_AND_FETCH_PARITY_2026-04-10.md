# Plan: Texture Throughput and Fetch Parity Hardening (2026-04-10)

## Objective
Reduce persistent white/untextured object rendering in dense live scenes by improving texture streaming throughput and texture fetch request parity.

## Scope
- Increase texture ingestion throughput on the app side for dense exported object windows.
- Harden texture HTTP fetch shape toward Firestorm-style request parity.
- Keep crate boundaries unchanged.

## Current known state
- Dense scenes export 70-100+ texture IDs while ready coverage stalls around ~20-36 in bounded runs.
- Many IDs remain unresolved (not failed), indicating request/throughput bottlenecks.
- Some IDs fail with 403; lane probe shows valid texture probe shapes can succeed on ViewerAsset.

## Files and components touched
- `crates/viewer_app/src/main.rs`
- `crates/viewer_net/src/asset_fetch.rs`
- `crates/viewer_net/src/lib.rs` (targeted fetch tests)

## Boundary check
- `viewer_app`: orchestration/streaming cadence only.
- `viewer_net`: transport request shaping only.
- No `viewer_core` scene model or cross-crate ownership changes.

## Step sequence
1. Raise app texture stream work budget (inflight + per-tick poll budget).
2. Add Firestorm UA/cookie-state texture fetch path in `viewer_net` texture fetch helper.
3. Add/update focused tests for texture fetch request behavior.
4. Run fmt/check/targeted tests.
5. Run bounded live proof and compare texture coverage lines.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net fetch_texture_asset_bytes_uses_firestorm_style_candidate_first_with_accept_header -- --nocapture`
- `cargo test -p viewer_net fetch_texture_asset_bytes_reuses_set_cookie_between_attempts -- --nocapture`
- bounded live: `cargo run -p viewer_app` with proof and network log envs

## Risks and open questions
- Higher throughput can increase CPU/network load.
- Some 403 textures may remain genuinely inaccessible.

## Deferred-too-early candidates captured
- None for this bounded fix.

## Learnings pre-check
- L04 dirty-only apply is preserved.
- L06 crate-boundary discipline preserved (`viewer_app` + `viewer_net` only).
- No other applicable learnings block this scope.

## Completion criteria
- Build/tests pass for touched crates.
- Live bounded run shows improved texture ready coverage versus prior baseline and reduced persistent unresolved ratio.
