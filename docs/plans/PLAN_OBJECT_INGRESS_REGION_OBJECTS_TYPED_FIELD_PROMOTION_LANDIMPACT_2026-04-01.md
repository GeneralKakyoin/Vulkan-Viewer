# Plan: Object Ingress RegionObjects Typed Field Promotion (landimpact) (2026-04-01)

## Scope
- Promote one additional bounded typed `RegionObjects` field (`landimpact`) into the existing compact `typed_sample=...` relay.
- Keep behavior bounded to existing `RegionObjects` extraction and summary formatting paths.
- Validate with the established Ahern reconnect baseline.

Out of scope:
- LLUDP object-ingress transport work
- New capability clients
- UI redesign

## Current known state
- `RegionObjects` typed-feed ingestion is working.
- `RegionObjectsPathfindingSummary` already parses `landimpact`.
- `RegionObjectsTypedObjectSample` and `typed_sample` summary do not currently surface `landimpact`.
- Reconnect target comparison confirms Ahern is a positive baseline for typed sample visibility.

## Files and components touched
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/reports/`
- continuity docs (`docs/CURRENT_STATE.md`, `docs/HANDOFF.md`, `docs/OBJECT_INGRESS_STATUS.md`, `docs/LEARNINGS.md`)

## Boundary check
- No crate-boundary changes.
- No protocol-semantics expansion beyond exposing an already-parsed field.
- No transport-flow changes.

## Step sequence
1. Add `landimpact` to `RegionObjectsTypedObjectSample`.
2. Populate it from `RegionObjectsPathfindingSummary` in typed sample construction.
3. Extend `viewer_app` typed summary formatting to include `landimpact` when present.
4. Update targeted tests in `viewer_net` and `viewer_app`.
5. Run static checks and targeted tests.
6. Run one bounded live `cargo run -p viewer_app` pass on Ahern baseline and confirm `typed_sample` includes `landimpact` when present.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net -p viewer_app`
- bounded live run:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_APP_AUTO_TELEPORT_SLURL=secondlife://Ahern/50/60/70`
  - `VIEWER_APP_AUTO_TELEPORT_DELAY_TICKS=40`
  - `VIEWER_APP_REGION_OBJECTS_REPROBE_DELAY_TICKS=80`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_region_objects_typed_landimpact_2026-04-01.jsonl`
  - `cargo run -p viewer_app`

## Risks and open questions
- The sampled objects in a given region may omit `landimpact`, so live evidence may still show no value even if promotion is correct.
- Summary width must remain bounded and readable.

## Deferred-too-early candidates captured
- none

## Learnings pre-check
- L52: prefer stable cross-region field promotion over tuple-first deepening.
- L56: confirm behavior against a positive reconnect target baseline before branch conclusions.

## Exact completion criteria
- `landimpact` is present in typed sample contracts and summary output when available.
- All listed checks pass.
- One bounded Ahern live run is documented with artifact paths and result.
