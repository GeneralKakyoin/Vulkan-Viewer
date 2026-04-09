# Review: Implementation Wave 2/3 Viewer App + Core Helper Extraction (2026-04-04)

## Verdict
Approved.

## Architecture and boundary fit
- `viewer_app` extraction stays within app orchestration crate.
- `viewer_core` extraction stays within core domain crate and preserves public helper availability via re-export.

## Correctness concerns
- No behavior changes identified.
- Start-location parsing and core math helper callsites compile and pass tests.

## Modularity and maintainability concerns
- Positive: helper concerns are now isolated in dedicated local modules.
- Positive: parent files are incrementally reduced while preserving stable paths.

## Validation adequacy
- `cargo fmt --all`: PASS
- `cargo check -p viewer_app -p viewer_core`: PASS
- targeted helper tests: PASS
- `cargo test -p viewer_app`: PASS
- `cargo test -p viewer_core`: PASS

## Risks and open questions
- `viewer_app/src/main.rs` and `viewer_core/src/lib.rs` remain large; further bounded extractions still required.

## Learnings delta verdict
`none` — no new durable lesson beyond existing locality/boundary guidance.

## Required revisions or approval status
No revisions required. Implementation approved.
