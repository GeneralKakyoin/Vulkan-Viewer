# Plan: Render Object Feed Parent Placement and Texture Coverage Hardening (2026-04-10)

## Objective
Fix live object-feed clumping and improve texture-coverage observability by carrying object parent relationships through net -> app -> core and applying parent-relative placement rules in scene ingestion.

## Scope
- Add decoded object parent-id propagation from LLUDP decode (`viewer_net`) to seam payload (`viewer_core`) via app mapping (`viewer_app`).
- In `viewer_core` object-feed upsert/transform path, place child objects relative to existing parent transform when parent id is present.
- Skip first-sighting child creation when parent transform is unavailable to prevent origin clumps.
- Add bounded diagnostics counters for parented vs root object-feed exports.
- Add targeted tests for parent-id decode and scene placement behavior.

## Current known state
- Live runs show object instance counts near ~900 with visible clustering at one location.
- Current decode and seam types retain local_id/position/rotation/mesh/material fields, but do not retain parent_id.
- Existing first-sighting no-position guard is in place, but does not cover parent-relative placement ambiguity.
- Texture parity remains inconsistent; logs can show zero exported texture ids in bad runs.

## Files and components touched
- `crates/viewer_net/src/object_decode_utils.rs`
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_core/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `crates/viewer_app/src/object_feed_diagnostics_utils.rs`
- continuity/docs artifacts for this task

## Boundary check
- `viewer_net` change limited to transport/decode payload extraction and summary fields.
- `viewer_app` limited to snapshot mapping + diagnostics relay.
- `viewer_core` limited to scene/seam object placement logic.
- No crate-boundary ownership shifts.

## Step sequence
1. Extend object ingress structs/exports with optional `parent_local_id`.
2. Decode parent id from `ObjectUpdate` and `ObjectUpdateCompressed` payloads (when present).
3. Map parent id into `LiveVisualSnapshot` object list in `viewer_app`.
4. Add parent id field to `WorldObjectIngestionItem` / `DecodedWorldObjectFeedObject` and seam mapping.
5. Update scene object-feed placement to resolve child position against parent transform and guard missing-parent first sightings.
6. Add parent-aware regression tests in `viewer_net` and `viewer_core`.
7. Add object-feed diagnostics relay fields for parented/root counts.
8. Run fmt/check/targeted tests and one bounded live run with proof flags.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_core -p viewer_app`
- targeted tests:
  - parent decode test(s) in `viewer_net`
  - parent placement test(s) in `viewer_core`
- bounded live run:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_APP_RENDER_PROOF=on`
  - proof log + network debug log capture and inspection

## Risks and open questions
- LL child transform semantics include parent rotation/scale nuances; first pass will use bounded parent-position anchoring to eliminate pileups without speculative full hierarchy math.
- Live ingress availability may vary per run; proof failure with zero object feed is non-actionable for placement correctness.

## Deferred-too-early candidates captured
- Full hierarchical parent-child transform composition (rotation/scale propagation) for all object families deferred pending stable parent-id field verification.

## Learnings pre-check
- L76 applies: decode positional payloads from bounded object-update families.
- L81 applies: avoid offset drift in payload decode changes.
- L82 applies: keep edits crate-local and behavior-local.

## Completion criteria
- Parent id is preserved from decode to seam payload.
- Scene no longer creates parented child proxies at unresolved fallback anchor locations on first sighting.
- Targeted tests pass.
- Live proof/log artifacts produced with explicit pass/fail interpretation.
