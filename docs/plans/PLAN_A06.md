# Plan: A06 Material/Texture Depth Expansion

## Summary
Activate a bounded, deterministic material+texture pipeline from `viewer_core` descriptors through fixture-backed acquisition in `viewer_app` into texture binding in `viewer_render`, supporting a bounded texture set (base color, normal, metallic/roughness, emissive) with deterministic `Loading`/`Missing` fallbacks. Shading level is **unlit textured** in this milestone.

## Objective
- Turn `viewer_core` material descriptors into an active cross-crate contract surface.
- Support **per-face** materials keyed by submesh `face_id`.
- Bind a bounded texture set per material:
  - base color (primary)
  - normal (optional; may be ignored by shader for now)
  - metallic/roughness (optional; may be ignored by shader for now)
  - emissive (optional; may be ignored by shader for now)
- Integrate the UV transform/animation matrix path as a data flow (deterministic; animation “off” by default).
- Ensure deterministic fallback behavior for `Loading` and `Missing`.

## Why now
`R05` provides reliable submission lanes and transparency behavior; `A06` can now increase material complexity without destabilizing ordering/correctness.

## In scope
- `viewer_core`
  - Expose the `material` module as a compiled, public contract.
  - Add a deterministic per-face material assignment container on instances.
- `viewer_app`
  - Collect texture requests from visible materials (deduped, stable order, bounded cap per tick).
  - Use fixture-backed acquisition when enabled; otherwise remain deterministic (`Loading`/`Missing`).
- `viewer_render`
  - Add texture binding + sampling for base color (unlit textured).
  - Provide deterministic fallback textures for `Loading` and `Missing`.
  - Bind per-face material textures based on submesh `face_id` and the instance’s material set.

## Out of scope
- Simple-lit / PBR shading models and tuning.
- Live capability-backed fetch / unbounded streaming.
- Avatar baking/system-layer composition.
- Broad shader-model redesign beyond what’s needed for bounded deterministic texture sampling.

## Current known state
- `viewer_render` has a `StreamingTextureProvider` and `upsert_texture_rgba8(...)` upload, but shader does not sample textures yet.
- `viewer_asset` has `FixtureTextureCache` with deterministic behavior and fixture PNGs in `test_assets/`.
- `viewer_core/src/material/*` exists but is not currently wired as `pub mod material;` in `viewer_core`.
- Renderer draws submeshes (has `face_id`) but does not select per-face materials.

## Files and components touched
- `crates/viewer_core/src/lib.rs`
  - Add `pub mod material;` and reexports as needed.
  - Add `MaterialSet` and attach to `RenderableInstance`.
- `crates/viewer_core/src/material/mod.rs`
  - Keep `MaterialDescriptor` stable; ensure it compiles as part of `viewer_core`.
- `crates/viewer_core/src/material/animation.rs`
  - Use existing UV matrix computation; ensure deterministic usage.
- `crates/viewer_app/src/main.rs`
  - Deterministic collection of required texture IDs from visible materials.
  - Fixture-driven request/poll/upload loop for those IDs.
- `crates/viewer_render/src/lib.rs`
  - Add texture bind group layout + sampler.
  - Choose texture views deterministically per slot (`Ready` vs `Loading` fallback vs `Missing` fallback).
  - Update shader and per-submesh binding behavior.
- Tests
  - `viewer_core`: `MaterialSet` lookup determinism and defaults.
  - `viewer_app`: “collect texture IDs from visible materials” (stable + bounded).
  - `viewer_render`: pure helper tests for view selection and material slot mapping.

## Boundary check
- `viewer_core` owns material semantics and deterministic helper logic.
- `viewer_asset` owns acquisition/cache policy.
- `viewer_app` wires acquisition to renderer upload; does not own render policy.
- `viewer_render` owns bind groups, shader binding, and GPU-side fallback behavior.

## Step sequence
1. **Activate `viewer_core::material`**
   - Add `pub mod material;` and reexport contract types required by app/render.
