# Plan: EventQueue LLSD Root Shape Hardening (2026-04-09)

## Scope
Harden LLSD XML parsing for capability decoders so valid non-map LLSD root shapes (especially `<undef />` timeout-style responses) do not fail with `missing llsd map` and repeatedly degrade EventQueue readiness.

## Current known state
- `viewer_net` EventQueue LLSD parsers (`parse_event_queue_from_llsd_xml`, `parse_event_queue_poll_from_llsd_xml`) currently assume a top-level `<map>` and fail with `missing llsd map` when that shape is absent.
- Recent bounded logs show repeated EventQueue decode errors with `missing llsd map` in simulator-host `:12043` runs.
- `parse_region_objects_from_llsd_xml` already uses an explicit LLSD root/value strategy and demonstrates root-shape tolerance.

## Files and components touched
- `crates/viewer_net/src/lib.rs`
  - LLSD root-map discovery helper
  - EventQueue LLSD parser hardening
  - (bounded) aligned handling for other LLSD map-assuming capability parsers
  - regression tests for LLSD root variants
- Continuity/report artifacts:
  - `docs/reviews/REVIEW_PLAN_EVENT_QUEUE_LLSD_ROOT_SHAPE_HARDENING_2026-04-09.md`
  - `docs/reviews/REVIEW_IMPL_EVENT_QUEUE_LLSD_ROOT_SHAPE_HARDENING_2026-04-09.md`
  - `docs/reports/REPORT_EVENT_QUEUE_LLSD_ROOT_SHAPE_HARDENING_2026-04-09.md`
  - `docs/CURRENT_STATE.md`
  - `docs/HANDOFF.md`

## Boundary check
- `viewer_net` owns capability decode semantics and LLSD parser behavior.
- No architecture/crate-boundary changes.
- No UI/runtime orchestration refactors in this slice.

## Step sequence
1. Add LLSD root helper that resolves root value via `<llsd>` first child (or document root fallback), and returns map/array/undef-aware outcomes.
2. Update EventQueue LLSD parsers to use helper:
   - map root: parse as before
   - undef/no-map root: treat as empty EventQueue response instead of hard decode error
3. Apply same helper to other LLSD map-assuming capability parsers in this file for consistency.
4. Add unit tests for:
   - EventQueue poll parse with `<llsd><undef /></llsd>`
   - EventQueue inspection parse with `<llsd><undef /></llsd>`
   - EventQueue poll parse with `<llsd><array><map>...` wrapper
5. Run validation ladder and capture outcomes.
6. Write execution report + continuity updates.

## Validation plan
1. `cargo fmt --all`
2. `cargo check -p viewer_net`
3. `cargo test -p viewer_net parse_event_queue_poll_from_llsd_xml`
4. `cargo test -p viewer_net parse_event_queue_from_llsd_xml`

## Risks and open questions
- Treating `<undef />` as successful empty response may hide malformed-server conditions if non-LLSD XML is misclassified.
- Array-root map fallback assumes first map represents the intended payload.

## Deferred-too-early candidates captured
- Deferred: broader parser-framework extraction into shared LLSD decoder module.
- Reason: this slice is bounded to unblock known EventQueue decode instability first.

## Learnings pre-check
- L64: preserve request-shape and protocol exactness while changing only decode robustness.
- L68/L69: EventQueue semantics can gate startup; reduce false decode failures before widening protocol hypotheses.

## Exact completion criteria
- EventQueue LLSD parsers no longer fail on valid `<undef />` root payloads.
- New regression tests cover undef-root and array-wrapped-map root for EventQueue poll/inspection paths.
- `cargo fmt`, targeted `cargo check`, and targeted tests pass.
- Continuity docs and report explicitly record validation and remaining functional next step.
