# Implementation Review: R05 (Rendering Contract & Transparency)

## Verdict
**Approved (with notes)**.

The milestone’s core draw-list work is present (RGBA contract, `AlphaMode`, deterministic pass bucketing/sorting, and unit tests for pure helpers). The remaining items are primarily plan-alignment notes and a couple of practical repo/behavior fixes that should be tracked as patches.

## Architecture and boundary fit
- **Good boundary discipline:** shared contracts (`AlphaMode`, RGBA instance color) live in `viewer_core`; renderer bucketing/sorting/pipeline policy lives in `viewer_render`; `viewer_app` changes are limited to contract updates and a transparent diagnostic instance.

## Plan alignment (matches)
- **RGBA contract:** `RenderableInstance.color` is RGBA (`[f32; 4]`) and the shader outputs that alpha.
- **Alpha modes:** `Opaque`, `AlphaTest { cutoff }`, `Blend` exist and drive bucketing/pipeline selection.
- **Pass bucketing:** draw submission is bucketed `opaque` → `alpha-tested` → `transparent`.
- **Determinism:** draw-list building is independent of `visibility_list` iteration order (explicit sort with stable tie-breaks).
- **Deterministic capping:** cap is applied after sorting (`max_objects`).
- **Alpha-test behavior:** fragment discard exists in WGSL based on cutoff.
- **Pure helper tests:** `viewer_render::draw_helpers` has unit tests without a `wgpu` device.

## Correctness / hygiene items (patched)
- **Vertex layout:** `crates/viewer_render/src/lib.rs` pipelines now declare vertex attributes matching `viewer_core::Vertex` (`position`, `normal`, `tex_coord`) and `SCENE_SHADER`’s `VsInput` locations (0/1/2).
- **Untracked module:** `crates/viewer_render/src/draw_helpers.rs` must be tracked since `crates/viewer_render/src/lib.rs` depends on it.
- **Empty texture ID semantics:** `crates/viewer_render/src/texture_provider.rs` should treat empty `AssetID` as `Missing` (not `Loading`). The renderer already treats empty base-color IDs as a deterministic white fallback via `white_view`.

## Correctness notes
- Sorting and tie-breaks are deterministic and stable as implemented; the main concern is plan divergence, not non-determinism.
- Geometry-key hashing is deterministic but very approximate in `draw_helpers` (e.g., byte-sum for UUID strings). This is likely OK for ordering/tie-break purposes but is not a robust batching key if batching becomes performance-critical later.

## Plan alignment mismatches (reconcile explicitly)
- **R05 alpha source:** `PLAN_R05` states R05 validation should use instance RGBA (not texture alpha). Current shader path samples a base texture and multiplies `base_color * instance_color`, so texture alpha participates in output alpha (and alpha-test discard).
- **`max_objects` meaning drift:** `PLAN_R05` treats `max_objects` as a cap on instances after sorting; the current renderer uploads uniforms per submesh (A06-style). `build_draw_list(...)` caps by instance count, but the uniform cap is applied by uniform slots. This is deterministic, but the effective cap is no longer “N instances”.

## Validation (run for this review)
- `cargo fmt --all -- --check`: PASS
- `cargo check --workspace`: PASS
- `cargo test -p viewer_core`: PASS
- `cargo test -p viewer_render`: PASS

## Not validated here
- Runtime smoke (`cargo run -p viewer_app`) to confirm `wgpu` pipeline creation succeeds and no transparency flicker under camera sweeps/stress tests.

## Notes / pointers
- Draw-list build/sort/cap logic: `crates/viewer_render/src/draw_helpers.rs`
- Pipeline selection and alpha-mode uniform packing: `crates/viewer_render/src/lib.rs`
- RGBA + `AlphaMode` contracts: `crates/viewer_core/src/lib.rs`
- Transparent validation cube: `crates/viewer_app/src/main.rs`
