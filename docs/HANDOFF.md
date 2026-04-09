# HANDOFF: EventQueue LLSD Root Hardening Live Verify (2026-04-09)

## What Changed
- Executed a bounded live verification run after EventQueue LLSD root-shape hardening.
- Captured fresh network-debug evidence at:
  - `artifacts/logs/network_debug_event_queue_llsd_root_hardening_verify_2026-04-09.jsonl`
- Confirmed in this run:
  - no `missing llsd map` EventQueue decode errors
  - `EventQueueGet:ok` and `probe_gate:open reason=EventQueueGet:ok`
  - readiness `ok` for `EventQueueGet`, `InterestList`, and `UntrustedSimulatorMessage`
  - LLUDP object ingress progressed (`lludp_object_gate: verdict=PASS`, sustained `ObjectUpdate*` traffic)
- Added verification report:
  - `docs/reports/REPORT_EVENT_QUEUE_LLSD_ROOT_HARDENING_LIVE_VERIFY_2026-04-09.md`

## Validation Run
- `cargo run -p viewer_app` with:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_APP_LLUDP_STARTUP_PARITY_BUNDLE=on`
  - `VIEWER_APP_CAPABILITY_PROBES_REQUIRE_EVENT_QUEUE_OK=true`
  - `VIEWER_APP_EVENT_QUEUE_CAP_NOT_FOUND_BEFORE_RECONNECT=3`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_event_queue_llsd_root_hardening_verify_2026-04-09.jsonl`
- Run was shell-timeout bounded; log artifact captured and analyzed.

## Exact Current State
- The prior EventQueue decode blocker (`missing llsd map`) is not reproduced in the latest bounded live evidence.
- Startup gate and capability-readiness sequencing are functioning in this path.
- LLUDP object ingress is active in-window on this route.
- Remaining instability: late-session EventQueue cap rotation still produces `404 cap not found` and bounded reconnect behavior.

## Exact Next Step
1. Implement and verify a bounded EventQueue cap-rotation re-prime branch (refresh/rebind EventQueue URL after cap-not-found reconnect trigger) to stabilize late-session polling.

## Blockers / Risks
- Existing working tree remains broadly dirty from prior continuity and refactor work; isolate staging by task scope.
- Late-session reconnect behavior may involve simulator cap lifecycle semantics outside viewer control; keep diagnostics explicit.