2. **Per-face material assignment**
   - Add `MaterialSet` in `viewer_core`:
     - `default: MaterialDescriptor`
     - `by_face: BTreeMap<u16, MaterialDescriptor>`
     - `material_for_face(face_id: u16) -> &MaterialDescriptor` (fallback to `default`)
   - Attach `materials: MaterialSet` to `RenderableInstance` with a stable default.
3. **Deterministic texture ID extraction (app-side)**
   - Define a single deterministic mapping from `MaterialDescriptor` → ordered list of `AssetID`s (base color, normal, metallic/roughness, emissive), skipping empty IDs.
   - Per-frame request policy (decision-complete):
     - consider visible instances only
     - dedupe via `BTreeSet` (stable)
     - cap requests per tick to a fixed number (64) in sorted order
4. **Fixture-backed acquisition + upload**
   - When `VIEWER_FIXTURE_TEXTURES` is enabled:
     - request IDs via `FixtureTextureCache`
     - `poll_png_rgba8` with a bounded `max_to_process`
     - upload `Ready` results using `renderer.upsert_texture_rgba8`
   - When disabled:
     - do not attempt live fetch; leave IDs in `Loading`/`Missing` deterministically
5. **Renderer binding + unlit textured shader**
   - Create fallback GPU textures at init:
     - `loading_fallback`: visible “loading” color
     - `missing_fallback`: visible “missing” color
   - Add a texture bind group with 4 texture slots + sampler:
     - base color, normal, metallic/roughness, emissive
   - Deterministic view selection:
     - `Ready(view)` → use view
     - `Loading` → `loading_fallback`
     - `Missing` or empty ID → `missing_fallback`
   - Shader policy (decision-complete for A06):
     - sample **base color** and multiply by tint (legacy tint uses `TextureEntry.rgba`; PBR uses `base_color_tint`)
     - output RGBA; alpha comes from sampled base color alpha * tint alpha
     - normal/metallic_roughness/emissive bindings are present and deterministic, but shader may ignore them (unlit).
   - UV transform:
     - apply deterministic UV matrix from `TextureEntry` (+ `TextureAnim`, default-off) to `tex_coord` before sampling.
6. **Per-face binding usage**
   - For each drawn submesh:
     - use `face_id` to select the `MaterialDescriptor` via `MaterialSet`
     - bind the material’s textures before drawing that submesh
7. **Tests + smoke**
   - Add targeted unit tests for deterministic mapping and selection.
   - Runtime smoke (offline):
     - `VIEWER_APP_LIVE_STARTUP=off`
     - `VIEWER_FIXTURE_TEXTURES=1`
     - confirm deterministic textured output and fallback behavior.

## Validation plan
- Always:
  - `cargo fmt`
  - `cargo check`
- Targeted:
  - `cargo test -p viewer_core`
  - `cargo test -p viewer_asset`
  - `cargo test -p viewer_app`
  - `cargo test -p viewer_render`
- Broader:
  - `cargo test` if wiring spans crates materially
- Runtime smoke:
  - `VIEWER_APP_LIVE_STARTUP=off`
  - `VIEWER_FIXTURE_TEXTURES=1`
  - `cargo run -p viewer_app`

## Risks and open questions
- Risk: per-face binding increases bind churn; acceptable for bounded milestone; later optimize by grouping by material keys.
- Risk: WGSL uniform alignment for UV matrices; implement with explicit padding rules.
- Open questions: none (shading is explicitly “unlit textured” for A06).

## Deferred-too-early candidates captured
- PBR/simple-lit shading and parity tuning (later milestones).
- Live capability-backed texture fetch/unbounded streaming (later asset/network milestones).

## Completion criteria
- `viewer_core` exposes an active material contract; instances support per-face material selection.
- `viewer_app` deterministically requests (deduped + capped) and uploads fixture textures when enabled.
- `viewer_render` binds/samples base color textures with deterministic loading/missing fallbacks and per-face selection.
- Required validation commands pass (or blockers are explicitly documented during execution).

