# Plan: Code Cleanup Warning Hygiene (2026-04-03)

## Scope
- Perform a bounded cleanup pass that removes currently observed compiler warnings in touched runtime crates without changing viewer behavior.

## Current Known State
- `cargo check -p viewer_render -p viewer_app` reports:
  - unused constant `DEBUG_CLIP_SPACE_TRIANGLE` in `viewer_render`.
  - unread config field `event_queue_poll_every_ticks` in `viewer_app`.

## Files / Components Touched
- `crates/viewer_render/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- continuity artifacts under `docs/`

## Boundary Check
- No crate-boundary or architecture changes.
- No protocol or rendering behavior changes intended.

## Step Sequence
1. Remove stale unused render constant.
2. Ensure `event_queue_poll_every_ticks` is read in non-behavioral startup diagnostics.
3. Run format/check/tests for touched crates.
4. Record results in report and continuity docs.

## Validation Plan
- `cargo fmt --all`
- `cargo check -p viewer_render -p viewer_app`
- `cargo test -p viewer_render -- --nocapture`
- `cargo test -p viewer_app in_process_live_feed_config_from_lookup -- --nocapture`

## Risks / Open Questions
- Emitting one extra startup relay line is log-visible but behavior-neutral.

## Deferred Too Early Candidates
- broader clippy/style refactor pass across all crates (out of scope for this bounded hygiene slice).

## Learnings Pre-check
- No existing durable learning entries force design changes for this bounded warning cleanup.

## Completion Criteria
- Targeted warnings are removed.
- Validation commands pass.
- Continuity docs reflect the cleanup pass.
