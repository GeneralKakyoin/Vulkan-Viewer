# HANDOFF.md

## Last Completed Work

- Extended first-simulator bounded probe with optional post-movement tail capture:
  - added `probe_first_simulator_handshake_window_with_tail(bind, timeout, max_packets, post_movement_tail_packets)`
  - preserved existing behavior for `probe_first_simulator_handshake_window(...)` via wrapper (`tail=0`)
  - probe report now captures:
    - `agent_movement_complete_observation_index`
    - `post_movement_observations`
- Updated manual example/README for bounded post-movement observation:
  - new env var `VIEWER_FIRST_SIM_POST_MOVEMENT_TAIL_PACKETS`
  - report output now prints movement-complete index and post-movement count
- Improved unknown inbound packet diagnostics:
  - packet-shaped but unmapped traffic now uses explicit signal `packet:0x........:unmapped`
  - avoids generic `text:none` for decodeable-but-unmapped packet IDs
- Added focused tests:
  - `probe_first_simulator_handshake_window_with_tail_collects_post_movement_packets`
  - additional assertions for probe-report movement-complete index/post-movement count
  - additional assertions for explicit unmapped packet signal
- Live manual bounded-tail validation (`tail=2`) now observes immediate post-movement packets:
  - `AgentDataUpdate` -> `TestMessage` -> `AgentMovementComplete` -> `Irrelevant(unmapped packet id 0xfffffffb)` -> `HealthMessage`
  - handshake stage still advances correctly to `AgentMovementComplete`

- Implemented bounded multi-packet same-socket live probe in `viewer_net`:
  - added `probe_first_simulator_handshake_window(bind, timeout, max_packets)`
  - one-shot probe now delegates to bounded-window probe (`max_packets=1`)
  - added probe result typing with ordered observations + timeout flag
  - receive diagnostics now include `observation_index` for sequence tracking
- Expanded typed inbound classification for early live progression:
  - added `TestMessage` (low 1)
  - added `HealthMessage` (low 138)
  - added `SimulatorViewerTimeMessage` (low 150)
- Live bounded-window probe now observes and classifies multi-packet progression:
  - `AgentDataUpdate` -> `TestMessage` -> `AgentMovementComplete`
  - `AgentMovementComplete` classified via packet message-number decode
  - handshake stage advanced from waiting to terminal completion in live run
- Updated manual example to support bounded receive window controls:
  - `VIEWER_FIRST_SIM_RECEIVE_MAX_PACKETS`
  - report now prints ordered observations from probe window
- Completed first-simulator outbound wire-fidelity pass in `viewer_net`:
  - replaced JSON-like UDP handshake payloads with binary LLUDP packet construction for:
    - `UseCircuitCode` (low 3)
    - `CompleteAgentMovement` (low 249)
  - packet construction now includes:
    - reliable LLUDP header flags
    - network-order packet-id sequencing
    - low-frequency message number encoding
    - protocol-faithful fixed field ordering for handshake blocks
  - handshake send diagnostics now include:
    - outbound packet id
    - outbound packet message number
  - send-path tests now assert packet invariants (header/message/body layout) instead of string payload matching
- Expanded typed inbound classification for live-observed early packet:
  - added `AgentDataUpdate` (low 387) classification in receive decoder
- Live same-socket probe rerun after wire-fidelity pass:
  - real login succeeded
  - seed capability fetch succeeded
  - both handshake sends succeeded with LLUDP packet diagnostics
  - real inbound packet observed and classified:
    - `AgentDataUpdate` (`packet:0xffff0183`)
  - handshake stage did not advance to `AgentMovementComplete` yet (still waiting)
