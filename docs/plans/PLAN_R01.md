# Plan: R01 Post-M5 Rendering Integration Hardening

## Objective
Deliver a boundary-safe, deterministic, and repeatable rendering integration slice that hardens the post-M5 geometry path from scene data to GPU submission, with explicit fallback behavior and no expansion into full material/performance milestones.

## Scope
In scope:
- Harden geometry handoff from `viewer_core` instance geometry to `viewer_render` dynamic mesh submission.
- Lock a deterministic fallback matrix for missing/invalid geometry sources.
- Preserve scene update invariants (dirty-only apply, seam ownership, `sync_spatial` before query/render).
- Define repeatable runtime verification scenarios for bounded mixed geometry (`Procedural`, `Sculpt`, `Mesh`, `Diagnostic`).

Out of scope:
- Draw-pool/instancing/occlusion optimization work.
- Full texture/material feature expansion.
- Avatar rigging/skinning/animation systems.
- Broader world/object decode expansion (belongs to `N03`).

## Current known state
- M2 geometry groundwork exists and is treated as complete.
- `viewer_app` currently prepares dynamic geometry for visible instances and calls `RenderBackend::upsert_geometry(...)`.
- `viewer_render` currently falls back to cube for non-diagnostic geometry without dynamic buffers.
- `viewer_core::Scene::sync_spatial()` is called in the frame loop before frustum query/render path.
- `viewer_asset::GeometryCache` already provides procedural/sculpt/mesh processed meshes for bounded local use.
- Live/world feed breadth is still bounded; this milestone must not assume full world decode.

## Files and components touched
- `crates/viewer_app/src/main.rs`
  Frame orchestration and geometry preparation policy.
- `crates/viewer_core/src/lib.rs`
  Scene/instance geometry metadata and spatial invariants used by renderer integration.
- `crates/viewer_render/src/lib.rs`
  Dynamic geometry buffer upsert/lookup, fallback path behavior, and draw submission.
- `crates/viewer_asset/src/lib.rs`
  Geometry cache usage contract as consumed by app render-preparation path.
- `docs/reports/` and continuity artifacts after implementation (execution phase only).

## Boundary check
- `viewer_app` remains orchestration-only: selects what to prepare, does not own GPU internals.
- `viewer_core` remains domain authority for scene state and geometry identity.
- `viewer_render` remains sole owner of GPU resources, pipelines, and submission behavior.
- `viewer_asset` remains CPU-side processed geometry provider; no `wgpu` handle ownership moves there.
- No grid/protocol semantics introduced into rendering crates.

## Step sequence
1. Baseline and contract capture:
   - Document current geometry path behavior (`GeometrySource` -> cache -> renderer dynamic map).
   - Define exact readiness conditions for “dynamic geometry available”.
2. Fallback matrix definition:
   - Lock behavior for each geometry source on missing/empty/invalid mesh data.
   - Ensure fallback behavior is deterministic and does not flicker frame-to-frame.
3. Integration hardening:
   - Tighten app-side preparation guards to avoid uploading invalid mesh payloads.
   - Ensure renderer-side lookup and fallback logic follows the locked matrix for all non-diagnostic paths.
4. Spatial correctness guardrails:
   - Preserve `sync_spatial` sequencing and world-AABB consistency when dynamic geometry updates local bounds.
   - Confirm no seam-owned role lifecycle is bypassed.
5. Deterministic verification coverage:
   - Add/adjust targeted tests around geometry availability and fallback selection.
   - Add repeatable bounded runtime smoke scenario using stress path.
6. Milestone closeout gating:
   - Confirm no performance target gates are used as exit criteria.
   - Confirm boundaries remain unchanged and explicitly documented in implementation report.

## Validation plan
- Static and build checks:
  - `cargo fmt`
  - `cargo check`
- Tests:
  - Targeted tests for touched rendering/scene preparation paths.
  - Broader `cargo test` if behavior changes materially across crate boundaries.
- Runtime smoke (required):
  - `VIEWER_APP_LIVE_STARTUP=off` with bounded stress scenario (including `STRESS_TEST=2` where applicable).
  - Run repeated launch/smoke cycles to verify deterministic fallback behavior.
- Acceptance evidence:
  - No missing/flat fallback regressions in bounded stress scene.
  - No crash/regression when geometry buffers are missing/empty/late.

## Risks and open questions
Risks:
- Hidden coupling between geometry cache outputs and renderer assumptions may create silent fallback regressions.
- Existing bounded mesh/sculpt placeholders may mask readiness issues if fallback policy is not explicit.
- Large local workspace deltas may introduce unrelated noise during verification.

Open questions:
- None for this milestone plan. Decisions required for execution are locked in this document.

## Completion criteria
- A documented and implemented fallback matrix exists for all geometry source categories in scope.
- Rendering integration behaves deterministically across repeated bounded runtime smoke runs.
- Boundary ownership remains intact (`viewer_app` orchestration, `viewer_core` domain, `viewer_render` GPU internals).
- Validation commands complete with passing results or explicitly documented blockers.
- No FPS or throughput target is required for milestone exit.
