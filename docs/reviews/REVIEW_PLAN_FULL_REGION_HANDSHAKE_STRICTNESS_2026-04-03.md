# Review: Plan Full Region Handshake Strictness + Recovery Nudge (2026-04-03)

## Verdict
- Approved.

## Architecture / Boundary Fit
- Changes stay inside existing crate responsibilities (`viewer_net` transport behavior and `viewer_app` orchestration).

## Correctness Concerns
- Must preserve one-shot reply semantics once handshake is observed.
- Nudge loop must be bounded and deterministic.

## Modularity / Maintainability
- Prefer small helper methods over ad-hoc receive-diagnostic scans in app loop.

## Validation Adequacy
- Targeted tests plus bounded live run are appropriate.

## Risks / Open Questions
- Strict mode may expose region/server variants that previously relied on fallback behavior.

## Learnings Delta Verdict
- none.

## Required Revisions
- None.
