# Plan: Single Live Second Life Texture in Scene (First Visible Asset)

## Objective
Deliver one real Second Life texture into the 3D scene by hardening the existing A13 live texture bridge (fetch -> decode -> cache -> renderer) with first-visible texture triggering.

## Scope
In scope:
- Shared robust texture decoding in `viewer_asset` (standard image + JPEG2000 fallback paths).
- `viewer_app` scene-driven request eligibility when no fixture seed IDs are configured.
- `viewer_grid` capability URL policy correction for `ViewerAsset` fallback to query-style `texture_id` URL form.
- Asset diagnostics expansion to split missing-capability failures from generic "other" failures.
- Message-template guardrail verification: no LLUDP message-ID changes in this slice.

Out of scope:
- Mesh/live geometry ingestion.
- UDP asset transfer path expansion.
- Full multi-class live asset parity.
- Credential/UI changes.

## Current known state
- A13 bridge exists but live scene decode path in `viewer_app` was PNG-only.
- `tick_scene_textures(...)` returned early when `VIEWER_FIXTURE_TEXTURES` was empty, blocking first-visible live-only flow.
- `AssetCapabilityPolicy` preferred `GetTexture` but used path-style fallback for `ViewerAsset`.
- Diagnostics grouped `MissingCapability` into generic `other` failures.

## Files and components touched
- `crates/viewer_asset/src/lib.rs`
- `crates/viewer_asset/src/texture_fixture.rs`
- `crates/viewer_asset/Cargo.toml`
- `crates/viewer_app/src/main.rs`
- `crates/viewer_grid/src/lib.rs`
- `crates/viewer_core/src/lib.rs`
- `crates/viewer_ui/src/lib.rs`

## Boundary check
- `viewer_asset` owns decode/cache behavior and live/fallback failure accounting.
- `viewer_app` remains orchestration-only for scene visibility request triggering.
- `viewer_grid` owns capability URL semantics.
- `viewer_net` remains transport-only; no LLUDP protocol expansion.
- `viewer_ui` remains diagnostics presentation only.

## Step sequence
1. Add shared robust texture decode API in `viewer_asset` and keep compatibility wrapper.
2. Route fixture-cache ingest and app live-update ingest through shared decode.
3. Update app request-ID selection so first visible ID is requested even without fixture seeds.
4. Correct `ViewerAsset` fallback URL policy in `viewer_grid`.
5. Add explicit `MissingCapability` failure metric bucket in `viewer_core` + `viewer_asset` + `viewer_ui`.
6. Add/adjust targeted tests for decode, request composition, URL shaping, and diagnostics aggregation.
7. Validate with fmt/check/targeted/workspace tests plus connected run attempt.

## Validation plan
- `cargo fmt --all`
- `cargo check --workspace`
- `cargo test -p viewer_asset -p viewer_grid -p viewer_ui -p viewer_app`
- `cargo test --workspace`
- Connected run attempt:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_ASSET_SOURCE_MODE=live`
  - screenshot smoke knobs for deterministic capture

## Risks and open questions
- Connected validation depends on `VIEWER_LOGIN_*` credentials.
- Live grid may still return unsupported texture encodings not covered by current decoders.

## Deferred-too-early candidates captured
- Full mesh-class live asset import remains deferred (existing `DEFERRED_FEATURES.md` parity entries still apply).

## Learnings pre-check
- `L06`: preserve `viewer_net` vs `viewer_grid` ownership split.
- `L10`: no LLUDP message-ID edits without template grounding.
- `L19`: preserve deterministic cache behavior and tie-breaking.
- `L22`: screenshot smoke requires manual image review.

## Completion criteria
- First-visible texture request path works without fixture seed IDs.
- Live texture decode supports PNG and JPEG2000 in shared `viewer_asset` path.
- `ViewerAsset` fallback URL uses query-style `texture_id` semantics.
- Missing-capability failures are surfaced in diagnostics as a dedicated counter.
- Validation ladder passes; connected validation result is explicitly documented.
