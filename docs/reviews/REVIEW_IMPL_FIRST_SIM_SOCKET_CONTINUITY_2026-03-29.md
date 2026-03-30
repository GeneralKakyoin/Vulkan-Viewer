# Review: IMPLEMENTATION_FIRST_SIM_SOCKET_CONTINUITY_2026-03-29

## Verdict
Approved with noted validation blocker outside the touched crate.

## Architecture and Boundary Fit
- The change stays inside `viewer_net`, which owns first-simulator UDP socket lifecycle and handshake transport.
- No `viewer_app` orchestration, `viewer_grid` semantics, or cross-crate ownership changes were introduced.

## Correctness Concerns
- Retained probe sockets are only reused after the handshake reaches `AgentMovementComplete`.
- The retained socket is cleared on login reset and disconnect, which avoids stale reuse across sessions.
- `open_social_circuit()` consumes the retained socket once, then falls back to the existing fresh-socket handshake path.

## Modularity and Maintainability Concerns
- The reuse behavior is localized to the social-circuit open path rather than spread across unrelated chat helpers.
- Added tests exercise both retained and fallback behavior at the datagram/socket level, which should make future protocol regressions easier to spot.

## Validation Adequacy
- `cargo fmt --all`: adequate formatting coverage for this slice.
- `cargo check -p viewer_net`: confirms the touched crate compiles.
- `cargo test -p viewer_net`: passes, including the new socket lifecycle tests.
- `cargo check -p viewer_net -p viewer_app`: blocked by an unrelated existing `viewer_app` compile error, so broader app validation remains incomplete.

## Risks and Open Questions
- Live runtime verification still needs to confirm that object ingress actually becomes non-zero once `viewer_app` is buildable again.
- If live object ingress remains zero, this slice should be treated as a necessary transport fix rather than the full RCA resolution.

## Learnings Delta Verdict
- add: L24 captures the durable socket-continuity constraint for first-simulator startup traffic.

## Required Revisions or Approval Status
Approved for this bounded `viewer_net` slice. Follow-up should focus on clearing the unrelated app compile blocker and rerunning live validation.