- Strengthened first-simulator receive-side protocol fidelity in `viewer_net`:
  - added typed inbound decode evidence fields:
    - decode source (`PacketMessageNumber`, `JsonField`, `TextScan`, `Unknown`)
    - optional packet message number for packet-shaped traffic
  - tightened handshake confirmation semantics:
    - waiting-stage promotion to `AgentMovementComplete` now requires packet-message decode evidence
    - JSON/text fallback detection remains diagnostic-only for movement completion
  - added one-shot handshake probe transport path:
    - `probe_first_simulator_handshake_once(bind, timeout)`
    - sends `UseCircuitCode` and `CompleteAgentMovement` using one shared UDP socket
    - waits for one inbound packet on the same socket and routes through typed receive classification
  - expanded focused tests:
    - decode source / packet number assertions
    - JSON `AgentMovementComplete` no longer advances waiting stage
    - same-socket probe send/receive flow coverage
  - wired manual example for live probe:
    - `VIEWER_INSPECT_FIRST_SIM_HANDSHAKE_ONCE`
    - `VIEWER_FIRST_SIM_RECEIVE_BIND`
    - `VIEWER_FIRST_SIM_RECEIVE_TIMEOUT_SECS`
  - live manual probe attempt result:
    - real login succeeded
    - seed capability fetch succeeded
    - both handshake sends reported transport success
    - no inbound handshake packet observed before timeout
- Replaced heuristic-first receive classification with a minimal typed UDP decode path in `viewer_net`:
  - added LLUDP packet header/message-number decode for high/medium/low frequency forms
  - mapped handshake-relevant low-frequency message IDs from Firestorm template semantics:
    - `RegionHandshake` (low 148)
    - `EnableSimulator` (low 151)
    - `AgentMovementComplete` (low 250)
  - handshake receive classification now prioritizes typed packet decode and uses JSON/text matching only as fallback
  - updated focused tests so receive classification and stage effects are validated with packet-shaped datagrams
- Updated repository agent-policy infrastructure (no product-code behavior change):
  - strengthened `AGENTS.MD` autonomous execution guidance for milestone-driven progress
  - clarified low-cost subagent usage expectations and main-session responsibilities
  - wired skill-discovery policy to check `.agents/skills/viewer/` first with `.agents/skills/` fallback
- Added repo Codex runtime config alignment in `.codex/config.toml`:
  - stronger main session model retained
  - `workspace-write` sandbox with network enabled
  - `on-request` approval policy
  - conservative subagent fan-out
- Added bounded low-cost subagent profiles:
  - `.codex/agents/worker.toml`
  - `.codex/agents/explorer.toml`
- Added minimal seed capability fetch support in `viewer_net` after logged-in session establishment
- Seed capability response parsing currently extracts top-level capability name -> URL pairs for initial inspection
- Added focused test coverage for login -> seed capability fetch path using LLSD capability map fixture
- Updated manual login example to optionally fetch and list capability names (`VIEWER_FETCH_SEED_CAPS=true`)
- Added one-shot EventQueueGet inspection path (no polling loop) with minimal top-level diagnostics and event-name extraction
- Corrected EventQueueGet one-shot timeout behavior for long-held responses
- Live behavior shifted from transport send failure to upstream HTTP proxy/server error response (no event payload yet)
- Added bounded one-shot EventQueueGet retry policy (3 attempts, retryable-only 5xx/transport cases)
- Added per-attempt diagnostics for EventQueueGet one-shot:
  - status (if present)
  - elapsed time
  - retryable classification
  - response headers when available
- Latest live run shows mixed retryable failures across attempts (HTTP 500 proxy-style and transport send) and still no parseable event names
- Added one-shot SimulatorFeatures fetch/inspection path to continue bootstrap observation with minimal top-level diagnostics
- Manual example now inspects SimulatorFeatures before EventQueueGet when both are enabled
- Added one-shot MapLayer fetch/inspection path with minimal top-level diagnostics
- Live MapLayer one-shot attempt currently returns HTTP 405 (method not allowed) with current GET request shape
- Added protocol-aligned LLSD `Accept` headers for one-shot capability requests (`SimulatorFeatures`, `MapLayer`, `EventQueueGet`) in `viewer_net`
- Added explicit `MapLayerLikelyLegacyUdp` error classification when one-shot MapLayer returns HTTP 405
- Added focused test coverage:
  - SimulatorFeatures one-shot sends LLSD `Accept` header
  - MapLayer one-shot sends LLSD `Accept` header
  - MapLayer HTTP 405 maps to explicit likely-legacy-UDP classification
