# Review: Implementation Failproof Mesh-ID Retention and Export Unblock 2026-04-03

## Verdict
Approved.

## Architecture and boundary fit
- `viewer_net` changes stay inside decode/state/export summary ownership.
- `viewer_app` changes stay inside snapshot interpretation, relay formatting, and runtime verification.
- No crate-boundary collapse occurred.

## Correctness concerns
- Real-payload replay tests now lock the embedded Firestorm mesh-ID path.
- Transport-side object totals are preserved instead of being silently overwritten by exported-slice counts.
- Export truncation now deterministically keeps mesh-bearing entries visible to the scheduler.

## Modularity and maintainability concerns
- Added small formatting helpers keep relay changes local.
- Additive summary counters are narrow and test-covered.

## Validation adequacy
- `cargo fmt --all` passed.
- `cargo check -p viewer_net -p viewer_app` passed.
- `cargo test -p viewer_net` passed.
- `cargo test -p viewer_app` passed.
- `cargo build -p viewer_app` passed.
- Bounded live decoded-only verification passed and produced decisive evidence.

## Risks and open questions
- The earlier stale-binary live diagnosis is superseded; future runs must always rebuild the executable before relying on relay output.
- Mesh rendering/scene geometry consumption remains a separate follow-up from the decode/export/scheduler path fixed here.

## Learnings delta verdict
- add: the embedded `ObjectUpdate.ExtraParams` layout differs from standalone `ObjectExtraParams`, and treating them as identical can hide live mesh IDs.

## Required revisions or approval status
- No revisions required.
