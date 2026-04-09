# Report: Object Ingress Capability Readiness Invocation Investigation (2026-04-01)

## Summary of implemented work
- Added reusable simulator capability fetch helpers in `viewer_net` and exposed:
  - `fetch_interest_list_once(...)`
  - `fetch_untrusted_simulator_message_once(...)`
- Added startup capability-readiness matrix tracking in `viewer_app` for:
  - `EventQueueGet`
  - `InterestList`
  - `UntrustedSimulatorMessage`
  - `RegionObjects`
- Added readiness relay summary in `parallel_protocol` output:
  - `... readiness: <capability matrix>`
- Added bounded startup invocation probes for:
  - `InterestList`
  - `UntrustedSimulatorMessage`
- Added bounded EventQueue cap-not-found reconnect policy:
  - `VIEWER_APP_EVENT_QUEUE_CAP_NOT_FOUND_BEFORE_RECONNECT` (default `3`)
- Kept strict LLUDP object gate intact (`lludp_object_gate` lines unchanged except for continued evidence output).

## Files changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/TESTING_REFERENCE.md`
- `docs/plans/PLAN_OBJECT_INGRESS_CAPABILITY_READINESS_INVOCATION_2026-04-01.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_CAPABILITY_READINESS_INVOCATION_2026-04-01.md`
- `docs/reviews/REVIEW_IMPL_OBJECT_INGRESS_CAPABILITY_READINESS_INVOCATION_2026-04-01.md`

## Validation run
- `cargo fmt --all`: PASSED
- `cargo check -p viewer_net -p viewer_app`: PASSED
- `cargo test -p viewer_net`: PASSED
- `cargo test -p viewer_app`: PASSED
- bounded live run (timeout-bounded):
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_APP_LLUDP_STARTUP_PARITY_BUNDLE=on`
  - `VIEWER_APP_EVENT_QUEUE_CAP_NOT_FOUND_BEFORE_RECONNECT=3`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_capability_readiness_2026-04-01.jsonl`
  - `cargo run -p viewer_app`

## Result status
- Capability-readiness evidence is now explicit and actionable.
- LLUDP object ingress remains blocked in this run:
  - `lludp_object_gate: verdict=FAIL`
  - `object_update=none`
  - `update_messages=0`
  - `total_objects=0`
  - `local_ids=none`

## Key live findings
- `InterestList` probe:
  - `404 Not Found`
  - body includes `Agent not found`
- `UntrustedSimulatorMessage` probe:
  - `405 Method Not Allowed`
- `EventQueueGet` initially succeeds (`ok ack_in=0 ack_out=1`) then later degrades into repeated `cap not found` failures.
- New bounded policy triggers reconnect:
  - `event queue cap-not-found threshold reached (3) ; reconnecting`

## Risks or follow-up items
- Next branch should interpret capability readiness/ordering (especially `InterestList` 404 and `UntrustedSimulatorMessage` 405) rather than add LLUDP startup control guesses.
- Preserve this run log as the baseline readiness artifact:
  - `artifacts/logs/network_debug_capability_readiness_2026-04-01.jsonl`

## Learnings delta
- added — see `L59` in `docs/LEARNINGS.md`.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md`, `docs/HANDOFF.md`, and `docs/OBJECT_INGRESS_STATUS.md` with readiness findings and exact next step.
