# Plan: N03 Bounded World/Object Decode Expansion for Richer Render Feed

## Summary
Implement a bounded, typed object-update ingest path from first-simulator LLUDP traffic into `LiveVisualSnapshot` and the world-ingestion seam so scene state can represent a richer set of world objects without breaking crate boundaries or seam ownership rules.

## Objective
Expand decode + ingest coverage just enough to move beyond coarse/health diagnostic-derived proxies and drive a bounded multi-entity object feed into scene/render preparation.

## Why now
- `R01` and `A02` are complete; the current bottleneck is world/object feed richness.
- Current decode coverage is still focused on early diagnostics (`CoarseLocationUpdate`, `HealthMessage`, `SimulatorViewerTimeMessage`) rather than object-family traffic.
- Roadmap and handoff place `N03` as the next required milestone before `U04`.

## Scope
In scope:
- Add bounded classification and decode for first-simulator object-family LLUDP traffic:
  - `ObjectUpdate` (high 12)
  - `ObjectUpdateCompressed` (high 13)
  - `ObjectUpdateCached` (high 14)
  - `ImprovedTerseObjectUpdate` (high 15)
  - `KillObject` (high 16)
- Introduce typed, bounded object-feed state in `viewer_net` suitable for snapshot export.
- Extend `LiveVisualSnapshot` and seam adapter types with bounded object entries and lifecycle signal fields.
- Add seam lanes + scene apply logic that create/update/remove seam-owned object proxy roles from decoded object feed entries.
- Preserve dirty-only apply behavior in `viewer_app` and seam-only create/remove ownership in `viewer_core::Scene`.

Out of scope:
- Full prim/mesh/material parity decode for every object update payload field.
- Protocol-to-render shortcuts that bypass `viewer_core` seam ownership.
- Unlimited world/object decode breadth or unbounded object retention.
- Multi-region continuity/handoff expansion (belongs to later `N07`).

## Current known state
- `viewer_net` currently classifies and minimally decodes handshake/early-traffic signals but does not type object-update family traffic yet (`crates/viewer_net/src/lib.rs`).
- `LiveVisualSnapshot` currently carries coarse/health/viewer-time decoded fields but no bounded object entry list (`crates/viewer_core/src/lib.rs`, `crates/viewer_app/src/main.rs`).
- `WorldObjectIngestionSeam` currently synthesizes object-state entity lanes from coarse+health diagnostics rather than decoded object-family updates (`crates/viewer_core/src/lib.rs`).
- `viewer_app` already enforces dirty-only seam/snapshot apply, which must remain unchanged (`crates/viewer_app/src/main.rs`).
- Firestorm protocol references for this milestone:
  - `reference/firestorm/scripts/messages/message_template.msg`
  - `reference/firestorm/indra/newview/llstartup.cpp`
  - `reference/firestorm/indra/newview/llviewerobjectlist.cpp`
  - `reference/firestorm/indra/newview/llviewermessage.cpp`

## Files and components touched
- `crates/viewer_net/src/lib.rs`
  - packet ID constants, classification, bounded object-family decode, typed decode summary/state.
- `crates/viewer_core/src/lib.rs`
  - `LiveVisualSnapshot` object-feed fields/types, seam lane/types, scene apply lifecycle behavior for object-feed entities.
- `crates/viewer_app/src/main.rs`
  - snapshot population from `viewer_net::Connection` decode summary; keep dirty-only apply semantics intact.
- `docs/RESEARCH/post_amc_bootstrap_boundary_map.md` (if needed during implementation)
  - only if new typed packet IDs or scope boundaries are added and require observation note updates.
- tests in touched crates:
  - `viewer_net` protocol decode/classification coverage.
  - `viewer_core` seam + lifecycle ownership coverage.
  - `viewer_app` seam/snapshot dirty-apply regression checks where behavior is affected.

## Boundary check
- `viewer_net` owns transport mechanics, packet classification, and bounded decode extraction.
- `viewer_grid` remains semantic/policy owner; no semantic collapse into `viewer_net`.
- `viewer_core` owns shared typed ingestion contracts and seam-owned scene lifecycle.
- `viewer_app` remains orchestration-only and must not create/remove seam-owned roles directly.
- `viewer_render` remains unchanged as GPU owner; no `wgpu` handle leakage across boundaries.

