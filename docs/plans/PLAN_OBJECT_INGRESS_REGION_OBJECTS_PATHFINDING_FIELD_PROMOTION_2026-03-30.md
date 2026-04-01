# Plan: Object Ingress RegionObjects Pathfinding Field Promotion (2026-03-30)

## Scope
Promote the now-identified `RegionObjects` pathfinding-linkset payload into a small typed, evidence-backed field set so the repo can work with named fields instead of only summary strings.

Out of scope:
- world/render integration
- broad schema coverage for every `RegionObjects` variant
- LLUDP object-update parity work

## Current known state
- The primary simulator `RegionObjects` capability is now proven to return pathfinding-linkset/object payloads.
- Firestorm evidence shows:
  - `reference/firestorm/indra/newview/llpathfindingobject.cpp` defines base fields such as `name`, `description`, `owner`, `position`, and `owner_is_group`
  - `reference/firestorm/indra/newview/llpathfindinglinkset.cpp` defines `landimpact`, `modifiable`, `navmesh_category`, `can_be_volume`, `phantom`, and walkability coefficients `A-D`
- The current live relay already classifies first child maps as `pathfinding_linkset` and derives semantics like `linkset_use=dynamic_phantom`.

## Files and components expected
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- continuity artifacts and execution report

## Boundary check
- `viewer_net` owns bounded typed extraction and normalization.
- `viewer_app` owns bounded relay/log presentation.
- No UI/render changes are authorized.

## Step sequence
1. Extend `RegionObjectsInspection` with a small typed field summary for pathfinding objects/linksets.
2. Populate it from the already-proven UUID-keyed child maps without broadening into full schema modeling.
3. Update the live relay so the first surfaced objects show named fields such as `name`, `description`, `owner`, `landimpact`, `navmesh_category`, `position`, and normalized booleans when present.
4. Re-run one bounded live session and confirm those typed fields appear in logs.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- bounded connected run: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`

## Risks and open questions
- Some pathfinding fields may be absent or encoded differently across objects.
- Position may require special formatting because it is likely array-valued rather than scalar.
- The first returned objects may not include all base object fields in the bounded window.

## Deferred-too-early candidates captured
- Full `RegionObjects` domain modeling remains deferred.
- Any seam/scene integration of `RegionObjects` remains deferred.

## Learnings pre-check
- L43: `RegionObjects` is already a valid object-related ingress lane even without LLUDP `ObjectUpdate*`.
- L44: bounded child-map extraction can expose stable inner fields before LLUDP parity exists.
- L45: the current child maps align with Firestorm pathfinding linkset/object schema, and bool-like `0/1` values must be normalized before deriving semantics.

## Exact completion criteria
- The repo surfaces at least one typed named field group from live `RegionObjects` pathfinding payloads.
- The next follow-up remains evidence-led and does not regress into speculative protocol guessing.
