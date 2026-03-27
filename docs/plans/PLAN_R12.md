# Plan: R12 Environment and Atmospheric Baseline

## Summary
Introduce a bounded environment rendering baseline (ambient/sky/fog/time-of-day influence) that materially improves scene readability and continuity perception, while preserving strict crate boundaries and deterministic fallback behavior.

## Objective
- Add typed environment state contracts in `viewer_core` that are renderer-consumable and serialization-safe.
- Render environment influence deterministically in `viewer_render` using startup-created pipelines/uniforms.
- Keep fallback environment behavior stable when environment inputs are missing/partial.
- Ensure environment rendering remains bounded and does not become full EEP parity scope.

## Why now
- After `N11`, transition-state robustness improves but visual continuity quality remains limited without environment context.
- A bounded atmosphere baseline improves usability and diagnostic confidence before broader parity pushes.
- Roadmap sequencing places render environment/polish work after continuity hardening.

## In scope
- `viewer_core`:
  - Add `EnvironmentState` contract (ambient color, sky tint/gradient params, fog density/color, bounded time-of-day scalar).
  - Add deterministic defaults.
  - Keep contracts independent from renderer internals.
- `viewer_render`:
  - Add bounded environment uniforms and shader usage.
  - Apply fog/ambient modulation deterministically.
  - Preserve existing material fallback behavior and pass ordering rules.
- `viewer_app`:
  - Map environment state into renderer call path.
  - Provide deterministic fallback environment state when live inputs are absent.
- `viewer_ui` (bounded):
  - Display read-only environment diagnostics lines (optional but recommended for verification).

## Out of scope
- Full EEP parity (full day-cycle scripting, advanced atmospheric scattering, water parity stack).
- Post-processing framework overhauls.
- Broad shader architecture rewrite.
- Environment-authoring UI toolchain.

## Current known state
- Rendering pipeline and pass bucketing are deterministic and bounded (`R05`/`A06`).
- Region continuity + asset continuity now exist (`N07`/`A10`), but environment context is minimal.
- `viewer_render` invariants require startup pipeline creation and resize-only depth recreation.
- UI currently focuses diagnostics and workflow surfaces; environment diagnostics are not yet first-class.

## Files and components touched
- `crates/viewer_core/src/lib.rs`
  - `EnvironmentState` structs/enums/defaults
  - serialization-safe additive fields where required
  - tests
- `crates/viewer_render/src/lib.rs`
  - environment uniform structs/buffer updates
  - shader updates for ambient/fog/sky influence
  - tests
- `crates/viewer_app/src/main.rs`
  - wiring and fallback mapping for environment state
  - tests
- `crates/viewer_ui/src/lib.rs` (if touched)
  - read-only diagnostics display for environment values
  - tests

## Boundary check
- `viewer_core` owns environment domain contracts.
- `viewer_render` owns GPU implementation details and shader/pipeline behavior.
- `viewer_app` owns orchestration/mapping only.
- `viewer_ui` owns display only.
- No environment policy in `viewer_net`/`viewer_grid` unless and until live source integration is explicitly planned.

## Step sequence
1. Define typed environment contract in core
   - add structs/enums with deterministic defaults
   - keep additive and backward-compatible.
2. Add renderer environment uniform path
   - create/update uniform buffers in existing frame update path
   - keep pipeline creation at startup only.
3. Integrate shader logic (bounded)
   - ambient modulation
   - fog influence with clamped parameters
   - bounded sky tint path for background/readability.
4. Wire app mapping/fallback
   - map current state to environment defaults
   - ensure deterministic behavior when no live environment source exists.
5. Add optional diagnostics lines
   - show environment state values in diagnostics for smoke verification.
6. Add tests
   - `viewer_core`: default/serialization tests
   - `viewer_render`: uniform packing, clamp behavior, fallback path tests
   - `viewer_app`: mapping/fallback tests
   - `viewer_ui` (if touched): deterministic line rendering helpers.
7. Runtime verification
   - offline smoke with screenshot mode
   - verify deterministic visual effect with fixed camera path.
8. Closeout continuity artifacts.

## Validation plan
- Always:
  - `cargo fmt --all -- --check`
  - `cargo check --workspace`
- Targeted:
  - `cargo test -p viewer_core`
  - `cargo test -p viewer_render`
  - `cargo test -p viewer_app`
  - `cargo test -p viewer_ui` (if touched)
- Broader:
  - `cargo test --workspace` (material behavior change)
- Runtime smoke:
  - `VIEWER_APP_LIVE_STARTUP=off STRESS_TEST=screenshot VIEWER_TEST_SCREENSHOT_DIR=artifacts/screenshots_r12_smoke VIEWER_TEST_SCREENSHOT_EVERY_N_FRAMES=1 VIEWER_TEST_SCREENSHOT_MAX_FRAMES=2 cargo run -p viewer_app`

## Risks and open questions
- Risk: environment parameters can unintentionally reduce readability if defaults are poorly tuned.
- Risk: shader modifications may introduce subtle regressions in alpha/fog interactions.
- Risk: over-scoping toward full EEP parity may creep into baseline milestone.
- Open question: whether environment state should be a single global profile for R12 or a bounded per-region profile cache.
- Open question: whether fog should apply to diagnostics proxy meshes identically or with bounded exceptions.

## Deferred-too-early candidates captured
- Full EEP day cycle and sky/water parity system.
- Advanced post-processing and atmospheric scattering pipeline.

## Learnings pre-check
- `L08`: depth lifecycle must remain resize-only.
- `L09`: pipeline creation must remain startup-only.
- `L16`: preserve white/missing/loading fallback semantics for textures.
- `L04`: preserve dirty-only app apply semantics while wiring environment updates.
- `L12`: preserve spatial sync and deterministic frame ordering.

## Completion criteria
- Environment baseline contract exists and is renderer-wired.
- Visual output shows bounded ambient/fog/sky influence with deterministic fallback.
- No crate boundary violations introduced.
- Validation commands pass (or blockers are documented).
- Continuity docs updated with exact state and next step.

