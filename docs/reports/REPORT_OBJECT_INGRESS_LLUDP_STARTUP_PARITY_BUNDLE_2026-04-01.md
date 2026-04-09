# Report: Object Ingress LLUDP Startup Parity Bundle (2026-04-01)

## Summary of implemented work
- Added `VIEWER_APP_LLUDP_STARTUP_PARITY_BUNDLE` runtime flag parsing in `viewer_app` startup config (default `false`).
- Added bounded startup prime mode behavior:
  - when flag is enabled, startup prime now performs an explicit pending-ACK flush inside the startup parity bundle path.
- Added explicit startup relay marker for selected startup prime mode:
  - `startup prime mode=lludp_parity_bundle:on|off`.
- Added strict LLUDP object-ingress gate relay line in first-simulator forensics:
  - includes verdict, first object-update index, update/object counts, and bounded local-id preview.
- Added targeted tests:
  - config parsing includes new flag.
  - gate line fail case (no object update/local IDs).
  - gate line pass case (object update index + local IDs).
- Updated `docs/TESTING_REFERENCE.md` to document the new env var.

## Files changed
- `crates/viewer_app/src/main.rs`
- `docs/TESTING_REFERENCE.md`
- `docs/plans/PLAN_OBJECT_INGRESS_LLUDP_STARTUP_PARITY_BUNDLE_2026-04-01.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_LLUDP_STARTUP_PARITY_BUNDLE_2026-04-01.md`
- `docs/reviews/REVIEW_IMPL_OBJECT_INGRESS_LLUDP_STARTUP_PARITY_BUNDLE_2026-04-01.md`

## Validation run
- `cargo fmt --all`: PASSED
- `cargo check -p viewer_net -p viewer_app`: PASSED
- `cargo test -p viewer_app`: PASSED
- `cargo test -p viewer_net`: PASSED
- bounded live run (timeout-bounded):
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_APP_LLUDP_STARTUP_PARITY_BUNDLE=on`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_lludp_startup_parity_bundle_2026-04-01.jsonl`
  - `cargo run -p viewer_app` (timed out after bounded capture window)
  - observed gate evidence:
    - `startup lludp_object_gate: verdict=FAIL object_update=none update_messages=0 total_objects=0 local_ids=none`
    - `after_first_steady_state_window lludp_object_gate: verdict=FAIL object_update=none update_messages=0 total_objects=0 local_ids=none`

## Result status
- Implementation complete and validated for formatting/build/tests.
- Live parity-bundle gate executed and failed: no `ObjectUpdate*` and no local-id evidence in the bounded capture.

## Risks or follow-up items
- Follow-up required:
  - pivot immediately to simulator-host capability-readiness invocation checks (per gate-fail policy).
  - preserve this run artifact as the baseline failure reference:
    - `artifacts/logs/network_debug_lludp_startup_parity_bundle_2026-04-01.jsonl`

## Learnings delta
- added — see `L58` in `docs/LEARNINGS.md` (parity-bundle gate failure is a branch decision point, not a cue for more LLUDP startup guesses).

## Continuity updates performed
- Updated plan/review/report artifacts for this slice.
- Updated `docs/CURRENT_STATE.md`, `docs/HANDOFF.md`, and `docs/OBJECT_INGRESS_STATUS.md` to reflect implementation status and exact next step.
