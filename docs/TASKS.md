# TASKS.md

## Current Focus
Only work on the smallest steps that advance post-login bootstrap while preserving crate boundaries.

---

## Active Tasks

### T1 - Seed capability bootstrap kickoff
Why it matters:
Live login is proven; seed capability bootstrap is now the critical path.

Dependencies:
- successful real login path
- existing transport/session diagnostics

Done when:
- first seed capability request is issued safely
- response is captured and sanitized
- response is parseable and classified at the correct boundary

---

### T2 - Define capability boundary ownership
Why it matters:
Bootstrap logic can easily erode `viewer_net` vs `viewer_grid` boundaries.

Dependencies:
- T1
- Firestorm behavior reference for ordering only

Done when:
- transport mechanics stay in `viewer_net`
- capability meaning/typing stays in `viewer_grid`
- boundary is documented clearly

---

### T3 - Bootstrap diagnostics and fixtures
Why it matters:
Post-login work needs reproducible traces and tests.

Dependencies:
- T1/T2 findings

Done when:
- capability/bootstrap traces are sanitized and useful
- focused tests/fixtures cover current bootstrap assumptions
- non-viable HTTP probe paths are explicitly classified (for example, MapLayer likely legacy/non-HTTP in this phase)

---

### T4 - Continuity updates for bootstrap phase
Why it matters:
Milestone transition must be obvious to future agents.

Dependencies:
- every meaningful bootstrap change

Done when:
- `docs/CURRENT_STATE.md` updated
- `docs/HANDOFF.md` updated
- `docs/MASTER_PLAN.md` updated if project priority/milestone status changes
- relevant `docs/RESEARCH/*` updated when protocol understanding changes

### T5 - First-simulator handshake research kickoff
Why it matters:
Bootstrap inspection is now characterized enough that the next risk is handshake sequencing ambiguity.

Dependencies:
- bootstrap capability findings
- successful real login/session bootstrap

Done when:
- first-region handshake ordering is documented from Firestorm behavior reference
- dependencies between seed grant, circuit setup, and movement completion are explicit
- crate-boundary ownership is mapped without starting simulator transport implementation
- typed handshake-stage scaffold exists in `viewer_net` with focused transition tests

---

## Next Tasks After Bootstrap Credibility

### T6 - Event queue startup
Done when:
- minimal `EventQueueGet` polling works
- early live events can be received and classified

### Near-term blocker focus (within current bootstrap inspection)
- stabilize one-shot `SimulatorFeatures` fetch against current live `503 Service Unavailable`
- stabilize one-shot `EventQueueGet` against current retryable upstream/proxy failures
- treat `MapLayer` HTTP one-shot as classified blocker (likely legacy/non-HTTP path) unless new protocol evidence emerges

### T7 - Simulator handshake preparation
Done when:
- simulator handshake requirements are documented
- typed first-simulator handshake stages are bound to minimal transport send/ack actions in `viewer_net`
- send-side handshake diagnostics exist for attempts/success/failure
- receive-side handshake observation/classification exists and can drive `AgentMovementComplete` stage confirmation
- receive-side handshake identity classification is driven by typed UDP message-number decode (not heuristic-only matching)
- handshake work is scoped without collapsing crate boundaries

### T7.1 - Handshake payload decode fidelity
Done when:
- `AgentMovementComplete` receive handling includes minimal typed block/field extraction (beyond message identity)
- receive diagnostics can surface typed decode success/failure for handshake payload fields
- out-of-order/irrelevant message handling remains stable with focused tests

### T7.2 - Live handshake probe fidelity
Done when:
- first-simulator one-shot probe sends handshake messages and listens on the same socket/port
- live manual probe outcome is captured with explicit diagnostics
- if no inbound packets are observed, blocker is classified clearly (transport continuity vs outbound payload fidelity)

### T7.3 - Handshake progression observation
Done when:
- first live inbound handshake-relevant packet after wire-fidelity pass is classified (currently `AgentDataUpdate`)
- receive-side typed classification covers observed early packets without heuristic dependence
- next blocker from `AgentDataUpdate` to `AgentMovementComplete` is explicitly characterized

Status update:
- materially advanced: bounded probe now observes `AgentDataUpdate` -> `TestMessage` -> `AgentMovementComplete` and stage advancement is confirmed in live runs

### T7.4 - Early post-movement inbound typing
Done when:
- the next inbound packets after initial `AgentMovementComplete` are captured with bounded diagnostics
- typed classification is expanded only for the small set repeatedly observed in this stage
- progression remains handshake/bootstrap-focused (no world-state subsystem implementation)

Status update:
- materially advanced: bounded post-movement tail capture now exists (`probe_first_simulator_handshake_window_with_tail`)
- live tail run (`tail=2`) observed post-movement packets beyond handshake completion, including:
  - `PacketAck` (`0xfffffffb`) now typed as transport-control
  - `HealthMessage` (typed, likely broader traffic)
- diagnostics now separate:
  - bootstrap-relevant traffic
  - transport-control traffic
  - likely broader traffic
- additional repeated post-AMC IDs are now typed where justified:
  - `OnlineNotification` (`0xffff0142`) as likely broader traffic
  - `ViewerEffect` (`0x0000ff11`) as likely broader traffic
- `CoarseLocationUpdate` (`0x0000ff06`) is now typed as likely broader traffic
- probe reports now include post-boundary summary counts + ordered post-boundary kinds for durable diagnostics
- small typed early-simulator-traffic scaffold now exists in `viewer_net` and captures observed broader packet kinds separately from bootstrap/control diagnostics
- scaffold diagnostics now include per-kind early-traffic summary counts
- phase status: early simulator traffic consolidation is done enough for current scope
- next phase: optionally type only targeted medium handoff IDs (`CrossedRegion` / `ConfirmEnableSimulator`) if repeatedly observed, without broad world/object decoding

### T8 - First connected world slice
Done when:
- world-derived placeholder state can be rendered from live connection data

---

## Explicitly Deferred

Do not work on these yet unless the plan changes:
- simulator/world integration implementation beyond bootstrap staging
- viewer login UI integration
- asset-backed world rendering
- inventory/chat/map shells
- media/voice
- broad OpenSim divergence support