- Live re-check confirms:
  - MapLayer: still 405, now explicitly classified as likely non-HTTP/legacy map path
  - SimulatorFeatures: still HTTP 503 Service Unavailable
  - EventQueueGet: still bounded retry failures (HTTP 500 proxy-style + occasional transport send failure)
- Added dedicated Firestorm first-simulator handshake research note:
  - `docs/RESEARCH/firestorm_first_simulator_handshake.md`
  - captures ordered bring-up sequence and dependencies for:
    - seed capability gating
    - `UseCircuitCode` send/ack phase
    - `CompleteAgentMovement` and `AgentMovementComplete` completion phase
    - early `EnableSimulator` / region-seed update interactions
- Implemented first-simulator handshake scaffold in `viewer_net` (typed sequencing model, no simulator transport yet):
  - added typed prerequisites (`agent/session/circuit/first-sim target`) captured from successful login bootstrap
  - added typed handshake stages:
    - `BootstrapPrerequisitesReady`
    - `FirstRegionTargetKnown`
    - `UseCircuitCode`
    - `CompleteAgentMovement`
    - `WaitingForAgentMovementComplete`
    - `AgentMovementComplete`
  - added strict ordered transition API on `Connection` for scaffold progression
  - added focused tests for:
    - happy-path ordered progression through waiting stage
    - out-of-order transition rejection
    - logged-in state precondition
- Bound first-simulator scaffold to real transport sends in `viewer_net`:
  - added `send_first_simulator_use_circuit_code()` transport action
  - added `send_first_simulator_complete_agent_movement()` transport action
  - both actions now send UDP datagram payloads to first-simulator target derived from bootstrap
  - send actions enforce ordered stage flow in code
  - added per-send diagnostics structure and history tracking:
    - action
    - target
    - payload length
    - elapsed time
    - success/failure + error text
  - added focused tests for:
    - `UseCircuitCode` send success + stage advancement + diagnostic capture
    - `CompleteAgentMovement` send success + waiting stage advancement + diagnostic capture
    - out-of-order send rejection
- Implemented substantial receive-side handshake observation/classification slice in `viewer_net`:
  - added `receive_first_simulator_handshake_datagram_once()` one-shot UDP receive hook
  - added `observe_first_simulator_inbound_payload()` reusable receive-observation path
  - added typed inbound message classification model for early handshake-relevant traffic:
    - `AgentMovementComplete`
    - `RegionHandshake`
    - `EnableSimulator`
    - `Irrelevant`
  - added typed receive diagnostics:
    - payload length
    - classification signal
    - stage before/after
    - whether stage was advanced
  - explicit receive-side handshake effect:
    - when waiting for movement complete, inbound `AgentMovementComplete` advances stage to terminal handshake completion
  - added focused tests for:
    - classification behavior
    - receive-side stage advancement
    - out-of-order/no-advance handling
    - one-shot UDP receive + diagnostics capture
- Achieved successful real Second Life login through the live endpoint
- Confirmed `GridLoginResult::Success` path with real bootstrap/session population (sensitive values intentionally not recorded here)
- Completed login compatibility milestone transition from payload-envelope debugging to post-login startup work
- Preserved crate boundaries while reaching live login:
  - `viewer_net` remained transport/codec/session lifecycle
  - `viewer_grid` remained request shaping and response interpretation

---

## What Changed In Project Understanding

