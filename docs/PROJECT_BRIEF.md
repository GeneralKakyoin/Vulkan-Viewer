# PROJECT_BRIEF.md

## Background and Vision

This repository is a from-scratch Rust rewrite of a Second Life / OpenSim viewer, aimed at replacing legacy architectural constraints with a modern, maintainable, compatibility-first design.

We are not building a game engine; we are building a networked virtual-world viewer. The final goal is long-term feature parity with conventional viewers, delivered in incremental, validated phases.

## Primary Ambitions

- **Modern Architecture**: Strict crate boundaries and clean data flows (see `docs/ARCHITECTURE.md`).
- **Compatibility First**: Proven login and bootstrap paths against live grids (see `docs/CURRENT_STATE.md`).
- **Performance & Stability**: High-performance Rust/wgpu rendering and a robust async runtime.
- **Continuity**: Durable documentation that allows new agents and models to orient instantly (see `docs/TASKS.md` and `docs/LEARNINGS.md`).

## Core Constitution

All work in this repository must align with the **Must-Nots** and **Firestorm Policy** defined in `docs/MASTER_PLAN.md`. We prioritize stability and maintainability over rapid feature breadth.
