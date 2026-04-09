# Plan: Object Ingress RegionObjects Post-Reconnect Reprobe Timing (2026-04-01)

## Scope
- investigate the `RegionObjects` post-reconnect timing gap observed in the latest broader-SLURL capture
- add a bounded delayed re-probe window after reconnect startup to determine whether `typed_sample=none` is a timing artifact
- keep changes constrained to probe scheduling and diagnostics

Out of scope:
- LLUDP object-ingress transport changes
- broad capability redesign
- UI redesign

## Current known state
- pre-teleport phase can surface strong `typed_sample=...` records.
- post-teleport in the latest run reached a different simhost and valid startup flow, but `RegionObjects` probe returned `<root>` and `typed_sample=none` in the bounded startup window.
- LLUDP object ingress remains blocked.

## Files and components touched
- `crates/viewer_app/src/main.rs` (bounded re-probe timing orchestration)
- optionally `crates/viewer_net/src/lib.rs` if a narrow helper is needed for non-invasive repeat probe
- continuity/report docs

## Boundary check
- no protocol guessing
- no crate-boundary changes
- no new long-running background subsystem

## Step sequence
1. Add one bounded post-reconnect `RegionObjects` re-probe point (or very small fixed attempt window).
2. Keep existing startup probe unchanged for comparability.
3. Emit clear relay markers distinguishing `initial_probe` vs `reprobe`.
4. Run one bounded live reconnect capture.
5. Compare:
   - `typed_sample` initial vs re-probe
   - object-family diversity
   - any regression in startup timing

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- bounded live run with dedicated artifacts

## Risks and open questions
- region may legitimately have sparse/empty `RegionObjects`, so re-probe can still be empty.
- too many re-probes could create noise; keep attempt count and cadence strictly bounded.

## Deferred-too-early candidates captured
- none

## Learnings pre-check
- L51: reconnect/login path remains the teleport evidence path.
- L52: prioritize stable typed fields over tuple-first decoding.
- L53: post-reconnect `RegionObjects` emptiness must be tested for timing before schema conclusions.

## Exact completion criteria
- bounded re-probe behavior implemented and validated without startup regression
- report states whether post-reconnect `typed_sample=none` is timing-related
- continuity docs updated with clear next branch decision
