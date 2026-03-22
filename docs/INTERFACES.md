# INTERFACES.md

## Purpose

This document records the important subsystem boundaries, responsibilities, and key interface surfaces in the viewer rewrite.

It exists to prevent:
- architecture drift
- boundary erosion
- transport/render/UI coupling
- loss of continuity across agents

This document should describe ownership and interaction patterns, not every internal implementation detail.

---

## Architectural Rule

Each crate owns a clearly defined responsibility.

If a task pressures a boundary change, that change must be made explicit and documented.

---

## Crate Ownership Summary

### `viewer_app`
Role:
- application orchestration

Owns:
- startup flow
- event loop wiring
- window lifecycle
- input routing
- scene/camera wiring
- high-level coordination between runtime systems

Must not own:
- rendering internals
- grid semantics
- transport details
- asset logic

Depends on:
- `viewer_core`
- `viewer_render`
- `viewer_ui`
- later `viewer_net` integration points as orchestration only

---

### `viewer_core`
Role:
- shared domain state

Owns:
- camera
- transforms
- scene model
- renderable instance descriptions
- shared non-transport data types

Must not own:
- HTTP/transport
- GPU details
- egui/UI state
- grid-specific protocol logic

Depends on:
- minimal shared crates only

Used by:
- `viewer_app`
- `viewer_render`
- `viewer_ui`
- potentially future gameplay/world-state translation layers

---

### `viewer_render`
Role:
- rendering backend and GPU-facing scene draw

Owns:
- GPU initialization
- render pipelines
- mesh/index/vertex buffers
- camera uniform handling
- object draw loop
- depth buffer
- debug/world-axis rendering
- mesh-kind to draw-path mapping

Must not own:
- transport
- login/session state
- grid-specific meaning
- viewer workflow logic

Consumes:
- camera and scene data from `viewer_core`

---

### `viewer_ui`
Role:
- debug and developer-facing UI

Owns:
- egui integration
- debug overlay
- display of runtime diagnostics
- display-only introspection of app/render/camera/login state when wired

Must not own:
- business logic
- protocol logic
- transport behavior
- application orchestration

Consumes:
- data snapshots provided by other systems

---

### `viewer_net`
Role:
- connection/session mechanics and wire transport

Owns:
- `Connection`
- `ConnectionConfig`
- login transport
- HTTP client behavior
- redirect-follow logic
- login trace/diagnostics
- wire-format selection
- transport-side error mapping
- session state transitions

Must not own:
- SL/OpenSim-specific semantics
- login field meaning
- response meaning beyond transport failure/success
- render or UI logic

Depends on:
- `viewer_grid` abstractions where needed for shaping/interpretation
- transport libraries like `reqwest`

---

### `viewer_grid`
Role:
- grid-specific meaning, shaping, and interpretation

Owns:
- `GridLoginAdapter`
- login intent/request shaping
- login response interpretation
- login result classification
- bootstrap/session typing
- grid-specific policy
- `SecondLifeAdapter`
- later OpenSim adapter behavior

Must not own:
- HTTP transport
- retry loops
- redirect transport control
- rendering or UI

Used by:
- `viewer_net`

---

### `viewer_asset`
Role:
- future asset pipeline

Reserved for:
- asset fetch coordination
- caching
- texture/mesh asset lifecycle
- decoding hooks
- future content streaming

Must not own:
- renderer internals
- login transport
- UI

---

### `viewer_platform`
Role:
- future platform integration

Reserved for:
- OS integration concerns
- platform-specific services
- system dialogs / clipboard / file integration later

---

## Current Important Interfaces

### Camera and Scene Interface
Owned by: `viewer_core`

Purpose:
- provide renderer-consumable world/camera state
- remain independent of GPU and UI

Key concepts:
- `Camera`
- `Transform`
- `Scene`
- `RenderableInstance`
- `MeshKind`

Current rule:
`viewer_render` consumes these types but does not define their higher-level meaning.

---

### Renderer Interface
Owned by: `viewer_render`

Purpose:
- hide GPU details from the app layer
- expose a narrow render/update surface

Expected shape:
- initialize renderer
- resize renderer
- render frame from camera + scene
- manage render resources internally

