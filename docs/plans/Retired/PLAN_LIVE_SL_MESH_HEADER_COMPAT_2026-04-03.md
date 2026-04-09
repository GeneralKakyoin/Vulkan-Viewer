# Plan: Live SL Mesh Header Compatibility (2026-04-03)

## Scope
- Fix the live Second Life mesh header parse failure without widening scene/render architecture.
- Keep the current geometry-first mesh pipeline intact.
- Limit code changes to `viewer_asset` parser/decoder logic plus tests and continuity docs.

## Current known state
- Live mesh fetch is healthy: mesh assets are discovered and fetched with `206`/`200`.
- Real live mesh bytes begin with valid-looking binary-LLSD map signatures such as `7b000000096b0000`.
- The downstream blocker is `mesh_asset: decode_failed ... failed to parse SL mesh LLSD header`.
- Firestorm’s `LLSDBinaryParser` supports additional binary token forms beyond the subset currently implemented in `viewer_asset`.

## Files and components touched
- `crates/viewer_asset/src/sl_mesh_loader.rs`
- `crates/viewer_asset/src/lib.rs`
- plan/review/report/continuity docs

## Boundary check
- `viewer_asset` remains the owner of live mesh byte parsing and SL mesh decode.
- `viewer_app`, `viewer_core`, and `viewer_render` interfaces stay unchanged in this slice.
- No cache/render contract changes are allowed unless the parser fix proves insufficient.

## Step sequence
1. Compare the current parser token support against Firestorm’s binary-LLSD parser.
2. Add the smallest missing token/key support needed for real live SL mesh headers.
3. Add regression tests covering the newly supported header value forms.
4. Run targeted crate tests.
5. Run a bounded live viewer session and confirm at least one real mesh asset decodes past the header stage.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_asset -p viewer_app`
- `cargo test -p viewer_asset`
- `cargo test -p viewer_app`
- bounded live run with dedicated logs:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_live_sl_mesh_header_fix_2026-04-03.jsonl`
  - `.\\target\\debug\\viewer_app.exe`

## Risks and open questions
- The first live parse failure may be caused by more than one missing token variant.
- A successful header parse may expose a later decode issue in decompressed LOD blocks.
- Live login/session behavior may still vary by route/window; use exact dated artifacts in the report.

## Deferred-too-early candidates
- None for this slice.

## Learnings pre-check
- `L78`: embedded and standalone protocol formats cannot be conflated; validate against real viewer references.
- `L79`: keep deterministic offline proof separate from live-compatibility fixes.

## Exact completion criteria
- `viewer_asset` accepts the real live SL mesh header variant that currently fails.
- targeted tests pass
- bounded live run shows at least one live mesh asset no longer failing at `failed to parse SL mesh LLSD header`
