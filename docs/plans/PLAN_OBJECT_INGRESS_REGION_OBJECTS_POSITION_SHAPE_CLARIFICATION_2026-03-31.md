# Plan: Object Ingress RegionObjects Position Shape Clarification (2026-03-31)

## Scope
Tighten the typed `RegionObjects` pathfinding lane so the next bounded live run can distinguish:
- `position` truly absent from the live child map
- `position` present but not yet decoded by our extractor
- `description` remaining a plain string field even when its content looks tuple-like

Out of scope:
- LLUDP object-update parity work
- scene/render integration
- broad reverse-engineering of every `RegionObjects` payload family

## Current known state
- The repo already surfaces typed `RegionObjects` pathfinding summaries plus bounded description-shape hints.
- Firestorm source shows `LLPathfindingObject` always parses `description` as a string field and expects `position` as a first-class object field:
  - `reference/firestorm/indra/newview/llpathfindingobject.cpp`
  - `reference/firestorm/indra/newview/llpathfindingobject.h`
- Live evidence still shows tuple-like `description` content on some records, but the current variant hints collapse two different possibilities into one label:
  - `position` may really be missing
  - or `position` may be present in a shape our extractor is not summarizing yet

## Files and components expected
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/OBJECT_INGRESS_STATUS.md`
- continuity artifacts and execution report

## Boundary check
- `viewer_net` owns raw child-field inspection, bounded typed summary enrichment, and tests.
- `viewer_app` owns bounded relay/log wording only.
- No UI/render changes are authorized.

## Step sequence
1. Add bounded `position` inspection support for `RegionObjects` child maps so typed summaries can record whether the key exists and what raw shape it had.
2. Update the pathfinding variant hinting so it distinguishes:
   - decoded `position`
   - present-but-unparsed `position`
   - missing `position` key
3. Surface the new `position` evidence in the relay/network-debug summary without expanding into raw-body dumps.
4. Re-run one bounded live session and use the result to decide whether the next branch is:
   - better `position` extraction
   - or accepting that the current live linkset lane genuinely omits `position`

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- bounded connected run:
  - `VIEWER_APP_LIVE_STARTUP=on VIEWER_NETWORK_DEBUG_LOG_PATH=<bounded-log> cargo run -p viewer_app`

## Risks and open questions
- The live lane may genuinely omit `position` even though Firestorm pathfinding objects normally carry it.
- The raw `position` value may use a shape not yet seen in local tests.
- The tuple-like `description` values may simply be ordinary object description content, not a protocol subtype.

## Deferred-too-early candidates captured
- Decoding the semantic meaning of every tuple element remains deferred until `position` presence is clarified.
- Any scene/world promotion of `RegionObjects` data remains deferred.

## Learnings pre-check
- L45: the proven field family aligns with Firestorm pathfinding linkset/object schema, and bool-like `0/1` values must be normalized before deriving semantics.
- L46: typed `RegionObjects` summaries can surface real names/owners, but the live lane may still contain multiple field-shape variants.

## Exact completion criteria
- The typed `RegionObjects` summary can now tell whether `position` was decoded, merely present, or absent.
- The next bounded live run answers the current `position` uncertainty more precisely than the existing `...no_position` labels do.
- The next follow-up branch is narrowed using that evidence.
