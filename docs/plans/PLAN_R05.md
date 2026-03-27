# Plan: R05 Draw Submission Efficiency and Transparency Reliability

## Summary
Make draw submission deterministic under growing scene load by introducing explicit pass buckets (`opaque`, `alpha-tested`, `transparent`), stable ordering, and predictable capping—so transparent content renders consistently across repeated camera movement.

## Objective
- Split draw submission into `opaque`, `alpha-tested`, and `transparent` buckets.
- Ensure deterministic transparent ordering (back-to-front) with stable tie-breaks.
- Reduce submission churn via stable ordering keys (without large renderer redesign).
- Remove dependence on upstream `visibility_list` iteration order for correctness.

## Why now
After `U04` improves operator workflow, the next likely failure mode is visual correctness drift (transparency/order flicker) and frame instability as scene richness increases.

## In scope
- Deterministic draw-list building independent of `visibility_list` order.
- Pass bucketing policy:
  - `opaque`: depth write enabled, no blending.
  - `alpha-tested`: depth write enabled, alpha discard with a defined cutoff.
  - `transparent`: depth write disabled, alpha blending, back-to-front sort.
- Deterministic ordering keys:
  - `opaque` / `alpha-tested`: stable batching key + stable tie-break.
  - `transparent`: camera-distance sort + stable tie-break.
- Deterministic capping to renderer `max_objects` (cap after sorting).
- Alpha source for R05 validation: instance RGBA (not texture alpha).

## Out of scope
- Post-processing overhaul, environment/EEP rendering.
- GPU-driven rendering, occlusion systems, broad instancing redesign.
- Full material/texture sampling pipeline (belongs to `A06`).

## Current known state
- `viewer_render` currently draws visible instances in a single pass with blend `REPLACE` and depth write enabled.
- `viewer_render` uploads `ObjectUniform { model, color }` but fragment output alpha is effectively `1.0`.
- `viewer_core::RenderableInstance` color is RGB-only today, preventing transparency validation without a core contract update.
- There are no existing renderer tests; new tests must target extracted pure helpers.

## Files and components touched
- `crates/viewer_core/src/lib.rs`
  - Extend render-facing instance color to RGBA.
  - Add a render-facing alpha-mode contract used for pass bucketing.
- `crates/viewer_render/src/lib.rs`
  - Build stable draw lists, implement pass buckets, and enforce deterministic ordering/capping.
  - Add pipelines/config for alpha-tested and transparent passes.
- `crates/viewer_app/src/main.rs`
  - Minimal updates for the RGB→RGBA contract change and any diagnostic instances used for validation.
- `crates/viewer_render` tests
  - Add pure unit tests for bucketing/sorting/capping helpers (no `wgpu` device required).

## Boundary check
- `viewer_render` owns pass building, ordering, and GPU submission.
- `viewer_core` owns shared typed contracts for render-facing metadata.
- `viewer_app` remains orchestration-only (no render-policy ownership).

## Step sequence
1. **Core contract: RGBA + alpha mode**
   - Change instance color from `[f32; 3]` to `[f32; 4]` (RGBA) in `viewer_core`.
   - Add `viewer_core::AlphaMode`:
     - `Opaque`
     - `AlphaTest { cutoff: f32 }`
     - `Blend`
   - Define bucket mapping:
     - `Opaque` → `opaque`
     - `AlphaTest {..}` → `alpha-tested`
     - `Blend` → `transparent`
2. **Renderer: stable draw item model**
   - Define internal `DrawItem` fields (decision-complete):
     - `instance_id`
     - `bucket`
     - `geometry_key` (stable hash derived from `GeometrySource`)
     - `distance_sq` (for transparent: camera position to `instance.world_aabb.center`)
3. **Renderer: deterministic ordering**
   - Sorting rules:
     - `opaque` / `alpha-tested`: `(distance_sq asc, geometry_key asc, instance_id asc)`
       - *Note: `distance_sq asc` (front-to-back) is used to maximize early-Z rejection on the GPU.*
     - `transparent`: `(distance_sq desc, instance_id asc)`
   - Apply `max_objects` cap *after* sorting, per bucket order.
4. **Renderer: pass buckets and pipelines**
   - Implement a single render pass with pipeline switches between buckets.
   - Pipeline policy:
     - `opaque`: blend replace, depth write true.
     - `alpha-tested`: blend replace, depth write true, fragment discard if `alpha < cutoff`.
     - `transparent`: alpha blend, depth write false, depth compare `LessEqual`.
   - Ensure `ObjectUniform` carries RGBA (alpha no longer forced to `1.0`).
5. **Tests**
   - Pure unit tests for:
     - bucket selection from alpha mode
     - transparent ordering stability (tie-break by `instance_id`)
     - deterministic cap behavior (cap after sort)
6. **Runtime validation hook**
   - Ensure at least one deterministic transparent diagnostic instance exists in bounded/offline mode (e.g., a cube with `alpha=0.5`, `AlphaMode::Blend`) so `STRESS_TEST=camera` can reveal ordering stability.

## Validation plan
- Always:
  - `cargo fmt`
  - `cargo check`
- Targeted:
  - `cargo test -p viewer_core`
  - `cargo test -p viewer_render`
  - `cargo test -p viewer_app`
- Broader:
  - `cargo test` (required if wiring changes are cross-crate and non-trivial)
- Runtime smoke:
  - `VIEWER_APP_LIVE_STARTUP=off`
  - `STRESS_TEST=camera` and/or `STRESS_TEST=screenshot` to confirm no transparent-order flicker.

## Risks and open questions
- Risk: upstream `visibility_list` order may be unstable; plan explicitly removes dependence on it.
- Risk: alpha-tested cutoff contract must be explicit; plan defines it in `AlphaMode`.
- Open questions: none.

## Deferred-too-early candidates captured
- Occlusion/indirect draws/instancing systems (later `R` perf milestones).
- Broad lighting/material redesign (later milestones after `A06` plumbing proves stable).

## Completion criteria
- Renderer submits `opaque` → `alpha-tested` → `transparent` with deterministic ordering and deterministic capping.
- Transparent ordering is stable across repeated camera sweeps.
- Ordering/capping correctness no longer depends on upstream iteration order.
- Required validation commands pass (or are explicitly documented if blocked).

