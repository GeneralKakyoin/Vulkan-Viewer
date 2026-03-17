# ROADMAP.md

## Overview

This roadmap defines the phased development of the viewer.

Development follows a **vertical slice approach**, prioritizing working functionality over isolated subsystems.

---

## Phase 0 — Project Setup

### Goals

* Establish project structure
* Define architecture and constraints
* Set up development environment

### Tasks

* Create Rust workspace and crates
* Add core documentation:

  * PROJECT_BRIEF.md
  * ARCHITECTURE.md
  * AGENTS.md
* Configure VS Code
* Set up formatting and linting
* Define build and run commands

### Outcome

A clean, buildable project skeleton ready for development.

---

## Phase 1 — Rendering Bootstrap (M1)

### Goals

Create a minimal application shell with rendering and UI.

### Tasks

* Initialize window using winit
* Initialize wgpu device and surface
* Implement render loop
* Clear screen each frame
* Integrate egui
* Render debug panel

### Outcome

A running application with:

* window
* GPU initialized
* UI overlay

---

## Phase 2 — Core Systems Foundation

### Goals

Define internal data flow and system structure.

### Tasks

* Implement basic ECS world (bevy_ecs)
* Define core data structures (scene, entities)
* Establish system scheduling
* Define resource management patterns

### Outcome

A structured internal application model ready for expansion.

---

## Phase 3 — Networking Layer

### Goals

Enable communication with grid servers.

### Tasks

* Implement login/session flow
* Add HTTP/CAPS handling
* Implement UDP/message system
* Create network abstraction layer

### Outcome

Ability to connect to a grid and receive data.

---

## Phase 4 — First World Rendering (M2)

### Goals

Display actual world data.

### Tasks

* Parse incoming scene data
* Create placeholder geometry
* Render terrain/objects (basic)
* Implement camera system
* Enable movement controls

### Outcome

User can:

* log in
* see a world
* move camera/avatar

---

## Phase 5 — Asset System

### Goals

Handle real assets and caching.

### Tasks

* Implement asset fetching
* Add local cache system
* Load textures and meshes
* Integrate assets into renderer

### Outcome

World renders with real assets instead of placeholders.

---

## Phase 6 — Grid Abstraction

### Goals

Support multiple grids cleanly.

### Tasks

* Define grid interface
* Implement Second Life adapter
* Implement OpenSim adapter
* Isolate grid-specific logic

### Outcome

Multi-grid compatibility without code duplication.

---

## Phase 7 — UI Expansion

### Goals

Build usable viewer interface.

### Tasks

* Inventory UI
* Chat system
* Settings panel
* Debug tools
* Docking/layout system

### Outcome

Functional user interface for interaction.

---

## Phase 8 — Rendering Improvements

### Goals

Improve visual quality and performance.

### Tasks

* Material system
* Lighting improvements
* Level of detail (LOD)
* GPU optimizations
* Performance profiling

### Outcome

Visually improved and efficient renderer.

---

## Phase 9 — Stability & Compatibility

### Goals

Ensure reliability and correctness.

### Tasks

* Bug fixing
* Compatibility adjustments
* Edge-case handling
* Regression testing

### Outcome

Stable viewer suitable for regular use.

---

## Phase 10 — Advanced Features (Future)

### Possible Additions

* Media/browser integration
* Voice support
* Advanced rendering (PBR, shadows)
* Plugin/modding system

---

## Development Strategy

* Always prioritize working vertical slices
* Avoid large untested subsystems
* Build incrementally
* Validate each phase before proceeding

---

## Milestones Summary

* M0: Project setup
* M1: Rendering + UI shell
* M2: First world rendering
* M3+: Incremental feature expansion

---

## Guiding Principle

> “Make it work, then make it correct, then make it fast.”

---
