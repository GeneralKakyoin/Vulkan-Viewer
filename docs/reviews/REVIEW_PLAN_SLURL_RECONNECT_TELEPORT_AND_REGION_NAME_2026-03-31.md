# Review: Plan SLURL Reconnect Teleport And Current Region Name (2026-03-31)

## Verdict
Approved.

## Architecture and boundary fit
- The plan keeps reconnect policy, SLURL normalization, and worker command orchestration in `viewer_app`, which matches current ownership.
- `viewer_core` only receives a derived display-safe field.
- `viewer_ui` remains input/display-only and does not gain protocol logic.

## Correctness concerns
- Normalize supported SLURL forms into Firestorm-style login start strings rather than passing raw SLURLs through to login.
- Keep region-name surfacing tolerant of the current blocked `RegionHandshake` path by allowing bootstrap-derived fallback values.
- Reconnect logging should make it explicit that this is a reconnect teleport, not in-session teleport parity.

## Modularity and maintainability concerns
- Reuse the existing command lane and `UiActions` flow; do not introduce a second ad hoc teleport channel.
- Keep SLURL parsing in a small helper with tests rather than scattering string logic across the event loop and worker.

## Validation adequacy
- The proposed targeted tests plus offline/live runtime smoke are appropriate.
- Manual live validation is necessary for the reconnect path and should be called out clearly in the report.

## Risks and open questions
- Users may still want true in-session teleport later; that should remain explicitly deferred.
- Some maps-style variants may not be worth supporting in this first slice if they materially complicate parsing.

## Learnings delta verdict
- none
- This is a plan review; any durable learning depends on implementation outcome.

## Required revisions or approval status
- Approved as written.
