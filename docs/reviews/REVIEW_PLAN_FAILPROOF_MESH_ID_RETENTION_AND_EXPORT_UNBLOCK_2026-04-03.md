# Review: Plan Failproof Mesh-ID Retention and Export Unblock 2026-04-03

## Verdict
Approved.

## Architecture and boundary fit
- The plan keeps LLUDP decode/state retention in `viewer_net` and scheduler/relay interpretation in `viewer_app`.
- No protocol meaning is pushed into UI or unrelated crates.

## Correctness concerns
- Replay tests must use real embedded `ObjectUpdate.ExtraParams` layout, not a guessed synthetic layout borrowed from standalone `ObjectExtraParams`.
- Export ordering change must preserve deterministic behavior under truncation.

## Modularity and maintainability concerns
- New counters should remain additive and summary-scoped; avoid widening public interfaces beyond what the relay needs.
- Bridge tests should prove count preservation without requiring live infrastructure.

## Validation adequacy
- Proposed fmt/check/test coverage plus a bounded live run is adequate for this bounded slice.

## Risks and open questions
- If a stale binary is used for the live run, conclusions will be invalid even if tests pass.
- Live route variance remains a risk, so report artifacts must cite the exact run used.

## Learnings delta verdict
- add expected if the slice proves a durable protocol-layout trap; otherwise update continuity only.

## Required revisions or approval status
- No revisions required.
