# Review: OBJECT_INGRESS_AGENT_ANIMATION_2026-03-30

## Verdict
Accepted.

## Architecture and Boundary Fit
- The change keeps LLUDP control-packet construction in `viewer_net`.
- No new UI, render, asset, or cross-thread ownership surface was introduced.
- The slice respects the staged control-block plan by promoting only `AgentAnimation`.

## Correctness Concerns
- The packet shape matches the observed working Firestorm startup packet:
  - one animation block
  - `ANIM_AGENT_DO_NOT_DISTURB`
  - `StartAnim = false`
  - one empty physical-avatar-event block
- Startup ordering remains bounded and explicit with `AgentAnimation` between `AgentUpdate` and `SetAlwaysRun`.

## Modularity and Maintainability Concerns
- The new helper is isolated and test-covered.
- The slice did not silently widen into broader animation-state parity or ACK scheduling.

## Validation Adequacy
- Formatting, targeted check, targeted tests, and a bounded connected run were all performed.
- The live command timed out at the tool layer before self-exit, but the produced log clearly showed the unchanged object-ingress result and the spawned process was cleaned up afterward.

## Risks and Open Questions
- The remaining blocker is still unresolved.
- The next smallest justified step is no longer another startup message; it is now a tighter ACK/control-reply investigation.

## Learnings Delta Verdict
- add: the live result is a durable lesson that the observed startup `AgentAnimation` packet alone is not sufficient.

## Required Revisions or Approval Status
- Approved as implemented.
