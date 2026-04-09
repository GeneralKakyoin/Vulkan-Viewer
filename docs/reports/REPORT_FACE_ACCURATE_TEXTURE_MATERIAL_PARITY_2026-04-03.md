# Report: Face-Accurate Texture + Material Parity (2026-04-03)

## Summary of implemented work
- Added additive per-face material payload fields to decoded object-feed exports in `viewer_net`:
  - default face material entry
  - per-face override entries
- Expanded `TextureEntry` decode to parse default + per-face override data and legacy face material fields (fail-soft behavior retained).
- Threaded per-face payload through `viewer_app` into `viewer_core` object ingestion seams.
- Updated scene material mapping in `viewer_core` to populate `MaterialSet.by_face` and use per-face override precedence.
- Expanded texture scheduling extraction in `viewer_app` to include per-face diffuse/normal/specular IDs.
- Added bounded face-material diagnostics and checklist auto-discovery artifact output:
  - `artifacts/logs/object_face_checklist_autodiscover.jsonl`

## Files changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_core/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_FACE_ACCURATE_TEXTURE_MATERIAL_PARITY_2026-04-03.md`
- `docs/reviews/REVIEW_PLAN_FACE_ACCURATE_TEXTURE_MATERIAL_PARITY_2026-04-03.md`
- `docs/reviews/REVIEW_IMPL_FACE_ACCURATE_TEXTURE_MATERIAL_PARITY_2026-04-03.md`
- `docs/reports/REPORT_FACE_ACCURATE_TEXTURE_MATERIAL_PARITY_2026-04-03.md`
- `docs/plans/DEFERRED_FEATURES.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/LEARNINGS.md`

## Validation run
- `cargo fmt --all` -> PASS
- `cargo check -p viewer_net -p viewer_app -p viewer_core -p viewer_render` -> PASS
- `cargo test -p viewer_net` -> PASS
- `cargo test -p viewer_app` -> PASS
- `cargo test -p viewer_core` -> PASS
- `cargo test -p viewer_render` -> PASS
- bounded live launch:
  - command: `VIEWER_APP_LIVE_STARTUP=on VIEWER_FIXTURE_MESHES=0 VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_face_material_parity_2026-04-03.jsonl cargo run -p viewer_app`
  - result: timed out by harness (expected for interactive runtime), with decisive evidence captured in log:
    - `tick face_materials: objects_with_default=89 objects_with_overrides=89 override_faces=577`
    - `after_first_steady_state_window lludp_object_gate: verdict=PASS ...`
  - checklist artifact present and populated:
    - `artifacts/logs/object_face_checklist_autodiscover.jsonl`

## Result status
- Complete for this bounded slice.
- Per-face decode/propagation/application is now active and scheduler coverage includes face-referenced texture IDs.

## Risks or follow-up items
- Full RenderMaterials extension resolution (for full normal/specular/PBR parity) remains a dedicated next slice.
- Visual parity is improved but not declared complete across all objects/routes yet.

## Learnings delta
- `added`: `L81`
- Reason: exception-encoded payload decoders are prone to silent offset drift when defaults are consumed twice; this now has explicit guardrail guidance.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/LEARNINGS.md`
- Updated `docs/plans/DEFERRED_FEATURES.md`
