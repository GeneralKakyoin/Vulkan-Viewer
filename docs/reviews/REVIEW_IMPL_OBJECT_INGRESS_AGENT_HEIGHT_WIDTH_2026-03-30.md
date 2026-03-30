# Review: OBJECT_INGRESS_AGENT_HEIGHT_WIDTH_2026-03-30

## Verdict
Accepted.

## Architecture and Boundary Fit
- The change keeps LLUDP control-packet construction in `viewer_net`.
- No new worker/UI/render ownership boundaries were introduced.
- Using a bounded startup default size avoids widening this slice into live window-metric plumbing.

## Correctness Concerns
- The `AgentHeightWidth` body shape matches the Firestorm source/template evidence:
  - `AgentID`
  - `SessionID`
  - `CircuitCode`
  - `GenCounter`
  - `Height`
  - `Width`
- Startup ordering remained bounded and explicit: `AgentThrottle` -> `AgentHeightWidth` -> `AgentUpdate`.

## Modularity and Maintainability Concerns
- The new helper is isolated and test-covered.
- The slice stayed within the staged control-block plan and did not silently promote later items.

## Validation Adequacy
- Formatting, targeted check, targeted tests, and a bounded connected run were all performed.
- The live command timed out at the tool layer before self-exit, but the produced log clearly showed the relevant behavior and the spawned process was cleaned up afterward.

## Risks and Open Questions
- The remaining blocker is still unresolved.
- The next smallest justified promotion is still a plan decision between `AgentAnimation`, `SetAlwaysRun`, or a narrower ACK-timing slice.

## Learnings Delta Verdict
- add: the live run produced a durable lesson that `AgentHeightWidth` alone is not sufficient.

## Required Revisions or Approval Status
- Approved as implemented.
