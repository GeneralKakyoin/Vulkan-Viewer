# Plan: Face-Accurate Texture + Material Parity (2026-04-03)

## Scope
- Promote decoded per-face `TextureEntry` data (default + overrides) from LLUDP decode through app/core scene ingestion.
- Prefer per-face material application over legacy single-texture fallback when face payload exists.
- Expand texture scheduling to include IDs referenced by face material data (diffuse + normal/specular references).
- Add bounded live diagnostics and checklist bootstrap output for repeatable face-target verification.

## Current known state
- Mesh discovery/fetch/decode path is active, and decoded mesh geometry can be uploaded.
- Object placement/scale parity is improved, but many live objects still show incomplete face texture/material appearance.
- Existing decode path previously extracted only default texture UUID, which under-represents face-override content.

## Files and components touched
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_core/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- continuity artifacts under `docs/plans`, `docs/reviews`, `docs/reports`, `docs/CURRENT_STATE.md`, `docs/HANDOFF.md`, `docs/LEARNINGS.md`

## Boundary check
- `viewer_net`: transport decode + exported decoded object-feed payload fields only.
- `viewer_app`: bridge/mapping/scheduling/diagnostic wiring only.
- `viewer_core`: scene material mapping and fallback semantics only.
- No crate-boundary ownership changes and no renderer contract widening in this slice.

## Step sequence
1. Add additive per-face decoded material payload fields to `viewer_net` object-feed export.
2. Replace default-only `TextureEntry` extraction with full default + face-override decode (fail-soft on malformed/truncated data).
3. Thread additive per-face payload through `viewer_app` into `viewer_core` decoded object ingestion contracts.
4. Update scene material construction to apply default face material first, then deterministic per-face overrides.
5. Expand texture scheduling extraction to include per-face diffuse and normal/specular IDs.
6. Add bounded diagnostics and checklist auto-discovery artifact for live face verification targets.
7. Add regression tests across touched crates and run bounded live verification.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app -p viewer_core -p viewer_render`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- `cargo test -p viewer_core`
- `cargo test -p viewer_render`
- bounded live run:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_FIXTURE_MESHES=0`
  - dedicated `VIEWER_NETWORK_DEBUG_LOG_PATH`

## Risks and open questions
- `TextureEntry` exception parsing order is easy to misalign; parser must remain fail-soft and deterministic.
- This slice carries additive material-extension references but does not yet implement full RenderMaterials capability resolution.
- Early startup windows may still show sparse face-material counts before steady-state ingress; acceptance should use steady-state ticks.

## Deferred-too-early candidates
- Full RenderMaterials capability fetch/resolve pipeline for complete normal/specular/PBR parity.
- Multi-grid face-material verification matrix; current acceptance remains single primary live route.

## Learnings pre-check
- `L73`: force deterministic live/fixture probes when diagnostic ambiguity is high.
- `L74`: route-level health does not imply mesh/material identifiers are present.
- `L76`: decoded transport payload should be consumed directly by ingest and scene mapping.
- `L78`: wire-format assumptions from adjacent messages are risky; parse the actual block grammar.
- `L80`: Firestorm-compat parser tolerance is critical before declaring payload unsupported.

## Exact completion criteria
- Decoded object-feed exports include additive default + per-face material payload.
- App/core pipeline applies per-face materials when present and falls back only when absent.
- Texture scheduling includes per-face referenced texture IDs.
- Diagnostics show non-zero steady-state face-material counts on live route.
- Targeted crate checks/tests pass.
