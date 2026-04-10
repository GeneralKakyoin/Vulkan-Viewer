# Plan: Render Object Feed Texture Fallback Merge (2026-04-09)

## Objective
Restore visible object texturing for live object-feed proxies when decoded face-material payloads are present but omit face/default texture IDs.

## Scope
- `viewer_core` only behavior fix for object-feed material mapping.
- Add focused regression tests in `viewer_core`.
- Verify with targeted crate tests plus bounded live runtime evidence.

## Current known state
- Live runs show healthy object-feed ingress with many default/override face-material payloads.
- Runtime logs show no `texture_fetch` activity during these runs.
- Current `world_object_feed_materials(...)` path only applies `decoded_object_texture_id` fallback when no default-face material exists at all.
- If default-face material exists but has empty texture ID, fallback is skipped and renderer binds white fallback.

## Files and components touched
- `crates/viewer_core/src/lib.rs`
- `docs/reviews/REVIEW_PLAN_RENDER_OBJECT_FEED_TEXTURE_FALLBACK_MERGE_2026-04-09.md`
- `docs/reports/REPORT_RENDER_OBJECT_FEED_TEXTURE_FALLBACK_MERGE_2026-04-09.md`
- continuity docs (`docs/CURRENT_STATE.md`, `docs/HANDOFF.md`) after validation

## Boundary check
- No crate boundary change.
- Keep transport decode in `viewer_net` unchanged for this slice.
- Keep renderer internals in `viewer_render` unchanged.

## Step sequence
1. Update `world_object_feed_materials(...)` to merge fallback texture ID into default material when default exists but has empty base texture.
2. Add regression tests proving fallback merge behavior.
3. Run fmt/check/tests.
4. Run bounded live `viewer_app` repro and verify texture-fetch activity appears.
5. Write report + continuity updates.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_core -p viewer_app -p viewer_render`
- `cargo test -p viewer_core world_object_feed -- --nocapture`
- bounded live run:
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_texture_fallback_merge_verify_2026-04-09.jsonl cargo run -p viewer_app` (timeout-bounded)
  - inspect log for `texture_fetch` lines and absence of obvious regressions

## Risks and open questions
- Some surfaces may depend on material-ID lookups (`RenderMaterials`) beyond this fallback; this fix targets the immediate regression where object-level texture fallback is skipped.
- Live visual quality still depends on mesh UV/material completeness.

## Deferred-too-early candidates captured
- Full material-ID to texture resolution via `RenderMaterials` capability integration (defer; larger cross-crate slice).

## Learnings pre-check
- L16 applies: empty/unassigned slots must use white fallback, but this assumes intended texture IDs are preserved where available.
- No additional applicable learnings identified.

## Completion criteria
- `viewer_core` tests cover fallback merge case and pass.
- Bounded live verification shows `texture_fetch` activity for object-feed IDs.
- Continuity docs updated with exact validation/evidence.
