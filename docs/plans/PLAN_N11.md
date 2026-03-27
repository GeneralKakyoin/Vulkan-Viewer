# Plan: N11 Region Handoff Hardening and Transition Diagnostics

## Summary
Harden bounded region handoff behavior after `N07` + `A10` so transition progression remains deterministic under degraded network conditions, and expose explicit transition diagnostics that make recovery state observable without collapsing `viewer_net` / `viewer_grid` boundaries.

## Objective
- Stabilize the handoff phase model (`None`, `Crossed`, `Confirming`, `Completed`) under packet delay, reordering, and bounded receive windows.
- Add typed handoff outcome/reason diagnostics that distinguish normal progression, delayed progression, and degraded/fallback progression.
- Keep continuity state bounded, deterministic, and compatible with existing seam/snapshot lifecycle rules.
- Improve operator confidence by surfacing transition health through existing diagnostics pathways.

## Why now
- `A10` now provides bounded continuity-aware asset policy; transition reliability is the highest remaining continuity bottleneck.
- Existing continuity signals are present but still shallow in failure-mode classification and recovery observability.
- Roadmap sequencing requires `N11` before broader visual/runtime expansion (`R12`).

## In scope
- `viewer_net`:
  - Harden transition-control observation handling around `CrossedRegion` and `ConfirmEnableSimulator`.
  - Add deterministic transition timers/budgets (bounded stale windows and bounded retry-classification windows).
  - Export typed transition diagnostic summary:
    - last observed phase
    - phase age/staleness bands
    - bounded reason code/classification for degraded state
    - bounded counter set for stalled/late confirmations
- `viewer_grid`:
  - Own semantic classification for transition outcomes/reasons where “meaning” exceeds raw transport facts.
  - Keep policy mapping explicit and testable, without introducing transport mechanics.
- `viewer_core`:
  - Add additive typed continuity/transition diagnostics to `LiveVisualSnapshot` with `serde(default)`.
  - Add bounded seam payload lane(s) for transition diagnostics visualization if scene-facing diagnostics are needed.
- `viewer_app`:
  - Map `viewer_net` summaries to `viewer_core` snapshot fields.
  - Preserve dirty-only apply and seam ownership discipline.
- `viewer_ui` (bounded):
  - Present transition diagnostics lines/chips from snapshot/core state only.

## Out of scope
- Full teleport/session orchestration parity.
- Destination-region socket/session rebuild parity.
- Unlimited region graph management or unbounded neighbor retention.
- Broad packet classification expansion beyond transition-related needs.
- Changing renderer/material policy.

## Current known state
- `N07` established bounded continuity phase and neighbor summaries.
- `A10` established bounded continuity asset priority and cache discipline.
- Transition diagnostics currently emphasize counters and phase state, but degraded-state reasoning remains coarse.
- Existing architecture rules require:
  - seam-owned create/remove only in `Scene::apply_world_object_ingestion_seam(...)`
  - dirty-only apply in app frame loop
  - strict `viewer_net` (transport) vs `viewer_grid` (meaning) split.

## Files and components touched
- `crates/viewer_net/src/lib.rs`
  - transition observation accumulation
  - bounded timers/staleness classification
  - typed diagnostic summary export
  - tests
- `crates/viewer_grid/src/lib.rs` (or adapter module)
  - transition meaning classifier/types
  - tests for classification correctness
- `crates/viewer_core/src/lib.rs`
  - additive snapshot diagnostic fields (serde-defaulted)
  - optional seam lane contracts for transition diagnostics
  - tests
- `crates/viewer_app/src/main.rs`
  - mapping path from connection summary to snapshot
  - app-level deterministic mapping tests
- `crates/viewer_ui/src/lib.rs` (if touched)
  - diagnostics rendering for new transition states/reasons
  - tests
- `docs/RESEARCH/post_amc_bootstrap_boundary_map.md` (only if new message observations are required)

