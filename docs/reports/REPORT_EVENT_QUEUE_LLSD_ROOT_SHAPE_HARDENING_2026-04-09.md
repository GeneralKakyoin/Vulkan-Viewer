# Report: EventQueue LLSD Root Shape Hardening (2026-04-09)

## Summary of implemented work
- Added LLSD root-shape helper in `viewer_net` to resolve a root map from LLSD XML without assuming every successful response is top-level `<map>`.
- Hardened EventQueue LLSD parsers to accept valid empty-root cases (`<llsd><undef /></llsd>`) as empty-success instead of decode failure.
- Added array-wrapped-map compatibility for EventQueue poll LLSD parsing.
- Applied the same root-map helper to other LLSD map-assuming decoders in the same file (`GetDisplayNames`, `SimulatorFeatures`) for consistency.
- Added EventQueue regression tests covering undef-root and array-root-map payloads.

## Files changed
- `crates/viewer_net/src/lib.rs`
- `docs/plans/PLAN_EVENT_QUEUE_LLSD_ROOT_SHAPE_HARDENING_2026-04-09.md`
- `docs/reviews/REVIEW_PLAN_EVENT_QUEUE_LLSD_ROOT_SHAPE_HARDENING_2026-04-09.md`
- `docs/reviews/REVIEW_IMPL_EVENT_QUEUE_LLSD_ROOT_SHAPE_HARDENING_2026-04-09.md`
- `docs/reports/REPORT_EVENT_QUEUE_LLSD_ROOT_SHAPE_HARDENING_2026-04-09.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`

## Validation run
- `cargo fmt --all` -> PASS
- `cargo check -p viewer_net` -> PASS
- `cargo test -p viewer_net parse_event_queue_poll_from_llsd_xml -- --nocapture` -> PASS (4 passed)
- `cargo test -p viewer_net parse_event_queue_from_llsd_xml -- --nocapture` -> PASS (1 passed)

## Result status
- Complete for the planned bounded scope.
- EventQueue LLSD decode path now tolerates valid non-map LLSD roots in unit-tested parser paths.

## Risks or follow-up items
- A bounded live run is still needed to confirm reduction/elimination of repeated runtime `missing llsd map` errors on simulator `:12043`.
- If decode warnings persist, add bounded diagnostics to capture sanitized response root-shape classification at failure points.

## Learnings delta
none: No durable new learning identified; this was a direct application of existing LLSD variability constraints.

## Continuity updates performed
- Added this report and the plan/plan-review/impl-review artifacts.
- Updated `docs/CURRENT_STATE.md` with latest notable parser-hardening change and validation summary.
- Replaced `docs/HANDOFF.md` with latest handoff focused on post-patch live verification next step.
