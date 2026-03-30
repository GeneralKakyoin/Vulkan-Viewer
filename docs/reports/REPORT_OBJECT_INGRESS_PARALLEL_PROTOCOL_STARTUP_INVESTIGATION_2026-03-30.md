# Report: Object Ingress Parallel Protocol Startup Investigation (2026-03-30)

## Summary of implemented work
- Added bounded cross-protocol startup diagnostics to `viewer_net` and `viewer_app`.
- Extended first-simulator classification/timeline support so the live summary now names:
  - `RegionHandshakeReply`
  - `CameraConstraint`
  - `GenericMessage`
- Added URL-family classification for capability URLs and seed-capability inventory summaries.
- Added live relay output that surfaces:
  - seed-cap fetch start/result
  - capability inventory grouped by URL family
  - `EventQueueGet` start/result state
  - simulator-host capability usage currently exercised by the app
- Ran a bounded connected viewer capture and a documented `tshark` extraction over `C:\\Users\\matti\\Desktop\\Firestorms.pcapng`.

## Files changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_OBJECT_INGRESS_PARALLEL_PROTOCOL_STARTUP_INVESTIGATION_2026-03-30.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_PARALLEL_PROTOCOL_STARTUP_INVESTIGATION_2026-03-30.md`
- `docs/plans/PLAN_OBJECT_INGRESS_PERSISTENT_EVENT_QUEUE_2026-03-30.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_PERSISTENT_EVENT_QUEUE_2026-03-30.md`

## Validation run
- `cargo fmt --all`: PASSED
- `cargo check -p viewer_net -p viewer_app`: PASSED
- `cargo test -p viewer_net`: PASSED
- `cargo test -p viewer_app`: PASSED
- bounded connected run `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`: PASSED
- `tshark` extraction saved to `artifacts/logs/firestorm_parallel_protocol_timing_2026-03-30.csv`: PASSED

## Result status
- The investigation succeeded in narrowing the next branch.
- The current viewer now proves that it fetches the same family of first-region capabilities from simulator-host `:12043`:
  - `EventQueueGet`
  - `AgentProfile`
  - `GetDisplayNames`
  - `SimulatorFeatures`
  - `MapLayer`
- The current viewer also proves that it starts `EventQueueGet` on that simulator-host URL during the bounded run.
- Even so, the bounded run still showed:
  - `RegionHandshake = none`
  - `RegionHandshakeReply = none`
  - `ObjectUpdate* = none`
  - `event_ack = 0`
  - no surfaced `EventQueueGet` completion, timeout, or failure before shutdown
- Firestorm evidence now makes the next gap more specific:
  - Firestorm source `reference/firestorm/indra/newview/llviewerregion.cpp` installs `EventQueueGet` by creating `LLEventPoll`
  - `reference/firestorm/indra/newview/lleventpoll.cpp` shows that consumer is persistent long-poll behavior rather than sparse one-shot probing
  - `reference/firestorm/indra/newview/llappcorehttp.h` explicitly documents simulator-host HTTPS policy classes on `:12043`
  - the provided pcap shows simulator-host `:12043` traffic beginning at `17.724375000s` and recurring through `41.825419300s` (`24` observed rows in the extraction), parallel to but distinct from heavy downstream CDN traffic

## Evidence-backed conclusion
- The strongest next branch is **persistent `EventQueueGet` adoption**.
- Why this branch won:
  - the investigation disproved the idea that we were simply missing all simulator-host capability access; we do fetch and classify those caps
  - the viewer currently reaches only `EventQueueGet:start ... ack=0` in the bounded run, with no surfaced completion
  - Firestorm’s reference path uses a persistent long-poll consumer for that same capability lane
  - more standalone LLUDP startup packet guessing is now less justified than closing this simulator-host polling gap
- Why the other branches lost for now:
  - missing simulator-host capability invocation: possible later, but not before we match the baseline persistent `EventQueueGet` behavior
  - LLUDP startup ordering correction: still possible later, but this investigation moved the highest-value clue to the capability lane
  - teleport/region-crossing refresh correction: not first, because the initial region-entry path is still incomplete

## Risks or follow-up items
- Persistent `EventQueueGet` may still be necessary but not sufficient.
- If that next slice succeeds mechanically and ingress still remains blocked, the next branch should likely move to a specific simulator-host capability invocation rather than another UDP parity guess.

## Learnings delta
- `added`
- Added L40 to `docs/LEARNINGS.md`.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/LEARNINGS.md`
- Updated `docs/plans/PLAN_OBJECT_INGRESS_NEXT_STEP_ROADMAP_2026-03-30.md`
- Added next active plan `docs/plans/PLAN_OBJECT_INGRESS_PERSISTENT_EVENT_QUEUE_2026-03-30.md`
