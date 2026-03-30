# Review: PLAN_OBJECT_INGRESS_AGENT_UPDATE_CADENCE_2026-03-29

## Verdict
Approved.

## Architecture and Boundary Fit
- The plan keeps LLUDP send mechanics in `viewer_net` and cadence scheduling in `viewer_app`.
- It avoids reopening the earlier socket-lifecycle fixes and does not push camera/render ownership across crate boundaries.

## Correctness Concerns
- Reuse the existing active `SocialCircuit`; the cadence must not bind fresh sockets or resend handshake setup.
- Keep startup throttle behavior intact while making recurring `AgentUpdate` explicit and testable.
- Do not combine this slice with speculative `PacketAck` / `CompletePingCheck` reply work; that already proved fragile.

## Modularity and Maintainability Concerns
- Prefer a small dedicated `AgentUpdate` send helper over threading startup-only helpers through more call sites.
- Keep cadence state explicit in the worker loop so reopen/connected behavior remains easy to reason about.

## Validation Adequacy
- `cargo fmt --all`, `cargo check -p viewer_net -p viewer_app`, targeted tests for both crates, and a bounded connected capture are appropriate for this slice.

## Risks and Open Questions
- If live object ingress is still zero afterward, the next repair should move to protocol-accurate ping/reliability or another specific control message, not more socket/orchestration churn.
- Bounded worker-side interest-state values are acceptable here, but they are not a substitute for eventual full camera/control parity.

## Learnings Delta Verdict
- none for the plan itself; implementation should update/add based on the connected result.

## Required Revisions or Approval Status
Approved to implement as written.
