# Plan: Fix R05 Implementation Review Findings

## Summary
Address vertex layout mismatch, alpha source divergence, and uniform slot capping drift reported in `docs/reviews/REVIEW_impl_r05.md`.

## Proposed Changes
### `viewer_render`
- Update pipeline vertex attribute descriptors in `lib.rs` to include `normal` and `tex_coord`.
- Document `max_objects` as a uniform slot/submesh cap in `draw_helpers.rs`.
- Reconcile alpha source (Texture * Instance) as a valid A06-superseded state that supports R05 color-only validation when fallback textures are used.

## Verification
- `cargo check --workspace`
- `cargo test -p viewer_render`
- `cargo run -p viewer_app` (startup smoke)
