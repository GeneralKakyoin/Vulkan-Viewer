# Review: PLAN_STARTUP_RECEIVE_ACTIVATION_2026-03-29

## Verdict
Approved.

## Architecture and Boundary Fit
- The plan correctly splits responsibility between `viewer_net` (socket retention policy) and `viewer_app` (startup receive activation timing).
- It stays within existing crate boundaries and does not push protocol semantics into the wrong layer.

## Correctness Concerns
- Preserve the socket whenever the probe successfully established the startup address, not only when AMC was locally observed.
- Ensure the startup drain is bounded and uses the existing social-circuit receive path so packet handling stays centralized.
- Avoid regressing the no-probe path; when no retained socket exists, startup should still open and drain a fresh social circuit deterministically.

## Modularity and Maintainability Concerns
- Prefer a small helper for startup drain over duplicating receive logic inline.
- Keep the startup reordering easy to reason about; the long-lived circuit should become active in one obvious place.

## Validation Adequacy
- `cargo fmt --all`, `cargo check -p viewer_net -p viewer_app`, `cargo test -p viewer_net`, `cargo test -p viewer_app`, and a bounded connected capture are appropriate for this slice.

## Risks and Open Questions
- If object ingress remains zero after this change, further work should shift to packet-parity instrumentation rather than more socket-orchestration tweaks.
- Startup drain budget should stay bounded to avoid introducing a long blocking startup stall.

## Learnings Delta Verdict
- update or add depending on whether the connected capture confirms early receive activation changes the observed packet mix.

## Required Revisions or Approval Status
Approved to implement as written.
