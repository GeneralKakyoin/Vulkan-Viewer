# PLAN_RENDER_OBJECT_TEXTURE_PLACEMENT_STABILIZATION_2026-04-10

## Scope
- Fix two user-visible rendering regressions in current branch:
  - world object-feed placement instability (cluster/random fallback behavior)
  - missing/flat object textures under live feed due insufficient throughput and weak decode-stage observability
- Keep scope bounded to existing crate boundaries (`viewer_core`, `viewer_app`) without protocol-shape expansion.

## Current Known State
- `viewer_core::world_object_feed_proxy_transform(...)` currently ring-scatters objects when decoded position is absent, which can appear random and unstable.
- `viewer_app` texture scheduling currently runs with a low in-flight cap and does not emit decode-stage success/failure diagnostics when `LiveFeedUpdate::TextureAsset` is ingested.
- Prior branch already includes mesh-axis conversion and texture request prioritization tuning.

## Files / Components Touched
- `crates/viewer_core/src/lib.rs`
  - world object-feed upsert and transform fallback behavior
  - bounded regression tests for stable placement retention
- `crates/viewer_app/src/main.rs`
  - live texture fetch throughput cap
  - texture decode-stage diagnostics in `LiveFeedUpdate::TextureAsset` handling
- `docs/reports/REPORT_RENDER_OBJECT_TEXTURE_PLACEMENT_STABILIZATION_2026-04-10.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`

## Boundary Check
- No crate-boundary redesign.
- No Firestorm/protocol decode-shape changes.
- No dependency or environment modifications.

## Step Sequence
1. Patch `viewer_core` object-feed transform/upsert to preserve prior transform components for partial updates and remove ring-scatter fallback for positionless first sightings.
2. Add `viewer_core` regression test proving no-position updates retain prior proxy placement.
3. Patch `viewer_app` to raise live texture in-flight cap and emit decode-stage relay diagnostics with byte-size/signature and decode status.
4. Run required validation ladder for touched crates.
5. Record execution report and continuity updates.

## Validation Plan
- `cargo fmt --all`
- `cargo check -p viewer_core -p viewer_app`
- Targeted tests:
  - `cargo test -p viewer_core scene_world_object_feed_positionless_update_retains_existing_position -- --nocapture`
  - `cargo test -p viewer_app test_merge_texture_request_ids_preserves_left_priority_order -- --nocapture`

## Risks / Open Questions
- Live visual correctness still requires runtime screenshot/manual review after code-level validation.
- If textures still appear missing after this slice, next likely branch is renderer material bind telemetry (per-face selected IDs vs cache status).

## Deferred-too-early Candidates
- None added in this bounded fix; no new feature deferrals identified.

## Learnings Pre-check
- L03/L04: keep seam-owned lifecycle and dirty-only apply semantics unchanged.
- L16: preserve white fallback behavior for legally empty texture slots.
- L79/L80/L81: avoid broad decode-shape changes in this slice; keep changes in placement/throughput/diagnostics.
- L82: keep crate-local, behavior-local edits.

## Completion Criteria
- Positionless object updates no longer ring-scatter; existing proxies keep stable placement across updates lacking fresh position payload.
- Live texture lane processes more concurrent requests and emits decode-stage relay evidence for success/failure.
- Validation commands pass and continuity artifacts are updated with exact command outcomes.
