# Review: R12 Environment and Atmospheric Baseline Plan

## Verdict
**APPROVED**

The plan is well-structured, respects the established crate boundaries, and provides a clear path for a bounded environment rendering baseline without slipping into full EEP parity too early.

## Architecture and Boundary Fit
- **Excellent**: The division of labor between `viewer_core` (contracts), `viewer_render` (uniforms/shaders), and `viewer_app` (fallback mapping) is consistent with the `ARCHITECTURE.md` and `INTERFACES.md` invariants.
- **No Boundary Erosion**: It avoid putting environment policy in `viewer_net` or `viewer_grid`, keeping it as a scene-level property.

## Correctness Concerns
- **Uniform Mapping**: Ensure that the uniform buffer alignment in `viewer_render` matches the shader's expectation (WGPU/WGSL requirements).
- **Default Tuning**: As noted in risks, defaults must be neutral enough to keep the geometry torture and axis markers readable.

## Modularity and Maintainability Concerns
- **Additive Design**: The plan specifies additive changes to `EnvironmentState`, which is good for backward compatibility if/when full EEP arrives.
- **Shader Complexity**: Keep the shader additions minimal to avoid performance regressions on lower-end hardware during this baseline phase.

## Validation Adequacy
- The inclusion of runtime smoke with `VIEWER_APP_LIVE_STARTUP=off` and `STRESS_TEST=screenshot` is a strong validation strategy for visual features.
- Targeted tests for `viewer_core` (serialization) and `viewer_render` (packing) are appropriate.

## Risks and Open Questions
- **Risk (Fog on Proxies)**: Applying fog to diagnostics/proxy meshes might make them harder to see. I recommend a bounded exception or a "diagnostics-neutral" fog setting if readability suffers.
- **Open Question (Global vs Per-Region)**: For R12, a single global profile derived from the active region's defaults is sufficient. Per-region profile caching is a valid `DEFERRED_FEATURES` candidate for later parity work.

## Learnings Delta Verdict
- **None**: No new durable learnings estimated at this stage, but the plan correctly references existing ones (`L08`, `L09`, `L16`, `L04`, `L12`).

## Approval Status
**APPROVED** for implementation.
