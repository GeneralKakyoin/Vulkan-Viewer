# Review: Plan Object Ingress Post-EnableSimulator Seed-Cap Follow-Up (2026-03-30)

## Verdict
Approved.

## Architecture and boundary fit
- The plan keeps EventQueue extraction and seed-cap transport in `viewer_net`.
- The runtime follow-up policy and evidence relay remain in `viewer_app`.
- No crate-boundary or UI/render ownership drift is introduced.

## Correctness concerns
- The implementation should distinguish between `EnableSimulator` port follow-up and `EstablishAgentCommunication` seed-cap follow-up; they are related but not equivalent.
- Firestorm evidence should be used only to justify the next capability step, not as a reason to mirror the full region-cap stack.
- If arbitrary seed-cap fetch support is added, it should reuse the existing seed-cap parsing path rather than duplicating request/parse logic.

## Modularity and maintainability concerns
- Prefer a small reusable helper for "fetch seed capabilities from explicit URL" over hard-coding child-region logic into the app worker.
- Keep any new follow-up bounded and de-duplicated so the worker does not turn into open-ended multi-region orchestration.

## Validation adequacy
- `cargo fmt`, targeted `cargo check`, targeted tests for `viewer_net` and `viewer_app`, and one bounded connected run are adequate for this slice.
- The connected run must confirm what capability inventory is actually returned after the new request/follow-up.

## Risks and open questions
- `EstablishAgentCommunication` may still be absent on the current path even after the capability request changes.
- `RegionObjects` and `InterestList` may be present as capabilities without immediately yielding object data.
- `UntrustedSimulatorMessage` is a region capability lane, but it is not itself proof of object-ingress success.

## Learnings delta verdict
- `none`
- This is a plan review; the implementation result will determine whether a new durable learning is warranted.

## Required revisions or approval status
- Approved to implement as written.
