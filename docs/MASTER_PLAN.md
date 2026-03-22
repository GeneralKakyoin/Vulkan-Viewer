# MASTER_PLAN.md

The durable operating map for the rewrite. Stable enough to orient a new agent from scratch
when combined with `ARCHITECTURE.md`, `HANDOFF.md`, and `TASKS.md`.
Update it only when the mission, scope, or core principles change materially.

---

## Project Mission

Build a modern, maintainable Rust viewer for Second Life / OpenSim with compatibility-first behavior, clean crate boundaries, and long-term parity with conventional viewers.

This project is a networked virtual-world viewer, not a game engine.

---

## Scope

### Final Scope
Long-term parity with conventional SL/OpenSim viewers, delivered in phases.

### Delivery Phases

1. **Phase 0: Project Setup** (Completed) — Workspace, crate layout, AGENTS.md.
2. **Phase 1: Rendering Bootstrap** (Completed) — wGPU init, egui, frame clearing.
3. **Phase 2: Core Runtime Slice** (Completed) — Camera, Scene, movement, depth buffer.
4. **Phase 3: Networking and Grid Login** (Completed) — LLSD/XML-RPC, real SL login, binary LLUDP handshake.
5. **Phase 4: First Connected World Slice** (Substantially Complete) — Bounded ingestion seam, coarse neighborhood, avatar proxies, chat UI.
6. **Phase 5: Asset-Backed Rendering** (Planned) — viewer_asset, LLVolume geometry, textures.
7. **Phase 6+: Parity expansion** — inventory, map, appearance, rigged mesh.

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

## Completed Milestones (Detailed)

- **Milestone A: Project Foundation (Phase 0-1)**
  - Rust workspace and crate layout
  - Baseline build/check workflow
  - Window + event loop (winit)
  - `wgpu` device/surface/depth init
  - `egui` overlay integration

- **Milestone B-D: Runtime & Spatial Sandbox (Phase 2)**
  - Camera data model and input wiring (mouselook + keyboard)
  - Minimal scene model with typed renderable instances
  - Spatial debug scene (ground plane, cube, axis marker)
  - Scene instance roles and MeshKind distinctions

- **Milestone E-F: Network & Login (Phase 3)**
  - `viewer_net` async connection/session skeleton
  - `viewer_grid` login adapter boundary
  - HTTP login transport with redirect/wire-fallback handling
  - Successful real live SL login (JSON/LLSD/XML-RPC)
  - Seed capability fetch + EventQueueGet with retry
  - First-simulator LLUDP handshake live-validated (AMC confirmed)
  - Post-AMC early traffic typing (Health, CoarseLoc, etc.)

- **Milestone 5: First Connected World Slice (Phase 4)**
  - In-process live-state worker + startup orchestration
  - Bounded ingestion seam with multi-lane typed payload paths
  - Bounded coarse neighborhood decode → seam → scene mapping
  - Avatar placeholder V1 (coarse + self + proxy mesh)
  - Chat/IM/nearby-people/profile UI
  - Pre-world object/state composition with lifecycle
  - Verified 2026-03-21

---

## What Is Proven

- runtime foundation is stable (wgpu/winit/egui stack)
- renderer architecture is viable
- crate boundaries are holding
- real SL login transport and diagnostics are viable (LLSD, XML-RPC, legacy credential shape)
- live SL login succeeds
- post-login bootstrap is viable: seed capability, EventQueueGet, SimulatorFeatures
- first-simulator LLUDP handshake succeeds in live runs (UseCircuitCode → AgentMovementComplete)
- early post-AMC traffic is typed and classified without broad decode
- bounded ingestion seam is viable: multi-lane decode → snapshot → seam → scene mapping
- pre-world object/state composition is viable with multi-entity family + lifecycle
- bounded coarse neighborhood ingestion is viable
- avatar placeholder presence (coarse + self + proxy mesh) is viable

---

## Current Known Gaps

