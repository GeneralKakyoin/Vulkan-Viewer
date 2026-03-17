# PROJECT_BRIEF

## Project Overview
This project is a from-scratch Rust rewrite of a Second Life / OpenSim viewer.

It is a networked virtual-world client, not a game engine. The priorities remain:
1. Compatibility
2. Stability
3. Maintainability
4. Performance
5. Feature breadth

## Current Implemented State
- Workspace crate boundaries are in place:
  - `viewer_app`, `viewer_core`, `viewer_render`, `viewer_ui`, `viewer_net`, `viewer_grid`, `viewer_asset`, `viewer_platform`
- First vertical slice is working:
  - Window creation and event loop
  - `wgpu` initialization and rendering
  - `egui` debug overlay
  - Keyboard + mouse-look camera controls
- Minimal 3D spatial scene is rendered:
  - Ground plane
  - Cube
  - World-axis marker
  - Depth buffering enabled
- Networking and grid boundaries are scaffolded:
  - `viewer_net`: async connection/session skeleton
  - `viewer_grid`: grid login adapter trait and typed login models
  - Mock login handshake between `viewer_net` and `viewer_grid` with passing tests
- Research baseline exists:
  - `docs/RESEARCH/firestorm_login_flow.md`

## Near-Term Goals
- Keep improving the thin vertical slice toward real login:
  - Replace mock login transport with real transport in `viewer_net`
  - Keep request shaping and response interpretation in `viewer_grid`
  - Preserve startup and rendering isolation in `viewer_app` / `viewer_render`
- Expand minimal world rendering carefully:
  - Continue with simple explicit scene data
  - Avoid broad scene-system refactors before login bootstrap is stable

## Non-Goals (Current Phase)
- No full protocol implementation yet (HTTP/CAPS/UDP incomplete)
- No asset pipeline implementation yet
- No ECS-heavy gameplay-style architecture
- No Firestorm architecture reuse

## Design Direction
- Keep crate boundaries explicit and strict.
- Prefer small reversible steps with compile-safe checkpoints.
- Treat Firestorm as behavior reference only, not implementation source.
