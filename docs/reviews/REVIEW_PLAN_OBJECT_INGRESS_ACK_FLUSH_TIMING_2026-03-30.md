# Review: PLAN_OBJECT_INGRESS_ACK_FLUSH_TIMING_2026-03-30

## Verdict
Approved.

## Architecture and Boundary Fit
- The plan keeps ACK/control mechanics in `viewer_net` and bounded trigger timing in `viewer_app`.
- It does not widen into renderer, asset, UI, or `viewer_grid` ownership.

## Correctness Concerns
- The plan correctly chooses ACK/control timing only after the forensics slice ruled out receive surfacing as the primary next branch.
- Keeping the next change to explicit `PacketAck` flush timing is safer than reviving the earlier broad reliability-reply experiment.

## Modularity and Maintainability Concerns
- The scope remains narrow and testable.
- Preserving the new forensic summaries is important so the next run remains interpretable.

## Validation Adequacy
- The validation ladder is appropriate for this transport-side change.
- Requiring a bounded connected run is necessary here because the question is timing-sensitive.

## Risks and Open Questions
- ACK flush timing may still not be the final blocker.
- If the queue drains cleanly and ingress is still zero, the next plan must stay evidence-led rather than widening immediately.

## Learnings Delta Verdict
- add: the completed forensics slice produced a new durable narrowing about receive surfacing versus ACK/control timing.

## Required Revisions or Approval Status
- Approved to implement as written.
