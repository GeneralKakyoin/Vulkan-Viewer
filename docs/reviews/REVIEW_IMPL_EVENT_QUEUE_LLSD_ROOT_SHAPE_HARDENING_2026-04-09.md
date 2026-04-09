# Review: Implementation EventQueue LLSD Root Shape Hardening (2026-04-09)

## Verdict
Approved.

## Architecture and boundary fit
- Changes are confined to `viewer_net` LLSD capability decoding (`crates/viewer_net/src/lib.rs`).
- No crate-boundary expansion and no protocol send-shape changes.

## Correctness concerns
- Addressed: EventQueue LLSD parse paths no longer hard-fail on `<llsd><undef /></llsd>`.
- Addressed: array-wrapped map roots are now accepted for EventQueue poll parsing.
- Residual: non-LLSD XML responses still fail decode (intended).

## Modularity and maintainability concerns
- Added a single helper (`find_llsd_root_map`) to remove duplicated root-map assumptions.
- Applied helper to EventQueue, avatar-name, and simulator-features LLSD map consumers for consistent semantics.

## Validation adequacy
- `cargo fmt --all` passed.
- `cargo check -p viewer_net` passed.
- Targeted `viewer_net` tests for EventQueue LLSD poll/inspection parsers passed, including new regression tests.

## Risks and open questions
- This patch hardens decode behavior; it does not yet prove runtime readiness stability against simulator `:12043` without a new bounded live run.

## Learnings delta verdict
none: No durable new invariant discovered; this is an expected parser hardening pattern consistent with existing LLSD variability learnings.

## Required revisions or approval status
Approved; proceed with continuity updates and next-step live verification run.
