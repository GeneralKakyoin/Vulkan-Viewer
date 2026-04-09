# Report: EventQueue LLSD Root Hardening Live Verification (2026-04-09)

## Summary of implemented work
- Ran a bounded live `viewer_app` verification capture after LLSD root-shape hardening.
- Confirmed EventQueue decode/readiness behavior on simulator-host `:12043` no longer shows `missing llsd map` decode failures in this capture.
- Verified probe gate and capability readiness progression (`EventQueueGet:ok`, gate-open, `InterestList:ok`, `UntrustedSimulatorMessage:ok`).
- Verified LLUDP ingress progression in the same capture (`lludp_object_gate: verdict=PASS`, object updates observed).

## Files changed
- `artifacts/logs/network_debug_event_queue_llsd_root_hardening_verify_2026-04-09.jsonl`
- `docs/reports/REPORT_EVENT_QUEUE_LLSD_ROOT_HARDENING_LIVE_VERIFY_2026-04-09.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`

## Validation run
- `VIEWER_APP_LIVE_STARTUP=on`
- `VIEWER_APP_LLUDP_STARTUP_PARITY_BUNDLE=on`
- `VIEWER_APP_CAPABILITY_PROBES_REQUIRE_EVENT_QUEUE_OK=true`
- `VIEWER_APP_EVENT_QUEUE_CAP_NOT_FOUND_BEFORE_RECONNECT=3`
- `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_event_queue_llsd_root_hardening_verify_2026-04-09.jsonl`
- `cargo run -p viewer_app` (bounded by shell timeout; log capture still completed)

Evidence extraction commands:
- `rg -n "missing llsd map|EventQueueGet:ok|probe_gate:open|after_first_steady_state_window readiness" artifacts/logs/network_debug_event_queue_llsd_root_hardening_verify_2026-04-09.jsonl`
- `rg -n "lludp_object_gate|ObjectUpdate|state_total_objects" artifacts/logs/network_debug_event_queue_llsd_root_hardening_verify_2026-04-09.jsonl`
- `rg -n "cap not found|cap-not-found threshold reached" artifacts/logs/network_debug_event_queue_llsd_root_hardening_verify_2026-04-09.jsonl`

## Result status
- EventQueue LLSD root-shape decode issue appears resolved in this bounded live run.
- Remaining runtime instability observed: EventQueue capability rotates to `404 cap not found` later in session and triggers reconnect threshold behavior.

## Risks or follow-up items
- Need a bounded follow-up branch focused on EventQueue capability rotation/re-prime behavior after late-session 404s.
- Since run was timeout-bounded, full graceful-shutdown semantics were not validated here.

## Learnings delta
none: no new durable invariant beyond existing EventQueue/capability readiness guidance; this run is confirmation evidence.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md` latest notable changes with live verification outcomes.
- Updated `docs/HANDOFF.md` latest handoff to reflect completed verification and the new next step.
