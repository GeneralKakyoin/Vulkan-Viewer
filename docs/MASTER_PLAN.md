# MASTER_PLAN.md

## Purpose

This file is the durable operating map for the rewrite.

It exists so a new model or agent can understand:
- what the project is
- what has already been proven
- what is blocked
- what should happen next
- what must not be broken

This repository must remain understandable without relying on chat memory.

---

## Project Mission

Build a modern, maintainable, high-performance Rust rewrite of a Second Life / OpenSim viewer with:

- compatibility-first design
- cross-platform support
- clean crate boundaries
- strong continuity across agents and chats
- eventual feature parity with conventional viewers
- modern rendering, tooling, and architecture

This project is not a game engine.  
It is a networked virtual-world viewer.

---

## Scope

### Final Scope
The long-term target is feature parity with conventional Second Life / OpenSim viewers while replacing legacy architecture with a modern Rust-based design.

This includes, over time:
- login and session handling
- live world connection
- region/bootstrap flows
- camera and movement
- terrain, objects, avatars, and attachments
- textures, meshes, materials, and lighting
- inventory and appearance workflows
- chat, IM, groups, friends, and presence basics
- map/minimap and teleport workflows
- diagnostics, settings, stability tooling
- compatibility with established SL/OpenSim runtime expectations

### Delivery Scope
The project will not attempt full parity at once.

It is delivered in phases:
1. Foundation and architecture proof
2. Real login compatibility
3. Post-login bootstrap and capability startup
4. First connected world slice
5. Asset-backed rendering
6. Core viewer workflows
7. Parity expansion and compatibility hardening

### Explicit Early Non-Goals
Deferred until the live connection path is credible:
- full feature parity
- advanced UI polish
- complete avatar fidelity
- media/voice completeness
- full OpenSim divergence handling

---

## Core Principles

1. Compatibility first
2. Stability second
3. Maintainability third
4. Performance fourth
5. Feature breadth last

Additional project rules:
- Firestorm is behavior reference only
- crate boundaries must remain explicit
- transport and grid semantics must stay separated
- continuity docs must be kept current
- prefer narrow, reversible progress

---

## Current Technology Direction

Preferred stack:
- Rust
- wgpu
- winit
- egui
- bevy_ecs (standalone only if needed)
- tokio
- serde
- tracing

Current login transport stack:
- reqwest
- JSON codec
- LLSD codec (minimal)
- typed grid adapter boundary
- real HTTP transport
- redirect handling
- diagnostic tracing

---

## Crate Responsibilities

### `viewer_app`
Owns:
- startup
- event loop wiring
- window orchestration
- camera/input wiring
- scene wiring

### `viewer_core`
Owns:
- camera
- transforms
- scene types
- shared domain types

### `viewer_render`
Owns:
- GPU init
- render pipelines
- buffers
- depth
- camera uniforms
- object rendering
- debug geometry

### `viewer_ui`
Owns:
- egui integration
- debug panels
- display-only diagnostics

### `viewer_net`
Owns:
- connection lifecycle
- session state
- HTTP transport
- redirect following
- codec selection
- login diagnostics

### `viewer_grid`
Owns:
- login shaping
- response interpretation
- result classification
- bootstrap/session typing
- SecondLifeAdapter behavior

### `viewer_asset`
Reserved for:
- fetching
- caching
- asset lifecycle

### `viewer_platform`
Reserved for:
- OS/platform integration

---

## Firestorm Usage Policy

Firestorm source is present as reference only.

Use Firestorm for:
- login/session behavior
- capability/bootstrap sequence
- compatibility behavior
- protocol expectations
- edge-case research

Do not use Firestorm for:
- architecture
- code reuse
- repo structure
- rendering model
- dependency patterns

Research workflow:
1. identify behavior
2. document in `docs/RESEARCH`
3. translate into clean Rust boundaries

---

## Completed Milestones

### Milestone A — Project Foundation
Completed:
- repo structure established
- core docs direction established
- AGENTS discipline established
- architecture direction fixed

### Milestone B — Runtime Slice
Completed:
- app launches
- window opens
- wgpu initializes
- egui overlay renders
- app exits cleanly

### Milestone C — Camera/Input Slice
Completed:
- keyboard movement
- mouse-look
- camera state in `viewer_core`
- debug display of camera data

### Milestone D — Spatial Sandbox
Completed:
- visible 3D geometry
- multiple objects
- depth buffer
- ground plane
- cube
- world-axis marker
- corrected controls

### Milestone E — Network/Grid Architecture
Completed:
- `viewer_net` skeleton
- `viewer_grid` adapter boundary
- typed login request/result/bootstrap models
- mock login handshake
- real HTTP login transport
- bounded redirect handling
- login diagnostics/trace
- codec boundary
- JSON codec
- minimal LLSD codec
- configurable wire format
- manual LLSD login example

### Milestone F — Firestorm-Informed Login Research
Completed:
- Firestorm login flow identified and documented
- boundary split extracted:
  - `viewer_net` = transport/session
  - `viewer_grid` = shaping/interpretation
  - app/startup = UX/orchestration

---

## What Is Proven

Already proven in code:
- the runtime foundation works
- the renderer architecture is viable
- the crate boundaries are workable
- input/camera/scene sandbox is viable
- transport and grid logic can be separated cleanly
- redirect-aware login flow works architecturally
- diagnostics can be captured safely
- wire format can evolve without architecture breakage

---

## Current Known Gaps

