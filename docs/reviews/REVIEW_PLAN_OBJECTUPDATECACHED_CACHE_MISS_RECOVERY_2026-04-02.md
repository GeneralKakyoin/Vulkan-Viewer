# Review: Plan ObjectUpdateCached Cache-Miss Recovery (2026-04-02)

## Verdict
- Approved.

## Architecture / Boundary Fit
- Fits existing `viewer_net` transport boundary.
- Does not require crate ownership changes.

## Correctness Concerns
- Must use correct LLUDP medium message shape for `RequestMultipleObjects`.
- Must avoid repeated unbounded resend of the same local IDs.

## Modularity / Maintainability
- Queue + flush model is consistent with existing pending-ACK handling.
- Keep encoder isolated as a dedicated helper.

## Validation Adequacy
- Planned checks are sufficient for this slice:
  - fmt/check/tests
  - bounded live runtime evidence

## Risks / Open Questions
- Live windows may still not surface mesh UUIDs even with request recovery.
- Rate/volume should remain bounded by dedupe and chunking.

## Learnings Delta Verdict
- none: applying existing learnings (especially L74) rather than introducing a new durable lesson at plan time.

## Required Revisions
- None.
