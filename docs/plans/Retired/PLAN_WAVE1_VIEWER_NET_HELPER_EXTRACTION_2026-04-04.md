# Plan: Wave 1 Viewer Net Helper Extraction (2026-04-04)

## Objective
Execute a larger bounded Wave 1 `viewer_net` extraction by moving cohesive helper clusters out of `lib.rs` into crate-local behavior modules, with no intended behavior changes.

## Scope
- In scope:
  - Extract cookie helpers into `cookie_utils.rs`.
  - Extract login codec helper functions/types into `login_codec_utils.rs`.
  - Extract asset-fetch helper lane into `asset_fetch.rs`.
  - Extract object decode helper block into `object_decode_utils.rs`.
  - Keep external API stable via `pub use` from `lib.rs`.
- Out of scope:
  - Protocol behavior changes.
  - Cross-crate refactors.
  - Feature additions.

## Current known state
- `viewer_net/src/lib.rs` is the largest file in the workspace and primary Wave 1 target.
- Prior micro-slice proved extraction pattern with tests.
- This slice expands the same pattern to additional tightly-related helper clusters.

## Files and components touched
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_net/src/cookie_utils.rs` (new)
- `crates/viewer_net/src/login_codec_utils.rs` (new)
- `crates/viewer_net/src/asset_fetch.rs` (new)
- `crates/viewer_net/src/object_decode_utils.rs` (new)

## Boundary check
- All changes stay inside `viewer_net`.
- `viewer_net`/`viewer_grid` ownership boundaries unchanged.
- No public cross-crate contract changes beyond preserved existing function exports.

## Step sequence
1. Move cookie helper logic to `cookie_utils.rs`.
2. Move XML-RPC/LLSD login codec support helpers to `login_codec_utils.rs`.
3. Move asset fetch helper lane to `asset_fetch.rs`.
4. Move object decode helper block to `object_decode_utils.rs`.
5. Rewire `lib.rs` via `mod` + `use`/`pub use`.
6. Validate targeted and full `viewer_net` tests.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net`
- `cargo test -p viewer_net merge_cookie_header_prefers_new_values_and_preserves_existing -- --nocapture`
- `cargo test -p viewer_net llsd_codec_decodes_simple_login_response -- --nocapture`
- `cargo test -p viewer_net decode_object_update_compressed_extracts_mesh_id_from_extra_params -- --nocapture`
- `cargo test -p viewer_net fetch_mesh_asset_bytes_reuses_set_cookie_between_probe_and_followup -- --nocapture`
- `cargo test -p viewer_net`

## Risks and open questions
- Risk: accidental behavior drift during relocation.
  - Mitigation: function-body carryover, focused regression tests, full crate test pass.
- Open question: next Wave 1 extraction cluster after helper lanes (likely LLUDP encode/decode utility cluster).

## Deferred-too-early candidates captured
- None for this structural slice.

## Learnings pre-check
- Applicable:
  - L06 (`viewer_net`/`viewer_grid` boundary discipline)
  - L70 (structured progression after churn)
  - L82 (crate-local behavior-local placement)

## Completion criteria
- New helper modules compile and are wired from `lib.rs`.
- Existing `viewer_net` API behavior remains unchanged.
- Validation commands pass.
- Continuity docs/report reflect the broader Wave 1 progress.
