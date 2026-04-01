# Plan: Object Ingress RegionObjects Typed Feed Broader SLURL Capture (2026-04-01)

## Scope
- run one bounded reconnect capture using a different SLURL target than the prior `Ahern` run
- verify whether `typed_sample=...` remains stable and whether object-family diversity improves
- update continuity docs with a clear current position and ordered next steps

Out of scope:
- protocol behavior changes
- LLUDP object-ingress fixes
- new field promotions in this slice

## Current known state
- `typed_sample=...` is live-validated on the current reconnect path.
- The last live validation run did not broaden the visible object family.
- LLUDP object ingress remains blocked (`RegionHandshake`, `RegionHandshakeReply`, `ObjectUpdate*` absent).

## Files and components touched
- live artifacts under `artifacts/logs/`
- continuity/report docs only

## Boundary check
- No architecture or crate-boundary changes.
- No code changes are expected; evidence capture and docs sync only.

## Step sequence
1. Run one bounded direct-binary live capture with:
   - `VIEWER_APP_LIVE_STARTUP=on`
   - `VIEWER_APP_AUTO_TELEPORT_SLURL=<different target>`
   - `VIEWER_APP_AUTO_TELEPORT_DELAY_TICKS=40`
   - dedicated JSONL/stdout/stderr artifacts
2. Inspect the JSONL for `region_objects` relay lines and `typed_sample=...`.
3. Record whether object-family diversity improved in this run.
4. Update continuity docs and write the execution report.

## Validation plan
- bounded live run via `target/debug/viewer_app.exe`
- artifact inspection for relay correctness and sample diversity

## Risks and open questions
- live credentials and endpoint must be available locally
- chosen SLURL may still converge on similar content depending on routing/region state

## Deferred-too-early candidates captured
- none

## Learnings pre-check
- L51: reconnect/login path should remain the teleport evidence mechanism.
- L52: prioritize stable cross-region `RegionObjects` fields over tuple-first decoding.

## Exact completion criteria
- capture artifacts are generated successfully
- report states whether broader sample diversity was achieved
- continuity docs explicitly list current position and next steps
