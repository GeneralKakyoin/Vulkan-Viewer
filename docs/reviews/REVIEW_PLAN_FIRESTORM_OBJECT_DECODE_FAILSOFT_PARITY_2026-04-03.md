# Review: Plan Firestorm Object Decode Fail-Soft Parity (2026-04-03)

## Verdict
- Approved.

## Architecture / Boundary Fit
- Keeps protocol parsing in `viewer_net` without crossing crate boundaries.

## Correctness Concerns
- Must avoid returning `None` for a whole packet when at least one valid object block is present.
- Must keep null/invalid mesh UUID filtering unchanged.

## Modularity / Maintainability
- Changes are localized to decode helpers and tests in one crate.
- No API churn required for sibling crates.

## Validation Adequacy
- Includes required fmt/check plus focused regression tests and full `viewer_net` suite.

## Risks / Open Questions
- Live blocked cases can still occur if upstream traffic lacks mesh-bearing extra params.

## Learnings Delta Verdict
- none — bounded hardening follows existing L73/L74/L77 guidance.

## Required Revisions / Approval Status
- No revisions required.
