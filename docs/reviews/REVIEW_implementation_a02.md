# Review: Implementation A02 Asset Acquisition and Cache Foundation

## Verdict
Approved. The implementation correctly fulfills the A02 plan, establishing a typed asset contract and a robust fixture-backed cache foundation.

## Scope fit
The changes are strictly within the A02 scope:
- Defined `AssetStatus<T>` and `AssetID` core contracts.
- Implemented `FixtureTextureCache` with deterministic LRU and negative-cache behavior.
- Integrated `StreamingTextureProvider` with VRAM budget and eviction logic.
- Wired a runtime smoke path in `viewer_app` triggered by `VIEWER_FIXTURE_TEXTURES`.

## Architecture and boundary fit
- **Crate Boundaries:** Strong. `viewer_asset` owns the CPU-side acquisition and decoding policy, while `viewer_render` owns the GPU-side texture view management and VRAM budget.
- **Contract Isolation:** The use of `AssetStatus<T>` in the asset crate and `PendingTexture` in the render crate correctly decouples asset acquisition from rendering internals.
- **Orchestration:** `viewer_app` correctly wires these components at the top level without leaking implementation details.

## Correctness concerns
- **Deterministic LRU:** The `FixtureTextureCache` implementation includes explicit tests for LRU eviction and negative caching, ensuring stable behavior across restarts.
- **VRAM Management:** `StreamingTextureProvider` correctly tracks current usage and performs eviction before inserting new views, preventing out-of-memory errors on the GPU.
- **Wait-Free Polling:** The use of `poll_png_rgba8(max_to_process)` in the frame loop prevents long-tail I/O or decode latencies from blocking the UI thread.

## Modularity and maintainability concerns
- The use of `AssetStatus<T>` as a generic enum allows for easy expansion to other asset types (meshs, sounds, etc.) in future milestones.
- The `texture_fixture` module is well-isolated and can be easily swapped for a live capability-backed provider when `N04` is addressed.

## Validation adequacy
- **Unit Tests:** `cargo test -p viewer_asset` provides good coverage for the new cache logic.
- **Runtime Smoke:** The `VIEWER_FIXTURE_TEXTURES=1` scenario was successfully exercised to confirm the upload-to-renderer path.
- **Env Overrides:** `VIEWER_ASSET_CACHE_BUDGET_MB` and `VIEWER_RENDER_VRAM_BUDGET_MB` correctly allow for deployment-specific tuning.

## Risks and open questions
- **Optimization:** Current texture uploads use `upsert_texture_rgba8` which creates a new texture every time. While acceptable for this milestone, future work should consider texture reuse/pooling to reduce allocation overhead.

## Required revisions or approval status
Approval status: **Approved**.
Required revisions: None.
