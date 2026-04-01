# Review: Plan Object Ingress RegionObjects Position Shape Clarification (2026-03-31)

## Verdict
Approved.

## Architecture and boundary fit
- The plan stays inside `viewer_net` extraction and `viewer_app` relay output.
- It does not widen into UI, renderer, or LLUDP transport work.

## Correctness concerns
- Do not overstate the tuple-like `description` as a new schema; Firestorm evidence supports treating it as string content until stronger proof appears.
- Keep the new `position` labels evidence-based and explicitly separate "present but unparsed" from "missing".

## Modularity and maintainability concerns
- Prefer a small typed `position` inspection helper over ad hoc boolean flags scattered across the extraction path.
- Keep relay output bounded; this slice should clarify uncertainty, not turn the log into a raw dump.

## Validation adequacy
- Targeted checks, targeted tests, and one bounded live run are sufficient for this slice.

## Risks and open questions
- The live `RegionObjects` response may use a position shape not represented in the current tests.
- The result may show the current pathfinding-linkset lane truly lacks `position`, which would shift the next branch from extraction to protocol interpretation.

## Learnings delta verdict
- none
- This is a plan review; any durable learning depends on the implementation result.

## Required revisions or approval status
- Approved to implement as written.
