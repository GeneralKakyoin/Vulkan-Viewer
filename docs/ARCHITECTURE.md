# ARCHITECTURE

A Rust rewrite of a Second Life / OpenSim viewer, built in eight bounded crates.
The rendering backend is `wgpu` (not raw Vulkan, despite the repo name). All communication
between subsystems passes through typed domain types owned by `viewer_core`; no crate reaches
into another crate's internals.

For invariants and boundary ownership rules, see also `docs/INTERFACES.md`.
For durable engineering wisdom and historical constraints, see `docs/LEARNINGS.md`.
For daily implementation state, see `docs/CURRENT_STATE.md`.

---

## Layers

The code falls into five conceptual layers:

1. **Orchestration** — `viewer_app`: composition root, startup mode, event loop, seam feed, live worker.
2. **Domain state** — `viewer_core`: Camera, Scene, seam types, social/avatar state, live snapshot bridge.
3. **GPU backend** — `viewer_render`: wgpu device/surface, pipelines, mesh buffers, draw loop.
4. **UI overlay** — `viewer_ui`: egui integration, debug overlay, chat/IM/profile panels.
5. **Transport + Protocol** — `viewer_net` + `viewer_grid`: wire transport, session lifecycle, grid semantics.

The boundary rule: `viewer_net` sends and receives bytes; `viewer_grid` shapes and interprets meaning.
They must never collapse into one another.

---

## Crate Map

### `viewer_app`

The entry point and coordinator only. It wires the other crates together; it does not own any
of their internals. `viewer_app` is the only crate allowed to hold handles to both the live
worker state and the render/UI state simultaneously.

Owns:
- Window and event-loop (winit)
- Input routing and camera update timing
- Startup mode selection (`VIEWER_APP_LIVE_STARTUP`: auto / on / off)
- In-process live-state worker lifecycle and status
- Per-frame adaptation: `LiveVisualSnapshot` → `WorldObjectIngestionSeam` (dirty-only)
- Per-frame seam and snapshot application to `Scene` (dirty-only)
- Wiring of `viewer_render`, `viewer_ui`, and scene/camera data from `viewer_core`

Must not own: rendering internals, protocol logic, capability semantics, asset internals.

---

### `viewer_core`

The shared domain vocabulary. Every other crate imports from `viewer_core` for type definitions;
`viewer_core` imports from nothing in the project.

**Scene and rendering state**
- `Camera`, `Transform`, `RenderableInstance`, `MeshKind`, `InstanceRole`
- `Scene` — the primary world state container consumed by `viewer_render`

**Live state bridge**
- `LiveVisualSnapshot` — sanitized live-derived data emitted by the worker; consumed by `viewer_app`
- `WorldEntryStage`, `FirstRegionPresence` — typed world-entry and region-presence model
- `WorldDiagnosticSlice`, `WorldTrafficSummary` — bounded typed diagnostic slice

**Ingestion seam**
- `WorldObjectIngestionSeam`, `WorldObjectIngestionItem`, `WorldObjectIngestionLane`
- `WorldObjectIngestionAdapter` — maps `LiveVisualSnapshot` → seam lanes
- Active lanes: `FirstRegionPresenceProxy`, `TrafficSignalPayload`, `DecodedSimulatorEndpointPayload`,
  `DecodedCoarseLocationPayload`, `DecodedHealthPayload`, `DecodedViewerTimePayload`,
  `DecodedCoarseNeighborhoodPayload`, `ObjectStateEntitySeedPayload`, `ObjectStateEntityLifecyclePayload`

**Scene roles (seam-owned)**
- World markers: `WorldRegionAnchor`, `WorldEntryBeacon`, `WorldSimTargetMarker`, traffic pillars
- Ingestion proxies: `WorldIngestionProxy`, `WorldIngestionTrafficPayload`, decoded payload roles
- Object/state entities: body / aura / wing / guard / cluster / pulse / stability family
- Avatar proxies: `AvatarProxy`

**Social and avatar state**
- `SocialState`, `DirectImThread`, `NearbyPersonEntry`, `AvatarRenderMode`
- `project_nearby_people(...)` — deterministic nearby list excluding self

---

### `viewer_render`

Owns all GPU rendering internals. Nothing outside this crate touches `wgpu` handles.

Owns:
- `wgpu` device, queue, surface, and swapchain lifecycle
- Render pipelines for each mesh kind (cube, ground plane, axis marker, avatar proxy mesh)
- Depth texture creation and resize handling
- Camera uniform and per-object transform uniforms
- Draw loop: iterates `viewer_core::Scene` instances, dispatches by `MeshKind`
- `MeshKind::AvatarProxy` → shared proxy mesh buffer (single mesh, many instances)

Must not own: protocol/grid logic, session state, UI event state.

---

### `viewer_ui`

Owns the `egui` context and all UI draw logic.

Implemented UI surfaces:
- Debug/runtime overlay: connection status, handshake stage, live state fields, avatar render mode
- Chat + IM workspace: two-pane (thread list + active conversation), per-filter views
- Nearby People / Friends grouped sections with quick-action affordances
- Profile window: grouped header, tab-jump combo, per-tab load status
- Runtime relay: collapsible lifecycle event log

