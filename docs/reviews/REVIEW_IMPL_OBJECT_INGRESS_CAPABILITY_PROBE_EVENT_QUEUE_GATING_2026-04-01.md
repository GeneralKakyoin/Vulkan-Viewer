# Review: Implementation Object Ingress Capability Probe EventQueue Gating (2026-04-01)

## Verdict
approved

## Architecture and boundary fit
- Change is confined to `viewer_app` orchestration ordering.
- `viewer_net` transport/capability ownership is unchanged.
- No crate-boundary drift observed.

## Correctness concerns
- Pending probe URLs are consumed once (`take()`), preventing repeated probe invocations.
- Gate-open transition is explicit and tied to first successful `EventQueueGet` result.
- Readiness counters correctly show deferred (`i=0`) at startup and invoked after gate opens.

## Modularity and maintainability concerns
- Probe logic was centralized into helper functions (`run_pending_*`, per-cap runners), reducing duplicate startup/loop code paths.

## Validation adequacy
- `cargo fmt --all`: pass
- `cargo check -p viewer_net -p viewer_app`: pass
- `cargo test -p viewer_net`: pass
- `cargo test -p viewer_app`: pass
- bounded live run with dedicated artifact: captured successfully (timeout-bounded by design)

## Risks and open questions
- `UntrustedSimulatorMessage` continues returning `405 Method Not Allowed` even after gate-open.
- LLUDP gate remains `FAIL`; this slice improves prerequisite evidence but does not unblock object ingress yet.

## Learnings delta verdict
- add (ordering changed a capability outcome materially)

## Required revisions or approval status
- Approved; proceed with continuity updates and next-step branch focusing on post-EventQueue capability semantics (`UntrustedSimulatorMessage` 405) versus LLUDP blocker.
