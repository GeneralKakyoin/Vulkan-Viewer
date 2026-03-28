# REPORT: R16 Transition Visual Continuity — Completion

**Date:** 2026-03-28  
**Milestone:** R16 — Transition Visual Continuity Polish  
**Status:** Complete

---

## Summary of Implemented Work

R16 adds deterministic, bounded transition visual cues to the render path. When a region handoff is degraded, stalled, or recovering, the scene receives a subtle per-fragment tint that communicates continuity health without disrupting material accuracy.

No new shader architecture, post-processing pipeline, or UI rendering policy was introduced. All changes flow through the existing environment uniform buffer.

Post-review hardening update:
- `derive_transition_visual_cue()` now requires probe-success recency (bounded by probe cooldown window) before emitting `Recovering`, preventing stale probe-success state from masking later degraded conditions.
- Additional app tests were added to lock stale-success behavior.

---

## Files Changed

| File | Change |
|------|--------|
| `crates/viewer_core/src/lib.rs` | Added `TransitionVisualCue` enum, `TransitionVisualState` struct, `sanitized()` helper, and 4 unit tests |
| `crates/viewer_render/src/lib.rs` | Extended `EnvironmentUniform` with `cue_params` vec4 (112→128 bytes buffer), updated `render_frame` signature, `upload_environment_uniforms`, WGSL struct, and added `cue_tint_color()` WGSL function + fragment tint logic. Added 3 unit tests. |
| `crates/viewer_app/src/main.rs` | Added `transition_visual_state` field to `AppState`, `derive_transition_visual_cue()` function, per-frame derivation alongside environment, `render_frame` call updated, `RenderInput` field added. Added recency-gated recovery mapping and stale-probe regression tests (7 cue tests total). |
| `crates/viewer_ui/src/lib.rs` | Added `transition_visual_state` to `RenderInput`, destructuring, and read-only Render Cue diagnostics line in the Environment panel. Added 1 unit test. |

---

## Visual Cue Mapping Contract

| Continuity State | Cue | Intensity | WGSL Mode Index | Fragment Blend Cap |
|---|---|---|---|---|
| Normal (no probe success / None phase) | `Healthy` | 0.0 → no-op | 0 | No tint applied |
| Normal + probe success + active phase | `Recovering` | 0.5 | 3 | 0.09 (18% × 0.5) |
| Degraded + no probe success | `Degraded` | 0.6 | 1 | 0.108 (18% × 0.6) |
| Degraded + probe success | `Recovering` | 0.6 | 3 | 0.108 |
| Stalled | `Stalled` | 0.9 | 2 | 0.162 (18% × 0.9) |

All blend amounts are conservative (≤18% max) to prevent material/texture washout.

---

## Validation Run

### Commands Executed

```
cargo fmt --all
cargo check --workspace
cargo test -p viewer_core -p viewer_render -p viewer_app -p viewer_ui
cargo test --workspace
cargo run -p viewer_app  [STRESS_TEST=screenshot, 2 frames, VIEWER_APP_LIVE_STARTUP=off]
cargo test -p viewer_app --bin viewer_app derive_transition_visual_cue_
cargo run -p viewer_app  [STRESS_TEST=screenshot, degraded snapshot, 2 frames, VIEWER_APP_LIVE_STARTUP=off]
cargo run -p viewer_app  [STRESS_TEST=screenshot, stalled snapshot, 2 frames, VIEWER_APP_LIVE_STARTUP=off]
```

### Results

| Command | Result |
|---------|--------|
| `cargo fmt --all` | ✅ Pass (no diffs) |
| `cargo check --workspace` | ✅ Pass — 0 errors |
| Targeted crate tests | ✅ Pass — all R16 tests and prior tests pass |
| `cargo test --workspace` | ✅ Pass |
| Screenshot smoke test | ✅ Pass — stable scene, no regressions |
| `cargo test -p viewer_app --bin viewer_app derive_transition_visual_cue_` | ✅ Pass — 7/7 cue-mapping tests |
| Degraded/stalled screenshot smoke tests | ✅ Pass — both runs captured expected frame outputs |

### Screenshot Smoke Test Verdict

Visual review of `artifacts/screenshots_r16_smoke/viewer_test_0001.png` confirms:
- Stable 3D scene with expected sky gradient and fog depth
- Scene geometry (test objects) visible and non-corrupt
- UI overlay ("SL Viewer Rewrite" header, diagnostics) intact
- No color corruption or black screen
- Healthy cue baseline: no tint applied (mode_index=0, intensity=0.0) as expected
- Degraded cue evidence captured: `artifacts/screenshots_r16_smoke/degraded/viewer_test_0001.png`
- Stalled cue evidence captured: `artifacts/screenshots_r16_smoke/stalled/viewer_test_0001.png`

---

## Architecture Boundary Compliance

- No new bind groups, pipelines, or descriptor set layouts created (L09 safe)
- No new shader files — WGSL inline extension only
- Buffer size increased 112→128 bytes (single additional vec4) within existing GPU object lifetime
- Mapping logic confined to `viewer_app`; policy-free in `viewer_render`
- `viewer_ui` display is read-only diagnostic only (no control path)
- `TransitionVisualState` is sanitized twice: in `derive_transition_visual_cue()` and in `upload_environment_uniforms()` for a defense-in-depth guarantee

---

## Risks and Follow-up Items

- The healthy→recovering transition at intensity 0.5 is subtle; if operator feedback indicates difficulty distinguishing it from healthy, intensity could be raised slightly (deferred to A17 review cycle)

---

## Learnings Delta

**Updated L22** (visual contracts must be deterministic and sanitized at both producer and consumer).  
**No new learning required** — the implementation confirmed existing L09 (pipeline lifecycle) and L21 (fog distance math) constraints without discovering new traps.

---

## Continuity Updates Performed

- `docs/CURRENT_STATE.md` — updated to reflect R16 complete
- `docs/HANDOFF.md` — updated with R16 exact state, next step A17
- `docs/LEARNINGS.md` — L22 delta recorded
