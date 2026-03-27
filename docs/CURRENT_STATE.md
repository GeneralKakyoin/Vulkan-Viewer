# Current State: Vulkan-Viewer

## Overview
The Vulkan-Viewer is a high-performance Second Life compatible viewer built in Rust. It currently supports basic region and avatar presence, nearby chat, direct IM, avatar profiles, and a robust diagnostics shell.

## Latest Notable Changes (A13 Completed)
- **Typed Live Fetch Contracts**: Added `AssetFetchRequest` and wired `LiveTextureProvider` to return `AssetFetchOutcome<DecodedRgbaImage>` so live status/failure semantics stay explicit through the cache boundary.
- **Source Mode Controls**: Added `VIEWER_ASSET_SOURCE_MODE=fixture|auto|live` handling in `viewer_app` startup wiring to control live-provider attachment without crossing crate boundaries.
- **Live Timeout Control**: Added `VIEWER_ASSET_LIVE_TIMEOUT_MS` (bounded) and worker wiring so texture fetch timeout policy is configurable and deterministic.
- **Failure Classification Path**: Added typed failure propagation from worker (`TextureAssetFailed`) through app/provider/cache to metrics (`transport`, `decode`, `timeout`, `other`) and bounded fallback accounting.
- **A13 Orchestration Path**: Renamed app texture tick lane to `tick_scene_textures(...)` and retained continuity-priority-driven request flow.
- **A13 Validation**: Confirmed with `cargo fmt --all`, targeted A13 crate tests, `cargo check --workspace`, `cargo test --workspace`, and offline screenshot smoke (`VIEWER_APP_LIVE_STARTUP=off VIEWER_FIXTURE_TEXTURES=1 STRESS_TEST=screenshot ... cargo run -p viewer_app`).

## Latest Notable Changes (R12 Gap Fix)
- **Environment Contract Hardening**: Extended `viewer_core::EnvironmentState` with additive, serde-defaulted controls (`time_of_day_normalized`, `sky_enabled`, `fog_enabled`) and added `EnvironmentState::sanitized()` clamping.
- **Fog Correctness Fix**: Updated `viewer_render` fog depth source to use camera-to-fragment world-space distance instead of `clip_position.w`.
- **Fog Density Activation**: Wired `fog.density` into the shader fog factor so the field is no longer inert.
- **Sky Baseline Improvement**: Added bounded sky-top/sky-bottom influence to both clear-color derivation and fragment tinting so both sky colors are used.
- **App Mapping Path**: Added `derive_environment_from_snapshot(...)` in `viewer_app` and per-frame environment refresh from live snapshot continuity state.
- **Diagnostics Expansion**: `viewer_ui` environment panel now shows time-of-day and sky/fog enabled flags plus sky top/bottom and fog values.
- **R12 Gap-Fix Validation**: Verified with `cargo fmt --all`, `cargo check -p viewer_core -p viewer_render -p viewer_app -p viewer_ui`, `cargo test -p viewer_core -p viewer_render -p viewer_app -p viewer_ui`, and screenshot smoke (`VIEWER_APP_LIVE_STARTUP=off STRESS_TEST=screenshot ... cargo run -p viewer_app`).

## Latest Notable Changes (R12)
- **Environment Rendering Baseline**: Introduced `EnvironmentState` (ambient, sky, fog) to `viewer_core` and `viewer_render` to improve scene readability and visual continuity.
- **GPU Environment Uniforms**: Added `EnvironmentUniform` (Group 3, Binding 0) to `SCENE_SHADER` to drive ambient modulation and linear fog parameters on the GPU.
- **Deterministic Atmospheric clear**: Updated `RenderBackend` to clear the color attachment using the `sky_bottom_color`, ensuring a stable horizon background.
- **Linear Fog Integration**: Implemented distance-based linear fog in the fragment shader, using `clip_position.w` as a depth proxy for R12.
- **Environment Diagnostics**: Integrated environment parameter visualization (ambient, sky, fog) into the `viewer_ui` diagnostics panel.
- **R12 Validation**: Verified with `cargo fmt`, `cargo check --workspace`, and full test passes for `viewer_core` and `viewer_render`.