- Autonomous execution can be more milestone-complete and less stop/start without weakening boundary controls.
- Repo skills and low-cost subagents are now first-class workflow defaults for repeatable bounded tasks.
- Login compatibility is now credible and proven in live conditions
- The highest-risk unknown is no longer login acceptance; it is post-login bootstrap sequencing and capability handling
- Seed capability startup is now the critical path before any simulator/world integration work
- MapLayer should no longer be treated as the best HTTP one-shot bootstrap candidate; immediate HTTP bootstrap evidence should prioritize SimulatorFeatures and EventQueueGet.
- Bootstrap capability probing is now characterized enough to begin handshake-sequencing planning without starting simulator transport implementation.
- Handshake sequencing is now represented in typed Rust state in `viewer_net`; the next risk moved from sequencing ambiguity to transport binding of the scaffold stages.
- Handshake transport binding for send actions is now in place; next risk is ack/receive-side classification and integration.
- Receive-side handshake observation/classification is now in place; next risk is protocol-level fidelity (actual UDP packet decoding semantics).
- Receive-side handshake identity classification now uses typed UDP message-number decoding; the next fidelity risk moved to block/field-level decode inside handshake messages.
- Same-socket live probe removed local bind-continuity ambiguity; primary remaining risk is likely outbound handshake packet wire fidelity.
- Outbound handshake wire fidelity is now materially improved and producing live inbound packets.
- Early inbound progression through `AgentMovementComplete` is now observed.
- Current risk shifted to narrow typed coverage expansion after initial movement completion (still within handshake/bootstrap scope).
- Immediate post-movement packet window is now directly observable with bounded tail capture; the next risk is minimal mapping of repeated unmapped packet IDs (starting with `0xfffffffb`) without broad world-state decode.

---

## Immediate Next Task

Focus only on:

**seed capability bootstrap using successful login output already available in-session**

This includes:
- expand capability typing/interpretation boundary in `viewer_grid`
- exercise seed capability fetch against live successful login state
- continue one-shot EventQueueGet transport stabilization until first parseable event envelope is observed
- use one-shot SimulatorFeatures inspection as parallel bootstrap evidence while EventQueueGet remains unstable
- keep MapLayer classified as likely legacy/unsupported for HTTP one-shot inspection unless new evidence suggests otherwise
- derive minimal Rust-facing handshake stage model from documented Firestorm first-region sequence (documentation/planning only)
- wire `UseCircuitCode` and `CompleteAgentMovement` scaffold stages to minimal transport send/ack mechanics in `viewer_net` (without world integration)
- add bounded ack/receive diagnostics and transition classification for first-simulator handshake actions
- improve receive classification from heuristic signal matching to a minimal typed decoder aligned to real packet schema
- extend typed decode from message identity to minimal block/field extraction for `AgentMovementComplete` while preserving current send/receive stage flow
- improve outbound `UseCircuitCode`/`CompleteAgentMovement` packet wire fidelity to unlock first real inbound handshake packet observation
- continue typed inbound expansion/progression analysis from `AgentDataUpdate` toward first observed `AgentMovementComplete`
- extend typed inbound coverage for the next early post-movement packets and add minimal field-level decode where it most helps diagnostics
- classify repeated unmapped post-movement packet IDs observed in bounded-tail runs and add the smallest bootstrap-relevant typed mappings
- capture and classify early bootstrap capability payloads without sensitive value leakage
- keep bootstrap diagnostics explicit and sanitized

Keep the current boundaries intact:
- `viewer_net` = transport/session/codec and capability HTTP mechanics
- `viewer_grid` = grid-specific meaning and typed interpretation

---

## Constraints

Do not:
- start simulator connection work
- start world streaming/integration
- integrate login/bootstrap into `viewer_app`
- do broad refactors
- break JSON or LLSD compatibility paths
- copy Firestorm code

---

## Files Most Likely Involved Next

- `crates/viewer_net/src/lib.rs`
- `crates/viewer_grid/src/lib.rs`
- `crates/viewer_net/examples/llsd_login_attempt.rs`
- `crates/viewer_net/examples/README.md`
- first-simulator handshake tests in `crates/viewer_net/src/lib.rs`
- `docs/RESEARCH/firestorm_login_flow.md`
- `docs/RESEARCH/firestorm_first_simulator_handshake.md`
- new capability/bootstrap research notes in `docs/RESEARCH/*`

---

## What To Check After The Next Change

- does seed capability fetch return a valid, parseable response?
- are capability responses logged in a sanitized way?
- are capability/bootstrap interpretations staying in `viewer_grid`?
- do automated tests still pass?
- were `CURRENT_STATE.md`, `HANDOFF.md`, `MASTER_PLAN.md`, and `TASKS.md` updated when project state changed?