## Step sequence
1. Protocol ID and classification lock
   - Add object-family message IDs sourced from `message_template.msg`.
   - Classify object-family inbound traffic explicitly (instead of leaving it under generic `Unknown`).
2. Bounded object decode primitives
   - Introduce typed structs for minimally required object feed fields (identity, update kind, transform/scale where present, lifecycle/remove signals).
   - Decode only bounded, well-validated fields; count and drop malformed/partial payloads without panics.
3. Connection-side bounded state and lifecycle
   - Add bounded per-object tracking keyed by local/object ID with explicit create/update/remove/stale rules.
   - Integrate `KillObject` handling as remove signal.
   - Add decode summary counters for creates/updates/removes/drops/evictions.
4. Snapshot bridge extension
   - Extend `SimulatorPayloadDecodeSummary` and `LiveVisualSnapshot` with bounded object-feed exports (size-capped list + counters).
   - Preserve sensitive-data boundary: no raw credentials/session secrets in snapshot.
5. Seam contract expansion
   - Add object-feed seam lane(s) and payload fields needed for scene lifecycle operations.
   - Keep seam payload bounded and deterministic (stable ordering/capping).
6. Scene apply lifecycle implementation
   - Implement seam-driven create/update/remove for object-feed proxies inside `Scene::apply_world_object_ingestion_seam(...)` only.
   - Preserve existing seam-derived diagnostic roles unless explicitly superseded by plan-approved migration.
7. App integration + dirty guards
   - Populate new snapshot fields from connection decode summary.
   - Verify seam and snapshot dirty checks still gate per-frame application.
8. Closeout and evidence
   - Confirm no architecture drift.
   - Prepare implementation review and execution report artifacts after implementation milestone execution.

## Validation plan
- Always run:
  - `cargo fmt`
  - `cargo check`
- Targeted tests:
  - `cargo test -p viewer_net` (object-family classification/decode/lifecycle counters)
  - `cargo test -p viewer_core` (seam lane mapping + seam-owned lifecycle behavior)
  - `cargo test -p viewer_app` (dirty-only apply and snapshot bridge behavior), if app tests are materially touched.
- Broader tests:
  - `cargo test` when cross-crate behavior changes materially.
- Runtime verification:
  - `cargo run -p viewer_app` with bounded startup mode (`VIEWER_APP_LIVE_STARTUP=off` fallback smoke first; live bounded run second when available) to confirm richer object-feed proxies appear and remove deterministically.

## Risks and open questions
Risks:
- LLUDP object-family payload structure is variable and easy to over-decode; strict bounded parsing is required.
- Lifecycle drift risk if object removal/expiry rules are underspecified.
- Snapshot growth risk if object export caps are not enforced.
- Mis-typed message IDs could silently distort classification; IDs must come from `message_template.msg` (L10).

Open questions:
- Exact cap values for exported object entries per snapshot/frame (proposed: plan-time default in implementation with explicit constant + test coverage).
- Whether existing synthetic object-state diagnostic roles should coexist with object-feed roles during transition, or be partially retired inside N03 scope.

## Deferred-too-early candidates captured
- Full per-face/material/texture decode from object update payloads (deferred to `A06`).
- Object-cache fetch/miss retry protocol parity beyond bounded local decode (`ObjectUpdateCached` deep handling) (deferred to `N07`+).
- Multi-region object continuity/handoff semantics tied to `CrossedRegion`/`ConfirmEnableSimulator` (deferred to `N07`).

## Completion criteria
- Object-family LLUDP traffic is explicitly classified and minimally decoded through bounded typed paths.
- `LiveVisualSnapshot` exposes a bounded object feed sufficient for seam ingestion without sensitive-data leakage.
- `WorldObjectIngestionSeam` and `Scene::apply_world_object_ingestion_seam(...)` perform lifecycle-safe create/update/remove for object-feed proxies.
- Dirty-only apply semantics remain intact and validated.
- Required validation commands run and are documented with pass/fail/unvalidated status during execution.
