# Plan: Firestorm vs Viewer Startup LLUDP First-Divergence Diff (2026-04-03)

## Scope
- Perform bounded startup parity investigation (no architecture change) to identify earliest LLUDP send-sequence divergence between Firestorm capture and current viewer behavior.
- Focus on first outbound viewer->simulator message sequence and adjacent receive evidence.

## Current Known State
- Firestorm Fidelis pcap exists and decodes to LLUDP traffic on `16.146.10.127:13015`.
- Recent strict-mode run showed no inbound `RegionHandshake` and no object updates.
- Need concrete first-difference evidence before changing behavior again.

## Files / Components Touched
- `docs/plans/PLAN_FIRESTORM_STARTUP_FIRST_DIVERGENCE_DIFF_2026-04-03.md`
- `docs/reviews/REVIEW_PLAN_FIRESTORM_STARTUP_FIRST_DIVERGENCE_DIFF_2026-04-03.md`
- `docs/reports/REPORT_FIRESTORM_STARTUP_FIRST_DIVERGENCE_DIFF_2026-04-03.md`
- optional generated artifact under `artifacts/logs/`

## Boundary Check
- Investigation-only; no runtime behavior change in this slice.
- Uses existing artifacts and bounded fresh live run for comparison.

## Step Sequence
1. Extract Firestorm startup LLUDP send sequence from pcap (`viewer->sim`).
2. Capture fresh non-strict viewer startup run and extract startup send transcript.
3. Normalize message labels and compare first 20 outbound sends.
4. Identify first divergence and supporting receive-side context.
5. Write report with precise next behavioral patch recommendation.

## Validation Plan
- Verify extraction commands complete successfully.
- Verify comparison output includes deterministic ordered sequences and first divergence index.

## Risks / Open Questions
- Firestorm and viewer sessions are not same timestamp, so exact packet IDs differ; compare by message kind/order only.
- If multiple local ports exist in pcap, endpoint selection must be explicit.

## Learnings Pre-Check
- L38/L39/L68/L69/L71 apply: prioritize evidence-led next change and avoid speculative broad startup packet edits.

## Completion Criteria
- Report contains: Firestorm first 20 sends, viewer first 20 sends, first divergence, and one recommended next patch target.
