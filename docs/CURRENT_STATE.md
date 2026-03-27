# Current State: Vulkan-Viewer

## Overview
The Vulkan-Viewer is a high-performance Second Life compatible viewer built in Rust. It currently supports basic region and avatar presence, nearby chat, direct IM, avatar profiles, and a robust diagnostics shell.

## Latest Notable Changes (R08)
- **Avatar Appearance Contracts**: Added `AvatarAppearanceSummary` and bounded `AvatarAttachmentProxy` scene-facing contracts in `viewer_core`.
- **Attachment Lifecycle**: Added `WorldAvatarAttachmentProxy` scene roles plus seam-owned attachment proxy lifecycle management in `Scene::apply_world_object_ingestion_seam(...)`.
- **Deterministic Attachment Projection**: `viewer_app` now projects bounded attachment proxies from existing avatar samples and feeds them into the seam path.
- **Attachment Diagnostics**: `viewer_ui` now shows avatar and attachment proxy counts in the diagnostics panel.
- **R08 Coverage**: Added deterministic attachment projection and seam create/remove tests in `viewer_core`; validated app, UI, renderer, and workspace test passes plus offline screenshot smoke.

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
**U09: Workflow depth and product usability expansion** (Next Planned)
Focus: build on the bounded R08 avatar/attachment foundation with clearer workflow and diagnostics depth.

## System Components
- `viewer_app`: Orchestration and worker state mapping (UI-agnostic).
- `viewer_core`: Shared domain types, session status contract, and deterministic logic.
- `viewer_ui`: Egui-based presentation layer (decoupled from app internals).
- `viewer_render`: Wgpu-driven rendering backend.
- `viewer_net`/`viewer_grid`: Protocol and asset transport layers.

- **Verification Status**: `cargo fmt --all`, `cargo check --workspace`, targeted crate tests, `cargo test --workspace`, and offline screenshot smoke all pass on the current R08 baseline.
