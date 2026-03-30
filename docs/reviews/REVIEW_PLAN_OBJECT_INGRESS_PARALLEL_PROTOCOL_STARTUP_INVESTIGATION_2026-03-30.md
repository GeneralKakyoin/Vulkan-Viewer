# Review: Plan Object Ingress Parallel Protocol Startup Investigation (2026-03-30)

## Verdict
Approved.

## Architecture and boundary fit
- The plan keeps behavior changes out of scope and limits code changes to diagnostics/classification in `viewer_net` and relay/transcript emission in `viewer_app`.
- It respects the existing crate split:
  - `viewer_net` for protocol and URL-family understanding
  - `viewer_app` for live-worker sequencing and operator-visible relay lines
- Firestorm is used correctly as protocol/timing reference only.

## Correctness concerns
- The plan correctly treats simulator-host `:12043` TLS traffic as a likely clue rather than automatic proof of the gating mechanism.
- It correctly avoids collapsing the investigation into CDN traffic volume alone.
- It keeps the already-known LLUDP blocker in scope by requiring the transcript to include `RegionHandshake`, `RegionHandshakeReply`, `CameraConstraint`, `GenericMessage`, and `ObjectUpdate*`.

## Modularity and maintainability concerns
- The plan should keep URL-family classification centralized in `viewer_net` so `viewer_app` does not grow ad hoc URL parsing logic.
- The transcript output should stay bounded and summary-oriented; avoid turning the live worker into an unbounded packet logger.

## Validation adequacy
- The planned validation ladder is adequate for a diagnostics slice:
  - fmt
  - targeted check
  - targeted tests
  - bounded connected run
  - external `tshark` extraction for the provided Firestorm pcap

## Risks and open questions
- The current one-shot `EventQueueGet` path is a strong suspect, but the report must still prove whether it is merely different from Firestorm or plausibly causal.
- If the bounded viewer run does not naturally exercise teleport/crossing markers, the report should say so explicitly instead of inferring teleport behavior from absence.

## Learnings delta verdict
- `none`
- Reason: the plan is constrained by existing learnings L32, L33, L38, and L39 but does not itself introduce a new durable lesson.

## Required revisions or approval status
- Approved as written.
- The resulting report must explicitly choose one next branch and reject the other candidate branches with evidence.
