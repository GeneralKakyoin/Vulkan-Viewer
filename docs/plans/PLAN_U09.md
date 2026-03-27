# Plan: U09 Workflow Depth and Product Usability Expansion

## Objective
Improve daily operator workflow depth so continuity, avatar state, social activity, and diagnostics can be monitored and acted on clearly in-app, without changing subsystem ownership boundaries.

## Scope
- Expand diagnostics grouping for continuity + avatar/attachment state visibility.
- Improve social/profile workflow clarity (thread freshness, refresh affordances, stale/partial-state messaging).
- Add bounded operator shortcuts for common actions (inspect/focus/toggle diagnostics) through existing app/UI action paths.
- Tighten reconnect/failure-state presentation for partial world and stale cache conditions.
- Lock U09 shortcut surface to this bounded set only:
  - `F1`: toggle diagnostics panel visibility
  - `F2`: focus continuity diagnostics section
  - `F3`: focus social/profile workspace section
- Lock profile refresh policy for U09:
  - explicit user-triggered refresh only (no automatic refresh loop)
  - per-avatar refresh cooldown of 10 seconds for repeated clicks

## Current known state
- U04 established a coherent shell with session/social/diagnostics grouping and standardized session states.
- N07 added continuity diagnostics fields and seam-backed continuity signals.
- Profile/social caching paths exist, including freshness indicators, but operator workflows for partial-state/reconnect handling remain shallow.
- Existing diagnostics lines are dense; richer runtime state from N07/R08 increases workflow complexity without additional structure.

## Files and components touched
- `crates/viewer_ui/src/lib.rs`
  - diagnostics grouping and workflow affordances
  - shortcut affordances and explicit failure-state messaging
- `crates/viewer_app/src/main.rs`
  - action routing for new bounded workflow commands
  - additional mapping for continuity/avatar/social status text used by UI
- `crates/viewer_core/src/lib.rs` (if needed)
  - additive source-of-truth state fields/enums for UI-facing workflow status
- tests in `viewer_ui`, `viewer_app`, and `viewer_core` for deterministic workflow behavior

## Boundary check
- `viewer_ui` owns rendering and user action emission only.
- `viewer_app` owns orchestration/dispatch for UI-originated commands.
- `viewer_core` remains source of truth for session/social/avatar/continuity state.
- No transport logic moves into UI/app presentation paths.
- No renderer policy ownership shifts into UI workflow code.
- UI callbacks must not mutate source-of-truth state directly; all state transitions route through app/core command pathways.

## Step sequence
1. Define U09 workflow slices and status groupings (continuity, avatar/attachment, social/profile, diagnostics relay).
2. Add/adjust additive UI-facing status types in `viewer_core` only where source-of-truth expansion is required.
3. Implement grouped diagnostics layout in `viewer_ui` with explicit stale/partial/reconnect messaging.
4. Add bounded workflow shortcuts (`F1`, `F2`, `F3`) in `viewer_ui` and route actions through `viewer_app` command handling.
5. Tighten social/profile workflow messaging and implement explicit-only refresh affordance with 10-second cooldown.
6. Add targeted tests for:
   - deterministic status label rendering
   - shortcut command routing
   - stale/failure-state presentation rules
   - refresh cooldown behavior and no-auto-refresh invariants
7. Run validation ladder, perform runtime smoke, and close with review/report/continuity updates.

## Validation plan
- Always:
  - `cargo fmt --all -- --check`
  - `cargo check --workspace`
- Targeted:
  - `cargo test -p viewer_ui`
  - `cargo test -p viewer_app`
  - `cargo test -p viewer_core` (if touched)
- Broader:
  - `cargo test --workspace` when workflow changes span crates materially
- Runtime smoke:
  - `VIEWER_APP_LIVE_STARTUP=off cargo run -p viewer_app`
  - `VIEWER_APP_LIVE_STARTUP=off STRESS_TEST=screenshot cargo run -p viewer_app` for deterministic UI/scene overlay capture checks

## Risks and open questions
- Risk: workflow shortcuts could bypass existing app orchestration if routed directly; must preserve UI->app command boundary.
- Risk: status expansion can become noisy; group by operator task and keep labels deterministic.
- Open question: whether continuity section focus (`F2`) should also pin a temporary visual highlight for accessibility in U09, or remain navigation-only.

## Deferred-too-early candidates captured
- In-app credential entry and secrets UX redesign (defer beyond U09 unless explicit security policy is approved).
- Full diagnostics export bundle tooling with redaction pipeline (defer to tooling/ops milestone).
- Advanced docking/layout persistence model beyond bounded panel-state save/restore (defer if broader UI framework changes are required).

## Learnings pre-check
- `L03`: Do not create/remove seam-owned roles from UI workflows.
- `L04`: Keep dirty-only scene/snapshot apply semantics while exposing richer workflow labels.
- `L06`: Preserve `viewer_net` vs `viewer_grid` semantic boundary; U09 should consume existing typed state only.
- `L07`: Do not surface sensitive session data in UI-facing snapshot/workflow fields.
- `L12`: Maintain frame-level spatial sync assumptions when adding inspect/focus workflow hooks.

## Completion criteria
- Operators can complete bounded continuity + social/profile + diagnostics workflows with explicit stable status/failure messaging.
- UI action shortcuts route through app orchestration, with no ownership boundary violations.
- Workflow status rendering is deterministic across repeated offline runs.
- Required validation commands pass, or blockers are explicitly documented in review/report artifacts.
- Continuity docs are updated with U09 state and precise next step.
