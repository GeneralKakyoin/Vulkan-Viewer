# Review: Plan World Object Feed True Placement + Scale (2026-04-03)

## Verdict
Approved.

## Architecture and boundary fit
- The plan keeps transform ownership in `viewer_core`, which is the correct boundary.
- No crate-boundary or interface drift is introduced.

## Correctness concerns
- Axis mapping must be explicit and tested (`X,Y,Z` decoded -> `X,Z,Y` world) to avoid silent swapped-height bugs.
- Missing-position fallback should stay deterministic so non-decoded objects remain visible.

## Modularity and maintainability concerns
- Keep the change localized to the transform helper and focused tests.
- Avoid introducing new scene-policy branching outside this helper.

## Validation adequacy
- `fmt`, `check -p viewer_core`, and `test -p viewer_core` are sufficient for this bounded slice.

## Risks and open questions
- Camera framing in existing smoke captures may shift because objects are no longer compressed into a tiny cluster.

## Learnings delta verdict
- `none`
- Reason: this slice applies existing `L76` and does not yet reveal a new durable trap.

## Required revisions or approval status
- Approved as written.
