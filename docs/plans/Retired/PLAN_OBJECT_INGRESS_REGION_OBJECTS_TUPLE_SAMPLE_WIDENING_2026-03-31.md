# Plan: Object Ingress RegionObjects Tuple Sample Widening (2026-03-31)

## Scope
Use the new tuple slot-analysis surface to gather a broader live sample before naming any tuple semantics.

Out of scope:
- LLUDP object-update parity work
- scene/render integration
- speculative tuple-field naming

## Current known state
- The bounded tuple analysis now shows a repeatable six-slot pattern on the live `RegionObjects` lane.
- In the current live sample:
  - slots `0/1/2/3/5` are constant
  - slot `4` varies between `2` and `3`
  - both tuple samples currently belong to `DSS Candlier Frame`
- Firestorm still treats `description` as a string field.

## Files and components expected
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- continuity artifacts and execution report

## Boundary check
- `viewer_net` owns any bounded widening of tuple sample collection.
- `viewer_app` owns bounded relay/log output only.
- No UI/render or LLUDP transport changes are authorized.

## Step sequence
1. Widen the bounded tuple sample collection enough to capture more than the current two tuple records when available.
2. Compare whether slot `4` remains the only varying slot across a broader live sample.
3. Check whether tuple variation correlates with object identity, position clusters, or repeated same-name records.
4. Only if a stronger pattern appears, add one more bounded content hint.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- bounded connected run with captured logs

## Risks and open questions
- The current region may simply not contain enough tuple-style records.
- The broader sample may reveal multiple tuple families and reduce confidence instead of increasing it.
- Overfitting the current same-name sample remains the main risk.

## Deferred-too-early candidates captured
- Naming tuple slots semantically remains deferred until the broader sample supports it.

## Learnings pre-check
- L47: missing canonical fields must be distinguished from extraction visibility gaps.
- L48: bounded tuple slot analysis can reveal stable and varying slots, but that alone is not enough to assign semantics.

## Exact completion criteria
- The next bounded live run either broadens the tuple sample or explicitly proves the current region only exposes a narrow sample.
- The next branch remains evidence-led and bounded.
