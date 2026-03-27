# Review: PLAN_R05 Draw Submission Efficiency and Transparency Reliability

## Verdict
**Approved**

## Architecture and Boundary Fit
- **Compliance**: The plan perfectly respects the established crate boundaries.
- **Contract Ownership**: Extending `RenderableInstance` to RGBA and introducing `AlphaMode` in `viewer_core` correctly places domain types in the shared vocabulary crate.
- **Logic Ownership**: Implementing the bucketing, sorting, and capping logic entirely within `viewer_render` preserves its role as the sole owner of GPU-facing policy.
- **Orchestration**: `viewer_app` updates are kept to a minimum, primarily handling the change in the color contract.

## Correctness Concerns
- **Determinism**: The decision to use `instance_id` (the `Scene` map key) as a stable tie-breaker for sorting is excellent. This ensures that even when distances or geometry keys match, the draw order remains identical across frames.
- **Capping Policy**: Capping *after* sorting is the correct approach to ensure that the most relevant (nearest or grouped) objects are rendered when reaching `max_objects`.
- **Transparency**: Back-to-front sorting for the `Blend` bucket is standard and necessary for correct alpha blending without depth writes.

## Modularity and Maintainability Concerns
- **Extracted Logic**: The plan wisely targets extracted "pure helpers" for unit testing. This keeps the core draw loop clean and allows for robust verification of sorting/bucketing logic without needing a complex `wgpu` mock or device.
- **Independence**: Removing dependence on `visibility_list` iteration order guards against subtle visual regressions if the underlying spatial partitioning (Octree) or visibility culling logic changes in the future.

## Validation Adequacy
- **Unit Tests**: Coverage for bucket selection, sorting stability, and capping behavior is comprehensive.
- **Runtime Verification**: Leveraging `STRESS_TEST=camera` and `STRESS_TEST=screenshot` (recently hardened in the repo) provides the necessary end-to-end visual confirmation of transparency stability.

## Risks and Open Questions
- **Risk Mitigation**: The risk of unstable upstream ordering is explicitly mitigated by the stable sorting keys.
- **Alpha source**: The decision to limit alpha to instance-level RGBA (avoiding texture alpha for now) is a sound engineering compromise that keeps the milestone bounded and achievable.

## Required Revisions or Approval Status
- **Status**: Approved without required revisions.
- **Recommendation**: Ensure that the `instance_id` used for tie-breaking is the consistent `usize` key from the `Scene::instances` map.
