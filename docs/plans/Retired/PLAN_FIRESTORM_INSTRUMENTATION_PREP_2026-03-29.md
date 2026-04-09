# PLAN: Firestorm Instrumentation Prep (2026-03-29)

## Scope
Prepare a practical Firestorm instrumentation kit so we can capture authoritative runtime behavior for login->handoff->object updates and compare to this viewer.

## Current Known State
- Our viewer logs show login success + simulator UDP traffic (layer/coarse/ping).
- Object updates are still zero.
- We need Firestorm ground-truth traces from the same account/region.

## Files/Components
- `docs/RESEARCH/FIRESTORM_INSTRUMENTATION_PLAYBOOK_2026-03-29.md`
- `tools/firestorm/compare_proto_logs.ps1`
- review/report artifacts.

## Boundary Check
- No runtime crate behavior changes in this prep slice.
- Docs/tools only.

## Step Sequence
1. Identify exact Firestorm hook points for packet send/receive path.
2. Produce copy-paste playbook with minimal invasive log lines.
3. Add local helper script to summarize Firestorm vs viewer log signals.
4. Record prep report.

## Validation Plan
- Verify referenced Firestorm paths/functions exist in `reference/firestorm`.
- Run helper script syntax check by executing help/usage path.

## Risks/Open Questions
- Firestorm user build environment can differ by machine.
- Logging volume may need temporary filtering in Firestorm log settings.

## Learnings Pre-check
- L05 and L10 apply: preserve unknown traffic visibility and template-driven IDs.

## Completion Criteria
- User can patch Firestorm quickly, run once, and extract comparable protocol counters.
