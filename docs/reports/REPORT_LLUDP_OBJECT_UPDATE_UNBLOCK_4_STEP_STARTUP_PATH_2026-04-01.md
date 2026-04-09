# Report: LLUDP Object-Update Unblock via 4-Step Startup Path (2026-04-01)

## Summary of implemented work
- Added a typed startup-interest gate summary in `viewer_net` that checks required startup sends (`AgentThrottle`, `AgentUpdate`, `AgentHeightWidth`) using full send diagnostics and returns order index + packet ids.
- Added configurable AgentUpdate transport call in `viewer_net` (`send_agent_update_custom_on_circuit`) and preserved existing default path via wrapper.
- Added runtime knobs in `viewer_app`:
  - `VIEWER_APP_AGENT_UPDATE_FAR`
  - `VIEWER_APP_AGENT_UPDATE_KEEPALIVE_TICKS`
- Added periodic keepalive relay line:
  - `agent_update_keepalive: period_ticks=... camera_center=... far=... control_flags=... reliable=...`
- Expanded first-simulator transcript tail window and added explicit invariant relay line:
  - `startup_interest_gate: verdict=PASS|FAIL required=... missing=... classification=...`
- Added diagnosis-only residency classifier line in protocol summaries:
  - `residency_state=unknown|root_likely|child_likely|degraded evidence=...`
- Updated test/reference docs for the new runtime env controls.

## Files changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/TESTING_REFERENCE.md`
- `docs/plans/PLAN_LLUDP_OBJECT_UPDATE_UNBLOCK_4_STEP_STARTUP_PATH_2026-04-01.md`
- `docs/reviews/REVIEW_PLAN_LLUDP_OBJECT_UPDATE_UNBLOCK_4_STEP_STARTUP_PATH_2026-04-01.md`
- `docs/reviews/REVIEW_IMPL_LLUDP_OBJECT_UPDATE_UNBLOCK_4_STEP_STARTUP_PATH_2026-04-01.md`

## Validation run
- `cargo fmt --all` -> PASSED
- `cargo check -p viewer_net -p viewer_app` -> PASSED
- `cargo test -p viewer_net -p viewer_app` -> PASSED
- bounded live captures (`cargo run -p viewer_app`, timeout-bounded) -> EXECUTED
  - baseline: `artifacts/logs/network_debug_lludp_unblock_4step_baseline_2026-04-01.jsonl`
  - FAR override: `artifacts/logs/network_debug_lludp_unblock_4step_far_2026-04-01.jsonl`
  - keepalive ticks override: `artifacts/logs/network_debug_lludp_unblock_4step_keepalive_2026-04-01.jsonl`
  - alternate route (`VIEWER_LOGIN_START=uri:Ahern&50&60&70`):
    `artifacts/logs/network_debug_lludp_unblock_4step_alternate_route_2026-04-01.jsonl`

## Result status
- Startup interest gate: PASS in all three runs.
- Keepalive observability: present and deterministic.
  - baseline period `17`, far `96`
  - FAR override period `17`, far `192`
  - keepalive override period `6`, far `96`
- Residency classifier: emitted in startup and first steady-state windows.
- Alternate route evidence includes sustained EventQueue `EnableSimulator` detail messages and stable keepalive logs, while LLUDP gate still fails.
- LLUDP hard gate (`lludp_object_gate`): still FAIL in all bounded runs (`ObjectUpdate*` absent).

## Risks / follow-up items
- EventQueue decode remains unstable (`capability response decode error: missing llsd map`) and can drive `residency_state=degraded`.
- Next smallest patch should focus on EventQueue decode/readiness stability and/or simulator-target EventQueue extraction completeness, not additional startup send guessing.

## Learnings delta
- added (L67): startup transcript tails can hide required startup-interest sends; explicit invariant gate should be first-class evidence.

## Continuity updates performed
- Added plan/reviews/report artifacts for this slice.
- Updated `CURRENT_STATE.md`, `HANDOFF.md`, and `LEARNINGS.md` for current truth and next step.

## OpenSimulator Cross-Reference Addendum (2026-04-01)
- Added OpenSimulator protocol reference source:
  - `reference/opensimulator/nebadon2025-opensimulator`
  - `reference/opensimulator/README.md`
- Scanned handshake/object-ingress implementation surfaces:
  - `OpenSim/Region/ClientStack/Linden/Caps/EventQueue/EventQueueGetModule.cs`
  - `OpenSim/Region/CoreModules/Framework/EntityTransfer/EntityTransferModule.cs`
  - `OpenSim/Region/ClientStack/Linden/UDP/LLUDPServer.cs`
  - `OpenSim/Region/ClientStack/Linden/UDP/LLClientView.cs`
  - `OpenSim/Region/Framework/Scenes/ScenePresence.cs`
- Evidence-aligned implications for this repo:
  - OpenSimulator sends `RegionHandshake` from `UseCircuitCode` accept path.
  - OpenSimulator validates `CompleteAgentMovement` fields before completion flow.
  - OpenSimulator ties initial-data progression to `RegionHandshakeReply` handling (`NeedInitialData` path), which then drives initial world/object update emission.
  - EventQueue neighbor endpoint details may arrive in different shapes (`SimulatorInfo.IP` binary + `Port` vs `sim-ip-and-port`), so endpoint extraction must preserve both forms.
