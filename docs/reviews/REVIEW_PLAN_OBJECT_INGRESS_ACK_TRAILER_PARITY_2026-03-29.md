# Review: PLAN_OBJECT_INGRESS_ACK_TRAILER_PARITY_2026-03-29

## Verdict
Approved.

## Architecture and Boundary Fit
- The plan stays inside `viewer_net` transport mechanics, which is the correct owner for LLUDP packet flags, ACK trailers, and reliable first-simulator send behavior.

## Correctness Concerns
- Queue ACKs only for inbound packets that actually carry the reliable flag.
- Do not queue or re-ack `PacketAck` packets themselves.
- Use the Firestorm-compatible ACK trailer layout exactly: appended network-order packet IDs followed by the trailing ACK-count byte.
- Any helper that slices message bodies must exclude ACK trailers before decoding variable-length payloads.

## Modularity and Maintainability Concerns
- Prefer one small packet-metadata helper reused by classification, observation, and decode paths rather than scattering ACK/trailer math through many functions.
- Keep ACK-trailer attachment centralized in the existing first-simulator send helper so later sends inherit the same transport behavior automatically.

## Validation Adequacy
- `cargo fmt --all`, `cargo check -p viewer_net -p viewer_app`, targeted `viewer_net` tests, `viewer_app` tests, and a bounded connected capture are appropriate.

## Risks and Open Questions
- If this does not restore object ingress, the next blocker is likely a different startup-interest or control-parity gap, not more socket work.
- Standalone `PacketAck`/ping reply behavior should remain out of scope unless the connected evidence after this slice still points there.

## Learnings Delta Verdict
- none expected by default; add or update only if the live capture produces a new durable protocol lesson.

## Required Revisions or Approval Status
- Approved to implement as written.
