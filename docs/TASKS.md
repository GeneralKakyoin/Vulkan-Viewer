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
- handshake work is scoped without collapsing crate boundaries

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
