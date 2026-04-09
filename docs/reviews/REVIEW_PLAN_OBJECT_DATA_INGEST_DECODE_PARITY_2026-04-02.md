# Review: Plan Object Data Ingestion/Decoding Parity 2026-04-02

## Verdict
Approved.

## Architecture And Boundary Fit
- Plan keeps decode mechanics in `viewer_net`, mapping in `viewer_app`, and seam/scene behavior in `viewer_core`.
- No net/grid ownership collapse.
- Firestorm/OpenSim use is protocol-reference only.

## Correctness Concerns
- Compressed/terse parsing must strictly honor per-block lengths and optional-field ordering.
- `ObjectUpdateCached` should remain ID/flags only; do not infer absent spatial payload.
- Position quantization/normalization should avoid introducing unstable proxy jitter.

## Modularity And Maintainability Concerns
- New per-object decoded fields should remain additive and serde-default compatible.
- Keep parsing helpers narrow and reusable; avoid embedding complex parsing in observation dispatch.

## Validation Adequacy
- Proposed validation ladder is sufficient for this bounded cross-crate change (`fmt`, `check`, targeted crate tests).

## Risks And Open Questions
- Potential malformed packet regressions if offset arithmetic is incorrect.
- Need explicit tests for each packet family to lock offsets.

## Learnings Delta Verdict
- none
- Reason: plan-phase review only; durable learning decision deferred to implementation report.

## Required Revisions Or Approval Status
- Approval granted; proceed with implementation as scoped.
