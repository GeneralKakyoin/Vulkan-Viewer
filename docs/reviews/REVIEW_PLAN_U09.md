# Review: PLAN_U09 Workflow Depth and Product Usability Expansion

## Verdict
Approved with required revisions.

## Architecture and Boundary Fit
- Plan maintains correct UI/app/core boundaries:
  - `viewer_ui` emits actions and renders status.
  - `viewer_app` routes/handles orchestration commands.
  - `viewer_core` remains source-of-truth for workflow state.
- It correctly avoids transport/render ownership drift and aligns with `L06`/`L07`.

## Correctness Concerns
- Scope is coherent, but two behavior details are currently too open for deterministic implementation:
  - default shortcut set is unspecified.
  - profile refresh behavior (manual-only vs bounded auto-refresh) is unspecified.
- These choices materially affect user workflow behavior and test expectations.

## Modularity and Maintainability Concerns
- The plan is modular and avoids broad redesign.
- Good use of additive status types in `viewer_core` only when necessary.
- Recommend explicitly banning direct state mutation in UI callbacks (all state transitions via app/core pathways only).

## Validation Adequacy
- Validation ladder is appropriate.
- Runtime smoke commands are suitable for deterministic UI verification.
- Add explicit targeted test requirement for reconnect/failure copy stability so regressions are caught as message/state contract changes.

## Risks and Open Questions
- Risk of UI noise is correctly identified.
- Open questions are valid but need pre-implementation locking to prevent scope drift.

## Learnings Delta Verdict
none — no new durable learning identified during plan review; current learnings set already constrains this plan well.

## Required Revisions or Approval Status
- Required revision 1: Define the exact bounded shortcut set included in U09.
- Required revision 2: Decide and document profile refresh policy (button-only or bounded auto-refresh, with interval/cap if auto).
- Required revision 3: Add an explicit rule that UI actions cannot mutate source-of-truth state directly; they must route through app/core command paths.
- Status after revisions: Approved for implementation.
