# Review: PLAN_LLUDP_RELIABILITY_REPLIES_2026-03-29

## Verdict
Approved.

## Architecture and Boundary Fit
- The plan stays fully inside `viewer_net` transport mechanics, which is the correct ownership for LLUDP reliability and ping replies.

## Correctness Concerns
- Ack only true reliable inbound packets, and do not ack `PacketAck` packets themselves.
- `CompletePingCheck` must echo the received ping ID.
- Keep the implementation bounded to the active social-circuit receive path rather than introducing a broad packet scheduler.

## Modularity and Maintainability Concerns
- Prefer small helpers for packet-flag inspection and outbound control-packet encoding.
- Keep automatic replies close to the receive loops that own the active socket.

## Validation Adequacy
- `cargo fmt --all`, `cargo check -p viewer_net -p viewer_app`, targeted `viewer_net` tests, and a bounded live capture are appropriate.

## Risks and Open Questions
- If the live packet mix still stops at the same small control set, another specific simulator-control reply may still be missing.

## Learnings Delta Verdict
- update expected if live validation confirms LLUDP reliability replies were the missing transport parity.

## Required Revisions or Approval Status
Approved to implement as written.
