# Plan: Render Alpha Mode Tuning For Object Feed (2026-04-09)

## Objective
Reduce avoidable blend-mode haze/sorting artifacts for near-opaque object-feed materials by introducing a bounded alpha classification that can emit `AlphaTest` instead of forcing `Blend` for all `alpha < 0.995`.

## Scope
- In scope:
  - update `viewer_core::world_object_feed_alpha_mode(...)` classification logic
  - add/update focused `viewer_core` tests for opaque/alpha-test/blend decisions
  - validation on touched crates
- Out of scope:
  - shader rewrite
  - full SL material parity redesign
  - protocol/decode changes

## Current known state
- Current logic returns `Blend` whenever any material alpha is below `0.995`.
- This is conservative but can overuse transparent pass for near-opaque content.

## Files and components touched
- `crates/viewer_core/src/lib.rs`
- continuity artifacts (`docs/CURRENT_STATE.md`, `docs/HANDOFF.md`, report)

## Boundary check
- Alpha classification belongs to domain/render-prep policy in `viewer_core`.
- No ownership drift into `viewer_render` or `viewer_net`.

## Step sequence
1. Add bounded alpha classification helper in `viewer_core`:
   - opaque for fully opaque
   - alpha-test for near-opaque alpha range
   - blend for clearly translucent range
2. Wire `world_object_feed_alpha_mode(...)` through helper.
3. Add regression tests covering all branches.
4. Run fmt/check/tests.
5. Update continuity docs.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_core -p viewer_app -p viewer_render`
- `cargo test -p viewer_core scene_world_object_feed_applies_face_override_materials_and_alpha_mode -- --nocapture`
- `cargo test -p viewer_core world_object_feed_alpha_mode_classifies_near_opaque_as_alpha_test -- --nocapture`

## Risks and open questions
- Threshold selection is heuristic; keep bounded and test-backed.

## Deferred-too-early candidates captured
- none.

## Learnings pre-check
- L22 (artifact-backed validation), L82 (local bounded ownership).

## Completion criteria
- New alpha classification logic + tests pass.
- Continuity docs reflect behavior change and validation.
