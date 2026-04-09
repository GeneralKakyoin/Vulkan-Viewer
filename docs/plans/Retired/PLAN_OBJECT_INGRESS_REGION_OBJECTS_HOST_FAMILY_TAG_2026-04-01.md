# Plan: Object Ingress RegionObjects Host-Family Transcript Tag (2026-04-01)

## Scope
- Add a lightweight `host_family=...` tag to `RegionObjects` transcript lines so reconnect route shifts are explicit.
- Apply tag to:
  - primary probe success/error
  - post-reconnect re-probe success/error
  - matching protocol-event entries

Out of scope:
- transport changes
- capability behavior changes
- UI redesign

## Current known state
- A/B captures show reproducible inversion across reconnect paths.
- Current transcript already includes classified URL text but no explicit host-family tag token for quick filtering.

## Files and components touched
- `crates/viewer_app/src/main.rs`
- docs plan/review/report and continuity docs

## Boundary check
- No crate-boundary changes.
- No protocol semantic expansion; diagnostics-only.

## Step sequence
1. Add helper to derive host-family tag from classified URL host label.
2. Thread tag into `RegionObjects` primary/reprobe success/error relay lines and protocol events.
3. Add focused unit test for host-family extraction.
4. Run static checks and targeted tests.
5. Run bounded `cargo run -p viewer_app` Ahern validation and confirm tag in transcript.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_app`
- bounded live run:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_APP_AUTO_TELEPORT_SLURL=secondlife://Ahern/50/60/70`
  - `VIEWER_APP_AUTO_TELEPORT_DELAY_TICKS=40`
  - `VIEWER_APP_REGION_OBJECTS_REPROBE_DELAY_TICKS=80`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_region_objects_host_family_tag_2026-04-01.jsonl`
  - `cargo run -p viewer_app`

## Risks and open questions
- Host labels may be unavailable in malformed URLs; fallback must remain stable (`unknown-host`).

## Deferred-too-early candidates captured
- none

## Learnings pre-check
- L57: paired-capture interpretation depends on clear route identity in transcript lines.

## Exact completion criteria
- `RegionObjects` lines include `host_family=...`.
- Validation commands and bounded live run pass.
- continuity docs updated with outcome.
