# Review: PLAN_FIRST_SIM_SOCKET_CONTINUITY_2026-03-29

## Verdict
Approved.

## Architecture and Boundary Fit
- Keeps the fix inside `viewer_net`, which owns first-simulator UDP transport and socket lifecycle.
- Avoids widening scope into `viewer_app` orchestration or `viewer_grid` semantics.

## Correctness Concerns
- Retain the probe socket only after a completed first-simulator handshake; do not reuse a socket from a partial/failed probe.
- Ensure the retained socket is cleared on disconnect/login reset and consumed only once so later reopen flows do not accidentally share stale state.
- Preserve current fresh-socket behavior when no retained probe socket exists.

## Modularity and Maintainability Concerns
- Reuse should remain a narrow helper-path change rather than special-casing social logic across multiple call sites.
- Tests should prove behavior at the socket/datagram level so future protocol debugging can trust the retained-socket path.

## Validation Adequacy
- `cargo fmt --all`, `cargo check -p viewer_net -p viewer_app`, and `cargo test -p viewer_net` are appropriate for this transport-only slice.
- Live verification remains desirable afterward but is not required to land the bounded code fix.

## Risks and Open Questions
- If object ingress is still absent after this fix, the remaining issue is likely receive-budget or downstream decode handling.
- Nearby chat currently uses fresh sockets by design; this review accepts leaving that unchanged for now.

## Learnings Delta Verdict
- add if implementation confirms that first-simulator object bursts depend on socket continuity across probe and long-lived receive paths; otherwise none with reason.

## Required Revisions or Approval Status
Approved to implement as written.
