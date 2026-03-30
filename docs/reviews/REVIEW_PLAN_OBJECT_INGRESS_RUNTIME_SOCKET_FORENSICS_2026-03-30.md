# Review: PLAN_OBJECT_INGRESS_RUNTIME_SOCKET_FORENSICS_2026-03-30

## Verdict
Approved.

## Architecture and Boundary Fit
- The plan keeps runtime socket-lifecycle instrumentation in `viewer_net` and only surfaces bounded summaries through `viewer_app`.
- It does not collapse transport concerns into grid semantics or broaden into unrelated crates.

## Correctness Concerns
- The diagnostics must report real runtime local-port usage, not inferred state.
- Tests should cover retain/reuse behavior and steady-state polling on the active circuit so the new logging cannot drift away from the actual transport path.
- The plan correctly avoids assuming that every extra Firestorm pre-burst request is the trigger before runtime continuity is revalidated.

## Modularity and Maintainability Concerns
- Prefer one typed first-simulator socket diagnostic path over scattered ad-hoc logging.
- Keep relay output concise and startup-bounded so diagnostics remain useful in live runs.

## Validation Adequacy
- `cargo fmt --all`, `cargo check -p viewer_net -p viewer_app`, targeted crate tests, and a bounded connected run are appropriate.
- Comparing the resulting runtime trace against the preserved pcap artifacts is the right validation target.

## Risks and Open Questions
- `App.pcapng` may represent a capture window that began after an earlier socket association, so the live diagnostic output must be interpreted carefully.
- If runtime continuity is proven clean, the next slice will still need a separate plan for the extra Firestorm pre-burst messages.

## Learnings Delta Verdict
- add expected if runtime diagnostics confirm that packet capture evidence can falsify code-level continuity assumptions.

## Required Revisions or Approval Status
- Approved to implement as written.
