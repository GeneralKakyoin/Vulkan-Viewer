# Review: Plan Object Ingress RegionObjects Capability Probe (2026-03-30)

## Verdict
Approved.

## Architecture and boundary fit
- The plan keeps capability transport and parsing in `viewer_net` and runtime policy in `viewer_app`.
- It does not widen into UI, rendering, or broad pathfinding feature work.

## Correctness concerns
- The implementation should treat `RegionObjects` as a bounded probe, not as proof that LLUDP object ingress is solved.
- The probe should remain read-only and avoid speculative request bodies unless the live response shape proves they are needed.

## Modularity and maintainability concerns
- A small generic inspection summary is preferable to prematurely modeling the full pathfinding/linkset schema.
- Relay output should stay compact and evidence-focused.

## Validation adequacy
- Targeted `cargo check`, targeted tests, and one bounded connected run are appropriate.

## Risks and open questions
- `RegionObjects` may be available but unhelpful for the object-ingress milestone.
- The capability may return data that is object-related but too pathfinding-specific to be the long-term ingestion path.

## Learnings delta verdict
- `none`
- This is a plan review; any durable lesson depends on the live probe result.

## Required revisions or approval status
- Approved to implement as written.
