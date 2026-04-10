# Plan: Render Truncated Feed Retention and Texture Bind Truthfulness (2026-04-10)

## Objective
Eliminate live proxy clumping from truncated object-feed accumulation and make texture coverage diagnostics reflect actual renderer-bound texture state.

## Scope
- Update `viewer_core` object-feed retention behavior so truncated exports do not accumulate stale proxies unboundedly.
- Update `viewer_app` texture coverage telemetry to count renderer-bound textures as ready before consulting transient live-result maps.
- Preserve existing crate boundaries and keep changes local to behavior owners.

## Current known state
- Live reports show hundreds of proxies clustering due to retained stale objects when export is truncated.
- Texture fetch/decode logs show many successful decodes while coverage logs still report unresolved due consuming/removing transient poll results.

## Files and components touched
- `crates/viewer_core/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- continuity artifacts in `docs/reports/`, `docs/CURRENT_STATE.md`, and `docs/HANDOFF.md`

## Boundary check
- `viewer_core`: scene lifecycle/retention policy only.
- `viewer_app`: diagnostics truthfulness only.
- No changes to transport semantics, renderer internals, or cross-crate ownership.

## Step sequence
1. Change object-feed retention path in `Scene::apply_world_object_ingestion_seam(...)` to retain against present export IDs regardless of truncation flag.
2. Update texture coverage relay in `maybe_emit_texture_coverage_summary(...)` to classify `renderer.has_texture(id)` as `ready` first.
3. Add/update targeted tests covering truncated retention behavior and coverage semantics as needed.
4. Run fmt/check/targeted tests.
5. Run bounded live proof with network/coverage logs to verify reduced clumping and truthful texture-ready counts.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_core -p viewer_app`
- targeted tests in touched crates (object-feed retention + texture coverage extraction paths)
- bounded live run of `viewer_app` with render-proof and network-debug logs

## Risks and open questions
- Always-retain-on-export can cause object popping under heavy truncation; preferred over unbounded stale accumulation for current correctness slice.
- Some textures may still fail capability fetch (403); this plan improves truthfulness and scene stability, not capability authorization.

## Deferred-too-early candidates captured
- Adaptive export window sizing and long-lived object-feed LRU synchronization policy deferred.

## Learnings pre-check
- L72: keep live verification bounded and explicit.
- L76: object-feed placement should prefer decoded data and avoid synthetic drift paths.
- L82: keep edits local to owning crate behavior.

## Completion criteria
- Truncated object-feed no longer grows proxy count unboundedly from stale retained IDs.
- Texture coverage logs report non-zero `ready` when decoded textures are actually renderer-bound.
- Validation + bounded live artifacts captured.