## Boundary check
- `viewer_net` owns wire observation, timing windows, and bounded counters.
- `viewer_grid` owns transition semantic meaning/reason mapping.
- `viewer_core` owns shared typed continuity/diagnostic contracts.
- `viewer_app` remains orchestration/mapping only.
- `viewer_ui` remains display-only and cannot own policy/state transitions.
- No seam-owned role lifecycle outside seam apply path.

## Step sequence
1. Define N11 transition diagnostic contract
   - introduce bounded reason enum(s)
   - define staleness/degraded bands and caps
   - document invariants in code comments + tests.
2. Implement `viewer_net` bounded hardening
   - enforce deterministic transition age windows and bounded counters
   - guard against out-of-order or repeated control packets
   - classify stale transition state without panics.
3. Add `viewer_grid` semantic classifier
   - map raw transition observations to policy-level reason categories
   - keep mapping explicit and unit-tested.
4. Extend `viewer_core` snapshot contracts
   - add serde-defaulted fields
   - preserve backward compatibility with existing snapshot file fallback.
5. Wire `viewer_app` mapping
   - map all new summary fields into snapshot
   - preserve dirty-only apply semantics and avoid additional per-frame churn.
6. Extend diagnostics presentation
   - render stable, low-noise transition status lines
   - include degraded reason where present.
7. Add tests
   - `viewer_net`: phase progression, out-of-order handling, stale-window classification
   - `viewer_grid`: reason classifier mapping
   - `viewer_core`: snapshot defaults + seam mapping stability
   - `viewer_app`: mapping correctness and dirty-apply guard behavior
   - `viewer_ui` (if touched): deterministic display helper tests.
8. Runtime verification
   - offline smoke (snapshot fallback)
   - bounded live run (if credentials available) to confirm degraded/healthy transitions are distinguishable.
9. Continuity closeout
   - review/report updates
   - handoff with explicit next step.

## Validation plan
- Always:
  - `cargo fmt --all -- --check`
  - `cargo check --workspace`
- Targeted:
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_grid`
  - `cargo test -p viewer_core`
  - `cargo test -p viewer_app`
  - `cargo test -p viewer_ui` (if touched)
- Broader:
  - `cargo test --workspace` (material cross-crate behavior change)
- Runtime smoke:
  - `VIEWER_APP_LIVE_STARTUP=off cargo run -p viewer_app`
  - Optional bounded live smoke: `cargo run -p viewer_app` with configured `VIEWER_LOGIN_*`

## Risks and open questions
- Risk: degraded-state classification can become noisy if thresholds are not bounded and deterministic.
- Risk: accidental semantic drift into `viewer_net` if reason mapping is implemented in transport layer.
- Risk: over-surfacing diagnostics may reduce operator signal-to-noise.
- Open question: whether degraded-state reason should include coarse confidence score now or be deferred.
- Open question: whether scene proxy lane for transition diagnostics is needed, or diagnostics panel-only is sufficient.

## Deferred-too-early candidates captured
- Teleport destination-session orchestration parity (deferred; too broad for N11 hardening scope).
- Multi-hop region transition prediction beyond bounded continuity state (deferred).

## Learnings pre-check
- `L03`: seam-owned lifecycle must remain exclusive to seam apply.
- `L04`: dirty-only apply must remain intact.
- `L05`: unknown traffic remains diagnostic signal; do not suppress.
- `L06`: enforce `viewer_net`/`viewer_grid` ownership split.
- `L07`: do not expose sensitive session data in snapshot diagnostics.
- `L10`: message IDs must come from message template/reference.
- `L11`: handle sparse/missing transition-related payload parts gracefully.

## Completion criteria
- Typed transition hardening/diagnostics contract exists and is bounded.
- Transition degraded/healthy states are explicitly classified and surfaced.
- No boundary violations between transport/policy/app/ui layers.
- Validation commands pass (or blockers are explicitly documented).
- Continuity artifacts updated with exact current state and next step.

