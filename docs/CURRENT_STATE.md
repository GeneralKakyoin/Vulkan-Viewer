# CURRENT_STATE.md

## Current Phase
Phase C - First-simulator handshake scaffolding

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
- Current handshake blocker is transport binding: typed first-simulator stages now exist, but `UseCircuitCode` / `CompleteAgentMovement` are not yet wired to simulator transport send/ack mechanics.

---

## Most Likely Immediate Work

- seed capability bootstrap kickoff
- expand capability interpretation typing in `viewer_grid` while keeping transport in `viewer_net`
- fetch and inspect early bootstrap capability responses from live login state
- stabilize one-shot EventQueueGet transport behavior and capture first event envelope
- stabilize one-shot SimulatorFeatures/EventQueueGet against live upstream instability and capture first parseable capability payload
- bind first-simulator handshake scaffold stages to minimal transport-side send/ack stubs in `viewer_net`
- keep capability/bootstrap logic separate from simulator transport
- expand diagnostics for post-login bootstrap flow

---

## Current Next Step

The smallest correct next step is:

**add minimal transport-bound handshake actions for `UseCircuitCode` and `CompleteAgentMovement` using the existing typed scaffold**

---

## Do Not Do Yet

- simulator connection
- event queue
- world streaming
- login UI integration
- broad viewer feature work
