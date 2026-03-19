# Firestorm First-Simulator Handshake (Reference Research)

## Question
After successful login, what is the minimal ordered sequence Firestorm uses to bring up the first simulator connection, and what are the key dependencies before early world traffic begins?

## Scope
- Source inspected: `reference/firestorm` (behavior reference only).
- Goal: sequence and dependency extraction only; no architecture or code reuse.

## Relevant Firestorm Files
- `reference/firestorm/indra/newview/llstartup.cpp`
  - `process_login_success_response(...)`
  - startup transitions around `STATE_WORLD_INIT`, `STATE_SEED_GRANTED_WAIT`, `STATE_WORLD_WAIT`, `STATE_AGENT_SEND`, `STATE_AGENT_WAIT`
- `reference/firestorm/indra/newview/llworld.cpp`
  - `process_enable_simulator(...)`
  - `LLEstablishAgentCommunication::post(...)`
  - `process_region_handshake(...)`
- `reference/firestorm/indra/newview/llviewerregion.cpp`
  - `setSeedCapability(...)`
  - `setCapability(...)` handling for `EventQueueGet` and `SimulatorFeatures`
- `reference/firestorm/indra/newview/llviewermessage.cpp`
  - `send_complete_agent_movement(...)`
  - `process_agent_movement_complete(...)`
- `reference/firestorm/indra/newview/lleventpoll.cpp`
  - event queue request/ack behavior after capability availability
- `reference/firestorm/scripts/messages/message_template.msg`
  - message semantics for `UseCircuitCode`, `RegionHandshake`, `EnableSimulator`, `CompleteAgentMovement`, `AgentMovementComplete`

## Observed Handshake Sequence (First Region)
1. Login success response is unpacked into session/bootstrap fields:
   - `agent_id`, `session_id`, `secure_session_id`, `circuit_code`
   - first simulator host/port and region handle
   - `seed_capability`
2. First region object is created and assigned as agent region context.
3. Seed capability is applied to that region (`setSeedCapability(...)`), and startup waits for seed capability grant.
4. Startup gate: Firestorm explicitly treats seed grant as a prerequisite before new simulator messages.
5. Circuit enable begins:
   - viewer enables circuit to first simulator host
   - sends `UseCircuitCode` with `code + session_id + agent_id`
6. Startup waits for `UseCircuitCode` ack/callback completion.
7. Agent movement bring-up:
   - sends `CompleteAgentMovement` with `agent_id + session_id + circuit_code`
   - waits for `AgentMovementComplete`
8. On `AgentMovementComplete`, identity is validated and movement/session startup is considered complete.
9. In parallel for region graph expansion:
   - `EnableSimulator` can trigger circuit enable + `UseCircuitCode` for additional simulators
   - `LLEstablishAgentCommunication` can deliver per-region `seed-capability` updates
10. Capability-driven startup continues:
   - `EventQueueGet` capability creation starts event polling
   - `SimulatorFeatures` capability triggers immediate fetch

## Early Dependency Notes
- Seed-capability readiness is an explicit gate before simulator message startup.
- `UseCircuitCode` must happen before `CompleteAgentMovement`.
- `AgentMovementComplete` acts as the first strong completion checkpoint for initial simulator presence.
- Capability startup (`EventQueueGet`, `SimulatorFeatures`) is coupled to region capability installation, not a separate login-only phase.

## Protocol Semantics Noted
- `UseCircuitCode`: viewer -> sim; provides circuit/session/agent identity.
- `RegionHandshake`: sim -> viewer after receiving `UseCircuitCode`.
- `EnableSimulator`: sim -> viewer instruction to prepare another simulator circuit.
- `CompleteAgentMovement`: viewer -> sim finalizes entering the region.
- `AgentMovementComplete`: sim -> viewer confirms movement completion with position/look-at data.
- `PacketAck` (fixed `0xFFFFFFFB`): transport-control ack for reliable packets; should be treated as UDP reliability-plane traffic, not bootstrap/world-state payload.

### Packet Identity Decode Details (for receive-side classification)
- Firestorm template reader (`lltemplatemessagereader.cpp`) decodes message identity from bytes after the LL packet ID header (`LL_PACKET_ID_SIZE = 6`):
  - high frequency: first header byte != `0xFF`
  - medium frequency: `0xFF` then one-byte ID
  - low frequency: `0xFF 0xFF` then two-byte network-order ID
- For first-simulator handshake-relevant inbound messages from `message_template.msg`:
  - `RegionHandshake` = low `148`
  - `EnableSimulator` = low `151`
  - `AgentMovementComplete` = low `250`
- For post-`AgentMovementComplete` control traffic seen in live bounded-tail probes:
  - `PacketAck` = fixed low `0xFFFB` (`0xFFFFFFFB` full number)
  - classification implication: `TransportControl` instead of bootstrap/world-state semantic message
- Additional post-`AgentMovementComplete` IDs now observed in bounded tails:
  - `OnlineNotification` = low `322` (`0xFFFF0142`) -> likely broader traffic
  - `ViewerEffect` = medium `17` (`0x0000FF11`) -> likely broader traffic
  - `CoarseLocationUpdate` = medium `6` (`0x0000FF06`) -> likely broader traffic
  - `AttachedSound` = medium `13` (`0x0000FF0D`) -> likely broader traffic
  - implication: these are not required bootstrap gates in this phase
- Practical translation for this rewrite:
  - minimal receive classifier can be protocol-aware by decoding message number first,
  - then mapping those low IDs before any heuristic text/JSON fallback.

## Crate-Boundary Mapping For This Rewrite

### `viewer_grid` (meaning/policy)
- Keeps typed meaning of login bootstrap prerequisites:
  - required fields for first simulator bring-up intent
  - policy-level classification of missing/invalid bootstrap data
- Defines typed handshake-stage policy states, but not transport mechanics.

### `viewer_net` (transport/session mechanics)
- Owns future handshake transport sequencing and diagnostics:
  - seed-grant precondition tracking
  - `UseCircuitCode` send/ack lifecycle
  - `CompleteAgentMovement` send/ack lifecycle
  - handshake timeout/retry/error classification
- Owns capability-trigger transport mechanics once region seed caps are known.

### Orchestration layer (later, not this task)
- Coordinates when to begin simulator handshake work and how to surface progress.
- Must remain thin and avoid owning protocol semantics or transport internals.

## What Remains Uncertain
- Exact minimum retry policy needed for first-region `UseCircuitCode`/movement completion in this rewrite.
- How much of Firestorm's neighboring-region setup should be deferred until after first-region success is proven.
- Whether additional startup messages are mandatory for a minimal first connected slice versus optional for richer behavior.
- Real-live acceptance threshold for outbound handshake packet payload fidelity in this rewrite:
  - same-socket one-shot probe now removes local receive-port continuity ambiguity,
  - after LLUDP wire-fidelity pass and bounded receive window, live probing now observes:
    - `AgentDataUpdate` (low 387)
    - `TestMessage` (low 1)
    - `AgentMovementComplete` (low 250)
  - remaining uncertainty is no longer initial movement-complete arrival, but which additional early packets should be typed next before broader simulator-state work.

## Practical Next Step (Non-implementation)
Document a minimal first-region handshake state chart for this codebase:
- `SeedGranted` -> `UseCircuitCodeSent` -> `CircuitAcked` -> `CompleteAgentMovementSent` -> `MovementCompleteReceived`
with explicit failure/timeout exits and crate ownership per transition.
