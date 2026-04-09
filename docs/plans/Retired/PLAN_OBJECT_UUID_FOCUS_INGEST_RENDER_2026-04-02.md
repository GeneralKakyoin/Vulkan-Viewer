# PLAN_OBJECT_UUID_FOCUS_INGEST_RENDER_2026-04-02

## Scope
Implement a bounded single-object focus path so live ingestion/render can target exactly one world object UUID (`10930d3b-1821-c584-a0c7-28a34999800d`) instead of the whole object feed.

## Current Known State
- LLUDP object feed currently exports by `local_id` only.
- `ObjectUpdate` decode already traverses `FullID` bytes but does not retain/export UUID.
- `viewer_app` maps decoded object feed directly into `LiveVisualSnapshot.decoded_object_feed_objects` and scene seam renders all exported objects.

## Files and Components Touched
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_core/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/TESTING_REFERENCE.md`
- continuity/report/review docs

## Boundary Check
- `viewer_net`: add protocol-level UUID retention/export for object-feed entries.
- `viewer_core`: extend snapshot object-feed contract with optional object UUID (additive field).
- `viewer_app`: add orchestration-only env filter (`VIEWER_APP_OBJECT_UUID_FOCUS`) and apply filtered object feed to snapshot.
- No architecture borrowing from Firestorm/OpenSim; behavior-only parity.

## Step Sequence
1. Add optional `object_id` to decoded object-feed contracts (`viewer_net` + `viewer_core`).
2. Persist `ObjectUpdate` FullID UUID in `viewer_net` object feed state and export it.
3. Add `VIEWER_APP_OBJECT_UUID_FOCUS` config parse + validation in `viewer_app`.
4. Filter snapshot object-feed entries to the focused UUID when configured.
5. Add/adjust targeted unit tests for decode export and app filtering.
6. Run fmt/check/tests and bounded live probe with focus UUID.

## Validation Plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_core -p viewer_app`
- targeted tests:
  - `viewer_net` object-feed export includes object UUID
  - `viewer_app` focus filter keeps only matching object UUID
- bounded live run with:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_ASSET_SOURCE_MODE=live`
  - `VIEWER_LOGIN_START=secondlife://Puppy/140/178/24`
  - `VIEWER_APP_OBJECT_UUID_FOCUS=10930d3b-1821-c584-a0c7-28a34999800d`

## Risks and Open Questions
- Some objects may arrive first via compressed/cached updates lacking full UUID; focus may initially appear empty until a full `ObjectUpdate` for that local id arrives.
- If no `ObjectUpdate` with FullID is observed during bounded capture, next probe is longer capture window.

## Deferred-too-early Candidates
- No deferred candidates identified for this bounded slice.

## Learnings Pre-check
- L03: seam-owned roles remain seam-only.
- L04: preserve dirty-only snapshot apply behavior.
- L74: RegionObjects may not expose mesh IDs; do not assume mesh candidate from linkset UUID.

## Completion Criteria
- Focus env var recognized and validated.
- Live snapshot contains only matching object-feed entry when focus var is set and matching UUID observed.
- Bounded live artifact shows object feed tick summaries with reduced/single-object feed under focus mode.
- Continuity/report/review docs updated.
