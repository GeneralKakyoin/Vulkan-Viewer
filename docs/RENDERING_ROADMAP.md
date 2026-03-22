# RENDERING_ROADMAP.md

## Purpose
This document outlines the phased roadmap for elevating the `Vulkan-Viewer` (`viewer_render`) from its current prototype state (Phase E, basic diagnostic cubes and markers) to full 3D rendering feature parity with standard Second Life/OpenSim viewers (e.g., Firestorm).

The plan is designed to respect the project's strict crate boundaries: `viewer_net` handles transport, `viewer_core` owns the scene and models, and `viewer_render` executes GPU operations.

---

## Phase 1: Spatial Partitioning & Scene Graph
**Goal:** Implement a scalable structure to manage world objects before we attempt to render millions of polygons.
**Behavioral Reference (Firestorm):** `indra/newview/llspatialpartition.h`, `indra/newview/lLoctree.h` (to understand spatial grouping expectations)

1.  **Core Scene Graph:** Replace the simple `Vec<RenderableInstance>` with a spatial hierarchy (Octree or BVH) in `viewer_core`.
2.  **Frustum Culling:** Implement camera frustum culling to only submit visible nodes to the GPU.
3.  **Tests:**
    *   *Unit Test:* Spawn 100,000 bounds in `viewer_core`, verify culling accurately returns only those intersecting a test frustum.
    *   *Visual Test:* Move camera in sandbox scene, verify objects disappear from `egui` debug count when outside frustum.

## Phase 2: Core Geometry & Primitives (LLVolume)
**Goal:** Translate network payload `ObjectUpdate` (Path, Profile, Textures) into actual 3D meshes.
**Behavioral Reference:** `indra/llmath/llvolume.cpp`, `indra/llprimitive/llprimitive.cpp` (to extract the math and parameter mapping for shape generation)

1.  **Volume Generation:** Implement a purely Rust-idiomatic geometry generator that produces vertices/indices identical to the visual output expected by protocol primitive params (Box, Cylinder, Sphere, Torus, Sculpties). Firestorm is referenced *only* to understand the formulas.
2.  **Scene Integration:** Map incoming `viewer_grid` object data into `viewer_core::RenderableInstance` holding `wgpu` buffers.
3.  **Tests:**
    *   *Unit Test:* Feed known primitive parameters (e.g., standard cube, twisted cylinder) into the generator, assert vertex counts and bounding box dimensions match Firestorm reference output.
    *   *Visual Test:* Hardcode primitive payloads into the sandbox seam, verify they render correctly as solid colored meshes.

## Phase 3: Textures & Asynchronous Assets
**Goal:** Fetch, decode, and apply textures to rendered geometry.
**Behavioral Reference:** `indra/newview/lltexturefetch.cpp`, `indra/llimage/` (to understand decode pipeline expectations)

1.  **Asset Worker:** Implement a background asset fetcher in `viewer_asset` (HTTP/Caps or UDP fallback).
2.  **J2K Decoding:** Integrate a JPEG2000 decoder (e.g., `openjpeg-sys` or similar) to convert SL texture payloads to raw RGBA.
3.  **GPU Upload:** Implement a texture manager in `viewer_render` to asynchronously upload decoded images to `wgpu` textures.
4.  **Tests:**
    *   *Unit Test:* `viewer_asset` requests a known UUID from a public grid, decodes it, and verifies RGBA output size matches expected resolution.
    *   *Visual Test:* Apply a fetched texture to the sandbox plane.

## Phase 4: Draw Pools & Batching
**Goal:** Optimize GPU submissions by grouping objects by material/shader type, replacing the naïve loop.
**Behavioral Reference:** `indra/newview/lldrawpool*.cpp`, `indra/newview/pipeline.cpp` (to examine how conventional viewers categorize render passes)

1.  **Draw Pools:** Categorize `RenderableInstance` into bins logic: Opaque, Alpha, Bump (Normal mapped), Fullbright, Materials (PBR).
2.  **Instancing/Batching:** Group identical meshes/materials to reduce API draw calls.
3.  **Tests:**
    *   *Performance Test:* Render 10,000 unique prims. Verify `viewer_render` consolidates them into <100 draw calls via instancing where applicable.

## Phase 5: Materials & Advanced Shaders (ALM)
**Goal:** Support modern SL graphics (Advanced Lighting Model, Specular/Normal maps, GLTF PBR).
**Behavioral Reference:** `indra/newview/lldrawpoolmaterials.cpp`, `indra/newview/llgltf*` (to understand ALM rendering expectations)

1.  **Deferred Rendering (Optional/Later):** Move from Forward rendering to a Deferred/Forward+ hybrid if matching SL's ALM exactly. Alternatively, implement modern Forward+ clustered shading.
2.  **PBR Support:** Map `viewer_core` material data (GLTF overrides, legacy specular/normal parameters) to `wgpu` shader uniforms.
3.  **Shadow Mapping:** Implement directional sun/moon shadow maps.
4.  **Tests:**
    *   *Visual Test:* Render a scene with a normal map, specular map, and a GLTF metallic-roughness material side-by-side with varying light angles.

## Phase 6: Avatar Appearance & Rigging
**Goal:** Render human avatars with skeletons, rigged meshes, and baked system layers.
**Behavioral Reference:** `indra/newview/llvoavatar.cpp`, `indra/character/` (to extract skeleton hierarchy rules and bake protocols)

1.  **Skeleton:** Extract the SL avatar skeleton hierarchy rules and implement a clean, idiomatic Rust bone system.
2.  **Mesh Skinning:** Implement compute-shader or vertex-shader skinning in `wgpu` for rigged mesh bodies and clothing.
3.  **Bake Service/Texture compositing:** Support fetching pre-baked textures from the grid for system layers (skin, eyes, system clothes).
4.  **Tests:**
    *   *Unit Test:* Apply a known animation (e.g., standard stand) to the skeleton, verify bone transforms.
    *   *Visual Test:* Render the local avatar in the sandbox, verifying mesh deforms correctly during an animation tick.

## Phase 7: Environment (EEP) & Post-Processing
**Goal:** Sky, water, atmospheric scattering, and screen-space effects.
**Behavioral Reference:** `indra/newview/llenvironment.cpp`, `indra/newview/llvowater.cpp` (to extract EEP math and parameter mapping)

1.  **EEP Parsing:** Ingest Environmental Enhancement Project (EEP) settings.
2.  **Sky & Atmosphere:** Implement atmospheric scattering shaders (Rayleigh/Mie) based on EEP sun/moon positions.
3.  **Water:** Implement water reflection/refraction planes.
4.  **Tests:**
    *   *Visual Test:* Time-of-day progression in the sandbox, ensuring sky colors transition smoothly from noon to sunset.

## Implementation Sequencing (Immediate Next Steps)
To avoid architectural collapse, we must not jump straight to Avatars or Shaders. The sequence must be:

1.  **Completed:** Phase E (`viewer_app` orchestration, basic entry proxy, diagnostic stability).
2.  **Next:** **Phase 1 (Spatial Partitioning)** -> Allows the scene to scale beyond a few diagnostic cubes without choking the frame rate.
3.  **Then:** **Phase 2 (LLVolume / Primitives)** -> Allows the engine to generate physical grid shapes rather than debug cubes.
