# Review: Plan Endpoint-Aware LLUDP Handshake Re-prime + Reply Targeting (2026-04-03)

## Verdict
- Approved.

## Architecture / Boundary Fit
- Stays within existing `viewer_net` transport responsibilities and `viewer_app` orchestration.
- No crate ownership violations.

## Correctness Concerns
- Ensure endpoint override for `RegionHandshakeReply` remains deterministic and fallback-safe.
- Ensure re-prime bundle is bounded and deduplicates endpoints.
- Ensure strict-mode reprime path does not send unbounded duplicate startup traffic.

## Modularity / Maintainability
- Prefer explicit helper methods for endpoint selection and sender tracking.
- Keep app loop changes minimal by calling one connection-level API.

## Validation Adequacy
- Targeted `viewer_net` tests for reply targeting and reprime bundle + existing startup tests are adequate.
- `viewer_app` test run should confirm config/runtime integration remains intact.

## Risks / Open Questions
- Region routes may still differ in acceptance policy even after endpoint-aware reprime.

## Learnings Delta Verdict
- none (pre-implementation).

## Required Revisions
- None.
