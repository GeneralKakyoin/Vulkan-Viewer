# Review: Plan U04 Core Usability and Workflow Shell Alignment

## Verdict
**Approved**

The plan is well-structured, respects the established crate boundaries, and addresses a clear need for UX consolidation as the rendering and networking tracks mature. The "pinning" of session status contracts and profile age formatting provides the necessary technical detail to ensure a consistent implementation.

## Architecture and Boundary Fit
- **Excellent alignment.** Moving the shared UX status types (`SessionUxStatus`, `SessionUxReason`) to `viewer_core` is the correct way to handle cross-crate visibility without introducing a circular dependency or leaking `viewer_app` internals into `viewer_ui`.
- **Constraint validation.** The plan explicitly reinforces the invariant that `viewer_ui` must not depend on `viewer_app` types, which is critical for maintaining the repo's architectural integrity.
- **Data flow.** The flow of `worker state -> app mapping -> UI rendering` is clean and keeps the UI predominantly as a presentation layer.

## Correctness Concerns
- **Clock Skew:** The plan correctly identifies and handles potential clock skew in profile age computation.
- **Status Mapping:** The derivation of `Reconnecting` from existing worker loop states (where `LiveStartupStatus` might be `Starting` but the loop is an explicit retry) is a valid UX-only mapping choice that avoids unnecessary churn in the underlying state machine for now.
- **Deterministic Formatting:** The range-based duration formatting (1m, 1h, etc.) is deterministic and avoids locale-specific complexities, which is preferred for this utility-style UI.

## Modularity and Maintainability Concerns
- **Consolidation:** Regrouping fragmented debug windows into a "Diagnostics" panel should reduce visual noise and improve maintainability by providing a logical place for future observation tools.
- **Filtering:** Minimal relay filtering is introduced as a pure presentation layer improvement. This should stay simple to avoid performance regressions in log-heavy scenarios.

## Validation Adequacy
- The ladder of `fmt` -> `check` -> `targeted tests` -> `offline smoke` is appropriate for a UI-heavy milestone.
- **Logic Tests:** The requirement for unit tests on helper functions (status chips, freshness math) ensures that the core logic is verified even without automated UI testing.

## Risks and Open Questions
- **Discoverability:** As noted in the plan, regrouping windows risks hiding previously always-visible debug info. Ensure the "Diagnostics" and "Session" sections are prominent enough that developers don't lose sight of the "Performance" metrics they rely on. 
- **Mapping Logic:** Ensure the `viewer_app` mapping explicitly handles the transition from `Starting` to `Reconnecting` based on the internal reconnect counter, as this makes the UI feel significantly more "alive" and honest about what the network stack is doing.

## Required Revisions or Approval Status
- **No revisions required.** The plan is implementation-ready.

## Continuity Updates
- Once execution starts, ensure the `DEFERRED_FEATURES.md` is updated with the items captured in this plan's "Deferred" section.
