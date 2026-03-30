# Review: Startup Protocol And Social Socket Discipline (2026-03-29)

## Verdict
Partially approved.

## Architecture and Boundary Fit
- The kept code changes stay inside `viewer_net` transport mechanics and `viewer_app` live-worker orchestration.
- The nearby-chat fix is a clean boundary-preserving correction.

## Correctness Concerns
- The startup parity additions are plausible and bounded, but live evidence shows they were not sufficient to restore region/object ingress.
- The LLUDP reliability-reply experiment was correctly reverted after it regressed startup handshake completion.
- `RegionHandshake` is still absent in the best non-regressed live capture, so the next protocol change must be based on stronger upstream evidence.

## Modularity and Maintainability Concerns
- The new startup relay line with receive-kind counts materially improves debugging without crossing boundaries.
- Reusing `SocialCircuit` for nearby polling/sending removes a risky hidden side effect from the worker loop.

## Validation Adequacy
- Static checks and targeted tests were adequate and passed.
- Multiple bounded connected runs were captured, including a failed experiment that was not left in the codebase.

## Risks and Open Questions
- The next missing parity item is still unresolved.
- `TestMessage` and the inbound `PacketAck` remain the most suspicious surviving startup control clues.

## Learnings Delta Verdict
- add
  - live worker nearby-chat polling must not re-handshake fresh simulator sockets
  - unreliable LLUDP ack/ping parity guesses can regress startup and should be validated against stronger source evidence first

## Required Revisions or Approval Status
- Approved only for the kept startup/social-socket corrections.
- Further protocol changes require a new bounded plan backed by stronger upstream/protocol evidence.
