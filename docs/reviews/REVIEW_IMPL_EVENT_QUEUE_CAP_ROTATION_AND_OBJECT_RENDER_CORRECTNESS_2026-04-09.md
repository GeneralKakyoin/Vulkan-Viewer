# Review: EventQueue Cap Rotation and Object Render Correctness (2026-04-09)

## Verdict
Approved with follow-up required.

## Architecture and boundary fit
- EventQueue cap re-prime logic is implemented in `viewer_app` orchestration and uses existing `viewer_net` capability fetch APIs.
- No crate-boundary shifts were introduced.
- `viewer_core` changes are bounded to math regression tests in `math_utils.rs`.

## Correctness concerns
- Cap-not-found threshold now attempts an in-session EventQueue cap re-prime before reconnect.
- Runtime evidence confirms the new branch executes (`cap_reprime:start`) and emits explicit success/failure diagnostics.
- In the bounded live run, re-prime fetch itself returned `404 cap not found`, so reconnect fallback still triggered. This is acceptable for this slice and correctly reported.

## Modularity and maintainability concerns
- New logic is bounded and does not broaden protocol scope.
- Added helper mapping function and targeted tests reduce future regression risk.
- No monolithic cross-cutting refactor was introduced.

## Validation adequacy
- `cargo fmt --all`: pass
- `cargo check --workspace`: pass
- `cargo test -p viewer_net`: pass
- `cargo test -p viewer_core`: pass
- `cargo test -p viewer_render`: pass
- bounded live `cargo run -p viewer_app`: command timed out by harness, log artifact captured and reviewed

## Risks and open questions
- Re-prime still depends on current seed capability fetch route; if that route is already invalidated, the branch will fail and reconnect.
- Follow-up should add a bounded alternate re-prime source path (for example, recently followed seed capability URLs) before reconnect.

## Learnings delta verdict
none — no new durable learning identified beyond existing EventQueue/cap lifecycle entries; this slice confirms expected late-session cap invalidation behavior with better diagnostics.

## Required revisions or approval status
- Approval status: Approved for merge as a bounded reliability/observability improvement.
- Required follow-up: implement alternate seed/cap re-prime source before reconnect when primary seed fetch returns 404.
