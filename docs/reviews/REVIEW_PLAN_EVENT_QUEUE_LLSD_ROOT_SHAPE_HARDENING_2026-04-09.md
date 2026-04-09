# Review: Plan EventQueue LLSD Root Shape Hardening (2026-04-09)

## Verdict
Approved.

## Architecture and boundary fit
- Scope stays inside `viewer_net` decode logic where capability LLSD parsing belongs.
- No cross-crate interface or ownership violations.

## Correctness concerns
- Treating `<undef />` as empty-success is appropriate for EventQueue long-poll timeout-style semantics but should remain bounded to LLSD roots only.
- Array-root fallback should parse only the first map to avoid ambiguous multi-item semantics.

## Modularity and maintainability concerns
- Shared helper for LLSD root handling is preferred over repeated ad hoc `<map>` scans.
- Keep helper local to `viewer_net/src/lib.rs` in this slice; broader extraction is deferred.

## Validation adequacy
- Planned checks (`fmt`, `check`, targeted EventQueue tests) are sufficient for this bounded decoder hardening.

## Risks and open questions
- Unknown if all observed `missing llsd map` failures are truly `<undef />`; some may still be non-LLSD HTTP bodies.
- If failures persist after this patch, next branch should add bounded body-shape diagnostics for EventQueue decode failures.

## Learnings delta verdict
none: No new durable learning before implementation; existing L64/L68/L69 constraints already cover this slice.

## Required revisions or approval status
Approved with no required revisions.
