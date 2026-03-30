# Review: Plan Object Ingress Persistent EventQueueGet Adoption (2026-03-30)

## Verdict
Approved.

## Architecture and boundary fit
- The plan stays within `viewer_net` transport responsibility and `viewer_app` worker orchestration responsibility.
- It is appropriately narrower than a general simulator-host capability implementation.

## Correctness concerns
- The plan correctly treats persistent `EventQueueGet` as the next most justified branch because the investigation showed:
  - simulator-host `:12043` caps are present
  - the current viewer starts but does not complete a one-shot EventQueue poll in the bounded run
  - Firestorm uses a persistent long-poll consumer for `EventQueueGet`
- The implementation should preserve explicit ack/event diagnostics so the result remains decision-friendly if ingress is still blocked.

## Modularity and maintainability concerns
- Keep EventQueue state in one place; avoid scattering poll-loop state across multiple ad hoc app variables.
- Avoid folding unrelated capability or teleport behavior into this slice.

## Validation adequacy
- The requested checks and bounded connected run are adequate for this next behavior-change slice.

## Risks and open questions
- If persistent EventQueue succeeds but ingress remains blocked, the following branch will likely be a specific simulator-host capability invocation rather than more UDP parity.

## Learnings delta verdict
- `none`
- Reason: the review applies the new investigation finding but does not add a separate durable lesson beyond the report's learnings delta.

## Required revisions or approval status
- Approved as written.
