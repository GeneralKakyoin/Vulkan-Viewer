# PROJECT_BRIEF.md

## Project Overview

This project is a **from-scratch rewrite of a Second Life / OpenSim viewer** using modern technologies.

The goal is to create a **high-performance, cross-platform, and maintainable viewer** that remains compatible with existing grids and content while improving architecture and long-term extensibility.

This is not a game engine.
This is a **networked virtual world client with strict compatibility requirements**.

---

## Goals

### Primary Goals

* Maintain compatibility with Second Life and OpenSim grids
* Achieve high rendering performance on modern hardware
* Support cross-platform execution (Linux, Windows, macOS)
* Provide a clean, modular, and maintainable architecture
* Enable long-term evolution without legacy constraints

### Secondary Goals

* Improve developer experience compared to legacy viewers
* Allow experimentation with modern rendering techniques
* Enable better debugging and internal tooling

---

## Non-Goals (Initial Phases)

* Full feature parity with Firestorm at launch
* Perfect visual parity with legacy viewers
* Advanced UI/UX polish
* Full media/browser/voice integration
* Mobile or web support

---

## Target Platforms

* Linux (primary development platform)
* Windows (first-class support)
* macOS (planned support)

---

## Supported Grids

Initial focus:

* Second Life (primary compatibility target)

Planned:

* OpenSim (via modular grid adapter system)

---

## Technology Stack

* Language: Rust
* Rendering: wgpu (multi-backend: Vulkan / Metal / D3D12)
* Windowing/Input: winit
* UI: egui
* ECS/Scheduling: bevy_ecs (standalone)
* Async runtime: tokio
* Serialization: serde
* Logging/Tracing: tracing

---

## High-Level Architecture

The system is modular and split into independent domains:

* Core: shared data structures and logic
* Renderer: GPU abstraction and rendering pipeline
* UI: interface and tools
* Networking: grid communication and protocols
* Grid layer: abstraction for SL/OpenSim differences
* Asset system: caching and asset handling
* Platform layer: OS integration

Strict boundaries are enforced between these components.

---

## Design Principles

* Compatibility-first architecture
* Strong separation of concerns
* Explicit data flow
* Minimal hidden coupling
* Incremental development
* Measurable performance improvements

---

## First Milestone (M1)

The first milestone is a minimal working viewer shell:

* Application launches
* Window opens
* wgpu initializes successfully
* Screen clears with a color
* egui debug panel renders
* Clean shutdown

No networking or asset loading is required at this stage.

---

## Second Milestone (M2)

* Connect to a test grid
* Establish session/login
* Receive basic world data
* Render minimal scene (even placeholder geometry)
* Camera movement

---

## Risks

* Protocol complexity (Second Life compatibility)
* Asset pipeline complexity
* Performance bottlenecks in early renderer design
* Scope creep from legacy feature expectations

---

## Success Criteria

The project is successful if:

* A user can log into a grid
* A world scene is rendered correctly
* The viewer is stable and responsive
* The architecture allows continued development without major rewrites

---

## Long-Term Vision

* Full viewer replacement for legacy clients
* Modern rendering pipeline (PBR, better lighting)
* Improved tooling for debugging and development
* Expandable support for multiple grids and features

---
