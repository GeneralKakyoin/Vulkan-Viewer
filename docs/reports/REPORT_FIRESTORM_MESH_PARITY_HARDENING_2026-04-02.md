# Report: Firestorm Mesh Parity Hardening (2026-04-02)

## Summary
Implemented a bounded Firestorm-parity hardening pass focused on:
- structured mesh-ID decode from `ObjectUpdateCompressed` ExtraParams,
- Firestorm-style mesh URL preference/shape policy,
- range-first mesh fetch request shaping with fallback.

## Files Changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_grid/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_FIRESTORM_MESH_PARITY_HARDENING_2026-04-02.md`
- `docs/reviews/REVIEW_PLAN_FIRESTORM_MESH_PARITY_HARDENING_2026-04-02.md`
- `docs/reviews/REVIEW_IMPL_FIRESTORM_MESH_PARITY_HARDENING_2026-04-02.md`

## Implementation Details
1. `viewer_net` compressed decode:
- `parse_compressed_object_update_data(...)` now walks compressed flags/optional fields to reach `ExtraParams` deterministically.
- Added helpers:
  - `skip_c_string(...)`
  - `read_compressed_extra_params_slice(...)`
  - `parse_mesh_id_from_compressed_extra_params(...)`
- Removed prior compressed-specific speculative mesh decode path from active use.

2. `viewer_grid` mesh URL policy:
- Added `AssetCapabilityPolicy::mesh_url_candidates(...)` with Firestorm-like preference order:
  - `ViewerAsset`, then `GetMesh2`, then `GetMesh`
- Mesh URL shape is query-style only:
  - `/?mesh_id=<uuid>` then `?mesh_id=<uuid>`

3. `viewer_app` mesh command lane:
- `LiveFeedCommand::RequestMesh` now consumes `AssetCapabilityPolicy::mesh_url_candidates(...)` instead of ad-hoc per-cap URL assembly.

4. `viewer_net` mesh fetch shaping:
- `fetch_mesh_asset_bytes(...)` now tries range-first (`Range: bytes=0-`) then plain GET fallback.
- `fetch_bytes_from_candidate_urls(...)` now accepts optional range header.

5. Test coverage:
- Updated compressed decode baseline test payload to include required compressed fields (`cflags`, owner, extra-params marker).
- Added test: `decode_object_update_compressed_extracts_mesh_id_from_extra_params`.
- Added tests for mesh URL policy helper in `viewer_grid`.

## Validation Run
- `cargo fmt --all` -> PASS
- `cargo check -p viewer_net -p viewer_grid -p viewer_app` -> PASS
- `cargo test -p viewer_net` -> PASS
- `cargo test -p viewer_grid` -> PASS
- `cargo test -p viewer_app test_extract_decoded_object_feed_mesh_ids_is_deterministic_and_capped -- --nocapture` -> PASS

## Live Validation
- Command: bounded `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`
- Artifacts:
  - `artifacts/logs/live_mesh_firestorm_parity_2026-04-02_192934.log`
  - `artifacts/logs/network_debug_mesh_firestorm_parity_2026-04-02_192934.jsonl`

Observed in-window:
- LLUDP object ingress healthy later in run (`lludp_object_gate: PASS`).
- `ViewerAsset mesh_id` lane probes still returned `403`.
- No `mesh_fetch: queued` events were emitted in this bounded run.
- `RegionObjects` summary continued to report `mesh_candidates=none` on sampled objects.

## Result Status
- **Code parity hardening implemented and validated locally.**
- **End-to-end live mesh fetch/render still blocked by runtime data/access behavior in this capture.**

## Risks / Follow-up
- Need deterministic live mesh-ID source in-session (fixture mesh IDs or proven decode source with emitted IDs) to force `mesh_fetch` lane execution every run.
- Need follow-up evidence on whether range-first mesh fetch reduces failures when valid mesh IDs are queued.

## Learnings Delta
- `none` — no new durable learning beyond reinforcement of existing L73/L74.

## Continuity Updates Performed
- Added plan/review/report artifacts for this slice.
- Updated handoff and current-state notes for latest implementation/live evidence.
