# Plan: N07 Region Continuity and World Scaling

## Summary
Implement a bounded, typed region-continuity path so `CrossedRegion` / `ConfirmEnableSimulator` traffic becomes actionable continuity state (active region, previous region, bounded neighbor summary, handoff phase) that flows `viewer_net` -> `LiveVisualSnapshot` -> seam lanes -> scene diagnostics without breaking crate boundaries.

## Objective
- Promote existing region-transition diagnostics from simple counters into a typed continuity contract.
- Add a bounded multi-region presence model that remains deterministic and serialization-safe.
- Keep seam ownership rules intact while exposing continuity state in scene-visible proxy payloads.
- Preserve diagnostics-first behavior for unknown traffic and out-of-order transition signals.

## Why now
- Roadmap dependency order in `docs/plans/PLAN.md` puts `N07` after `R05` + `A06`.
- Current code already classifies `CrossedRegion` and `ConfirmEnableSimulator` but only exposes aggregate counters; there is no explicit handoff phase model or bounded neighbor continuity surface.
- `N03` object feed is now available, so continuity breadth is the next bounded network-scale step.

## In scope
- `viewer_net`
  - Extend transition-control handling from count-only summaries to bounded typed continuity state.
  - Track a deterministic handoff phase model driven by `CrossedRegion`, `ConfirmEnableSimulator`, and first-simulator context.
  - Export bounded active/previous region identity hints and bounded neighbor summary hints where available.
- `viewer_core`
  - Extend `LiveVisualSnapshot` with typed continuity fields (handoff phase, active/previous region coords, bounded neighbor list/counters).
  - Extend `WorldObjectIngestionSeam` with continuity lane(s) for region continuity payload visualization.
  - Apply continuity payload lanes in `Scene::apply_world_object_ingestion_seam(...)` only (no out-of-band seam-owned create/remove).
- `viewer_app`
  - Map continuity state from `Connection::simulator_payload_decode_summary()` / transition summaries into snapshot fields.
  - Keep dirty-only seam/snapshot apply semantics unchanged.
- `viewer_ui` (bounded)
  - Surface continuity phase + region continuity summary in diagnostics panel if required for milestone verification.

## Out of scope
- Unlimited region streaming / full region graph management.
- Teleport pipeline parity and destination-region connect orchestration.
- Full parcel/estate/environment continuity semantics.
- Any `viewer_net`/`viewer_grid` boundary collapse or render-policy migration.

## Current known state
- `viewer_net` already classifies `CrossedRegion` / `ConfirmEnableSimulator` and stores observations (`RegionTransitionControlObservation`) plus count summaries.
- `viewer_app` currently maps only transition counters (`crossed_region`, `confirm_enable_simulator`, `region_transition_control_observations`) into `LiveVisualSnapshot`.
- `viewer_core::LiveVisualSnapshot` and seam lanes include first-region + traffic/object-feed diagnostics, but no explicit multi-region continuity model.
- `viewer_core::Scene` already enforces seam-owned lifecycle discipline; this must remain the sole create/remove path for seam-owned continuity roles.
- Firestorm references to use during implementation:
  - `reference/firestorm/scripts/messages/message_template.msg`
  - `reference/firestorm/indra/newview/llviewermessage.cpp`
  - `reference/firestorm/indra/newview/llstartup.cpp`

## Files and components touched
- `crates/viewer_net/src/lib.rs`
  - continuity types, bounded transition-state accumulator, deterministic export helpers, decode summary integration, tests.
- `crates/viewer_core/src/lib.rs`
  - `LiveVisualSnapshot` continuity fields (+ serde defaults), seam lane additions, continuity payload mapping, scene apply logic, tests.
- `crates/viewer_app/src/main.rs`
  - snapshot update bridge (`update_live_visual_from_connection`) for continuity fields, dirty-apply regression tests.
- `crates/viewer_ui/src/lib.rs` (if diagnostics rendering is needed for verification)
  - display-only continuity summary line(s), no source-of-truth ownership.
- `docs/RESEARCH/post_amc_bootstrap_boundary_map.md` (only if new protocol observations are captured while validating).

## Boundary check
- `viewer_net` owns transport/decode mechanics and continuity signal observation.
- `viewer_grid` remains the owner of grid semantics/policy interpretation; no transport logic moves there.
- `viewer_core` owns shared typed continuity contracts and seam lane types.
- `viewer_app` remains orchestration-only and must not directly create/remove seam-owned continuity roles.
- `viewer_ui` remains read-only presentation for continuity state.

