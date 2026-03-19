# CURRENT_STATE.md

## Current Phase
Phase D - Early simulator traffic observation slice (post-bootstrap boundary)

Real login compatibility is now proven against the live Second Life endpoint. The active focus has moved to safe post-login bootstrap sequencing, beginning with seed capability handling.
Bootstrap capability probing is now sufficiently characterized to begin first-simulator handshake sequencing research/documentation.

---

## Working

### Runtime / Rendering
- renderer/ui/input/camera stack
- sandbox scene
- multiple objects
- ground plane
- cube
- world-axis marker
- depth buffer
- corrected controls

### Architecture
- clear crate boundaries
- `viewer_app` remains orchestration-only
- `viewer_render` owns rendering
- `viewer_ui` owns egui/debug UI
- `viewer_net` owns transport/session
- `viewer_grid` owns grid-specific semantics

### Login Stack
- HTTP login transport
- redirect handling
- typed login request/result/bootstrap models
- login diagnostics/trace
- JSON codec
- minimal LLSD codec
- experimental XML-RPC `login_to_simulator` codec path in `viewer_net`
- LLSD `passwd` normalization to legacy `$1$<md5>` form
- XML-RPC credential shaping normalized to legacy `first`/`last` semantics
- configurable wire format
- manual login attempt example
- successful real live login achieved
- minimal seed capability fetch transport path implemented in `viewer_net`
- one-shot EventQueueGet inspection path implemented in `viewer_net`
- one-shot SimulatorFeatures inspection path implemented in `viewer_net`
- one-shot MapLayer inspection path implemented in `viewer_net`
- capability one-shot requests now send explicit LLSD `Accept` headers
- MapLayer HTTP 405 is now classified explicitly as likely non-HTTP/legacy-UDP semantics in this one-shot transport path
- typed first-simulator handshake scaffold is now implemented in `viewer_net`:
  - bootstrap prerequisites ready
  - first region target known
  - `UseCircuitCode` stage
  - `CompleteAgentMovement` stage
  - waiting-for-`AgentMovementComplete` stage
  - terminal `AgentMovementComplete` stage
- strict ordered handshake-stage transition API exists with focused test coverage
- first-simulator handshake stages are now transport-bound in `viewer_net`:
  - real `UseCircuitCode` send action (UDP datagram transport scaffold)
  - real `CompleteAgentMovement` send action (UDP datagram transport scaffold)
  - ordered send flow enforced by stage-aware send APIs
  - per-send diagnostics captured for attempts/success/failure
- receive-side handshake observation/classification slice is now implemented in `viewer_net`:
  - one-shot UDP receive hook for handshake datagrams
  - typed inbound message classification (`AgentMovementComplete`, `RegionHandshake`, `EnableSimulator`, `Irrelevant`)
  - typed receive diagnostics (classification signal, stage before/after, stage advancement flag)
  - explicit `AgentMovementComplete` receive-side stage confirmation when waiting
- receive-side handshake classification now includes a minimal typed LLUDP packet-header/message-number decoder:
  - decodes high/medium/low message-number forms from simulator UDP packet shape
  - classifies handshake-relevant low-frequency messages by packet message number:
    - `RegionHandshake` (low 148)
    - `EnableSimulator` (low 151)
    - `AgentMovementComplete` (low 250)
  - keeps JSON/text matching only as compatibility fallback (no longer primary path)
- receive-side diagnostics now include decode evidence details:
  - decode source (`PacketMessageNumber`, `JsonField`, `TextScan`, `Unknown`)
  - optional decoded packet message number for unknown packet-shaped traffic
- receive-side diagnostics now include explicit traffic-scope classification:
  - `BootstrapRelevant`
  - `TransportControl`
  - `LikelyBroaderTraffic`
  - `Unknown`
- handshake-stage confirmation was tightened:
  - `AgentMovementComplete` stage advancement now requires packet-message decode evidence (not JSON/text fallback alone)
- one-shot probe path now exists to send `UseCircuitCode` + `CompleteAgentMovement` and wait on the same UDP socket:
  - `Connection::probe_first_simulator_handshake_once(...)`
  - manual example wiring added for live receive validation
- bounded receive-window probe now exists on the same socket:
  - `Connection::probe_first_simulator_handshake_window(...)`
  - bounded packet count + timeout with ordered observation reporting
  - receive diagnostics now include `observation_index`
- bounded post-movement tail probing now exists (opt-in):
  - `Connection::probe_first_simulator_handshake_window_with_tail(...)`
  - default bounded-window behavior remains unchanged (`tail=0`)
  - probe report now includes first `AgentMovementComplete` index and post-movement observation count
- outbound handshake send packets are now protocol-shaped binary LLUDP datagrams:
  - reliable LLUDP flags + packet-id header
  - low-frequency message numbers for `UseCircuitCode` (3) and `CompleteAgentMovement` (249)
  - field ordering/layout aligned to template expectations (UUID + U32 blocks)
  - per-send diagnostics now include packet id and packet message number
