# Review: Plan Object Ingress Network Debug And Decision Acceleration (2026-03-30)

## Verdict
Approved.

## Architecture and boundary fit
- Correctly keeps transport ownership in `viewer_net`, aggregation/logging in `viewer_app`, typed UI-facing state in `viewer_core`, and presentation in `viewer_ui`.
- Avoids direct `viewer_ui` dependency on live transport internals.

## Correctness concerns
- The log file should contain structured, timestamped events rather than ad hoc text blobs.
- The `Network Debug` window should present bounded summaries, not attempt to become a packet inspector.
- Reuse existing relay/event categorization where possible to avoid divergence between the window and the log sink.

## Modularity and maintainability concerns
- Prefer one new typed network-debug state surface over many one-off UI parameters.
- Keep the decision gates explicit so the work does not turn into endless instrumentation-first loops.

## Validation adequacy
- Static checks, targeted tests across the touched crates, one bounded live run, and manual window/log verification are adequate for this slice.

## Risks and open questions
- The plan is intentionally broad enough to accelerate the next branch, so implementation must keep the UI/log surface compact.
- If new env vars are introduced for log path or window behavior, `docs/TESTING_REFERENCE.md` must be updated in the same change.

## Learnings delta verdict
- `none`
- This is a planning artifact; implementation outcome will determine whether a new durable learning is needed.

## Approval status
Approved to implement as written.
