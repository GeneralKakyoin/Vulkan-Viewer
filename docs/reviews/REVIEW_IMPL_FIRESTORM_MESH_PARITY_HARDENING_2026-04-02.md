# Review: Implementation Firestorm Mesh Parity Hardening (2026-04-02)

## Verdict
- Approved with residual runtime blocker noted.

## Architecture / Boundary Fit
- `viewer_net`: compressed packet decode + mesh fetch shaping only.
- `viewer_grid`: capability URL policy helper for mesh request generation.
- `viewer_app`: mesh command lane now consumes policy helper output; no protocol logic drift into app.

## Correctness Concerns
- Structured compressed parsing now locates ExtraParams by walking OpenSim/Firestorm compressed fields and flags instead of speculative UUID scans.
- Mesh URL generation now follows Firestorm preference/shape for mesh requests (`ViewerAsset` -> `GetMesh2` -> `GetMesh`, query-style `mesh_id`).
- Mesh fetch uses bounded range-first then plain fallback.

## Modularity / Maintainability
- New `AssetCapabilityPolicy::mesh_url_candidates(...)` centralizes mesh URL policy and removes duplicated app-side assembly.
- Reuses existing `decode_mesh_asset_id_from_extra_params(...)` parser, avoiding duplicate mesh-ID decode logic.

## Validation Adequacy
- `cargo fmt --all` PASS
- `cargo check -p viewer_net -p viewer_grid -p viewer_app` PASS
- `cargo test -p viewer_net` PASS
- `cargo test -p viewer_grid` PASS
- `cargo test -p viewer_app test_extract_decoded_object_feed_mesh_ids_is_deterministic_and_capped -- --nocapture` PASS
- bounded live run PASS (startup/runtime healthy; see report artifacts)

## Risks / Open Questions
- Asset-CDN `mesh_id` probes still return `403` in live capture.
- In this bounded run there were no `mesh_fetch` queue events, so end-to-end live mesh byte acquisition remains unproven.

## Learnings Delta Verdict
- none (no new durable rule yet; outcome reinforces existing L74/L73 guidance).

## Approval Status
- Implementation acceptable for this bounded parity slice.
