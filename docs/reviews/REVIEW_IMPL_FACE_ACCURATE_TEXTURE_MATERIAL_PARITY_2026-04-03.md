# Review: Implementation Face-Accurate Texture + Material Parity (2026-04-03)

## Verdict
Approved.

## Architecture and boundary fit
- Implementation remains within `viewer_net` decode/export, `viewer_app` bridge/scheduling/diagnostics, and `viewer_core` scene material mapping.
- No crate-boundary drift or renderer API breakage observed.

## Correctness concerns
- Added tests caught and fixed two concrete `TextureEntry` parser alignment bugs:
  - UUID section default-field double-consume
  - material-byte default-field double-consume
- Fail-soft truncation behavior is now explicitly tested in `viewer_net`.
- `viewer_core` now applies per-face overrides and alpha mode behavior deterministically while preserving fallback semantics.

## Modularity and maintainability concerns
- Additive payload contracts keep compatibility with existing single-texture paths.
- Diagnostic and checklist additions are bounded and low-risk.

## Validation adequacy
- `cargo fmt --all` passed.
- `cargo check -p viewer_net -p viewer_app -p viewer_core -p viewer_render` passed.
- `cargo test -p viewer_net` passed.
- `cargo test -p viewer_app` passed.
- `cargo test -p viewer_core` passed.
- `cargo test -p viewer_render` passed.
- Bounded live run (`cargo run -p viewer_app`) timed out by harness as expected for interactive runtime, but produced decisive face-material and gate evidence in dedicated logs.

## Risks and open questions
- Full normal/specular parity still depends on a future RenderMaterials resolution pipeline.
- Visual parity across all regions/grids is not claimed; this slice is tuned for the active live route.

## Learnings delta verdict
- `add`
- Reason: this implementation exposed a durable, repeated parser trap (default field consumed twice) in exception-encoded blocks.

## Required revisions or approval status
- Approved as implemented.
