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
  - extend `UiActions` with explicit recovery commands:
    - `retry_continuity_probe: bool`
    - `refresh_visible_assets: bool`
    - `clear_recovery_banner: bool`
  - add "Recovery Controls" block in Diagnostics window with:
    - button labels
    - disabled reasons
    - last action result chip
  - add degraded-state messaging helper:
    - `fn recovery_hint_lines(...) -> Vec<String>`
  - tests for action emission + deterministic hint formatting
- `crates/viewer_app/src/main.rs`
  - add bounded action gate constants:
    - `RECOVERY_PROBE_COOLDOWN_MS`
    - `RECOVERY_ASSET_REFRESH_COOLDOWN_MS`
  - add state in `AppState`:
    - last action timestamps
    - `last_recovery_result: Option<viewer_core::RecoveryActionResult>`
  - route new `UiActions` fields to subsystem commands
  - map execution result to UI-facing acknowledgement payload
  - tests for cooldown and repeated input behavior
- `crates/viewer_core/src/lib.rs`
  - add typed contracts:
    - `enum RecoveryAction`
    - `enum RecoveryResultCode { Accepted, CooldownActive, Unavailable, Failed }`
    - `struct RecoveryActionResult { action: RecoveryAction, code: RecoveryResultCode, detail: Option<String>, cooldown_remaining_ms: Option<u64> }`
  - add serializer-safe defaults where needed
  - tests for deterministic serialization and default values
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
1. **Define canonical recovery actions (`viewer_core`)**
   - Add `RecoveryAction` variants:
     - `RetryContinuityProbe`
     - `RefreshVisibleAssets`
     - `ClearRecoveryBanner`
   - Add `RecoveryActionResult` contract with result code + optional cooldown remaining.
2. **Core tests first**
   - Add serde round-trip test for `RecoveryActionResult`.
   - Add default test ensuring absent detail/cooldown fields serialize safely.
3. **App state plumbing (`viewer_app`)**
   - Extend `AppState` with:
     - `last_probe_retry_ms: Option<u64>`
     - `last_asset_refresh_ms: Option<u64>`
     - `last_recovery_result: Option<RecoveryActionResult>`
   - Keep state local to app orchestration; UI remains stateless emitter.
4. **App action dispatcher (`viewer_app`)**
   - Add helper:
     - `fn dispatch_recovery_action(&mut self, action: RecoveryAction, now_ms: u64) -> RecoveryActionResult`
   - Implement deterministic gating:
     - if cooldown active => `CooldownActive` + remaining ms
     - if subsystem unavailable => `Unavailable`
     - on accepted command => `Accepted`
5. **Bounded subsystem hooks**
   - `RetryContinuityProbe` -> trigger existing continuity refresh/probe path (no new protocol behavior).
   - `RefreshVisibleAssets` -> reuse existing texture request path for currently visible IDs only.
   - `ClearRecoveryBanner` -> app-local UI acknowledgement reset only.
6. **UI action emission (`viewer_ui`)**
   - Extend `UiActions` and set fields when operator clicks buttons.
   - Show buttons disabled when app-reported state indicates cooldown/unavailable.
   - Display `last_recovery_result` chip text and optional countdown.
7. **UI messaging quality pass**
   - Add `recovery_hint_lines(...)` that maps degraded continuity + asset pressure signals to one-line operator guidance.
   - Keep messages deterministic and severity-ordered.
8. **Wire render input**
   - Add recovery status input fields to `RenderInput`:
     - `last_recovery_result: Option<&RecoveryActionResult>`
     - `can_retry_probe: bool`
     - `can_refresh_assets: bool`
   - Populate from `viewer_app` each frame.
9. **Tests**
   - `viewer_app`: cooldown math and result-code mapping.
   - `viewer_app`: repeated button spam does not bypass cooldown.
   - `viewer_ui`: button press emits correct `UiActions` flags.
   - `viewer_ui`: hint lines deterministic for degraded/stalled/healthy states.
10. **Runtime smoke**
   - Offline mode: force degraded-like state and verify all controls/responses.
   - Live mode (if available): verify accepted path and cooldown feedback.
11. **Continuity updates**
   - Report exact action set, cooldown constants, and verification outcomes in handoff/report/current-state.

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