## Latest Notable Changes (Clippy Workspace Cleanup)
- **Workspace Clippy Cleanup**: Resolved warning classes across `viewer_ui`, `viewer_app`, `viewer_core`, `viewer_asset`, `viewer_render`, `viewer_grid`, and `viewer_net` so `cargo clippy --workspace --all-targets -- -D warnings` now passes.
- **UI Render API Hardening**: Replaced `UiSystem::render`’s long argument list with `RenderInput` to remove argument-count lint pressure and reduce callsite fragility.
- **Deterministic Idiomatic Pass**: Applied lint-safe refactors (`collapsible_if`, `manual_is_multiple_of`, `field_reassign_with_default`, `new_without_default`, `redundant_closure`, `manual_ignore_case_cmp`, etc.) without changing behavior.
- **Validation**: Confirmed `cargo fmt --all`, `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` pass. `cargo run -p viewer_app` was started successfully and timed out after launch during smoke attempt.

## Latest Notable Changes (Deferred Promotions)
- **Screenshot Diff Harness**: Added `viewer_app` binary `screenshot_diff` for baseline-vs-candidate PNG comparison with configurable mean-absolute-error threshold.
- **Camera Waypoint Scripts**: Added `VIEWER_TEST_CAMERA_PATH_FILE` JSON waypoint support for deterministic scripted camera paths in `STRESS_TEST=camera|screenshot` modes.
- **Deferred Sync**: Marked the corresponding deferred entries as `promoted` in `docs/plans/DEFERRED_FEATURES.md`.
- **Verification**: Confirmed `cargo fmt --all`, `cargo check --workspace`, and `cargo test -p viewer_app` pass with new script + harness paths.

## Latest Notable Changes (A10)
- **Continuity-Aware Asset Streaming**: Implemented `AssetPriority` (Active, Previous, Neighbor, Normal) and priority-based cache eviction in `viewer_asset`.
- **Deterministic Cache Discipline**: Refactored `FixtureTextureCache` to use metadata-based priority queuing with deterministic tie-breaking (`last_touched_tick`, `AssetID`).
- **Cache Metrics & UI**: Added `CacheMetrics` tracking (budget, pressure, counts) and integrated it into the `viewer_ui` Diagnostics panel.
- **Visibility-Driven Priority**: `viewer_app` now automatically promotes visible scene textures to `Active` priority during the streaming tick.
- **A10 Validation**: Verified with workspace-wide test pass, targeted cache eviction unit tests, and compilation of the full Vulkan-Viewer suite.

## Latest Notable Changes (R08)
- **Avatar Appearance Contracts**: Added `AvatarAppearanceSummary` and bounded `AvatarAttachmentProxy` scene-facing contracts in `viewer_core`.
- **Attachment Lifecycle**: Added `WorldAvatarAttachmentProxy` scene roles plus seam-owned attachment proxy lifecycle management in `Scene::apply_world_object_ingestion_seam(...)`.
- **Deterministic Attachment Projection**: `viewer_app` now projects bounded attachment proxies from existing avatar samples and feeds them into the seam path.
- **Attachment Diagnostics**: `viewer_ui` now shows avatar and attachment proxy counts in the diagnostics panel.
- **R08 Coverage**: Added deterministic attachment projection and seam create/remove tests in `viewer_core`; validated app, UI, renderer, and workspace test passes plus offline screenshot smoke.

## Latest Notable Changes (U09)
- **UI Shortcuts (F1-F3)**: Implemented F1 (toggle diagnostics), F2 (focus diagnostics/continuity), and F3 (toggle social) toggles in `viewer_app` event routing.
- **Diagnostics Grouping**: Reorganized the diagnostics panel into "Performance & Metrics" and "Presence & Continuity" collapsing headers in `viewer_ui`.
- **Profile Refresh Cooldown**: Added a 10-second refresh cooldown policy in `viewer_ui` with a countdown timer, backed by `last_refresh_unix_ms` in `viewer_core::AvatarProfileState`.
- **U09 Validation**: Verified with workspace tests, `cargo check`, and borrow-checker fixes for egui window state updates.