### Networking / Protocol
Not implemented yet:
- broad world/object decode (planned)
- region/world state streaming beyond current bounded diagnostic slice
- `RegionHandshake` payload decode beyond message classification
- `SimName` extraction from RegionHandshake
- Crossed-region / EnableSimulator handoff flow

### Rendering
Not started yet:
- Phase 1: Spatial partitioning (Octree + frustum culling) — see `RENDERING_PHASE_1.md`
- Phase 2: LLVolume primitive geometry generation
- Phase 3+: Textures, materials, avatar rigging

### UI / Workflows
Not implemented yet:
- real login UI integration
- inventory/map shell
- keyboard navigation improvements in nearby/friends UI

---

## Current Highest Priorities

### Priority 1 - Avatar Placeholder Stabilization (T0)
Goal:
Stabilize avatar lifecycle (seen/updated/stale/removed) through reconnect cycles.
Ensure self placeholder persists when coarse ID blocks are absent.
Keep label projection readable while camera moves.

### Priority 2 - Narrower Bounded World/Object Refinement
Goal:
Extend the bounded seam/object-state slice with one more tightly-scoped refinement
  while focusing on architectural integrity.

### Priority 3 - Spatial Partitioning Phase 1
Goal:
Replace flat `Vec<RenderableInstance>` with an Octree + frustum culling system
so the scene can scale beyond diagnostic markers without GPU performance loss.
See `docs/RENDERING_PHASE_1.md` for the full action plan.

---

## Ordered Future Milestones

### M6 - Avatar Stabilization + Bounded World Refinement
### M7 - Spatial Partitioning (Phase 1 Rendering)
### M8 - Asset and Appearance Foundations (LLVolume + textures)
### M9 - Viewer Usability Layer (login UI, chat shell, map)
### M10 - Parity Expansion

---

## Must-Nots

These are non-negotiable. If a task seems to require crossing one of these lines, stop and document why before proceeding.

- **Do not mix grid semantics into `viewer_net`.**
  `viewer_net` sends and receives bytes. What those bytes mean for the grid belongs in `viewer_grid`.
  The `viewer_net` ↔ `viewer_grid` boundary is the most load-bearing architectural line in this project.

- **Do not mix transport mechanics into `viewer_grid`.**
  Retry policy, codec selection, LLUDP framing — these live in `viewer_net`.
  `viewer_grid` shapes intent and interprets meaning, nothing else.

- **Do not begin broad world/object decoding before dependencies are met.**
  Ensure the ingestion seam and spatial partitioning are ready.
  The active TASKS.md is the authoritative record of what is in scope.
- **Do not rely on session memory over repository docs.**
  Docs are the source of truth. A new agent with no prior context must be able to orient from docs alone.

- **Do not create or remove seam-owned scene roles outside the seam apply path.**
  Only `Scene::apply_world_object_ingestion_seam(...)` is allowed to create or remove seam-owned roles.

- **Do not suppress `Unknown` traffic classifications without a live observation record.**
  `Unknown` in post-AMC diagnostics is signal. Expanding typed classification requires live evidence
  documented in `docs/RESEARCH/post_amc_bootstrap_boundary_map.md`.

---

## Firestorm Policy

Firestorm is used only as a behavior reference. It informs login flow, bootstrap sequencing,
compatibility expectations, and protocol behavior. It must not dictate project structure,
architecture, code reuse, or rendering design.

All Firestorm-derived behavior must pass through clean Rust-typed interfaces in the correct owning crate.
Use the `firestorm_research` skill (`/.agents/skills/firestorm_research/SKILL.md`) when researching Firestorm.
Never let Firestorm-shaped logic live directly in `viewer_net` or collapse the crate boundary.

---

## Current Recommended Next Step

Stabilize the avatar placeholder lifecycle (T0) and extend the bounded seam/object-state slice
with one small bounded refinement.

See `docs/TASKS.md` for active task detail. See `docs/RENDERING_PHASE_1.md` for the spatial partitioning plan.
