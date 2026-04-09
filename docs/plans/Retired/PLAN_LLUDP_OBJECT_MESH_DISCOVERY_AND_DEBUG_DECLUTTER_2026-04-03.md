# Plan: LLUDP Object Mesh Discovery + Network Debug Declutter 2026-04-03

## Objective
Shift live mesh-ID request discovery to decoded real LLUDP object updates and reduce debug-window noise so operators see object-ingress-relevant signals first.

## Scope
- `viewer_app`: adjust mesh request candidate collection to prioritize decoded object-feed mesh IDs (real object updates) over generic visible-scene proxy guesses.
- `viewer_app` + `viewer_ui`: declutter network debug presentation to emphasize handshake/object-feed/mesh-ingest signals and reduce untied noise.
- Add or update targeted tests for mesh-ID extraction and debug rendering behavior where deterministic.

## Current known state
- `tick_scene_meshes` currently merges fixture mesh IDs, visible-scene mesh IDs, and decoded object-feed mesh IDs.
- Visible-scene mesh IDs can include proxy-derived geometry and are not guaranteed to be real in-scene mesh asset IDs.
- Network debug window currently renders many broad sections and verbose lines, making object-ingress triage noisy.

## Files and components touched
- `crates/viewer_app/src/main.rs`
- `crates/viewer_ui/src/lib.rs`
- `docs/reviews/REVIEW_PLAN_LLUDP_OBJECT_MESH_DISCOVERY_AND_DEBUG_DECLUTTER_2026-04-03.md`
- `docs/reviews/REVIEW_IMPL_LLUDP_OBJECT_MESH_DISCOVERY_AND_DEBUG_DECLUTTER_2026-04-03.md`
- `docs/reports/REPORT_LLUDP_OBJECT_MESH_DISCOVERY_AND_DEBUG_DECLUTTER_2026-04-03.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`

## Boundary check
- `viewer_app` owns live request scheduling and network-debug state shaping.
- `viewer_ui` owns presentation-level declutter only.
- No `viewer_net` protocol decode semantics changes in this slice.
- No crate-boundary movement.

## Step sequence
1. Refactor `tick_scene_meshes` candidate assembly to prefer decoded object-feed mesh IDs; keep fixture IDs as explicit deterministic override source.
2. Remove default visible-scene mesh-ID contribution from live discovery path.
3. Update/add tests covering deterministic decoded mesh-ID extraction behavior used by scheduler.
4. Trim and focus network debug session lines in `viewer_app`.
5. Adjust network debug UI collapsing defaults to prioritize object-ingress sections.
6. Run formatting/check/tests and capture results.
7. Write report + continuity updates.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_app -p viewer_ui -p viewer_net`
- `cargo test -p viewer_app test_extract_decoded_object_feed_mesh_ids_is_deterministic_and_capped -- --nocapture`
- `cargo test -p viewer_ui -- --nocapture`

## Risks and open questions
- Removing visible-scene mesh-ID sourcing may reduce opportunistic mesh fetches outside decoded object updates; this is intentional to avoid proxy/guess requests.
- UI declutter must keep enough data for triage; avoid removing event history entirely.

## Deferred-too-early candidates captured
- None.

## Learnings pre-check
- L73: fixture IDs are useful for deterministic verification; keep explicit fixture path.
- L74: RegionObjects mesh candidates are not guaranteed; decoded LLUDP object feed should be a first-class source.
- L77: keep startup/object ingress observability focused on LLUDP object gate signals.

## Completion criteria
- Live mesh scheduler requests are driven by fixture IDs + decoded object-feed mesh IDs, without default visible-proxy mesh-ID discovery.
- Network Debug window is materially less noisy while preserving object-ingress actionable signals.
- Validation commands pass for touched crates/tests.
