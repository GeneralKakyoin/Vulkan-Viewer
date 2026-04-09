# Plan: RegionObjects Mesh Candidate Extraction (2026-04-02)

## Objective
Add a bounded runtime diagnostic that extracts candidate mesh asset UUIDs from incoming `RegionObjects` payload inspections and exposes them in relay output so live mesh-ingest probes can use real candidate IDs.

## Scope
- Extend `viewer_net::RegionObjectsInspection` with a bounded `candidate_mesh_asset_ids` list.
- Populate this list during JSON/LLSD child-map inspection using key/value heuristics and UUID validation.
- Surface candidates in `viewer_app` `RegionObjects` summary relay text.
- Add targeted tests for extraction heuristics and summary exposure.

## Current Known State
- Mesh fetch lane is wired and emits `mesh_fetch: queued|failed`.
- Manual UUID probe can fail with 403 when using object UUID instead of mesh asset UUID.
- `RegionObjectsInspection` already inspects child-map scalar fields but does not expose mesh-candidate UUIDs.

## Files and Components Touched
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/reports/REPORT_REGION_OBJECTS_MESH_CANDIDATE_EXTRACTION_2026-04-02.md`
- `docs/reviews/REVIEW_IMPL_REGION_OBJECTS_MESH_CANDIDATE_EXTRACTION_2026-04-02.md`
- continuity docs (`CURRENT_STATE`, `HANDOFF`, `LEARNINGS`) if implementation succeeds

## Boundary Check
- `viewer_net`: transport/inspection mechanics only (candidate extraction from observed payload fields).
- `viewer_app`: diagnostics summarization only.
- No ownership shift into `viewer_grid`, no renderer/asset refactor.

## Step Sequence
1. Add `candidate_mesh_asset_ids: Vec<String>` to `RegionObjectsInspection`.
2. Add bounded helper in `viewer_net` to record candidate mesh UUIDs:
   - accept only valid UUID-like values.
   - prioritize keys with mesh/model hints (`mesh`, `sculpt`, `model`, `asset`, `shape`).
   - cap list size to avoid noisy/unbounded diagnostics.
3. Invoke helper from both JSON and LLSD child-map record paths.
4. Update `viewer_app::summarize_region_objects_inspection(...)` to include `mesh_candidates=...`.
5. Add targeted tests:
   - `viewer_net` extraction from representative scalar maps.
   - `viewer_app` summary includes `mesh_candidates=...` when present.
6. Run validation and one bounded live probe to capture `mesh_candidates` lines.

## Validation Plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- targeted tests:
  - new `viewer_net` candidate extraction test(s)
  - new/updated `viewer_app` region-object summary test(s)
- bounded runtime probe:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - capture `region_objects: ... mesh_candidates=...`

## Risks and Open Questions
- Heuristic may produce non-mesh UUIDs if keys are too broad.
- Mitigation: bound extraction to mesh-shape key hints + strict UUID parse + small capped list.

## Deferred-Too-Early Candidates Captured
- None for this bounded diagnostic slice.

## Learnings Pre-Check
- L64, L68, L70, L73.

## Completion Criteria
- RegionObjects summaries include bounded `mesh_candidates` evidence when present.
- At least one live artifact line shows the new field.
- Validation commands pass.