Must not own: protocol, transport, business logic, orchestration, or any network state directly.
UI reads session-derived data only from `viewer_core` types passed in by `viewer_app`.

---

### `viewer_net`

Owns wire transport and session lifecycle mechanics. It knows how to send and receive bytes;
it does not know what those bytes mean for the grid.

Owns:
- `Connection`, `ConnectionConfig`
- Async HTTP login transport (reqwest), redirect handling
- JSON / LLSD / XML-RPC codec paths
- `$1$<md5>` credential normalization (Firestorm-compatible)
- LLSD → XML-RPC login wire fallback (single-step)
- Seed capability fetch transport
- One-shot EventQueueGet with bounded exponential retry
- SimulatorFeatures and MapLayer one-shot HTTP probes
- Typed first-simulator handshake: UseCircuitCode → CompleteAgentMovement → AgentMovementComplete
- Binary LLUDP datagram send (UseCircuitCode=3, CompleteAgentMovement=249)
- Typed inbound LLUDP packet classification (high/medium/low message number decode)
- Bounded post-AMC receive window with policy controls and tail capture
- Bounded coarse location decode (first + second + third sample), health decode, viewer-time decode
- `simulator_payload_decode_summary()` — decode counters propagated into `LiveVisualSnapshot`
- Typed early simulator traffic scaffold and per-kind summary

Must not own: grid-specific semantics, capability meaning, rendering/UI logic.

---

### `viewer_grid`

Owns grid-specific policy: what to send, how to interpret what comes back.

Owns:
- `GridLoginAdapter` trait
- Typed login intent / request / response / bootstrap models
- Typed login result and error classification
- `SecondLifeAdapter` implementation
- `SessionBootstrap`, `FirstSimulator` typed response models

Must not own: HTTP transport mechanics, retry logic, rendering/UI.

---

### `viewer_asset`

Reserved for asset/cache responsibilities (texture fetch, mesh lifecycle, caching).
Not implemented beyond scaffold.

### `viewer_platform`

Reserved for OS/platform-specific integration.
Minimal scaffold only.

---

## Key Abstractions

Three types form the structural language of the app — every subsystem speaks them:

- **`LiveVisualSnapshot`** — The sanitized bridge between the live worker and the render loop.
  It carries only what the app needs to update scene and UI state: no raw session handles,
  no sensitive capability URLs, no protocol bytes. Every field is either a plain scalar,
  an enum, or a small typed struct. `viewer_app` is the only consumer; `viewer_net` is the
  only producer.

- **`WorldObjectIngestionSeam`** — The typed channel between live network state and scene state.
  A seam is a set of named lanes; each lane carries a bounded typed payload. `viewer_app`
  adapts the latest snapshot into a seam each frame (dirty-only). `viewer_core::Scene` applies
  the seam: seam-owned roles are created or removed based on lane presence. Nothing outside
  `Scene::apply_world_object_ingestion_seam(...)` may create or remove seam-owned roles.

- **`MeshKind` / `InstanceRole`** — The rendering vocabulary. `MeshKind` determines which
  GPU draw path is used. `InstanceRole` records the semantic meaning of a scene instance
  (why it exists and what it represents). Together they ensure the renderer can evolve
  draw paths independently of scene composition logic.

---

## Startup Sequence

`viewer_app::main` starts a winit event loop and calls the startup orchestrator on first resume.

1. **Startup mode check** — Read `VIEWER_APP_LIVE_STARTUP` env var (auto / on / off).
2. **Worker spawn** (if live mode) — In-process tokio task runs `viewer_net` login → bootstrap → handshake.
3. **Snapshot channel** — Worker emits `LiveVisualSnapshot` updates over an mpsc channel.
4. **Event loop** — Each frame: receive latest snaps hot (if available), adapt to seam (dirty-only),
   apply seam to scene (dirty-only), apply snapshot to scene (dirty-only), render, draw UI.
5. **Fallback path** — If no live worker, loads `live_visual_snapshot.json` from disk (dev path).

The key design split: the worker runs independently of the render loop. Snapshot updates arrive
asynchronously; the render loop applies only what has changed since last frame.

---

## Login and Bootstrap Flow

This flow runs inside the in-process worker:

1. `viewer_app` calls `viewer_net::Connection::login_to_simulator(...)` via the worker.
2. `viewer_net` selects codec: tries LLSD; falls back to XML-RPC on failure.
3. `viewer_grid::SecondLifeAdapter` shapes the login request (credential normalization, required fields).
4. `viewer_grid::SecondLifeAdapter` interprets the response (session bootstrap, first-simulator info).
5. `viewer_net` fetches the seed capability URL from the bootstrap response.
6. `viewer_net` probes: EventQueueGet (bounded retry), SimulatorFeatures, MapLayer.
7. `viewer_net` sends UseCircuitCode + CompleteAgentMovement via LLUDP on the simulator's UDP port.
8. `viewer_net` listens for AgentMovementComplete + post-AMC traffic in a bounded window.
9. Worker emits a sanitized `LiveVisualSnapshot`; `viewer_app` bridges it to seam and scene.

