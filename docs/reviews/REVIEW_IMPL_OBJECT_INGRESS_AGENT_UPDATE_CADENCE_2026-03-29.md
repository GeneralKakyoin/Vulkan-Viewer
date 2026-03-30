# Review: IMPLEMENTATION_OBJECT_INGRESS_AGENT_UPDATE_CADENCE_2026-03-29

## Verdict
Approved as a bounded protocol-validation slice with a clear negative-result outcome.

## Architecture and Boundary Fit
- `viewer_net` changes stay within first-simulator LLUDP send helpers on the active `SocialCircuit`.
- `viewer_app` changes stay within live-worker scheduling and reconnect re-priming.
- No renderer, UI, asset, or `viewer_grid` ownership drift was introduced.

## Correctness Concerns
- The new recurring `AgentUpdate` path reuses the active social circuit and does not bind fresh sockets.
- Startup parity remains intact because `send_startup_interest_messages(...)` still sends throttle first and a reliable `AgentUpdate` second.
- The live result shows no protocol improvement from this slice alone: the packet mix and object-feed counters remained unchanged.

## Modularity and Maintainability Concerns
- Splitting `send_agent_update_on_circuit(...)` out of the startup-only helper keeps the recurring path explicit and testable.
- The worker-side keepalive scheduler is deterministic and isolated in small helper functions.

## Validation Adequacy
- `cargo fmt --all`: passed.
- `cargo check -p viewer_net -p viewer_app`: passed.
- `cargo test -p viewer_net`: passed.
- `cargo test -p viewer_app`: passed.
- Connected `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app` capture: completed with bounded 60-second runtime and explicit artifact logs.

## Risks and Open Questions
- Because the live packet mix stayed identical, the next fix should not spend another slice on `AgentUpdate` cadence alone.
- The strongest remaining hypothesis is protocol-accurate first-simulator reliability/control behavior (`StartPingCheck` / `PacketAck` / related LLUDP control handling) backed by Firestorm source evidence.

## Learnings Delta Verdict
- add: `L29` because recurring `AgentUpdate` cadence on the retained active circuit did not change the live startup packet mix or object ingress.

## Required Revisions or Approval Status
Approved. Next work should move to a new planned slice for protocol-accurate first-simulator control/reliability behavior rather than more socket or `AgentUpdate` cadence tuning.
