# Plan: Object Ingress RegionObjects Tuple Slot Analysis (2026-03-31)

## Scope
Extend the current `RegionObjects` tuple-content investigation with bounded slot-level analysis so the next live run can show:
- which tuple slots are constant
- which tuple slots vary
- which typed object fields co-occur with the observed tuple variants

Out of scope:
- LLUDP object-update parity work
- scene/render integration
- speculative naming of tuple slots without evidence

## Current known state
- The current typed `RegionObjects` lane already surfaces:
  - `position`
  - `position_shape`
  - tuple-like `description` content as `description_tuple=...`
- Live evidence now shows these tuple-description records still carry valid `position`.
- Firestorm still treats `description` as a plain string field in `LLPathfindingObject`.

## Files and components expected
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/OBJECT_INGRESS_STATUS.md`
- continuity artifacts and execution report

## Boundary check
- `viewer_net` owns bounded tuple sample aggregation and slot analysis.
- `viewer_app` owns bounded relay/log presentation only.
- No UI/render or LLUDP transport changes are authorized.

## Step sequence
1. Add bounded tuple sample aggregation to the `RegionObjects` inspection path.
2. Derive slot-level analysis from the observed tuple samples:
   - tuple count
   - slot count
   - constant vs varying slots
   - small bounded distinct-value sets per slot
3. Add one bounded correlation surface tying tuple samples to existing typed fields such as `name` and `position`.
4. Re-run one bounded live session and record whether the tuple content now shows a repeatable pattern strong enough to support a future hint.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- bounded connected run with captured logs

## Risks and open questions
- The sample set may remain too small to justify anything beyond “mostly constant except slot N”.
- Slot variation may correlate with nothing meaningful in the bounded run.
- Overfitting a small sample would be worse than keeping the tuple raw.

## Deferred-too-early candidates captured
- Naming tuple slots semantically remains deferred until stronger evidence exists.
- Any use of tuple content for world/object modeling remains deferred.

## Learnings pre-check
- L45: the proven field family aligns with Firestorm pathfinding linkset/object schema.
- L46: mixed content shapes can appear inside the same typed lane.
- L47: missing canonical fields must be distinguished from extraction visibility gaps.

## Exact completion criteria
- The relay/log now shows bounded tuple slot analysis rather than only raw tuple strings.
- The next live run identifies at least one constant slot and at least one varying slot, or explicitly proves the sample was too small.
- The next step stays evidence-led and bounded.
