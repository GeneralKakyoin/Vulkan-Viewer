# Implementation Review: A06 (Material/Texture Depth Expansion)

## Verdict
**Mostly implemented; needs follow-up fixes for tint + strict capping + validation hygiene.**

The core cross-crate wiring exists (material contract in `viewer_core`, per-face selection, per-submesh texture binding, UV matrix path, deterministic GPU fallbacks, and base-color sampling in WGSL). The earlier determinism + fixture gating gaps in `viewer_app` appear addressed in the current working tree. Remaining deviations are primarily around shader tint semantics, strict request capping, and a few validation/test gaps.

## Architecture and boundary fit
- **Good boundary discipline overall:** `viewer_core` owns the material + animation contracts; `viewer_app` orchestrates fixture cache polling and upload; `viewer_render` owns bind groups, sampling, and fallback textures.
- No obvious crate boundary violations introduced by A06.

## Plan alignment (matches)
- `viewer_core::material` is active and public; `MaterialDescriptor` is used cross-crate.
- `MaterialSet` exists and supports `material_for_face(face_id)` fallback to default.
- Renderer binds a bounded texture set via a dedicated bind group and samples base color (unlit textured).
- UV matrix path is wired through object uniforms and applied in the vertex shader before sampling.
- Per-face selection is used at draw time via submesh `face_id`.
- App-side visible texture ID extraction is deterministic (`BTreeSet`) and has a unit test.
- Fixture acquisition is gated by `VIEWER_FIXTURE_TEXTURES` via `fixture_texture_ids_from_env()` (empty disables acquisition).

## Plan deviations / correctness concerns
### 1) Shader tint semantics (A06 decision-complete, but not implemented)
`PLAN_A06` calls for sampling base color then multiplying by a tint:
- legacy tint: `TextureEntry.rgba`
- PBR tint: `PbrDescriptor.base_color_tint`

Current shader uses `object.color` only (wired from `RenderableInstance.color`), and object uniforms are not updated per-face with `TextureEntry.rgba` / `base_color_tint`. This means per-face material tints are currently ignored.

### 2) Strict request capping (app-side)
`extract_visible_texture_ids_from_scene(...)` stops after finishing an instance when `ids.len() >= cap`, but can still exceed `cap` if a single visible instance contributes more than `cap` unique texture IDs (e.g., many per-face overrides).

The plan’s “cap requests per tick to 64” intent is not strictly enforced by this logic.

### 3) Fallback policy nuance (renderer)
Renderer uses a `white_view` fallback for empty texture IDs (in addition to loading yellow + missing magenta). This differs from the plan’s “empty → missing fallback” wording; acceptable if intentional, but should be explicitly recorded as a contract.

### 4) Test coverage gaps (A06-specific)
Good: `viewer_core` has UV matrix tests; `viewer_app` has deterministic+cap extraction test.

Still missing (per plan intent): a small pure test in `viewer_render` that exercises deterministic view selection (`Ready` vs `Loading` vs `Missing` vs empty) for each slot, ideally via a factored helper rather than GPU bind group creation.

## Validation (run during this review)
- `cargo fmt --all -- --check`: **FAIL** (import formatting in `crates/viewer_app/src/main.rs`)
- `cargo check --workspace`: **PASS** (warnings: dead code in `viewer_app`)
- `cargo test -p viewer_core`: **PASS**
- `cargo test -p viewer_render`: **PASS** (warning: unused import in test module)
- `cargo test --workspace`: **FAIL** (unrelated `viewer_app` `social_cache` tests fail on Windows due to `/tmp/...` path)

## Not validated here
- Runtime smoke: `VIEWER_APP_LIVE_STARTUP=off`, `VIEWER_FIXTURE_TEXTURES=1`, `cargo run -p viewer_app`
- Visual confirmation that textured output + loading/missing fallbacks behave as intended.

## Recommended follow-up (small, bounded)
1. Implement plan-defined tint semantics by incorporating `TextureEntry.rgba` / `PbrDescriptor.base_color_tint` into the per-submesh `object.color` value (or extend uniforms if needed).
2. Enforce strict cap semantics in `extract_visible_texture_ids_from_scene(...)` (never return more than `cap`).
3. Add a small pure unit test in `viewer_render` for slot view selection (`Ready`/`Loading`/`Missing`/empty) to lock fallback behavior.
4. Run `cargo fmt` to make `fmt --check` clean.
5. Fix (or gate) `viewer_app` `social_cache` tests to be platform-correct so `cargo test --workspace` can be used as a real validation signal on Windows.
