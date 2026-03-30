# Review: Object Ingress Runtime Socket Forensics (2026-03-30)

## Verdict
Approved as implemented.

## Architecture and Boundary Fit
- The runtime socket event model stays in `viewer_net`.
- The worker only relays bounded summaries in `viewer_app`.
- No ownership drift into render, UI, asset, or grid crates was introduced.

## Correctness Concerns
- The implementation now records first-simulator probe bind, fresh bind, retain, reuse, send, and receive events with local address and remote target context.
- Targeted tests lock the three key continuity claims:
  - retained probe socket reuse stays on one local port
  - fresh open without probe stays on one local port
  - steady-state nearby-chat polling does not trigger a second handshake socket
- The bounded live run is the decisive check for this slice, and it showed one local UDP port (`65241`) across probe, open, startup prime, and the first steady-state window.

## Modularity and Maintainability Concerns
- Centralizing the diagnostics in one typed `viewer_net` path is cleaner than scattering ad-hoc relay logging.
- The `viewer_app` relay helper stays startup-bounded and reports both counts and a short tail, which is enough for live RCA without flooding the console.

## Validation Adequacy
- `cargo fmt --all`: passed
- `cargo check -p viewer_net -p viewer_app`: passed
- `cargo test -p viewer_net`: passed
- `cargo test -p viewer_app`: passed
- bounded connected run `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`: passed as a command and answered the planned runtime question

## Risks and Open Questions
- Socket continuity is no longer the best live explanation for missing object ingress on the current code path.
- Object ingress still remains at zero even though the same socket now clearly receives continued simulator traffic.
- The next slice needs to target post-`AgentMovementComplete` message/control parity or inbound classification gaps on that same one-port flow.

## Learnings Delta Verdict
- add: runtime diagnostics proved that the current blocked path is one-port end to end, so future work should not keep revisiting socket reuse alone.

## Required Revisions or Approval Status
- No implementation revisions required for this slice.
- The next step should be a separate plan for message/control parity, not another socket-lifecycle pass.
