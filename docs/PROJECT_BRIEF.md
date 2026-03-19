# PROJECT_BRIEF.md

## Project Overview

This repository is a from-scratch Rust rewrite of a Second Life / OpenSim viewer.

The aim is to replace legacy architectural constraints with a modern, maintainable, compatibility-first design while preserving the long-term ambition of conventional-viewer feature parity.

This project is not a game engine.  
It is a networked virtual-world viewer.

---

## Final Goal

Build a modern Rust-based viewer with long-term feature parity with conventional Second Life / OpenSim viewers while improving:

- architecture
- maintainability
- compatibility
- performance
- debuggability
- continuity across contributors and agents

---

## Final Scope

Over time, the viewer is intended to cover:

- login and session handling
- live world entry and region transitions
- camera, movement, and controls
- terrain, objects, avatars, and attachments
- textures, meshes, materials, and lighting
- inventory and appearance basics
- chat, IM, groups, and presence basics
- map/minimap and teleport workflows
- diagnostics, settings, and stability tooling
- compatibility with established SL/OpenSim server expectations

---

## Delivery Strategy

The project will not attempt full parity at once.

It is being delivered in phases:

1. Foundation and architecture proof
2. Real login compatibility
3. Post-login bootstrap and capability startup
4. First connected world slice
5. Asset-backed rendering
6. Core viewer workflows
7. Parity expansion and compatibility hardening

---

## Current Phase

The project is currently in:

**Phase 2 — Real login compatibility**

The runtime foundation is working. The current task is to move from architectural proof to real protocol acceptance.

---

## Primary Goals

- preserve strict crate boundaries
- keep transport and grid semantics separated
- reach a successful real login path
- build toward a connected-world slice without architectural debt
- maintain repository-resident continuity for future agents/models

---

## Non-Goals For Early Phases

The following are intentionally deferred:
- full conventional-viewer parity
- advanced UI polish
- full avatar fidelity
- media/voice completeness
- broad OpenSim divergence support
- simulator/world integration before login/bootstrap is credible

---

## Current Architecture Direction

- `viewer_app` = orchestration
- `viewer_core` = shared domain state
- `viewer_render` = GPU/rendering
- `viewer_ui` = egui/debug UI
- `viewer_net` = transport/session
- `viewer_grid` = grid-specific semantics
- `viewer_asset` = future asset pipeline
- `viewer_platform` = future OS/platform concerns

---

## Firestorm Policy

Firestorm is used only as a behavior reference.

It informs:
- login flow
- bootstrap sequencing
- compatibility expectations
- protocol behavior

It must not dictate:
- project structure
- architecture
- code reuse
- rendering design

---

## Current Success Condition

The current near-term success condition is:

**a real login request that reaches the live endpoint and progresses beyond the current payload/auth-compatibility blocker**

After that, the next success condition becomes:
- real successful login
- populated bootstrap/session data
- seed capability/bootstrap work
