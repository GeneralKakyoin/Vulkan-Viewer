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
  - extend `EnvironmentState` with bounded atmosphere controls:
    - `time_of_day_normalized: f32` (0.0..=1.0)
    - `fog_enabled: bool`
    - `sky_enabled: bool`
  - add `impl EnvironmentState { fn sanitized(self) -> Self }` to clamp/normalize values before renderer use
  - serialization-safe additive fields (`#[serde(default)]` on all new fields)
  - tests for defaults + serde backward compatibility + clamping behavior
- `crates/viewer_render/src/lib.rs`
  - extend `EnvironmentUniform` packing:
    - `flags: vec4<f32>` where x=`fog_enabled`, y=`sky_enabled`, z/w reserved
    - `time_params: vec4<f32>` where x=`time_of_day_normalized`, y/z/w reserved
  - update `upload_environment_uniforms(...)` to consume `env.sanitized()`
  - shader updates:
    - use deterministic ambient mix (no unbounded brighten)
    - fog blend guarded by `fog_enabled`
    - keep alpha test/discard semantics unchanged
  - tests:
    - uniform packing/flags
    - clamped fog behavior
    - alpha path unaffected by environment toggles
- `crates/viewer_app/src/main.rs`
  - keep `AppState.environment` as source of truth
  - add `fn derive_environment_from_snapshot(snapshot: Option<&LiveVisualSnapshot>) -> EnvironmentState`
  - call derivation only when snapshot meaningfully changes (preserve dirty-only behavior)
  - tests for deterministic fallback when snapshot is absent or partial
- `crates/viewer_ui/src/lib.rs` (if touched)
  - add read-only diagnostics lines in existing Diagnostics window:
    - `env.time_of_day_normalized`
    - `env.fog_enabled`
    - `env.sky_enabled`
    - fog start/end/density
  - tests for deterministic line formatting

## Boundary check
- `viewer_core` owns environment domain contracts.
- `viewer_render` owns GPU implementation details and shader/pipeline behavior.
- `viewer_app` owns orchestration/mapping only.
- `viewer_ui` owns display only.
- No environment policy in `viewer_net`/`viewer_grid` unless and until live source integration is explicitly planned.

## Step sequence
1. **Core contract hardening (`viewer_core`)**
   - Extend `EnvironmentState` with the three additive fields listed above.
   - Add `#[serde(default)]` for each new field so old snapshots deserialize.
   - Add `sanitized()` that:
     - clamps `time_of_day_normalized` to `[0.0, 1.0]`
     - clamps `fog.density` to `[0.0, 1.0]`
     - guarantees `fog.end >= fog.start + 0.001`
   - Keep existing defaults visually neutral (no dramatic tint shift).
2. **Core tests**
   - Add `environment_state_defaults_are_deterministic`.
   - Add `environment_state_deserializes_from_pre_r12_payload`.
   - Add `environment_state_sanitized_clamps_invalid_values`.
3. **Renderer uniform expansion (`viewer_render`)**
   - Extend WGSL `EnvironmentUniform` and matching Rust `#[repr(C)]` struct with `flags` and `time_params`.
   - Preserve uniform alignment and `bytemuck::Pod` validity.
   - Update bind/write path only; do not add new runtime-created pipelines.
4. **Renderer upload path**
   - In `upload_environment_uniforms`, call `let env = env.sanitized();`.
   - Convert booleans to `0.0/1.0` in `flags`.
   - Keep all writes in existing per-frame uniform update flow.
5. **Shader behavior adjustments**
   - Ambient: replace additive brighten with bounded mix, e.g. `mix(final_color.rgb, final_color.rgb + env.ambient_color.rgb, 0.35)`.
   - Fog: apply only when `env.flags.x > 0.5`.
   - Sky: if `env.flags.y <= 0.5`, skip sky gradient influence and use current fallback.
   - Keep alpha-test and blend ordering exactly as current R05/A06 behavior.
6. **Renderer tests**
   - Add unit test for `EnvironmentUniform` packing and flag encoding.
   - Add regression test ensuring fog-disabled path is visually no-op in shader math helpers (or CPU-side precompute helper if used).
7. **App mapping (`viewer_app`)**
   - Add `derive_environment_from_snapshot(...)` near snapshot mapping helpers.
   - Derivation rules:
     - `None` snapshot => `EnvironmentState::default()`
     - missing/partial fields => defaults for missing pieces
     - continuity degraded/stalled may slightly increase fog density, but clamp via `sanitized()`
   - Assign `self.environment` only when derived value changed.
8. **App tests**
   - Add `derive_environment_from_snapshot_uses_defaults_when_absent`.
   - Add `derive_environment_from_snapshot_clamps_and_is_deterministic`.
9. **UI diagnostics (`viewer_ui`)**
   - In Diagnostics panel, add four to six read-only lines in the existing environment section (or create a small subsection under Performance & Metrics).
   - Do not add mutating controls in R12.
10. **UI tests (if UI touched)**
   - Add deterministic text-line test similar to existing `live_visual_lines_*` tests.
11. **Runtime verification**
   - Run screenshot smoke with fixed camera path and compare two sequential captures for deterministic environment output.
12. **Continuity closeout**
   - Update report/handoff/current-state with exact field additions and validation outcomes.

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