- live same-socket probe now receives parseable real inbound simulator traffic:
  - inbound low message `387` classified as `AgentDataUpdate`
  - inbound low message `1` classified as `TestMessage`
  - inbound low message `250` classified as `AgentMovementComplete`
  - inbound classification source is typed packet message-number decode
  - handshake stage now advances to `AgentMovementComplete` in live bounded-window probe runs
  - typed inbound classification now also includes:
    - `HealthMessage` (low 138)
    - `SimulatorViewerTimeMessage` (low 150)
- live bounded post-movement tail probe now captures immediate post-movement traffic:
  - with bounded post-AMC tails (`tail=2..4`), observed sequence examples include:
    - `AgentDataUpdate` -> `TestMessage` -> `AgentMovementComplete` -> `PacketAck` -> `HealthMessage`
    - additional early post-AMC traffic now typed as likely broader traffic:
      - `OnlineNotification` (`0xFFFF0142`, low 322)
      - `ViewerEffect` (`0x0000FF11`, medium 17)
      - `CoarseLocationUpdate` (`0x0000FF06`, medium 6)
    - remaining medium unknowns can still appear in this window and remain explicitly `Unknown`
  - `0xfffffffb` is now typed as `PacketAck` (transport-control) instead of unmapped traffic
  - known typed packets are now explicitly separated from likely broader traffic in diagnostics
  - bounded probe report now includes post-boundary summary counters and ordered post-boundary kinds

### Research / Continuity
- Firestorm login flow documented
- Firestorm first-simulator handshake sequence documented
- continuity-stack docs established
- scope model established
- workflow discipline defined in AGENTS
- autonomous milestone-driven execution policy clarified in AGENTS
- repo Codex config and low-cost subagent profiles added for bounded worker/explorer use
- skills policy now explicitly checks `.agents/skills/viewer/` first (then `.agents/skills/` fallback)

---

## Current Blocker

Login payload compatibility is no longer the primary blocker. The immediate blocker is implementing post-login bootstrap safely while preserving crate boundaries and avoiding premature simulator/world integration.

Current concrete blocker inside bootstrap:
- EventQueueGet one-shot now reaches the service path but currently returns upstream proxy/server failure in live conditions (no event payload yet).
- EventQueueGet one-shot now has bounded retry diagnostics; live attempts show retryable mixed failures (HTTP 500 proxy-style responses and occasional transport send failure) with no events returned yet.
- SimulatorFeatures one-shot still returns HTTP 503 in live conditions (request method/shape now aligned to observed Firestorm behavior: GET).
- MapLayer one-shot remains non-parseable by HTTP and is now classified as likely legacy-UDP behavior for this viewer path (live 405 + Firestorm behavior evidence).
- Current handshake blocker narrowed to post-movement inbound coverage:
  - first inbound progression to `AgentMovementComplete` is now observed and classified
  - immediate post-movement tail is now observable in bounded live runs
  - repeated `0xfffffffb` is now classified as transport-control `PacketAck`
  - no additional repeated post-AMC packet IDs have been confirmed as bootstrap-gating so far
  - current gap is selecting the first bounded non-bootstrap simulator-traffic diagnostics slice (still no broad world/object decode)

---

## Most Likely Immediate Work

- seed capability bootstrap kickoff
- expand capability interpretation typing in `viewer_grid` while keeping transport in `viewer_net`
- fetch and inspect early bootstrap capability responses from live login state
- stabilize one-shot EventQueueGet transport behavior and capture first event envelope
- stabilize one-shot SimulatorFeatures/EventQueueGet against live upstream instability and capture first parseable capability payload
- bind first-simulator handshake scaffold stages to minimal transport-side send/ack stubs in `viewer_net`
- add bounded receive/ack classification for first-simulator handshake transport actions without starting world integration
- improve outbound handshake packet fidelity toward real LLUDP message layout while preserving current scaffold boundaries
- extend typed UDP decode from message identity into minimal block/field extraction for handshake-relevant inbound messages
- classify and observe additional early inbound low-frequency packets that appear before `AgentMovementComplete` in live traffic
- expand typed coverage for additional early post-movement inbound traffic while keeping handshake scope bounded
- map and type only repeated bootstrap-relevant post-movement packet IDs; keep transport-control and broader-traffic diagnostics explicit
- keep capability/bootstrap logic separate from simulator transport
- expand diagnostics for post-login bootstrap flow

---

## Current Next Step

The smallest correct next step is:

**keep bootstrap boundary fixed and expand only narrow early non-bootstrap simulator traffic observation (for example medium handoff/control visibility) without broad world-state decoding**

---

## Do Not Do Yet

- simulator connection
- event queue
- world streaming
- login UI integration
- broad viewer feature work
