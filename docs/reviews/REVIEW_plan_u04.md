# Review: PLAN_U04 Core Usability and Workflow Shell Alignment

## Verdict

Approved with minor required revisions (below) before implementation starts.

## Architecture and boundary fit

The plan respects the repo’s crate boundaries:

- `viewer_core` owns shared domain/status types and deterministic helpers.
- `viewer_app` owns orchestration and maps internal worker state into shared, UI-friendly types.
- `viewer_ui` remains presentation-only and emits `UiActions` without owning session logic.

Scope is appropriately bounded for a `U` milestone: workflow clarity and guardrails, not protocol expansion or renderer ownership changes.

## Correctness concerns

- **Session status mapping accuracy:** the plan must ensure `reconnecting` is not conflated with `failed`. If the current state machine cannot reliably distinguish these states, add the minimum additional internal signal in `viewer_app` (not `viewer_ui`) to keep the UX mapping correct.
- **Reason strings:** ensure any “failed reason” is sanitized/concise, stable, and does not include credentials, endpoints, or large error chains. Prefer a short enum-like reason key plus a short message when needed.
- **Profile freshness parity:** “fresh/stale/unknown” must be derived from the same TTL policy used by `viewer_app` fetch decisions. Avoid any UI-only TTL constants that could drift.

## Modularity and maintainability concerns

- Avoid passing ad-hoc formatted status strings across crates; standardize on a shared `viewer_core` type for session status and compute display labels/colors in `viewer_ui`.
- Keep UI regrouping incremental: reusing existing sections and introducing simple tabs/collapsers is preferable to large layout churn.
- Implement relay filtering with minimal persistent UI state and deterministic defaults to avoid “UI state explosion.”

## Validation adequacy

The validation plan is sufficient. Add/keep:

- `cargo fmt`, `cargo check`
- targeted: `cargo test -p viewer_ui`, `cargo test -p viewer_app`
- run `cargo test` if wiring changes span multiple crates materially
- runtime smoke with `VIEWER_APP_LIVE_STARTUP=off` to confirm offline behavior and guardrails

## Risks and open questions

- UI regrouping could hide diagnostics; mitigate via clear headings and sensible defaults.
- The plan’s “reconnecting” semantics depend on existing `viewer_app` state granularity; confirm early during implementation.

## Required revisions (must-do before implementation)

1. **Pin the session-status contract location and shape:** define a concrete `viewer_core` type for the UX-facing session status with a bounded set of variants and an optional bounded “reason” field.
2. **Specify deterministic age formatting for profile freshness:** document the exact age display rules (e.g., seconds/minutes rounding and caps) to avoid ad-hoc formatting spread across UI code.

Once the above revisions are made (as small edits to `docs/plans/PLAN_U04.md` or as explicit implementation notes in the execution report), implementation may begin.

## Approval status

Plan is approved to proceed after the two required revisions are satisfied.