Current rule:
`viewer_app` orchestrates, but does not own render internals.

---

### UI Interface
Owned by: `viewer_ui`

Purpose:
- render debug/developer UI over the world view
- accept read-only state snapshots

Expected shape:
- initialize UI system
- collect/display debug state
- remain optional and non-authoritative

Current rule:
UI may display state, but it must not own the source of truth.

---

### Login/Grid Interface
Boundary between: `viewer_net` and `viewer_grid`

Purpose:
- keep transport and protocol meaning separate

Current interface concepts:
- `LoginIntent`
- `GridLoginRequest`
- `GridLoginResponse`
- `GridLoginResult`
- `GridLoginAdapter`
- `SecondLifeAdapter`
- `SessionBootstrap`
- `FirstSimulator`

Rule:
- `viewer_grid` shapes requests and interprets responses
- `viewer_net` sends/receives and manages session state

This is one of the most important boundaries in the project.

---

### Codec Interface
Owned by: `viewer_net`

Purpose:
- allow transport wire format to change without breaking architecture

Current interface concepts:
- `LoginCodec`
- `JsonLoginCodec`
- `LlsdLoginCodec`
- `LoginWireFormat`

Rule:
- codec handles encoding/decoding
- grid adapter handles meaning
- transport chooses codec based on config

---

### Diagnostic Interface
Owned by: `viewer_net` and consumed later by UI/logging

Purpose:
- make login/debug state observable without coupling logic to presentation

Current concepts:
- `LoginTrace`
- sanitized request/response trace objects
- redirect chain trace
- final interpreted result trace

Rule:
diagnostics must never become the source of truth for login logic.

---

## Current Dependency Intent

Preferred direction:

- `viewer_app` -> `viewer_core`, `viewer_render`, `viewer_ui`
- `viewer_render` -> `viewer_core`
- `viewer_ui` -> read-only data / `viewer_core` types as needed
- `viewer_net` -> `viewer_grid` abstractions
- `viewer_grid` -> shared types only
- `viewer_asset` -> `viewer_core` and later integration boundaries

Avoid:
- `viewer_grid` depending on `viewer_net`
- `viewer_render` depending on `viewer_net`
- `viewer_ui` depending directly on transport behavior
- `viewer_app` absorbing subsystem internals

---

## Current Invariants

These must remain true unless a deliberate architectural decision changes them:

1. `viewer_net` does not own grid meaning
2. `viewer_grid` does not own transport
3. `viewer_render` consumes scene state but does not define world semantics
4. `viewer_ui` displays state but does not own authoritative logic
5. `viewer_app` orchestrates but should stay thin
6. Firestorm-derived behavior must pass through clean Rust interfaces

---

## Interface Change Policy

If a task requires changing an interface:

1. identify the owning crate
2. explain why the current interface is insufficient
3. make the change narrowly
4. update this file if the boundary meaning changed
5. update `CURRENT_STATE.md` and `HANDOFF.md` if the change matters for future work

---

## Current Sensitive Interfaces

These are especially important and should be changed carefully:

- login adapter boundary
- codec boundary
- session/bootstrap typing
- camera/scene to renderer interface
- future capability/bootstrap boundary

---

## What This Document Is Not

This file is not:
- a full API reference
- a dump of every struct
- a substitute for code comments
- a protocol spec

It is a boundary-preservation document.

---

## Near-Term Interface Priorities

The next likely interface pressure points are:

1. **Spatial partitioning** — `viewer_core::Scene` interface will change when flat `Vec<RenderableInstance>`
   is replaced with an Octree; `viewer_render` consumes the visibility result, not the tree itself.
   See `docs/RENDERING_PHASE_1.md` for plan.

2. **RegionHandshake payload decode** — minimal typed block/field extraction will need
   a bounded new lane in the seam without collapsing `viewer_net`/`viewer_grid` boundary.

3. **Asset pipeline boundary** — when `viewer_asset` becomes active, the boundary between
   it and `viewer_core` (texture/mesh lifecycle ownership) must be explicitly defined before
   the first asset request crosses boundaries.

These should be addressed without collapsing `viewer_net` and `viewer_grid` together,
and without mixing asset lifecycle into renderer internals.
