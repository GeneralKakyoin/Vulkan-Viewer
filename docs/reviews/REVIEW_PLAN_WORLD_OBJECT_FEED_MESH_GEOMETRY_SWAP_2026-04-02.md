# Review: Plan World Object Feed Mesh Geometry Swap 2026-04-02

## Verdict
Approved.

## Architecture And Boundary Fit
- Keeps scene geometry ownership in `viewer_core`.
- Does not move protocol parsing concerns into app/core.

## Correctness Concerns
- Existing instances must update geometry when mesh ID appears/disappears.
- Spatial sync must remain correct after geometry replacement.

## Validation Adequacy
- Proposed targeted crate tests plus bounded live run are sufficient for this slice.

## Learnings Delta Verdict
- none
- Reason: plan-phase review only.
