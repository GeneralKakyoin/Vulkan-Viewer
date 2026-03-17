# ARCHITECTURE

## Purpose
This document describes the current crate boundaries and runtime responsibilities for the implemented vertical slice.

## Crate Boundaries

### `viewer_app`
- Owns application orchestration.
- Owns window/event-loop wiring.
- Owns input routing and camera update timing.
- Wires `viewer_render`, `viewer_ui`, and scene/camera data from `viewer_core`.
- Does not own rendering internals, protocol logic, or asset internals.

### `viewer_core`
- Owns shared domain models used across subsystems.
- Current implemented models include:
  - Camera state
  - Transform and renderable scene instance models
  - Minimal scene container used by the prototype renderer

### `viewer_render`
- Owns all GPU rendering internals.
- Current implemented scope includes:
  - `wgpu` setup and surface lifecycle
  - Render pipelines for scene meshes and axis marker
  - Depth texture creation/recreation on resize
  - Camera and per-object uniforms
  - Drawing scene instances from `viewer_core::Scene`
- Must not own grid/protocol logic.

### `viewer_ui`
- Owns `egui` state and draw path.
- Handles UI event consumption and debug overlay rendering.
- Displays debug information (including camera values).
- Must not own protocol or transport behavior.

### `viewer_net`
- Owns transport/session lifecycle mechanics.
- Current implemented scope is a skeleton:
  - async connection state transitions
  - placeholder connect/login/disconnect lifecycle
  - mock adapter-driven login handshake path for tests
- Does not own grid-specific request/response policy.

### `viewer_grid`
- Owns grid-specific login shaping and response interpretation policy.
- Current implemented scope includes:
  - `GridLoginAdapter` trait
  - typed login intent/request/response/bootstrap models
  - typed grid login result/error classification
  - `SecondLifeAdapter` stub implementation
- Does not own transport mechanics.

### `viewer_asset`
- Reserved for asset/cache responsibilities.
- Not implemented beyond scaffold.

### `viewer_platform`
- Reserved for platform/OS-specific integration.
- Minimal scaffold only.

## Runtime Flow (Current Slice)
1. `viewer_app` receives window/input events and updates camera state.
2. `viewer_app` passes camera + scene data to `viewer_render`.
3. `viewer_render` renders world geometry with depth testing.
4. `viewer_ui` overlays debug UI in a separate pass.

## Login Boundary Flow (Current Mock)
1. Call site provides `LoginIntent`.
2. `viewer_grid` adapter shapes grid-specific request payload.
3. `viewer_net` performs mock transport exchange.
4. `viewer_grid` adapter interprets response into typed result.
5. `viewer_net` updates session state only on success.

## Invariants
- Renderer code stays in `viewer_render`.
- Grid policy stays in `viewer_grid`.
- Transport/session mechanics stay in `viewer_net`.
- Startup orchestration stays in `viewer_app`.
- UI draws/debug only in `viewer_ui`.
