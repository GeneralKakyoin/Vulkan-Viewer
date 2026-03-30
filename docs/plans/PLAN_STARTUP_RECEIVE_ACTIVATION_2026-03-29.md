# Plan: Startup Receive Activation Under Retained Socket (2026-03-29)

## Objective
Make the `CLUES.md` fix actually protect the startup object burst by activating the retained first-simulator socket as the live receive path immediately after probe, before blocking startup HTTP/name work can starve or overflow that socket.

## Scope
- Adjust `viewer_net` retained-socket rules so a successful probe can preserve the startup socket even when `AgentMovementComplete` was not yet locally observed.
- Adjust `viewer_app` startup ordering so the first social circuit opens immediately after the probe/login handshake path, not after capability fetches and bootstrap friend-name resolution.
- Add a bounded startup drain on the active social circuit before capability/name work so buffered startup LLUDP traffic is observed by `observe_first_simulator_inbound_payload(...)` as early as possible.
- Keep the change bounded to startup sequencing, socket lifecycle, and validation diagnostics; do not expand into unrelated protocol guessing or object decode refactors.

## Current Known State
- `docs/CLUES.md` correctly identified that the original probe socket was dropped before the long-lived social circuit was opened.
- The current implementation now reuses that socket, but connected validation still shows `handshake_complete=true` with `update_messages=0 total_objects=0`, proving the receive path still becomes active too late.
- Startup currently does this in order: probe, fetch seed capabilities, resolve bootstrap friend names, then `open_social_circuit()`. That leaves the retained socket idle during multiple awaited HTTP operations.
- `poll_social_events(...)` already classifies all packets it receives through `observe_first_simulator_inbound_payload(...)`, so it is the correct bounded receive path to reuse for early startup draining.

## Files and Components Touched
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- continuity artifacts under `docs/reviews/` and `docs/reports/`

## Boundary Check
- `viewer_net` owns socket retention policy and first-simulator receive mechanics.
- `viewer_app` owns startup sequencing and when the long-lived receive loop becomes active.
- No `viewer_grid`, renderer, UI contract, or asset ownership changes are allowed in this slice.

## Step Sequence
1. Revise retained-socket policy in `viewer_net` so a successful probe can preserve the startup socket even if AMC has not yet been observed locally.
2. Open the social circuit immediately after the probe/login startup path in `viewer_app`, before capability fetch and bootstrap name resolution.
3. Add a bounded startup drain helper that polls the active social circuit right away and feeds startup LLUDP packets into the existing decode/classification path before further awaited startup work.
4. Keep capability fetch, friend-name resolution, and later loop behavior intact after the early drain so the change stays bounded.
5. Add or update targeted tests for the revised retained-socket behavior and run connected verification with object-feed relay summaries.

## Validation Plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- connected capture: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app` with bounded log collection under `artifacts/logs/`

## Risks and Open Questions
- If object ingress still remains zero after early receive activation, the remaining blocker is likely deeper packet parity/classification rather than startup sequencing.
- A larger startup drain budget could delay non-network startup work slightly; keep it bounded and use existing config-derived limits where practical.
- This plan intentionally does not change object decode semantics or guess at missing packet IDs.

## Deferred-too-Early Candidates Captured
- None. No new deferred feature was identified; this is still a bounded startup transport/orchestration correction.

## Learnings Pre-check
- L05: keep unknown traffic visible while touching startup receive paths.
- L23: success must be judged by actual non-zero object ingress, not handshake or retarget symptoms.
- L24: preserve first-simulator socket continuity across probe and long-lived receive.
- L25: socket continuity alone is insufficient; startup receive activation must happen early enough to matter.

## Completion Criteria
- The retained startup socket becomes the active social-circuit receive path immediately after probe/login.
- A bounded startup drain occurs before capability/name HTTP work.
- Connected verification shows either non-zero `object_feed` updates or a tighter, evidence-backed remaining blocker after the now-correct receive activation path.
