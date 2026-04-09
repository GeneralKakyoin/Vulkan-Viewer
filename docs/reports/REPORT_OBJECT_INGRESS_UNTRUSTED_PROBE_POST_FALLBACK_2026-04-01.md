# Report: Object Ingress UntrustedSimulatorMessage POST Probe Fallback (2026-04-01)

## Summary of implemented work
- Updated `viewer_net::fetch_untrusted_simulator_message_once(...)` to align with Firestorm method behavior:
  - primary attempt remains GET
  - on GET `405 Method Not Allowed`, fallback to LLSD POST envelope (`message`, `body`)
- Added LLSD POST helper for simulator capability probes.
- Added bounded untrusted probe payload builder using startup agent/session identifiers.
- Added decode fallback for successful but non-LLSD capability responses:
  - returns transport-ok synthetic inspection (`probe_transport_status`, `probe_body_bytes`, `probe_content_type`, `probe_decode`)

## Files changed
- `crates/viewer_net/src/lib.rs`
- `docs/plans/PLAN_OBJECT_INGRESS_UNTRUSTED_PROBE_POST_FALLBACK_2026-04-01.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_UNTRUSTED_PROBE_POST_FALLBACK_2026-04-01.md`
- `docs/reviews/REVIEW_IMPL_OBJECT_INGRESS_UNTRUSTED_PROBE_POST_FALLBACK_2026-04-01.md`

## Validation run
- `cargo fmt --all`: PASSED
- `cargo check -p viewer_net -p viewer_app`: PASSED
- `cargo test -p viewer_net`: PASSED
- `cargo test -p viewer_app`: PASSED
- bounded live run (EventQueue-gated capability probes):
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_APP_LLUDP_STARTUP_PARITY_BUNDLE=on`
  - `VIEWER_APP_EVENT_QUEUE_CAP_NOT_FOUND_BEFORE_RECONNECT=3`
  - `VIEWER_APP_CAPABILITY_PROBES_REQUIRE_EVENT_QUEUE_OK=true`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_untrusted_probe_post_fallback_2026-04-01.jsonl`
  - `cargo run -p viewer_app` (bounded capture)

## Result status
- `UntrustedSimulatorMessage` readiness is now working in bounded live evidence:
  - transitioned from previous `err:http status 405 Method Not Allowed`
  - now reports `ok` with probe transport summary (`probe_transport_status=200`, `probe_decode=unparsed_body`)
- `InterestList` remains `ok` post-EventQueue gate.
- LLUDP ingress remains blocked in same run:
  - `lludp_object_gate: verdict=FAIL object_update=none ... local_ids=none`

## Risks or follow-up items
- Untrusted probe response is transport-success but semantically unparsed; method alignment is fixed, schema parity remains open.
- This does not unblock LLUDP object ingress yet.

## Learnings delta
- added
- Added L61 to `docs/LEARNINGS.md` about treating successful untrusted capability invocation with non-LLSD payload as transport-ready rather than hard failure.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/OBJECT_INGRESS_STATUS.md`
- Updated `docs/LEARNINGS.md`
