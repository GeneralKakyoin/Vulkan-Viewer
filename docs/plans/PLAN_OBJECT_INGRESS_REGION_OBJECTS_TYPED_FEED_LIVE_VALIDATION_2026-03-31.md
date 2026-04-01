# Plan: Object Ingress RegionObjects Typed Feed Live Validation (2026-03-31)

## Scope
- run one bounded live session against the current typed `RegionObjects` feed promotion
- confirm the new `typed_sample=...` relay appears on-wire during login and reconnect/region change
- record the result in continuity/docs

Out of scope:
- new code changes unless the live run reveals a trivial logging defect
- LLUDP object-ingress fixes
- wider `RegionObjects` field expansion

## Current known state
- The typed `RegionObjects` feed promotion is code-validated.
- The reconnect-based SLURL teleport path is implemented and previously produced a successful region-change capture.
- The remaining gap is live validation of the new `typed_sample=...` relay formatting.

## Files and components touched
- live artifacts under `artifacts/logs/`
- continuity/report artifacts

## Boundary check
- No architectural widening.
- Prefer artifact capture and documentation over additional code.

## Step sequence
1. Run one bounded live session with:
   - live startup on
   - auto teleport enabled
   - dedicated stdout/err and JSONL artifact paths
2. Inspect the resulting `region_objects` relay lines for the new `typed_sample=...` summary.
3. Record whether the summary stayed readable across both the initial region and the reconnect target.
4. Update continuity docs and report with the exact result.

## Validation plan
- bounded direct-binary live run with captured artifacts
- no additional compile/test cycle unless a code change becomes necessary

## Risks and open questions
- The run depends on working local credentials in `.env` / environment.
- The reconnect target may vary in content, so success should be judged by the presence and readability of `typed_sample=...`, not by specific object names.

## Deferred-too-early candidates captured
- none

## Learnings pre-check
- L51: keep SLURL teleport on the reconnect/login path.
- L52: stable cross-region `RegionObjects` fields are now the primary target, not tuple-first decoding.

## Exact completion criteria
- A bounded live run is captured successfully.
- The result clearly states whether `typed_sample=...` appeared and remained readable across region change.
- Continuity/report docs reflect that live result.
