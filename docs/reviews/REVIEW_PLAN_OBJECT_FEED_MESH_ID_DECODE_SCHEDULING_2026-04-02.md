# Review: Plan Object Feed Mesh ID Decode + Scheduling 2026-04-02

## Verdict
Approved.

## Architecture And Boundary Fit
- Decode/state in `viewer_net`, mapping/scheduling in `viewer_app`, additive contract in `viewer_core`.
- No boundary erosion into `viewer_grid`.

## Correctness Concerns
- Extra-param parsing must enforce count/type/length bounds.
- Sculpt entries should only produce mesh IDs when sculpt type denotes mesh.
- Compressed payload handling must remain fail-safe on malformed data.

## Modularity And Maintainability Concerns
- Keep mesh extraction isolated in helper functions.
- Avoid coupling request scheduling to protocol parser internals.

## Validation Adequacy
- Proposed `fmt` + `check` + targeted crate tests are adequate.

## Risks And Open Questions
- CDN/capability auth failures may still block live bytes despite successful decode.

## Learnings Delta Verdict
- none
- Reason: plan-phase review only.

## Required Revisions Or Approval Status
- Approved as scoped.
