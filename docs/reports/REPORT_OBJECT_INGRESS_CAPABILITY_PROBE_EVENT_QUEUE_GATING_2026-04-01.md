# Report: Object Ingress Capability Probe EventQueue Gating (2026-04-01)

## Summary of implemented work
- Added `viewer_app` startup probe-ordering gate for capability-readiness probes:
  - `VIEWER_APP_CAPABILITY_PROBES_REQUIRE_EVENT_QUEUE_OK` (default `true`)
- Reworked `InterestList` and `UntrustedSimulatorMessage` startup probes to run as deferred one-shots after gate-open.
- Added explicit protocol-event and relay evidence for:
  - gate waiting
  - deferred probe state
  - gate open after `EventQueueGet:ok`
- Refactored probe execution into helper functions to keep startup and loop paths consistent.

## Files changed
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_OBJECT_INGRESS_CAPABILITY_PROBE_EVENT_QUEUE_GATING_2026-04-01.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_CAPABILITY_PROBE_EVENT_QUEUE_GATING_2026-04-01.md`
- `docs/reviews/REVIEW_IMPL_OBJECT_INGRESS_CAPABILITY_PROBE_EVENT_QUEUE_GATING_2026-04-01.md`
- `docs/TESTING_REFERENCE.md`

## Validation run
- `cargo fmt --all`: PASSED
- `cargo check -p viewer_net -p viewer_app`: PASSED
- `cargo test -p viewer_net`: PASSED
- `cargo test -p viewer_app`: PASSED
- bounded live run (timeout-bounded capture):
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_APP_LLUDP_STARTUP_PARITY_BUNDLE=on`
  - `VIEWER_APP_EVENT_QUEUE_CAP_NOT_FOUND_BEFORE_RECONNECT=3`
  - `VIEWER_APP_CAPABILITY_PROBES_REQUIRE_EVENT_QUEUE_OK=true`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_capability_probe_event_queue_gating_2026-04-01.jsonl`
  - `cargo run -p viewer_app`

## Result status
- Gate behavior worked as designed:
  - startup shows deferred probes (`InterestList` and `UntrustedSimulatorMessage` not invoked yet)
  - gate opens only after `EventQueueGet:ok`
  - probes execute immediately after gate-open
- Outcome shift observed:
  - `InterestList` changed from prior `404 Agent not found` to `ok` (`keys=mode,stats`)
  - `UntrustedSimulatorMessage` remained `405 Method Not Allowed`
- LLUDP ingress remained blocked in the same run:
  - `lludp_object_gate: verdict=FAIL object_update=none ... local_ids=none`

## Risks or follow-up items
- `UntrustedSimulatorMessage` may require a different method/body/ordering contract not yet implemented.
- LLUDP blocker is still unresolved; this slice only narrowed prerequisite uncertainty.

## Learnings delta
- added
- Added a durable learning: EventQueue ordering is a real prerequisite signal for `InterestList` on this simulator-host path.

## Continuity updates performed
- Updated `docs/TESTING_REFERENCE.md`
- Updated `docs/LEARNINGS.md`
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/OBJECT_INGRESS_STATUS.md`
- Updated `docs/HANDOFF.md`