## Latest Notable Changes (N07)
- **Bounded Region Continuity Model**: Added typed continuity state (`None`, `Crossed`, `Confirming`, `Completed`) in `viewer_net` and propagated it through `viewer_app` into `LiveVisualSnapshot`.
- **Continuity Snapshot Contract**: Extended `viewer_core::LiveVisualSnapshot` with `RegionContinuitySummary` (active/previous region coords + bounded neighbors, serde-defaulted).
- **Continuity Seam Lane**: Added `WorldObjectIngestionLane::ContinuityPayload` and seam->scene lifecycle wiring so continuity visualization remains seam-owned.
- **Continuity Diagnostics UI**: Added read-only continuity lines in `viewer_ui` diagnostics (`phase`, active/previous region, neighbor count).
- **N07 Test Coverage**: Added targeted tests for continuity mapping, continuity seam payload emission, and seam-owned continuity role lifecycle removal.

## Latest Notable Changes (R05)
- **R05 Review Fixes**: Resolved vertex layout mismatch in `viewer_render` pipelines and reconciled alpha/capping logic.
- **RGBA Rendering Contract**: Standardized all color handling to RGBA `[f32; 4]` across `viewer_core` and `viewer_render`.
- **AlphaMode Support**: Integrated `Opaque`, `AlphaTest`, and `Blend` modes into the `Scene` and `RenderableInstance` types.
- **Pass Bucketing & Sorting**: Implemented a pass-based rendering system with deterministic front-to-back sorting for opaque/alpha-tested items and back-to-front sorting for transparent items.
- **Stable Draw Item Model**: Created `DrawItem` and `draw_helpers.rs` to ensure consistent frame-to-frame draw submission.
- **Fragment Alpha Discard**: Added alpha-testing logic directly to the GPU shader for performance.

## Latest Notable Changes (A06)
- **Texture & Material Integration**: Fully integrated `MaterialSet` and `TextureAnim` into `RenderableInstance` for per-instance and per-face overrides.
- **Wired UV Matrix Pipeline**: Connected `RenderableInstance::texture_anim` to the `RenderBackend` uniform upload, enabling scrolling and flipbook animations on the GPU.
- **Ergonomic Instance API**: Added `with_texture_anim` builder and automated default initialization to `RenderableInstance`.
- **Verified Animation Logic**: Added comprehensive unit tests for UV matrix math and a visual verification case in the Geometry Torture stress test.
- **Deterministic Texture Fallbacks**: Robust handling of loading (yellow) and missing (magenta) states integrated into the descriptor binding.

## Latest Notable Changes (U04)
- **Unified Session Status**: Standardized UX-facing session states (`disabled`, `starting`, `connected`, `reconnecting`, `failed`) implemented across the codebase.
- **Improved UI Shell**: Reorganized into Session, Social, and Diagnostics panels.
- **Diagnostics Relay Filters**: Added category and level filtering to the integrated Runtime Relay.
- **Enhanced Profile Headers**: Implemented cache freshness labels (`Fresh`/`Stale`) and human-readable age readout.
- **Crate Modernization**: Resolved all `wgpu` deprecation warnings in `viewer_render`.

## Active Milestone
**Finalizing N11 / Resuming U09 Path**
Focus: Wrap N11 continuity-aware handoff hardening and resume the workflow/usability depth of U09.

## System Components
- `viewer_app`: Orchestration and worker state mapping (UI-agnostic).
- `viewer_core`: Shared domain types, session status contract, and deterministic logic.
- `viewer_ui`: Egui-based presentation layer (decoupled from app internals).
- `viewer_render`: Wgpu-driven rendering backend.
- `viewer_net`/`viewer_grid`: Protocol and asset transport layers.

- **Verification Status**: `cargo fmt --all`, `cargo check --workspace`, targeted crate tests, `cargo test --workspace`, and offline screenshot smoke all pass on the current R08 baseline.
