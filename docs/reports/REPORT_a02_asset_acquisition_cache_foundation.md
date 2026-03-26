# REPORT: A02 Asset Acquisition and Cache Foundation (Fixture-Backed)

## Summary

Executed milestone **A02** by adding a deterministic, local+stubbed asset acquisition path for textures plus bounded cache policy, and wiring a minimal renderer-consumer upload path for runtime smoke.

This milestone explicitly does **not** add live capability-backed fetching.

## Implemented work

- Defined a generic pending-state contract in `viewer_asset`:
  - `AssetStatus<T>` = `Loading | Ready(T) | Missing`
- Implemented a fixture-backed texture source + bounded CPU cache:
  - `FixtureTextureCache` loads PNG fixtures from `test_assets/*`
  - deterministic pending behavior via `request_png_rgba8(...)` + `poll_png_rgba8(...)`
  - bounded LRU eviction (default **512 MB**) with env override `VIEWER_ASSET_CACHE_BUDGET_MB`
  - bounded negative-cache for missing/invalid fixture lookups
- Wired a minimal runtime smoke path:
  - `viewer_app` can optionally request fixture textures and upload them to the renderer
  - enabled via `VIEWER_FIXTURE_TEXTURES` (truthy = default IDs; otherwise comma list)

## Files changed

- `crates/viewer_asset/src/lib.rs`
- `crates/viewer_asset/src/texture_fixture.rs`
- `crates/viewer_asset/Cargo.toml`
- `crates/viewer_app/src/main.rs`
- `crates/viewer_core/src/lib.rs`
- `crates/viewer_render/src/lib.rs`
- `crates/viewer_render/src/texture_provider.rs`

## Validation run

- `cargo fmt` (pass)
- `cargo check` (pass)
- `cargo test -p viewer_asset` (pass)
- `cargo test` (pass)
- Runtime smoke (time-bounded):
  - `VIEWER_APP_LIVE_STARTUP=off`
  - `VIEWER_FIXTURE_TEXTURES=1`
  - launched `target/debug/viewer_app.exe` and stopped after ~6 seconds

## Result status

**Complete** for A02 scope.

## Follow-ups / risks

- Texture sampling/material binding is intentionally not implemented yet; this milestone only establishes acquisition/cache + upload seams.
- Renderer-side texture budget uses `VIEWER_RENDER_VRAM_BUDGET_MB` and currently emits `wgpu` deprecation warnings for copy structs; this is non-blocking for A02.

