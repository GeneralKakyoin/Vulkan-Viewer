# Plan: R05 Patch 02 (Vertex Layout + Empty Texture Semantics)

## Scope
Fix issues discovered during `PLAN_R05` implementation review:
- Fix `wgpu` pipeline vertex layout mismatch for the main scene shader.
- Clarify empty `AssetID` semantics for texture lookup to avoid “loading forever” on empty IDs.
- Ensure required new module files are tracked (`draw_helpers.rs`).

No changes to crate boundaries, renderer architecture, or material system design.

## Current known state
- `crates/viewer_render/src/lib.rs`’s `SCENE_SHADER` declares `VsInput` locations:
  - `@location(0) position`
  - `@location(1) normal`
  - `@location(2) tex_coord`
- The render pipelines for world drawing (`scene_pipeline`, `alpha_test_pipeline`, `transparent_pipeline`) only declare a single vertex attribute at location 0, which is likely to fail `wgpu` validation at runtime.
- `crates/viewer_render/src/texture_provider.rs` returns `PendingTexture::Loading` for empty `AssetID`, which hides missing/invalid IDs and shows “loading” fallback indefinitely.
- `crates/viewer_render/src/draw_helpers.rs` exists but is untracked; `crates/viewer_render/src/lib.rs` has `mod draw_helpers;` and depends on it.

## Files touched
- `crates/viewer_render/src/lib.rs`
- `crates/viewer_render/src/texture_provider.rs`
- `crates/viewer_render/src/draw_helpers.rs` (tracking / no behavior change expected)
- `docs/reviews/REVIEW_impl_r05.md`
- `docs/reports/REPORT_r05_patch_02.md`

## Boundary check
- `viewer_render` owns GPU pipeline definitions and texture binding behavior.
- No changes to `viewer_core` contracts or `viewer_app` orchestration ownership.

## Step sequence
1. Update vertex buffer layouts for the three world pipelines to provide attributes at locations 0/1/2 matching `viewer_core::Vertex`.
2. Decide and implement empty-`AssetID` semantics:
   - `TextureProvider::get_texture(empty)` returns `Missing` (not `Loading`).
   - `RenderBackend` treats empty base-color IDs as “white” fallback (so “no texture” doesn’t become magenta/missing).
3. Ensure `draw_helpers.rs` and any required docs files are tracked.
4. Validation:
   - `cargo fmt --all -- --check`
   - `cargo check --workspace`
   - `cargo test -p viewer_render`
5. Update `docs/reviews/REVIEW_impl_r05.md` verdict/notes and write `docs/reports/REPORT_r05_patch_02.md`.

## Validation plan
- Always: `cargo fmt --all -- --check`, `cargo check --workspace`
- Targeted: `cargo test -p viewer_render`
- Optional runtime smoke: `cargo run -p viewer_app` (not required for this patch, but recommended to confirm `wgpu` pipeline creation).

## Risks / open questions
- Changing empty-ID handling may change the visual fallback of objects with “no texture” (intended to be neutral/white, not “loading”).
- No other behavior changes intended.

## Completion criteria
- World pipelines match the shader’s declared vertex inputs (0/1/2) and `viewer_core::Vertex` layout.
- Empty texture IDs no longer map to “loading forever”; base color uses a deterministic neutral fallback.
- Targeted validation passes.

