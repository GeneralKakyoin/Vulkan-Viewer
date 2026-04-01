# Plan: Object Ingress RegionObjects Tuple Content Interpretation (2026-03-31)

## Scope
Use the now-correct typed `RegionObjects` lane to determine whether the six-value tuple-like `description` strings should be treated as:
- ordinary object description content
- or a bounded, repeated content pattern worth promoting separately

Out of scope:
- LLUDP object-update parity work
- scene/render integration
- broad modeling of all `RegionObjects` payload families

## Current known state
- Live `RegionObjects` pathfinding-linkset records now clearly include `position`, with `position_shape=llsd_array_len3`.
- The prior `...no_position` labels were misleading; the current lane now surfaces `...with_position`.
- Firestorm `LLPathfindingObject` still treats `description` as a plain string field.
- Some live records still carry repeated six-value tuple-like description strings such as `0,10.000000,30,0,2,0`.

## Files and components expected
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/OBJECT_INGRESS_STATUS.md`
- Firestorm pathfinding references
- continuity artifacts and execution report

## Boundary check
- `viewer_net` owns bounded tuple-content inspection helpers.
- `viewer_app` owns bounded relay/log wording only.
- No UI/render or LLUDP transport changes are authorized.

## Step sequence
1. Gather the currently surfaced tuple-like descriptions and compare them against the same records’ names, positions, and other typed fields.
2. Re-check Firestorm pathfinding references for any evidence that those values map to a known editor/property presentation rather than plain description text.
3. If evidence supports it, add one bounded content hint for the tuple values without replacing the raw description string.
4. Re-run one bounded live session and confirm the relay can explain the tuple content more clearly than the current raw string alone.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- bounded connected run with captured logs

## Risks and open questions
- The tuple-like value may simply be creator-authored object description text with no protocol-level meaning.
- Over-decoding the tuple would be worse than leaving it raw.
- A broader live sample may show more than one tuple shape.

## Deferred-too-early candidates captured
- Treating tuple values as authoritative geometry or navmesh semantics remains deferred until stronger evidence exists.
- Any scene/world promotion of `RegionObjects` data remains deferred.

## Learnings pre-check
- L45: the proven field family aligns with Firestorm pathfinding linkset/object schema.
- L46: typed `RegionObjects` summaries can surface real names/owners, and mixed content shapes can appear inside the same lane.
- L47: live `RegionObjects` linkset records do include `position`; missing-position conclusions must distinguish extraction visibility from true field absence.

## Exact completion criteria
- The repo either proves the tuple-like description is just raw string content worth leaving alone, or adds one bounded evidence-backed content hint.
- The next branch remains evidence-led and does not widen into speculative transport work.
