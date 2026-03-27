# Plan: U14 Workflow Resilience and Operator Recovery Controls

## Summary
Expand the operator workflow from visibility-only diagnostics to bounded recovery control loops, enabling reliable intervention during degraded transition/asset conditions while preserving strict UI/app/core ownership boundaries.

## Objective
- Provide clear recovery affordances for degraded transition and asset states.
- Improve degraded-state messaging quality and actionability.
- Keep UI as presentation/action-emission only, with all state transitions routed through app/core.
- Ensure deterministic behavior in recovery controls and cooldowns.

## Why now
- `N11` and `A13` introduce richer transition/asset runtime states that need bounded operator controls.
- Current workflow depth is improved (`U09`) but recovery operations remain sparse and manual.
- A bounded resilience milestone improves day-to-day operability before larger parity work.

## In scope
- `viewer_ui`:
  - Add bounded recovery controls (retry probe trigger, bounded cache refresh trigger, stale-indicator reset action where allowed).
  - Add stronger degraded-state callouts with explicit reason labels.
  - Add clear status grouping for transition health + asset source health.
- `viewer_app`:
  - Route U14 recovery actions to existing subsystem command paths.
  - Enforce cooldown/debounce where needed to prevent action flooding.
  - Keep deterministic action outcomes and explicit status acknowledgements.
- `viewer_core`:
  - Add or refine typed status/action result models for recovery workflows.
  - Preserve source-of-truth ownership and deterministic state transitions.
- `viewer_net` / `viewer_asset` (bounded touch only):
  - expose safe bounded command endpoints consumed via app orchestration.

## Out of scope
- Full product UX redesign.
- In-app credential/security architecture redesign.
- Non-bounded automated self-healing loops.
- Moving transport or asset policy ownership into UI.

## Current known state
- `U09` introduced workflow shortcuts and diagnostics grouping.
- `A10` introduced asset continuity metrics and diagnostics.
- Recovery actions exist in scattered forms but are not unified into explicit operator flows.
- UI boundary rules require all state transitions/actions to route through app/core.

## Files and components touched
- `crates/viewer_ui/src/lib.rs`
  - recovery controls and degraded-state panels/chips
  - action emission wiring
  - tests
- `crates/viewer_app/src/main.rs`
  - action dispatch handling and bounded cooldown/debounce guards
  - action result feedback mapping
  - tests
- `crates/viewer_core/src/lib.rs`
  - typed action/result and status contracts
  - tests
- `crates/viewer_net/src/lib.rs` and/or `crates/viewer_asset/src/*` (if minor command endpoints needed)
  - bounded command hooks only
  - tests

## Boundary check
- `viewer_ui` owns rendering + action emission only.
- `viewer_app` owns command dispatch/orchestration and cooldown policies.
- `viewer_core` owns typed status/action contracts.
- `viewer_net` and `viewer_asset` retain policy ownership; UI does not directly mutate them.
- No direct UI-to-net or UI-to-asset control paths.

## Step sequence
1. Define U14 recovery action contract
   - enumerate bounded operator actions
   - define typed result/acknowledgement model.
2. Add typed status/action fields in core
   - additive and deterministic.
3. Implement app command dispatch
   - route each UI action via app handlers
   - add cooldown/debounce/rate caps.
4. Implement UI controls + messaging
   - recovery buttons/shortcuts with clear disabled reasons
   - degraded-state callouts with action hints.
5. Integrate subsystem hooks (bounded)
   - invoke existing net/asset functionality where applicable.
6. Add tests
   - deterministic action gating/cooldown
   - action-result mapping and status transitions
   - UI display helper tests for degraded-state messages.
7. Runtime workflow smoke
   - offline degraded simulation paths
   - bounded live verification when available.
8. Closeout artifacts and continuity updates.

## Validation plan
- Always:
  - `cargo fmt --all -- --check`
  - `cargo check --workspace`
- Targeted:
  - `cargo test -p viewer_ui`
  - `cargo test -p viewer_app`
  - `cargo test -p viewer_core`
  - `cargo test -p viewer_net` / `cargo test -p viewer_asset` (if touched)
- Broader:
  - `cargo test --workspace` (if cross-crate behavior is materially changed)
- Runtime smoke:
  - `VIEWER_APP_LIVE_STARTUP=off cargo run -p viewer_app`
  - `VIEWER_APP_LIVE_STARTUP=off STRESS_TEST=screenshot VIEWER_TEST_SCREENSHOT_DIR=artifacts/screenshots_u14_smoke VIEWER_TEST_SCREENSHOT_EVERY_N_FRAMES=1 VIEWER_TEST_SCREENSHOT_MAX_FRAMES=2 cargo run -p viewer_app`

## Risks and open questions
- Risk: recovery controls can become noisy or unsafe if not tightly bounded.
- Risk: action feedback ambiguity can reduce operator trust.
- Risk: cooldown logic may conflict with existing U09 refresh policies if not unified.
- Open question: whether U14 should add keyboard shortcuts for recovery controls or keep controls panel-only.
- Open question: whether action audit trail belongs in runtime relay now or in a later ops milestone.

## Deferred-too-early candidates captured
- Full diagnostics bundle export/redaction pipeline.
- Workspace docking/persistent layout system with role-specific profiles.
- Automated recovery/autopilot loops beyond bounded operator-triggered controls.

## Learnings pre-check
- `L18`: egui window state updates must occur outside closures.
- `L04`: preserve dirty-only apply semantics while adding workflow control paths.
- `L03`: no seam-owned lifecycle mutation from UI flows.
- `L07`: avoid exposing sensitive session/capability information in workflow panels.
- `L19`: preserve deterministic behavior under repeated operator action bursts.

## Completion criteria
- U14 introduces bounded recovery controls with deterministic app dispatch.
- Degraded transition/asset states are clearly surfaced with actionable status.
- UI boundary discipline remains intact (no direct policy/state mutation paths).
- Validation commands pass (or blockers are explicitly documented).
- Continuity docs/reports updated with exact state and next step.

