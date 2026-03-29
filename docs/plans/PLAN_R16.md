# Plan: R16 Transition Visual Continuity Polish

## Summary
Add deterministic, bounded transition-health visual cues in the renderer so degraded, stalled, and recovering continuity states are visible without disrupting material readability or changing render architecture.

## Objective
- Introduce a typed transition cue contract and app-side mapping from continuity/probe state.
- Apply subtle bounded tinting in the existing render path.
- Expose cue status read-only in diagnostics for operator verification.

## Why now
- `N11` and `U14` establish typed continuity/recovery state but in-scene transition health was still hard to read quickly.
- A small visual cue layer improves observability while preserving bounded architecture and render determinism.

## In scope
- `viewer_core`: typed cue state contract and sanitization.
- `viewer_app`: deterministic continuity/probe -> cue mapping with dirty-only update semantics.
- `viewer_render`: extend existing environment uniform and shader logic for bounded cue tint.
- `viewer_ui`: read-only cue diagnostics line.
- Targeted tests and screenshot-smoke verification for healthy/degraded/stalled scenarios.

## Out of scope
- Post-processing pipeline additions.
- New shader files or render-graph architecture changes.
- Interactive UI controls for cue authoring.
- Broad visual redesign outside transition cue intent.

## Current known state
- Continuity diagnostics and handoff outcome typing are already present from `N11`.
- Environment-uniform wiring exists in renderer from `R12`, providing a bounded carrier for cue params.
- Recovery controls and diagnostics scaffolding exist from `U14`.

## Files and components touched
- `crates/viewer_core/src/lib.rs`
  - Add `TransitionVisualCue`, `TransitionVisualState`, and sanitization behavior.
- `crates/viewer_app/src/main.rs`
  - Add deterministic `derive_transition_visual_cue(...)` mapping and render wiring.
- `crates/viewer_render/src/lib.rs`
  - Extend `EnvironmentUniform` with cue params and WGSL cue tint helper.
- `crates/viewer_ui/src/lib.rs`
  - Add read-only diagnostics line for cue mode/intensity.
- `docs/reports/REPORT_R16_completion.md`
- `docs/reviews/REVIEW_IMPL_R16.md`

## Boundary check
- `viewer_core` owns typed contracts.
- `viewer_app` owns mapping/orchestration only.
- `viewer_render` owns GPU implementation details.
- `viewer_ui` remains display-only.
- No transport/grid semantics moved into render/UI.

## Step sequence
1. Define cue contract types and sanitization in `viewer_core`.
2. Implement deterministic cue derivation in `viewer_app` from continuity/probe status.
3. Thread cue state into existing render call input.
4. Extend renderer uniform packing and shader tint path with bounded blend caps.
5. Add diagnostics display in `viewer_ui`.
6. Add targeted tests for mapping, sanitization, and uniform/shader behavior.
7. Run screenshot smoke in healthy/degraded/stalled cases and review captured images.
8. Apply fix pass if needed (e.g., stale probe-success recency gating) with regression tests.

## Validation plan
- `cargo fmt --all`
- `cargo check --workspace`
- `cargo test -p viewer_core -p viewer_render -p viewer_app -p viewer_ui`
- `cargo test --workspace`
- Runtime screenshot smoke:
  - `VIEWER_APP_LIVE_STARTUP=off STRESS_TEST=screenshot ... cargo run -p viewer_app`
  - Additional degraded/stalled snapshot runs to verify cue mapping

## Risks and open questions
- Risk: cue intensity may be too subtle for operators or too strong for scene readability.
- Risk: stale probe success can misclassify `Recovering` without explicit recency gating.
- Open question: whether recover-state intensity should vary by continuity phase or remain fixed.

## Deferred-too-early candidates captured
- Full post-processing transition effect stack.
- Operator-configurable cue palettes/intensity presets.

## Learnings pre-check
- `L08`: no per-frame GPU resource churn for new render parameters.
- `L09`: no runtime pipeline creation; keep startup-owned pipeline lifecycle.
- `L21`: use physically sensible bounded visual math.
- `L22`: screenshot-based validation requires manual image review evidence.

## Completion criteria
- Cue contracts and mapping are implemented and tested.
- Renderer applies bounded cues through existing uniform path with no architectural drift.
- Diagnostics expose cue state read-only.
- Healthy/degraded/stalled visual evidence is captured and reviewed.
- Validation commands pass or blockers are explicitly documented.

