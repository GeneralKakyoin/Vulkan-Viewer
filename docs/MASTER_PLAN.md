# MASTER_PLAN.md

## Purpose

This file is the durable operating map for the rewrite so new agents can resume from repository docs alone.

---

## Project Mission

Build a modern, maintainable Rust viewer for Second Life / OpenSim with compatibility-first behavior, clean crate boundaries, and long-term parity with conventional viewers.

This project is a networked virtual-world viewer, not a game engine.

---

## Scope

### Final Scope
Long-term parity with conventional SL/OpenSim viewers, delivered in phases.

### Delivery Phases
1. Foundation and architecture proof
2. Real login compatibility
3. Post-login bootstrap and capability startup
4. First connected world slice
5. Asset-backed rendering
6. Core viewer workflows
7. Parity expansion and compatibility hardening

---

## Core Principles

1. Compatibility first
2. Stability second
3. Maintainability third
4. Performance fourth
5. Feature breadth last

---

## Crate Responsibilities

- `viewer_app`: orchestration only
- `viewer_core`: shared domain state
- `viewer_render`: GPU/rendering internals
- `viewer_ui`: display/debug UI only
- `viewer_net`: transport/session/codec selection/diagnostics
- `viewer_grid`: request shaping/response interpretation/policy
- `viewer_asset`: reserved for asset lifecycle
- `viewer_platform`: reserved for platform integration

---

## Completed Milestones

- Milestone A: Project Foundation (completed)
- Milestone B: Runtime Slice (completed)
- Milestone C: Camera/Input Slice (completed)
- Milestone D: Spatial Sandbox (completed)
- Milestone E: Network/Grid Architecture (completed)
- Milestone F: Firestorm-Informed Login Research (completed)

### M1 - Auth Payload Compatibility
Status:
- Completed (endpoint recognition and auth-path progression achieved)

### M2 - Successful Real Login
Status:
- Completed (real successful login achieved; sensitive values intentionally not recorded in continuity docs)

---

## What Is Proven

- runtime foundation is stable
- renderer architecture is viable
- crate boundaries are holding
- real login transport and diagnostics are viable
- login compatibility has progressed to live successful login

---

## Current Known Gaps

### Networking / Protocol
Not implemented yet:
- seed capability usage
- event queue startup
- simulator transport
- region/world state streaming

### UI / Workflows
Not implemented yet:
- real login UI integration
- chat/inventory/map/preferences workflows

---

## Current Highest Priorities

### Priority 1 - Seed Capability Bootstrap
Goal:
Use returned bootstrap data safely, starting with seed capability and early capability startup.

Why:
- live login success is proven
- post-login bootstrap is now the critical path
- simulator/world integration should remain deferred until bootstrap is credible

### Priority 2 - Event Queue Startup
Goal:
Implement minimal post-login event queue bootstrap after seed capability handling.

### Priority 3 - Simulator Connection Readiness
Goal:
Prepare simulator handshake work only after capability/bootstrap startup is documented and stable.

---

## Ordered Future Milestones

### M3 - Seed Capability Bootstrap
### M4 - Event Queue / Early Live State
### M5 - Simulator Connection
### M6 - First Connected World Slice
### M7 - Asset and Appearance Foundations
### M8 - Viewer Usability Layer
### M9 - Parity Expansion

---

## What Must Not Happen

- do not mix grid semantics into `viewer_net`
- do not mix transport into `viewer_grid`
- do not jump to simulator/world integration before bootstrap is credible
- do not rely on chat memory over repository docs

---

## Required Update Discipline

When meaningful progress is made, update:
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/TASKS.md`
- relevant `docs/RESEARCH/*` when protocol understanding changes

---

## Current Recommended Next Step

Begin seed capability bootstrap from successful live login state.

Do not move to simulator/world integration until capability/bootstrap startup is credible and documented.
