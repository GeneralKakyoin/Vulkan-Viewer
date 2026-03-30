# Review: PLAN_STARTUP_PROTOCOL_PARITY_2026-03-29

## Verdict
Approved.

## Architecture and Boundary Fit
- The plan keeps LLUDP message construction in `viewer_net` and startup sequencing in `viewer_app`.
- It preserves the existing crate split and does not treat Firestorm as an architecture template.

## Correctness Concerns
- `RegionHandshakeReply` should only be sent in response to observed startup handshake traffic; do not fabricate unrelated region state.
- `AgentThrottle` and `AgentUpdate` should be bounded startup parity messages, not the start of an uncontrolled continuous movement subsystem.
- Reuse the already-active social circuit socket so the fix stays aligned with the earlier continuity repair.

## Modularity and Maintainability Concerns
- Prefer small encoding helpers and one startup orchestration helper over scattering packet construction across call sites.
- Keep packet-default constants explicit and documented by source so future parity work can refine them safely.

## Validation Adequacy
- `cargo fmt --all`, `cargo check -p viewer_net -p viewer_app`, targeted crate tests, and a bounded live capture are appropriate.

## Risks and Open Questions
- If `RegionHandshake` still never arrives after startup parity sends, the remaining blocker may be another first-simulator control message not yet instrumented.
- One-shot `AgentUpdate` may be sufficient for startup ingress but not for longer-term motion parity; that is acceptable for this bounded slice.

## Learnings Delta Verdict
- add or update depending on whether the live run confirms `RegionHandshakeReply` / startup parity as the gating factor.

## Required Revisions or Approval Status
Approved to implement as written.