---

## Data Flow: Live State Bridge

```
viewer_net (decode / probe) → LiveVisualSnapshot (emitted via mpsc)
    → viewer_app (receive latest per frame, dirty-check)
        → WorldObjectIngestionAdapter (maps snapshot fields → seam lanes)
            → WorldObjectIngestionSeam
                → Scene::apply_world_object_ingestion_seam(...)
                    → seam-owned RenderableInstance roles created/removed
        → Scene::apply_live_visual_snapshot(...) (non-seam fields: camera hint, raw presence, etc.)
```

The seam is the only path allowed to create or remove seam-owned scene roles.
`apply_live_visual_snapshot` must not create or remove seam-owned roles.

---

## Data Flow: Social and Avatar State

```
viewer_net (CoarseLocationUpdate decode) → LiveVisualSnapshot::coarse fields
    → viewer_app (merge coarse into SocialState, resolve sim name)
        → SocialState::nearby_people (NearbyPersonEntry list, excluding self)
        → project_nearby_people(...) → 2D label positions per avatar
        → Scene::apply_avatar_proxies(...) → AvatarProxy RenderableInstance roles
viewer_ui reads SocialState for chat/IM/nearby-people panels (read-only from viewer_core types)
```

`viewer_ui` never reads from `viewer_net` state directly.
Avatar identity and render mode are owned by `viewer_core::SocialState`.

---

## Data Flow: Scene to Render

```
viewer_core::Scene (Vec<RenderableInstance>)
    → viewer_app passes camera + scene to viewer_render each frame
        → viewer_render iterates instances, dispatches by MeshKind
            → cube: indexed cube mesh, per-instance transform uniform
            → ground plane: flat quad
            → axis marker: line-like geometry
            → AvatarProxy: shared proxy mesh, per-instance transform uniform
        → depth pass → present
        → viewer_ui egui pass (separate render pass, no depth)
```

---

## Key Invariants

- **Seam ownership**: only `Scene::apply_world_object_ingestion_seam(...)` creates or removes seam-owned roles.
- **Dirty-only apply**: seam and snapshot apply to scene only when content has changed since last frame.
- **Sensitive data boundary**: `LiveVisualSnapshot` must not carry raw session IDs, capability URLs, or agent secrets. Only `viewer_net` holds these.
- **No cross-crate wgpu handles**: GPU resources (`Buffer`, `Texture`, `RenderPipeline`) never leave `viewer_render`.
- **Unknown traffic is signal**: inbound LLUDP packets classified as `Unknown` are intentional diagnostic signal. Do not suppress them.
- **Grid policy stays in `viewer_grid`**: what a capability URL means, what a response field implies — this lives in `viewer_grid`, not in `viewer_net`.
- **Transport mechanics stay in `viewer_net`**: retry policy, codec selection, LLUDP framing — not `viewer_grid`.
- **Self-avatar fallback**: when coarse ID blocks are absent, self placeholder must persist using a deterministic fallback ID.

---

## Architectural Don'ts

- **Don't collapse `viewer_net` and `viewer_grid`**. Capability semantics must not bleed into transport code. Any time a decode decision depends on what a field *means* for the grid, that decision belongs in `viewer_grid`.
- **Don't create or remove seam-owned roles outside the seam apply path**. Not in snapshot apply, not in startup logic, not in UI callbacks.
- **Don't pass `wgpu` handles out of `viewer_render`**. Pipelines, buffers, and textures are renderer internals.
- **Don't create pipelines per-frame**. Pipeline creation is expensive and must happen at startup.
- **Don't let `viewer_ui` reach into `viewer_net` state directly**. Social/avatar data flows through `viewer_core` types set by `viewer_app`.
- **Don't begin broad world/object decoding** before the bounded ingestion seam and spatial partitioning are ready as documented in `TASKS.md`.
- **Don't expand `Unknown` traffic classifications** without a live observation record in `docs/RESEARCH/post_amc_bootstrap_boundary_map.md`.
- **Don't use `unwrap()` in decode or handshake paths**. Use `?` or explicit error classification.

---

## Crate Communication

Crates do not import each other's singletons. `viewer_app` is the only crate that creates and
wires together instances from `viewer_net`, `viewer_grid`, `viewer_render`, `viewer_ui`, and
`viewer_core`. Each integration point is a typed domain value (snapshot, seam, scene, camera)
passed by value or behind a shared reference — never a raw handle to another crate's internals.

- `viewer_app` → `viewer_render`: `Camera` + `&Scene` each frame
- `viewer_app` → `viewer_ui`: `&SocialState`, `&LiveVisualSnapshot`, `&StartupStatus`
- `viewer_net` worker → `viewer_app`: `LiveVisualSnapshot` over mpsc
- `viewer_core` types → all: `WorldObjectIngestionSeam`, `Scene`, `SocialState` (read-only in UI)
