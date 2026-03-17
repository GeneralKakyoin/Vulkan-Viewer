# AGENTS.md

## Project Overview

This project is a **from-scratch rewrite of a Second Life / OpenSim viewer**.

Primary goals:

* High performance
* Cross-platform compatibility (Linux, Windows, macOS)
* Strong compatibility with existing grids and content
* Clean, maintainable architecture

This is **not a game engine**.
This is a **networked virtual world viewer with strict compatibility requirements**.

---

## Core Principles

1. **Compatibility first, performance second, features third**
2. **Cross-platform is mandatory**
3. **Do not bind the project to a single graphics backend**
4. **Keep architecture modular and decoupled**
5. **Avoid hidden coupling between systems**
6. **Prefer simple, explicit solutions over clever abstractions**
7. **All major decisions must be documented (ADR)**

---

## Technology Stack (Default)

* Language: Rust
* Rendering: `wgpu`
* Windowing/Input: `winit`
* UI: `egui`
* ECS/Scheduling: `bevy_ecs` (standalone only)
* Async runtime: `tokio`
* Serialization: `serde`
* Logging/Tracing: `tracing`

Do NOT introduce:

* Full Bevy engine
* Raw Vulkan-only rendering paths
* Large framework dependencies without explicit approval

---

## Architecture Rules

### Separation of Concerns

The project is organized into independent modules/crates:

* `viewer_app` → application entry point and orchestration
* `viewer_core` → shared data structures and domain logic
* `viewer_render` → rendering (wgpu only)
* `viewer_ui` → UI (egui)
* `viewer_net` → networking and protocol handling
* `viewer_grid` → grid-specific behavior (SL/OpenSim)
* `viewer_asset` → asset pipeline and caching
* `viewer_platform` → OS/platform integration

### Hard Boundaries

* Renderer MUST NOT depend on grid logic
* Grid logic MUST NOT depend on renderer internals
* UI MUST NOT contain business logic
* Networking MUST be isolated and testable
* Asset system MUST be independent of rendering

---

## Grid Compatibility

* Grid-specific logic MUST be isolated behind interfaces
* Implement separate adapters:

  * `grid_secondlife`
  * `grid_opensim`
* Do NOT scatter `if grid == ...` logic across the codebase
* Compatibility behavior must be explicit and documented

---

## Rendering Guidelines

* Use `wgpu` abstraction (no backend-specific code unless necessary)
* Design for:

  * scalability
  * fallback quality levels
  * old content compatibility
* Do NOT optimize prematurely
* Performance work must be measurable

---

## UI Guidelines

* Use `egui` for all UI
* Prefer developer-friendly and debuggable UI over polished visuals initially
* UI should be:

  * modular
  * state-driven
  * easy to extend

---

## Code Style

* Follow idiomatic Rust
* Use clear, descriptive naming
* Avoid unnecessary abstractions
* Avoid deep inheritance-style patterns
* Prefer composition

---

## Testing Requirements

Agents must:

* Add unit tests for:

  * parsing
  * protocol handling
  * asset logic
* Ensure code compiles before completing tasks
* Use `cargo test` as baseline validation

---

## Documentation Rules

### Required Documents

* `docs/PROJECT_BRIEF.md` → source of truth
* `docs/ARCHITECTURE.md` → system design
* `docs/ROADMAP.md` → milestones

### Decisions (ADR)

All significant decisions MUST be recorded in:

docs/DECISIONS/000X-description.md

Each ADR must include:

* context
* decision
* consequences

### Research

Exploration and notes go in:

docs/RESEARCH/

Do NOT mix research with final decisions.

---

## Git and Workflows

* Keep changes small and focused
* Prefer multiple small commits over large ones
* Use worktrees for parallel development
* Do NOT refactor unrelated code in the same change

---

## Subagent Usage

Subagents should be used for **bounded, well-defined tasks only**.

### Allowed Subagent Roles

* Architecture design
* Renderer setup
* Networking/protocol analysis
* Asset pipeline design
* Tooling/CI setup
* Firestorm/legacy code research

### Rules

* Each subagent task must have:

  * clear goal
  * defined scope
  * expected output
* Do NOT assign open-ended tasks
* Main agent is responsible for integration and final decisions

---

## Codex Behavior Rules

When working on this repository, Codex must:

1. Read `AGENTS.md` before making changes
2. Respect architecture boundaries
3. Prefer minimal, incremental changes
4. Ask for clarification if scope is unclear
5. Avoid large speculative implementations
6. Keep code buildable at all times
7. Update documentation when making structural changes

---

## Anti-Patterns (Do NOT Do)

* Do NOT introduce engine-style global state
* Do NOT mix rendering and networking logic
* Do NOT hardcode grid-specific behavior
* Do NOT over-engineer early systems
* Do NOT create large, unreviewed code dumps
* Do NOT bypass module boundaries

---

## First Milestone Definition

A valid first milestone is:

* Application launches
* Window opens
* `wgpu` initializes
* Screen clears successfully
* `egui` overlay renders
* Application exits cleanly

Nothing else is required before this milestone.

---

## Priority Order

1. Correctness
2. Compatibility
3. Stability
4. Maintainability
5. Performance

---

## Final Notes

This project is long-term and complex.

Agents must optimize for:

* clarity over speed
* structure over shortcuts
* incremental progress over large rewrites

When in doubt:
→ simplify
→ isolate
→ document

---
