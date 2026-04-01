# HANDOFF: Object Ingress Host-Family Transcript Tag Completed (2026-04-01)

## What Changed
- Added `host_family=...` tags to `RegionObjects` transcript lines and matching protocol-event entries.
- Tag now appears on:
  - primary probe success/error
  - post-reconnect re-probe success/error
- This makes reconnect route shifts explicit without parsing full URLs.

## Validation Run
- `cargo fmt --all`: PASSED
- `cargo check -p viewer_net -p viewer_app`: PASSED
- `cargo test -p viewer_app`: PASSED
- authoritative bounded run:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_APP_AUTO_TELEPORT_SLURL=secondlife://Ahern/50/60/70`
  - `VIEWER_APP_AUTO_TELEPORT_DELAY_TICKS=40`
  - `VIEWER_APP_REGION_OBJECTS_REPROBE_DELAY_TICKS=80`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_region_objects_host_family_tag_2026-04-01.jsonl`
  - `cargo run -p viewer_app`
- artifact outputs:
  - `artifacts/logs/live_region_objects_host_family_tag_2026-04-01.out.log`
  - `artifacts/logs/live_region_objects_host_family_tag_2026-04-01.err.log`
  - `artifacts/logs/network_debug_region_objects_host_family_tag_2026-04-01.jsonl`

## Exact Current State
- `RegionObjects` typed feed remains operational and includes `landimpact` when present.
- Transcript lines now include explicit host-family tags (example: `host_family=simhost-0629fe9f6de4b8693`).
- LLUDP world object ingress remains unresolved:
  - `RegionHandshake` absent
  - `RegionHandshakeReply` absent
  - `ObjectUpdate*` absent

## Exact Next Step
1. Execute a bounded LLUDP startup parity bundle behind a runtime flag.
2. Validate with a hard pass/fail gate: first decoded `ObjectUpdate*` (local-id evidence > 0).
3. If the gate fails, immediately switch to simulator-host capability-readiness invocation investigation.

## Blockers / Risks
- Reconnect routing/content drift remains a risk; single-run conclusions are still unsafe.
- Diagnostics are improved, but object ingress is still unresolved until LLUDP hard-gate criteria pass.