## Step sequence
1. **Freeze N07 continuity contract (types + caps)**
   - Define handoff phase enum and bounded continuity payload structs in `viewer_net` and mirrored snapshot-safe equivalents in `viewer_core`.
   - Decide and codify hard caps (for example: max continuity observations retained, max neighbor summaries exported).
2. **Implement bounded continuity accumulator in `viewer_net`**
   - Extend transition-control observation path to compute phase transitions deterministically.
   - Track active region hint + previous region hint from existing decoded context without introducing unbounded history.
   - Keep malformed/insufficient payload handling non-panicking and counted.
3. **Export continuity through decode summary**
   - Add continuity fields to `SimulatorPayloadDecodeSummary` and export API with stable ordering.
   - Ensure summary reset/clear semantics remain correct across reconnect/restart paths.
4. **Bridge continuity into `LiveVisualSnapshot`**
   - Add continuity fields to snapshot with `#[serde(default)]` for backward-compatible fallback JSON loading.
   - Update `viewer_app::update_live_visual_from_connection(...)` to map continuity summary fields exactly once per tick.
5. **Add seam lanes for continuity payload**
   - Add new `WorldObjectIngestionLane` entries dedicated to region continuity summary and neighbor payload rows.
   - Extend `WorldObjectIngestionAdapter::adapt(...)` / `WorldObjectIngestionSeam::from_live_snapshot(...)` mapping with deterministic lane ordering.
6. **Apply continuity lanes in scene (seam-owned path only)**
   - Add/update/remove continuity proxy roles only inside `Scene::apply_world_object_ingestion_seam(...)`.
   - Reuse existing diagnostics conventions (bounded proxy markers + clear remove behavior when continuity data is absent).
7. **UI diagnostics exposure (bounded, read-only)**
   - Add continuity phase + active/previous region summary line(s) in diagnostics panel if milestone verification needs visual confirmation.
   - Keep UI display-only and derived from snapshot/core state.
8. **Targeted tests per crate**
   - `viewer_net`: phase progression tests (`none -> crossed -> confirm`), out-of-order handling, cap truncation determinism.
   - `viewer_core`: seam mapping tests for continuity lanes, seam-owned continuity role lifecycle tests.
   - `viewer_app`: bridge mapping test for new continuity fields and dirty-only apply guard behavior.
   - `viewer_ui` (if touched): formatting/presence tests for continuity status lines.
9. **Runtime bounded verification**
   - Offline smoke (`VIEWER_APP_LIVE_STARTUP=off`) confirms no regressions in fallback snapshot handling.
   - Live/bounded observation run confirms continuity counters/phase/lanes update without boundary violations.
10. **Closeout artifacts**
   - Write implementation review + report artifacts and update continuity docs after implementation, not during planning.

## Validation plan
- Always:
  - `cargo fmt --all -- --check`
  - `cargo check --workspace`
- Targeted:
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_core`
  - `cargo test -p viewer_app`
  - `cargo test -p viewer_ui` (if touched)
- Broader (cross-crate behavior change):
  - `cargo test --workspace`
- Runtime smoke:
  - `VIEWER_APP_LIVE_STARTUP=off cargo run -p viewer_app`
  - bounded live run with existing transition controls where credentials are available.

## Risks and open questions
- **Risk:** `HANDOFF.md` currently names "A07 (Lighting & PBR)" as next step while roadmap sequencing in `docs/plans/PLAN.md` specifies `N07` next; continuity docs need reconciliation before implementation starts.
- **Risk:** Transition signals can be sparse or out-of-order; phase machine must be resilient and never panic.
- **Risk:** Snapshot field growth can increase churn; caps + dirty-only guards must remain in place.
- **Open question:** Whether continuity neighbor summaries should be visual-only in N07 or also feed any object placement heuristics; this plan assumes visual/diagnostic-only.
- **Open question:** Exact source and fidelity of neighbor-region hints available from current decoded payloads; implementation must stay bounded to observed data.

## Deferred-too-early candidates captured
- Full teleport handoff parity (including destination-region socket/session orchestration) deferred beyond N07.
- Unlimited region graph streaming and retention policies deferred beyond N07.
- Parcel/estate/environment continuity semantics deferred until later rendering/parity milestones.

## Completion criteria
- `viewer_net` exports bounded typed continuity state (phase + active/previous/neighbor hints) derived from transition control signals.
- `viewer_app` maps continuity state into `LiveVisualSnapshot` deterministically with backward-compatible defaults.
- `viewer_core` seam includes continuity lanes and scene apply handles their lifecycle strictly inside seam-owned paths.
- Validation commands in this plan pass (or blockers are explicitly documented in execution artifacts).
- Continuity state is visible in diagnostics/scene behavior during bounded verification without crate-boundary violations.
