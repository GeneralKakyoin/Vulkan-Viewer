# Report: First-Simulator Socket Continuity Fix (2026-03-29)

## Summary of Implemented Work
- Added a retained probe socket field to `viewer_net::Connection`.
- Updated the first-simulator probe path to keep the probe socket alive after a successful `AgentMovementComplete`.
- Updated `open_social_circuit()` to consume that retained socket on the first long-lived social-circuit open instead of binding a second socket.
- Preserved existing fresh-socket behavior when no retained probe socket exists.
- Added targeted tests for retained-socket reuse and fresh-socket fallback.

## Files Changed
- `crates/viewer_net/src/lib.rs`
- `docs/plans/PLAN_FIRST_SIM_SOCKET_CONTINUITY_2026-03-29.md`
- `docs/reviews/REVIEW_PLAN_FIRST_SIM_SOCKET_CONTINUITY_2026-03-29.md`
- `docs/reviews/REVIEW_IMPL_FIRST_SIM_SOCKET_CONTINUITY_2026-03-29.md`
- `docs/reports/REPORT_FIRST_SIM_SOCKET_CONTINUITY_2026-03-29.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/LEARNINGS.md`

## Validation Run
- `cargo fmt --all`
  - Passed.
- `cargo check -p viewer_net`
  - Passed.
- `cargo test -p viewer_net`
  - Passed.
- `cargo check -p viewer_net -p viewer_app`
  - Failed.
  - Failure detail: `crates/viewer_app/src/main.rs:2406` missing field `texture_id` in `DecodedWorldObjectFeedObject` initializer.

## Result Status
- Implemented and validated for the touched transport crate (`viewer_net`).
- Not fully validated end-to-end because the broader `viewer_app` build is currently blocked by unrelated existing work.

## Risks or Follow-up Items
- Resolve the unrelated `viewer_app` compile failure before claiming repo-wide completion for this slice.
- Run connected live verification after the app builds again and confirm the viewer log reports non-zero `object_updates`.
- If live ingress is still zero, continue RCA in downstream receive/drain behavior rather than reverting this socket-continuity fix.

## Learnings Delta
- added: `L24` because the first long-lived receive path must preserve the probe socket when startup object traffic begins immediately after `AgentMovementComplete`.

## Continuity Updates Performed
- Added the bounded task plan and plan review artifacts.
- Added the implementation review artifact.
- Updated `docs/CURRENT_STATE.md` with the new transport status.
- Replaced `docs/HANDOFF.md` with the latest exact state and next step.
- Added `L24` to `docs/LEARNINGS.md`.
