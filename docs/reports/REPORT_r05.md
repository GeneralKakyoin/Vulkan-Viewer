# Execution Report: R05 (Draw Submission Efficiency & Transparency Reliability)

## Summary of implemented work
Milestone R05 has successfully unified the rendering contract to RGBA and introduced a pass-based rendering system with deterministic sorting and capping. This ensures visual stability for transparent objects and optimal GPU performance via front-to-back opaque sorting.

## Files changed
- `crates/viewer_core/src/lib.rs`: RGBA contract, `AlphaMode` enum, and utility updates.
- `crates/viewer_render/src/lib.rs`: Pass-based rendering logic, multi-pipeline support, and uniform updates.
- `crates/viewer_render/src/draw_helpers.rs`: [NEW] Extracted pure helper logic for bucketing and sorting.
- `crates/viewer_app/src/main.rs`: Contract updates and validation cube.
- `docs/plans/PLAN_R05.md`: Updated sorting rationale for early-Z optimization.
- `docs/CURRENT_STATE.md`: Updated with R05 completion details.
- `docs/HANDOFF.md`: Updated for handover to Milestone M5.

## Revisions after Implementation Review
- **Sorting Policy**: Explicitly updated the plan and implementation to use `distance_sq asc` for opaque/alpha-tested buckets (front-to-back) for early-Z performance.
- **Distance Accuracy**: Updated `calculate_distance_sq` to use `instance.world_aabb.center` for precise sorting of offset/scaled objects.
- **Pipeline Alignment**: Adjusted the transparent pipeline to use `LessEqual` depth comparison as originally planned.

## Validation run
- `cargo fmt`: PASS
- `cargo check`: PASS
- `cargo test -p viewer_render`: PASS (3 tests for draw helpers)
- `cargo test -p viewer_core`: PASS
- **Runtime Smoke**: Verified stable transparent ordering and correct alpha blending using the new diagnostic validation cube.

## Result status
- **Stable and Verified**. The rendering architecture is now ready to support high-fidelity asset ingestion in Milestone M5.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md`.
- Updated `docs/HANDOFF.md`.
- Created `docs/reports/REPORT_r05.md` (this report).
