# Review: PLAN_SINGLE_SOCIAL_SOCKET_DISCIPLINE_2026-03-29

## Verdict
Approved.

## Architecture and Boundary Fit
- The plan keeps socket mechanics in `viewer_net` and live-worker orchestration in `viewer_app`.
- It is a bounded correction to existing ownership, not an architecture change.

## Correctness Concerns
- The fix must remove fresh-socket `UseCircuitCode` / `CompleteAgentMovement` sends from the continuous live worker loop.
- Nearby chat receive should still observe inbound payloads on the reused socket so diagnostics and object-feed accounting remain intact.
- Do not regress the explicit social-circuit reopen path.

## Modularity and Maintainability Concerns
- Prefer new helpers that take `&SocialCircuit` over threading bind strings through the live worker when no new socket should be created.
- Keep legacy fresh-socket helpers only where they remain intentionally bounded and non-looping.

## Validation Adequacy
- `cargo fmt --all`, `cargo check -p viewer_net -p viewer_app`, targeted tests, and a bounded connected run are appropriate.

## Risks and Open Questions
- If the live run still reports zero object ingress, the next work should focus on remaining control-message parity rather than more socket-orchestration changes.

## Learnings Delta Verdict
- update expected if live verification confirms repeated re-targeting was the remaining blocker.

## Required Revisions or Approval Status
Approved to implement as written.
