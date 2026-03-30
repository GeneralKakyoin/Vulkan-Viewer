# PLAN: Live Object Feed Unblock (March 29, 2026)

## Scope
Unblock first live object/texture UUID arrival by fixing simulator handoff parsing and UDP ingest throughput limits without widening crate boundaries.

## Current Known State
- Login succeeds.
- UDP inbound observed (`CoarseLocationUpdate`, `LayerData` high 11).
- `EnableSimulator` events observed in EventQueue.
- Object update counters remain zero.

## Files/Components
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- continuity artifacts under `docs/reviews/` and `docs/reports/`.

## Boundary Check
- `viewer_net`: protocol decode/transport mechanics only.
- `viewer_app`: orchestration, logging, runtime policy knobs.
- No `viewer_grid` policy changes in this slice.

## Steps
1. Add/complete missing inbound message coverage for startup world stream visibility (including `MultipleObjectUpdate` classification lane and unknown-id diagnostics).
2. Tighten EventQueue `EnableSimulator` extraction and retarget behavior to only act on actionable endpoint changes.
3. Increase default social UDP drain budget (`max_packets`) to avoid starving object stream under burst startup.
4. Run connected `cargo run -p viewer_app` capture and verify at least one object update is observed; if still absent, record exact remaining blocker from diagnostics.

## Validation Plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- connected run: `cargo run -p viewer_app` with log capture under `artifacts/logs/`

## Risks/Open Questions
- EventQueue `EnableSimulator` body may remain partial (port-only) and refer to neighbor sims.
- Even with decode coverage, simulator may withhold object updates if movement/update cadence is still incomplete.

## Deferred Candidates
- none for this bounded unblock slice.

## Learnings Pre-check
- L05, L10: keep unknown packet IDs visible; do not hide unknown traffic.
- L01/L02: preserve login compatibility path while debugging post-login traffic.

## Completion Criteria
- At least one live run log shows non-zero object update count OR precise packet-level blocker documented with new evidence and no unknown blind spots.
