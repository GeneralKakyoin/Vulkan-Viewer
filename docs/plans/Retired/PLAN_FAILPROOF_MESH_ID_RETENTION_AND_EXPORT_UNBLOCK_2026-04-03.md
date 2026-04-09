# Plan: Failproof Mesh-ID Retention and Export Unblock 2026-04-03

## Objective
Make live mesh-ID flow provable end-to-end across LLUDP decode, object-feed state, exported snapshot, and mesh scheduling; remove misleading object-feed accounting; and validate the result with a bounded live run.

## Scope
- `viewer_net`: add additive mesh-retention counters, preserve full object-feed map counts, prioritize mesh-bearing objects under export truncation, and add replay-grade regression tests from real Firestorm payloads.
- `viewer_app`: preserve transport-side object totals, report state-vs-export object-feed counts explicitly, and add regression coverage for the net-to-app bridge.
- Continuity docs for this slice only.

## Current known state
- `FirestormsFidelis.pcapng` shows mesh IDs inside `ObjectUpdate.ExtraParams` even when standalone `ObjectExtraParams` packets are absent.
- Earlier live diagnostics were inconclusive because the relay only exposed exported counts and could not distinguish full transport state from the exported snapshot slice.
- Previous bounded decoded-only live runs reported `mesh_id_count=0`, but the path from real decode to exported scheduler input was not yet provable.

## Files and components touched
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_FAILPROOF_MESH_ID_RETENTION_AND_EXPORT_UNBLOCK_2026-04-03.md`
- `docs/reviews/REVIEW_PLAN_FAILPROOF_MESH_ID_RETENTION_AND_EXPORT_UNBLOCK_2026-04-03.md`
- `docs/reviews/REVIEW_IMPL_FAILPROOF_MESH_ID_RETENTION_AND_EXPORT_UNBLOCK_2026-04-03.md`
- `docs/reports/REPORT_FAILPROOF_MESH_ID_RETENTION_AND_EXPORT_UNBLOCK_2026-04-03.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/LEARNINGS.md`

## Boundary check
- `viewer_net` owns LLUDP decode, object-feed state retention, and export summary fields.
- `viewer_app` owns relay formatting, scheduler-visible snapshot interpretation, and runtime verification wiring.
- No crate-boundary movement and no asset HTTP redesign in this slice.

## Step sequence
1. Extend `SimulatorPayloadDecodeSummary` with additive state/export and mesh-hit counters.
2. Preserve full transport-side object counts while computing export counts separately.
3. Change object-feed export truncation ordering to deterministic mesh-first priority.
4. Add real-payload replay tests showing mesh IDs survive decode and later cached/terse updates.
5. Update `viewer_app` relay formatting to expose state/export distinctions and per-family mesh-hit counts.
6. Add app regression coverage for total-count preservation through the net-to-app bridge.
7. Run fmt/check/tests.
8. Rebuild the runnable binary and perform a bounded live decoded-only verification run.
9. Write report/reviews and update continuity docs.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- `cargo build -p viewer_app`
- bounded live run:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_FIXTURE_MESHES=0`
  - dedicated `VIEWER_NETWORK_DEBUG_LOG_PATH`
  - `.\target\debug\viewer_app.exe`

## Risks and open questions
- Embedded `ObjectUpdate.ExtraParams` layout may differ from standalone `ObjectExtraParams`; replay tests must lock the real on-wire layout.
- A stale previously-built `viewer_app.exe` can invalidate live conclusions if the binary is not rebuilt after code changes.
- Live route/window variability remains possible, so evidence must be captured and cited from a concrete bounded run.

## Deferred-too-early candidates captured
- None.

## Learnings pre-check
- L06: keep transport decode and app relay responsibilities clearly separated.
- L10: all LLUDP message behavior must be grounded in the Firestorm message template and source references.
- L73: fixture mesh IDs remain valuable for deterministic command-lane checks, but this slice focuses on decoded live discovery.
- L74: do not rely on `RegionObjects` mesh candidates as the sole discovery path.
- L76: full/compressed/terse object updates are legitimate object-feed inputs and must retain their useful payload.

## Completion criteria
- Real Firestorm-derived replay tests prove mesh IDs survive object-update decode and later cache/terse follow-up.
- Live relay distinguishes `state_total_objects` from `export_objects` and `state_mesh_objects` from `export_mesh_objects`.
- Bounded live run produces decisive evidence for whether live mesh IDs reach the exported scheduler-visible slice.
- Validation commands pass for touched crates, and continuity docs reflect the actual live result.
