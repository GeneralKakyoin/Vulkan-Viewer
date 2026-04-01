# Plan: Object Ingress RegionObjects Pathfinding Variant Discrimination (2026-03-31)

## Scope
Use the new typed `RegionObjects` pathfinding summary lane to distinguish the live payload variants we are actually receiving, rather than assuming every UUID-keyed child map is the same shape.

Out of scope:
- LLUDP object-update parity work
- scene/render integration
- broad domain modeling of every possible `RegionObjects` payload

## Current known state
- The repo now surfaces typed pathfinding/object fields from the `RegionObjects` lane.
- Live evidence shows mixed-looking records:
  - some entries have sensible names and `(No Description)`
  - some entries surface comma-separated numeric payloads in `description`
  - a separate `position` field did not appear in the bounded first-object summary
- Firestorm references still indicate the canonical object fields are `name`, `description`, `owner`, `position`, and `owner_is_group`, with additional linkset/pathfinding fields layered on top.

## Files and components expected
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/OBJECT_INGRESS_STATUS.md`
- Firestorm pathfinding references already in use
- continuity artifacts and execution report

## Boundary check
- `viewer_net` owns bounded variant detection and extraction support.
- `viewer_app` owns bounded relay/log clarity.
- No UI/render changes are authorized.

## Step sequence
1. Inspect the first typed live records and determine which fields are consistently present versus variant-specific.
2. Add bounded variant labeling or shape hints when a child map deviates from the canonical pathfinding-object field set.
3. Surface whichever raw/typed field is needed to explain the comma-like `description` cases and the missing `position` cases.
4. Add a separate continuity file that explicitly tracks what is proven working, what is proven not working, and the current open questions on object ingress.
5. Re-run one bounded live session and confirm the repo can explain at least one of those live variant differences.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- bounded connected run: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`

## Risks and open questions
- The variant difference may reflect a different pathfinding record type rather than malformed data.
- The first few returned objects may still not be representative of the broader region payload.
- The relay must remain bounded and readable despite the extra discrimination detail.

## Deferred-too-early candidates captured
- Full schema reverse-engineering remains deferred.
- World/seam integration of `RegionObjects` remains deferred.

## Learnings pre-check
- L45: the currently proven field family aligns with Firestorm pathfinding linkset/object schema, and bool-like `0/1` values must be normalized before deriving semantics.
- L46: the new typed summary lane can surface real names/owners, but the live payload still appears to contain multiple field-shape variants.

## Exact completion criteria
- The repo can explain at least one live `RegionObjects` pathfinding variant difference more clearly than the current typed summary does.
- `docs/OBJECT_INGRESS_STATUS.md` exists and separates working versus not-working object-ingress behavior.
- The next step remains evidence-led and bounded.
