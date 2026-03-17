# Project Brief

## Project Name
Second Life Viewer Rewrite

## Purpose
This project aims to build a modern viewer rewrite focused on performance, maintainability, and compatibility with Second Life-style grids and systems.

## Background
The legacy viewer architecture carries technical debt and design decisions that make a full modernization difficult. Since this is a rewrite, the goal is not to preserve old structure unnecessarily, but to rebuild with a cleaner and more future-proof approach.

## Main Goals
- Build a performant viewer with Vulkan-based rendering
- Improve maintainability compared to legacy viewers
- Keep compatibility in mind where it matters
- Support structured, scalable development from the beginning
- Create a codebase that is easier to debug, test, and extend

## Core Priorities
- Performance
- Compatibility
- Modular architecture
- Cross-platform direction
- Clean development workflow

## Non-Goals for Early Development
- Full Firestorm feature parity
- Perfect UI at the start
- Immediate support for every legacy feature
- Overengineering before a working prototype exists

## Proposed Direction
- Modern language and tooling
- ECS-style architecture where useful
- Vulkan renderer
- Strong separation of rendering, networking, assets, UI, and world state

## Success Criteria
A working prototype that can:
- launch reliably
- initialize rendering
- connect core systems together cleanly
- demonstrate the new architecture direction
- serve as the base for future viewer features
