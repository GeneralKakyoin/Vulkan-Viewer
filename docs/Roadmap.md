# ROADMAP

## Status Snapshot
This roadmap tracks what is complete now and what is next, based on the current repository state.

## Phase 0: Project Setup
Status: Completed

Completed:
- Rust workspace and crate layout
- Baseline build/check workflow
- Core repository operating guidance (`AGENTS.md`)

## Phase 1: Rendering Bootstrap (M1)
Status: Completed

Completed:
- Window + event loop
- `wgpu` device/surface init
- Frame rendering and clear
- `egui` overlay
- Clean app lifetime behavior

## Phase 2: Core Runtime Slice
Status: Mostly completed for initial slice

Completed:
- Camera data model in `viewer_core`
- Keyboard movement + mouse-look input wiring in `viewer_app`
- Minimal scene model with typed renderable instances
- Spatial debug scene (ground plane, cube, axis marker)
- Depth buffering in renderer

Remaining in this phase:
- Keep model and transform layer simple while preparing for network-fed scene updates

## Phase 3: Networking and Grid Login Skeleton
Status: In progress (scaffold level)

Completed:
- `viewer_net` async connection/session skeleton
- `viewer_grid` login adapter boundary and typed models
- Mock login handshake between `viewer_net` and `viewer_grid`
- Unit tests for mock success and non-success flow
- Firestorm login behavior research note

Next:
- Replace mock transport with real transport primitives in `viewer_net`
- Keep grid-specific request shaping and response interpretation in `viewer_grid`

## Phase 4: First Real Login To World (M2)
Status: Planned next major milestone

Target outcome:
- Real login/session establishment
- First simulator/bootstrap data applied
- Existing scene/camera loop remains stable while world data starts flowing

## Later Phases
- Asset/cache implementation (`viewer_asset`)
- Expanded grid compatibility policy and adapters
- UI expansion beyond debug overlays
- Rendering quality/performance improvements
- Stability and compatibility hardening

## Execution Rules
- Preserve strict crate boundaries
- Prefer small reversible diffs
- Validate with narrow checks first (`cargo check`, targeted tests)
