# Plan: Mesh Fetch To Visible Objects (2026-04-03)

## Scope
- Implement the downstream path from successful live mesh fetch to visible mesh geometry.
- Keep `viewer_core::GeometrySource::Mesh(uuid, lod)` unchanged.
- Add deterministic offline verification so mesh decode/render work can be proven without live login stability.
- Defer full live object per-face material/texture parity to a later slice.

## Current known state
- Live mesh discovery/export/fetch is already unblocked on the current route/window.
- `viewer_app` already requests mesh geometry for `GeometrySource::Mesh(...)`, but `viewer_asset` still assumed glTF for mesh bytes.
- `viewer_render` already supports dynamic geometry upload and material assignment once decoded mesh data exists.
- `DecodedWorldObjectFeedObject.texture_id` exists, but live object-feed texture/material propagation is not yet wired end-to-end.

## Files and components touched
- `crates/viewer_asset/Cargo.toml`
- `crates/viewer_asset/src/lib.rs`
- `crates/viewer_asset/src/sl_mesh_loader.rs`
- `crates/viewer_app/src/main.rs`
- `docs/TESTING_REFERENCE.md`
- continuity docs and task report artifacts

## Boundary check
- `viewer_asset` owns mesh-byte format detection, typed staging, and SL mesh decode.
- `viewer_app` owns mesh asset lifecycle state, runtime verification hooks, and scene/render handoff.
- `viewer_core` and `viewer_render` interfaces stay unchanged for this slice.
- No crate-boundary widening is allowed unless the SL mesh decoder proves `ProcessedMesh` is insufficient.

## Step sequence
1. Add typed mesh cache lookup status in `viewer_asset` and distinguish empty, cached-ready, decoded, and decode-failed states.
2. Replace glTF-only live mesh decode with a Second Life mesh decoder driven by captured format evidence.
3. Extend `viewer_app` live mesh state from raw bytes to lifecycle states: `Requested`, `Fetched`, `Decoded`, `Failed`.
4. Drive scene mesh upload from decoded staged assets and update instance-local AABBs from the decoded mesh.
5. Add deterministic offline verification by seeding a synthetic SL mesh asset into the screenshot torture scene.
6. Document the deferred-too-early follow-up for live object texture/material parity.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_asset -p viewer_app`
- `cargo test -p viewer_asset`
- `cargo test -p viewer_app`
- bounded offline runtime smoke with:
  - `VIEWER_APP_LIVE_STARTUP=off`
  - `STRESS_TEST=screenshot`
  - `VIEWER_TEST_SCREENSHOT_EVERY_N_FRAMES=1`
  - `VIEWER_TEST_SCREENSHOT_MAX_FRAMES=1`
  - `VIEWER_APP_MESH_VERIFY=true`

## Risks and open questions
- The compact mesh-verification JSON log may still need hardening even if the screenshot artifact proves visible geometry.
- Live object material/texture parity remains incomplete because feed-side texture/material promotion is not yet wired.
- Synthetic SL mesh verification proves decode/upload/render, but not live object selection/camera framing.

## Deferred-too-early candidates
- Full live object per-face material/texture parity after world-object mesh geometry is stable.

## Learnings pre-check
- `L73`: keep deterministic verification available instead of relying only on opportunistic live discovery.
- `L74`: do not assume one capability/object lane is sufficient for mesh proof.
- `L76`: preserve bounded positional/object-feed semantics while changing downstream render behavior.
- `L78`: treat SL mesh byte format as protocol-specific and validate against Firestorm/OpenSim references, not glTF assumptions.

## Exact completion criteria
- `viewer_asset` detects and decodes SL mesh assets into non-empty `ProcessedMesh`.
- `viewer_app` stores mesh assets as typed lifecycle states and promotes decoded meshes into renderer uploads.
- A bounded offline screenshot run shows non-placeholder mesh geometry in the viewer.
- Validation commands above pass.
- Continuity docs, reviews, report, and deferred-feature entry are updated.
