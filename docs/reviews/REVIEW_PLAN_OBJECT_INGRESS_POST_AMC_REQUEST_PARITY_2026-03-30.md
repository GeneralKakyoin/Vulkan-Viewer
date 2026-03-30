# Review: PLAN_OBJECT_INGRESS_POST_AMC_REQUEST_PARITY_2026-03-30

## Verdict
Approved.

## Architecture and Boundary Fit
- The plan keeps LLUDP message construction and first-simulator classification inside `viewer_net`.
- It keeps orchestration of one-shot startup sends inside `viewer_app`.
- It does not cross into rendering, asset transport, or grid-meaning ownership.

## Correctness Concerns
- The three targeted requests must match message-template IDs and required blocks exactly.
- Startup wiring must remain one-shot and bounded; do not accidentally create a recurring scheduler for these requests.
- `LayerData` classification should stay classification-only in this slice and must not masquerade as object-feed success.

## Modularity and Maintainability Concerns
- Prefer dedicated helper functions for the three new requests rather than embedding ad-hoc packet assembly directly in the app worker path.
- Keep the new `LayerData` observability integrated into the existing first-simulator summary path so diagnostics do not fragment.

## Validation Adequacy
- `cargo fmt --all`, targeted `cargo check`, targeted crate tests, and a bounded connected run are appropriate for this slice.
- Re-checking live packet mix against the prior `improved.pcapng` evidence is the right validation target.

## Risks and Open Questions
- `MuteListRequest` and `MoneyBalanceRequest` may be correlated startup traffic rather than the true gating trigger.
- If ingress remains zero, the next plan will still need to consider `SetAlwaysRun`, `AgentAnimation`, or another narrower control/reply difference.

## Learnings Delta Verdict
- none if this remains planning-only; add only if implementation or live validation produces a durable lesson beyond L32.

## Required Revisions or Approval Status
- Approved to implement as written.
