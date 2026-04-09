# Plan: Wave 8 Viewer Asset Helper Extraction

## Objective
Apply the repository's cross-crate modularization pattern (Wave 8) to `viewer_asset`. Reduce `crates/viewer_asset/src/lib.rs` size by extracting cohesive, bounded helper clusters into dedicated crate-local utility modules.

## Scope
### In Scope
- Extracting the standalone texture decoding utility cluster into a new `texture_decode_utils` module.
- Extracting the standalone mesh utility cluster into a new `mesh_decode_utils` module.
- Retaining existing external API bounds by re-exporting the moved functions in `lib.rs`.

### Out of Scope
- Altering any procedural geometry, texture decoding logic, or asset fetch behavior.
- Altering the implementation of `GeometryCache` or `AssetFetchRequest` types.
- Moving functions between crates. The extraction strictly targets `viewer_asset` internal structure.
- Modifying `sl_mesh_loader.rs` or `mesh_loader.rs`.

## Current known state
- `viewer_asset/src/lib.rs` is currently ~456 lines and contains a mix of core structs/traits (`AssetStatus`, `AssetFetchOutcome`, `GeometryCache`) and raw byte-decoding helper functions (`decode_texture_rgba8`, `load_mesh_bytes`).
- The repository has a mandate (from Wave 1-7) to extract utility functions from the `lib.rs`/`main.rs` monoliths into behavior-focused bounded submodules.

## Files and components touched
### `viewer_asset`
- **[MODIFY]** `crates/viewer_asset/src/lib.rs`
- **[NEW]** `crates/viewer_asset/src/texture_decode_utils.rs`
- **[NEW]** `crates/viewer_asset/src/mesh_decode_utils.rs`

## Boundary check
- The changes are strictly localized within `viewer_asset`.
- No new dependencies will be introduced.
- Existing consumer calls from `viewer_app` or `viewer_render` will implicitly remain intact as the target helper functions (`decode_texture_rgba8`, `decode_png_rgba8`, etc.) will still be exported by `viewer_asset`.

## Step sequence
1. **Create `texture_decode_utils.rs`:**
   - Move `TextureDecodeError`, `decode_texture_rgba8`, `sample_jp2_component_u8`, and `decode_png_rgba8` from `viewer_asset/src/lib.rs` to this file.
   - Move related tests (`decode_texture_rgba8_decodes_png`, `decode_texture_rgba8_decodes_jpeg2000`, `decode_texture_rgba8_rejects_unsupported_bytes`) to this file.
   - Adjust `use` imports to ensure dependencies (e.g., `DecodedRgbaImage`) are available.
2. **Create `mesh_decode_utils.rs`:**
   - Move `get_lod_level`, `hash_bytes`, and `load_mesh_bytes` from `viewer_asset/src/lib.rs`.
   - Export them publicly (or crate-visibly) as required by `lib.rs`.
   - Update imports for `ProcessedMesh`, `MeshSourceFormat`, and mesh loader delegates.
3. **Wire in `lib.rs`:**
   - Add `pub mod texture_decode_utils;` and `pub mod mesh_decode_utils;`.
   - Re-export the required helpers via `pub use texture_decode_utils::*;` and `use mesh_decode_utils::*;`.
   - Remove the extracted code and tests from `lib.rs`.
4. **Validation pass:**
   - Run formatting, check, and tests.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_asset`
- `cargo test -p viewer_asset` (to ensure the removed/moved unit tests execute successfully in the new modules).
- `cargo check` across the workspace to ensure `viewer_app` and `viewer_render` still compile and their imports haven't broken.

## Risks and open questions
- **Risk:** Missing imports in the extracted files.
  - *Mitigation:* `cargo check` will immediately pinpoint unresolved symbols. We will import precisely what is necessary.
- **Risk:** Type trait leak.
  - *Mitigation:* Ensure `TextureDecodeError` implements `thiserror::Error` explicitly using `std::fmt` and `std::error` if needed inside the new module. 

## Deferred-too-early candidates captured
- Extracting `GeometryCache` into its own module (`geometry_cache.rs`) will be deferred. The current wave specifically limits extraction to low-level parsing/helper utilities. `GeometryCache` acts as a primary entry point type and moving it now is out of bounds for the "Utility helper extraction" strategy.

## Learnings pre-check
- **L06 (Boundary drift)**: We are strictly enforcing localized separation of logic (internal structure of `viewer_asset` only).
- **L14 (Explicit type annotations for gltf)**: We will not alter `gltf` implementation, only transferring the helper function signature (`load_mesh_bytes`) to a smaller module.
- There are no learnings opposing a structural modularization within the same crate.

## Completion criteria
- Extracting texture and mesh byte decoding utilities into `crates/viewer_asset/src/texture_decode_utils.rs` and `crates/viewer_asset/src/mesh_decode_utils.rs`.
- `crates/viewer_asset/src/lib.rs` uses `.rs` files as explicit decoupled modules.
- `cargo check` and `cargo test` for `viewer_asset` pass.
- No warnings about unused code.
