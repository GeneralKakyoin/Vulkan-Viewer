# Review: Plan Live Mesh Blocker Isolation (2026-04-03)

## Verdict
- Approved.

## Architecture / Boundary Fit
- The plan keeps capability interpretation in `viewer_grid`, transport observation in `viewer_net`, and relay/log presentation in `viewer_app`.
- No crate ownership drift is introduced.

## Correctness Concerns
- Diagnostics must stay additive only and avoid changing existing scheduler behavior except for ordered mesh-cap fallback.
- Mesh fetch attempt logging must not suppress the final success/failure relay already used by operators.

## Modularity / Maintainability
- Cap metadata should be added via a dedicated request-candidate struct rather than overloading plain URL lists.
- Transport attempt capture should remain reusable and bounded.

## Validation Adequacy
- `fmt`, `check`, full touched-crate tests, and bounded runtime smoke are appropriate.
- Live verification is still necessary for final blocker classification, but only when `VIEWER_LOGIN_*` env is available.

## Risks / Open Questions
- The slice may successfully isolate the blocker without fully unblocking live mesh bytes.
- Route/session authorization may still dominate the result even after cap fallback is fixed.

## Learnings Delta Verdict
- none.

## Required Revisions
- None.
