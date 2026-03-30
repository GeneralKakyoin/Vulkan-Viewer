# Review: OBJECT_INGRESS_SET_ALWAYS_RUN_2026-03-30

## Verdict
Accepted.

## Architecture and Boundary Fit
- The change keeps LLUDP control-packet construction in `viewer_net`.
- No new cross-thread or cross-crate ownership surface was introduced.
- The slice respects the staged control-block plan by promoting only `SetAlwaysRun`.

## Correctness Concerns
- The `SetAlwaysRun` body shape matches Firestorm/template evidence:
  - `AgentID`
  - `SessionID`
  - `AlwaysRun`
- The startup ordering stays bounded and explicit with `SetAlwaysRun` placed after `AgentUpdate`.
- The startup value is intentionally fixed to `false` for this bounded parity slice.

## Modularity and Maintainability Concerns
- The new helper is isolated and test-covered.
- The slice did not silently promote `AgentAnimation` or ACK scheduling.

## Validation Adequacy
- Formatting, targeted check, targeted tests, and a bounded connected run all completed successfully.
- The live run produced a clear behavioral answer: `SetAlwaysRun` is on-wire, but object ingress remains zero.

## Risks and Open Questions
- The remaining blocker is still unresolved.
- The next smallest justified promotion is now either:
  - `AgentAnimation`
  - or a narrower ACK-timing slice

## Learnings Delta Verdict
- add: the live result is a durable lesson that `SetAlwaysRun` alone is not sufficient.

## Required Revisions or Approval Status
- Approved as implemented.
