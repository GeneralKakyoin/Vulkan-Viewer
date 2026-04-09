# Plan: Live Mesh Texture Propagation (2026-04-03)

## Objective
Restore non-fallback texture assignment for live decoded mesh objects by carrying decoded texture IDs from LLUDP object feed into scene materials.

## Scope
- Decode and retain per-object default texture UUID from LLUDP `ObjectUpdate` texture entry bytes.
- Propagate texture ID through `viewer_net` summary export and `viewer_app` snapshot bridge.
- Apply propagated texture ID to `WorldObjectFeedProxy` materials in `viewer_core`.
- Validate with bounded live run + delayed screenshot capture.

## Current known state
- Live mesh fetch/decode is healthy and repeatedly succeeds.
- Live screenshots still show magenta fallback appearance.
- `viewer_app` currently drops object-feed texture data (`texture_id: None`) when bridging net->core.
- `viewer_net` object-feed export currently carries `mesh_id` and `object_id`, but not texture ID.

## Files and components touched
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `crates/viewer_core/src/lib.rs`
- continuity docs/review/report updates

## Boundary check
- Keep ownership unchanged:
  - decode/export in `viewer_net`
  - snapshot bridge in `viewer_app`
  - scene/material application in `viewer_core`
- No render backend or shader contract changes in this slice.

## Step sequence
1. Extend object-feed ingress/state/export in `viewer_net` with optional `texture_id`.
2. Decode `ObjectUpdate` default texture UUID from `TextureEntry` bytes (bounded parser: first UUID only).
3. Propagate exported texture ID in `viewer_app` bridge to `DecodedWorldObjectFeedObject.texture_id`.
4. Extend `WorldObjectIngestionItem` with optional decoded texture ID and apply it to proxy `MaterialSet` in `Scene::upsert_world_object_feed(...)`.
5. Add targeted tests for:
   - net export includes texture ID when decoded
   - app bridge preserves texture ID
   - scene proxy material default texture updates from decoded object-feed texture ID
6. Run validation and bounded delayed live screenshot verification.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app -p viewer_core`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- `cargo test -p viewer_core`
- bounded live delayed screenshot run with:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `STRESS_TEST=screenshot`
  - delayed screenshot interval (high frame interval)
  - dedicated `VIEWER_NETWORK_DEBUG_LOG_PATH`

## Risks and open questions
- LL texture entry format is richer than a single UUID; this slice intentionally decodes only default texture UUID first.
- Some objects may still appear fallback-colored if they have missing/inaccessible texture assets or per-face overrides not yet propagated.

## Deferred-too-early candidates captured
- Full per-face texture entry decode and material override propagation.
- Full material params parity (`PRIM_MATERIAL`, normal/spec maps, alpha mode parity).

## Learnings pre-check
- `L73`, `L79`, `L80` apply:
  - keep live verification bounded and deterministic
  - separate transport/decode success from downstream presentation
  - prefer focused parser expansion over speculative protocol rewrites

## Completion criteria
- At least one live mesh proxy instance carries a non-empty texture ID into material binding path.
- Bounded live delayed screenshot no longer appears as pure magenta-only fallback for all visible objects.
- Targeted checks/tests pass.
