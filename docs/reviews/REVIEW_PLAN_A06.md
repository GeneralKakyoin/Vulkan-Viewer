# Review: PLAN_A06 Material/Texture Depth Expansion

## Verdict
**Approved**

## Architecture and Boundary Fit
- **Compliance**: The plan strictly adheres to the crate ownership rules defined in `ARCHITECTURE.md`.
- **Core Domain**: Activating `viewer_core::material` correctly centralizes the "common language" for materials and UV transforms.
- **Asset/Acquisition Layer**: Correctly identifying `viewer_asset` as the owner of cache and fixture policy prevents acquisition logic from leaking into the renderer or orchestration.
- **Render Layer**: `viewer_render` is appropriately limited to bind group management, shader execution, and local fallback texture behavior.

## Correctness Concerns
- **Per-face materials**: Mapping submesh `face_id` to `MaterialDescriptor` is the correct approach for SL/OpenSim compatibility. The fallback logic in `MaterialSet` (default material) ensures robustness.
- **Acquisition Determinism**: The choice to use fixture-backed acquisition and deterministic fallbacks (`Loading`/`Missing`) is critical for stable, repeatable development and testing.
- **Resource Management**: The 64-request-per-tick cap for texture acquisition in `viewer_app` is a sensible guard against frame-rate spikes during heavy scene ingestion.

## Modularity and Maintainability Concerns
- **Reusable Contracts**: By exposing `MaterialDescriptor` as a project-wide contract, future milestones (like `A07 PBR`) can easily build upon this foundation without re-wiring.
- **Seam Integrity**: The plan does not violate the ingestion seam; it simply extends the existing `RenderableInstance` to carry more metadata, which is the correct way to pass material state.

## Validation Adequacy
- **Testing Surface**: The plan includes unit tests for deduped ID extraction and material selection logic, which are the most error-prone parts of the plumbing.
- **Smoke Testing**: Clear instructions for running in offline mode with fixture textures (`VIEWER_FIXTURE_TEXTURES=1`) allow for reliable end-to-end verification.

## Risks and Open Questions
- **Bind Churn**: While acknowledged, per-face binding is acceptable for this milestone. Future grouping optimizations (e.g., sorting draw calls by material) should be deferred as planned.
- **Shading Level**: The explicit "unlit textured" scope effectively bounds the complexity and prevents "milestone bloat."

## Required Revisions or Approval Status
- **Status**: Approved without required revisions.
- **Recommendation**: Ensure the `loading_fallback` and `missing_fallback` textures are visually distinct (e.g., standard checkerboard or solid high-contrast colors) to simplify visual debugging.
