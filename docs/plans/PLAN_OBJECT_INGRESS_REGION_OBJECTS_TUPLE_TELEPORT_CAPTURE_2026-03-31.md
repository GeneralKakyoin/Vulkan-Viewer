# Plan: Object Ingress RegionObjects Tuple Teleport Capture (2026-03-31)

## Scope
Move the tuple-capture branch from a longer same-region run to a bounded region-change / teleport capture, using the existing tuple-analysis surface without further decoder changes.

Out of scope:
- LLUDP object-update parity work
- scene/render integration
- speculative tuple-field naming

## Current known state
- The code-side tuple widening is already in place.
- A longer same-region capture still produced only:
  - `samples=2`
  - `names=DSS Candlier Frame`
- That means duration alone is not broadening the tuple family in the current region.

## Files and components expected
- continuity artifacts and execution report
- existing runtime logs
- existing network-debug tuple-analysis surface

## Boundary check
- No new architecture or cross-crate behavior changes are required unless a small capture-control knob becomes necessary.
- The focus remains evidence gathering, not protocol implementation.

## Step sequence
1. Run a bounded capture that includes a region change or teleport while the existing tuple-analysis instrumentation is active.
2. Compare whether the tuple family broadens beyond the current same-name pair.
3. If it does, check whether slot `4` remains the only varying slot across regions.
4. If it still does not, conclude the currently observed tuple pattern is likely object-local content and stop spending more slices trying to decode it.

## Validation plan
- bounded connected live run with captured logs
- no code validation unless a capture-control change becomes necessary

## Risks and open questions
- Region changes may surface multiple unrelated tuple families.
- Manual teleporting introduces operator-dependent timing.
- The tuple family may still remain narrow even across regions.

## Deferred-too-early candidates captured
- Tuple semantics remain deferred until a broader cross-region sample exists.

## Learnings pre-check
- L48: repeatable slot structure does not yet imply semantics.
- L49: once code-side tuple widening is in place, a narrow sample may indicate region/content scarcity rather than an extraction cap.
- L50: if a longer same-region capture does not broaden the tuple family, the next evidence step should change region rather than only extend time.

## Exact completion criteria
- The next evidence run either broadens the tuple family across a region change or justifies treating the current tuple as object-local rather than generally decodable.
