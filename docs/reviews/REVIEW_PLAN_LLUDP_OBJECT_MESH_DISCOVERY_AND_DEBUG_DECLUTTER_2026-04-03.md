# Review: Plan LLUDP Object Mesh Discovery + Network Debug Declutter 2026-04-03

## Verdict
Approved.

## Architecture and boundary fit
- Plan keeps ownership aligned: scheduler/debug-state shaping in `viewer_app`, presentation-only changes in `viewer_ui`.
- No protocol-semantic movement into UI or cross-crate boundary collapse.

## Correctness concerns
- Ensure fixture mesh IDs remain supported for deterministic verification while removing proxy-driven discovery.
- Ensure decoded mesh-ID path remains normalized/deduplicated and capped.

## Modularity and maintainability concerns
- Keep helper extraction functions narrow and test-covered.
- Avoid mixing scheduling policy with UI rendering code.

## Validation adequacy
- Proposed fmt/check + targeted tests are adequate for this bounded slice.

## Risks and open questions
- Reduced opportunistic fetch from visible-scene IDs is expected behavioral tightening; confirm this is reflected in report.

## Learnings delta verdict
- none: this is an application of existing learnings (L73/L74/L77), not a new durable lesson by itself unless live behavior reveals a new trap.

## Required revisions or approval status
- No revisions required.
