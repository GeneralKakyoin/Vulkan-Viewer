# Plan: Object Ingress RegionObjects Tuple Multi-Region Capture (2026-03-31)

## Scope
Move the tuple widening branch from code changes to capture-scope changes by gathering tuple samples across a longer or different-region live session.

Out of scope:
- LLUDP object-update parity work
- scene/render integration
- speculative tuple-field naming

## Current known state
- The code-side tuple sample widening is already in place.
- In the current region and bounded window, the widened capture still produced only:
  - `samples=2`
  - `names=DSS Candlier Frame`
- That means the current bottleneck is no longer the code-side sample cap.

## Files and components expected
- continuity artifacts and execution report
- existing runtime logs
- optionally bounded live-capture settings if needed

## Boundary check
- No new architecture or cross-crate behavior changes are required for this slice unless a small capture-bound knob is needed.
- The focus is evidence gathering, not protocol implementation.

## Step sequence
1. Run a longer bounded capture or a short multi-region/teleport capture using the existing tuple-analysis surface.
2. Compare whether tuple samples broaden beyond the current same-name pair.
3. If they do, re-check whether slot `4` remains the only varying slot.
4. If they do not, conclude the current tuple pattern may be object-local rather than broadly region-local.

## Validation plan
- bounded connected live run(s) with captured logs
- no additional code validation unless a small capture-bound knob is introduced

## Risks and open questions
- The target region may simply not contain many tuple-style records.
- A multi-region run may mix unrelated tuple families and require a split in analysis.
- Longer captures increase noise if not kept bounded.

## Deferred-too-early candidates captured
- Tuple semantics remain deferred until a broader sample exists.

## Learnings pre-check
- L48: repeatable slot structure does not yet imply semantics.
- L49: once code-side tuple widening is in place, a narrow sample may indicate region/content scarcity rather than an extraction cap.

## Exact completion criteria
- The next evidence run either broadens the tuple sample beyond the current same-name pair or explicitly proves the current environment is too narrow to justify further tuple decoding.