### Rendering / World
Not implemented yet:
- real scene graph
- terrain/parcel rendering
- avatar rendering
- live world object ingestion
- asset-driven textures and meshes
- PBR/material pipeline

### Networking / Protocol
Not implemented yet:
- simulator UDP path
- seed capability use
- event queue polling
- region bootstrap
- world state streaming
- inventory/capability usage

### Login
Partially implemented:
- real HTTP transport works
- LLSD codec exists
- request reaches the real endpoint
- diagnostics work
- payload/auth compatibility is still incomplete
- live successful login is not yet proven

### UI
Not implemented yet:
- real login UI
- chat/inventory/map/preferences
- conventional viewer workflows
- docking layout

---

## Current Highest Priorities

### Priority 1 — Real Login Compatibility
Goal:
make the real LLSD login request shape compatible enough that the endpoint recognizes credentials correctly.

Why:
- current blocker is payload correctness, not architecture
- transport and diagnostics are already real

### Priority 2 — Successful Live Login
Goal:
obtain a valid `GridLoginResult::Success` and populated real bootstrap/session data.

### Priority 3 — Post-Login Bootstrap
Goal:
use returned bootstrap data safely, starting with seed capability and early capability startup.

---

## Ordered Future Milestones

### M1 — Auth Payload Compatibility
Goal:
the endpoint recognizes credentials and returns a meaningfully different result than “Missing password.”

Subtasks:
- finalize `passwd` semantics
- finalize username vs first/last handling
- finalize LLSD request envelope
- improve LLSD fixtures and decode cases
- document remaining uncertainty

Done when:
- the live endpoint returns a new auth outcome
- or login succeeds
- or the remaining mismatch is precisely characterized

### M2 — Successful Real Login
Goal:
obtain a valid `GridLoginResult::Success` from a live grid.

Subtasks:
- validate response parsing
- validate bootstrap fields
- validate session transition behavior

Done when:
- `Connection` reaches `LoggedIn`
- session/bootstrap data is populated from a real response

### M3 — Seed Capability Bootstrap
Goal:
begin using seed capability and first-simulator bootstrap data.

Subtasks:
- document capability startup sequence from Firestorm behavior
- implement minimal capability client
- fetch and inspect bootstrap responses
- keep this separate from simulator transport

Done when:
- seed capability can be queried
- capability bootstrap data can be logged/interpreted

### M4 — Event Queue / Early Live State
Goal:
start receiving post-login world-related updates.

Subtasks:
- implement minimal `EventQueueGet` polling
- define event envelope handling
- log and classify early events

Done when:
- the viewer can receive and inspect live event queue traffic

### M5 — Simulator Connection
Goal:
establish simulator-side handshake after successful login/bootstrap.

Subtasks:
- document handshake sequence from Firestorm behavior
- implement minimal region/simulator connection
- receive first world-state data

Done when:
- the viewer has a live connection to the first region

### M6 — First Connected World Slice
Goal:
map live world state into the existing sandbox renderer.

Subtasks:
- define world object ingestion
- bridge region/object updates into scene representation
- render first live placeholders

Done when:
- logged-in viewer renders connected world-derived placeholder content

### M7 — Asset and Appearance Foundations
Goal:
move from placeholders toward real assets.

Subtasks:
- texture fetch/cache
- mesh fetch/cache
- asset lifecycle basics
- appearance/avatar research
- inventory-root support if needed

### M8 — Viewer Usability Layer
Goal:
become a usable client, not just a technical prototype.

Subtasks:
- login UI
- status/debug panels
- settings
- chat/map/inventory shells
- error visibility

### M9 — Parity Expansion
Goal:
expand toward conventional viewer parity.

Subtasks:
- groups/friends/presence
- attachments/wearables
- media/voice where feasible
- compatibility hardening
- performance passes
- OpenSim divergence handling

---

## What Must Not Happen

Avoid these failures:
- mixing grid semantics into `viewer_net`
- mixing rendering into `viewer_app`
- using Firestorm structure as architecture guidance
- jumping to simulator work before login/bootstrap is credible
- adding broad features without updating docs
- relying on model memory instead of repository docs

---

## Required Update Discipline

When meaningful progress is made, update:
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/TASKS.md`
- relevant ADRs if a real decision changed
- relevant `docs/RESEARCH/*` if protocol understanding changed

This matters more than preserving chat context.

---

## Development Discipline

All contributors and agents must follow the workflow defined in `AGENTS.md`.

This project relies on repository-based continuity, not chat memory.

---

## Resume Protocol

A new agent should read:
1. `AGENTS.md`
2. `docs/MASTER_PLAN.md`
3. `docs/CURRENT_STATE.md`
4. `docs/HANDOFF.md`
5. `docs/ARCHITECTURE.md`
6. relevant `docs/RESEARCH/*`
7. then inspect code

Before acting, the agent should identify:
- current phase
- immediate blocker
- smallest next step
- boundaries not to break

---

## Current Recommended Next Step

Continue login compatibility alignment until the real LLSD login endpoint recognizes credentials correctly.

Do not move to simulator/bootstrap until login compatibility is either:
- working
- or well-characterized and documented

---

## Success Criteria for Continuity

This repository is in good continuity shape if a new agent can answer all of these from docs alone:

- What is the project trying to become?
- What already works?
- What is blocked?
- What crate owns what?
- What should happen next?
- What must not change?
- How should Firestorm be used?

If those answers are not obvious, update docs before writing more code.
