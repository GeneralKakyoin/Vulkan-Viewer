# Plan: Live Texture Capability URL Parity 2026-03-29

## Objective
Harden the existing A13 live texture fetch path so live scene textures use Firestorm-aligned capability URL shaping and bounded HTTP request fallback behavior.

## Scope
In scope:
- Firestorm-aligned `/?texture_id=` URL shaping for `GetTexture` / `ViewerAsset`.
- Shared bounded candidate URL generation for texture capability requests.
- Shared bounded HTTP fetch helper behavior for live textures and profile images, including the existing image-friendly `Accept` header.
- Targeted tests for URL ordering and fetch fallback behavior.

Out of scope:
- LLUDP startup/control-plane changes.
- Mesh or non-texture asset transport expansion.
- New UI/diagnostic surfaces beyond existing failure reporting.
- Broad asset pipeline redesign.

## Current known state
- The current live texture request path in `viewer_app` asks `viewer_grid::AssetCapabilityPolicy::select_texture_url(...)` for one URL and then calls `viewer_net::fetch_asset_bytes(...)` once.
- `viewer_grid` currently builds `GetTexture` as `{base}?texture_id=...`, while Firestorm appends `/?texture_id=...` after choosing either `ViewerAsset` or `GetTexture` in `reference/firestorm/indra/newview/lltexturefetch.cpp`.
- `viewer_net::fetch_profile_image_bytes(...)` already uses a bounded multi-candidate URL strategy plus `Accept: image/x-j2c,image/jp2,image/*,*/*`, but live scene textures do not reuse that path.
- The provided `C:\\Users\\matti\\Desktop\\fire.pcapng` contains Second Life-related DNS lookups (`login.agni.lindenlab.com`, `asset-cdn.glb.agni.lindenlab.com`, `simhost-...agni.secondlife.io`) but no actual simulator UDP or texture transfer payloads, so it is useful as a weak environment clue, not as a transfer-level parity oracle.

## Files and components touched
- `crates/viewer_grid/src/lib.rs`
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`

## Boundary check
- `viewer_grid` owns capability URL meaning and candidate ordering.
- `viewer_net` owns HTTP transport execution and response handling only.
- `viewer_app` remains orchestration-only and should switch to the shared helper without taking on capability semantics.
- No renderer/UI/LLUDP ownership changes are allowed in this slice.

## Step sequence
1. Add a bounded `viewer_grid::AssetCapabilityPolicy` helper that produces ordered texture capability URL candidates with Firestorm-style `/?texture_id=` first.
2. Add or refactor a `viewer_net` HTTP helper to try candidate URLs with the existing image-oriented `Accept` header and preserve clear failure propagation.
3. Update the live texture request path in `viewer_app` to use the shared candidate/helper flow instead of the single-shot URL path.
4. Refactor profile image fetch to reuse the same shared fetch helper where practical, keeping behavior bounded and consistent.
5. Add targeted tests for candidate ordering and HTTP fallback behavior.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_grid -p viewer_net -p viewer_app`
- `cargo test -p viewer_grid -p viewer_net -p viewer_app`

## Risks and open questions
- Live proof still depends on valid `VIEWER_LOGIN_*` credentials and a scene that exposes real texture IDs.
- The capture does not include transfer payloads, so the fix must rely on Firestorm source parity plus current repo code evidence.

## Deferred-too-early candidates captured
- None. HTTP/3/QUIC parity and broader CDN telemetry are useful follow-ups but too early for this bounded fix.

## Learnings pre-check
- `L06`: keep capability semantics in `viewer_grid` and transport execution in `viewer_net`.
- No additional applicable learnings.

## Completion criteria
- Live texture requests no longer rely on a single exact capability URL shape.
- Firestorm-style `/?texture_id=` is tried first for `GetTexture` and `ViewerAsset`.
- The live texture path and profile image path share the same bounded HTTP texture fetch behavior.
- Targeted validation commands pass and the remaining live-proof gap, if any, is documented explicitly.
