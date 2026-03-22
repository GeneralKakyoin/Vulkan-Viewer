# Vulkan-Viewer: Rendering Roadmap V2

The goal of the Vulkan-Viewer renderer is to provide a high-performance, modular, and diagnostic-first visualization of the Second Life / OpenSim protocols.

## HOW TO PLAN A MILESTONE

If the user asks you to plan a milestone, these are the steps to take.

1. Read all of `docs/RENDERING_ROADMAP_V2.md` (the current document) to learn about the milestone, in the context of past and future milestones. This document lists only the bare essential deliverables and validation steps for each milestone.
2. Read all prior `PLAN_M{n}.md` milestone documents as well (if any exist in `docs/planning/`).
3. Ask any important initial clarifying questions about the milestone you might have.
   - If you're not asking any questions at all for the entire planning, something's wrong! I know that this file isn't fully specified. There must be important things for you to clarify.
   - It's better to eliminate unknowns in the milestone by discovering facts, not by asking the user. Do not ask questions that can be answered from the repo or system (for example, "where is this struct?" or "which WGPU feature is needed?" when exploration can make it clear). Only ask once you have exhausted reasonable research.
      - **Discoverable facts**: explore first. Before asking, run targeted searches and check likely sources of truth (configs/types/wgpu-caps). Ask only if: multiple plausible candidates; nothing found but you need a missing identifier/context; or ambiguity is actually product intent.
     - **Preferences/tradeoffs**: ask early. These are intent or implementation preferences that cannot be derived from exploration. Provide 2–4 mutually exclusive options + a recommended default.
   - When you ask a question, the user doesn't have your context. You must phrase your questions so as to include FULL context, tradeoffs, background, explanation of terms. Don't use jargon. A good question is typically 2-5 sentences long.
   - Questions should where possible offer multiple choices, and your recommendation.
   - Keep asking until you can clearly state: goal + success criteria, audience, in/out of scope, constraints, current state, and the key preferences/tradeoffs.
4. Research milestone-relevant aspects of how the protocol and renderer work:
   - Read `docs/ARCHITECTURE.md` and `docs/INTERFACES.md`.
   - Consult `docs/RESEARCH/` and the `firestorm_research` skill for protocol behavior.
   - Research `wgpu` best practices for the specific feature (e.g., skinning, draw pools).
5. Flesh out the milestone deliverables and validation steps as needed, if any are missing.
   - You should have a focus on validation in everything you do.
   - The validation steps should be about how someone who implements this milestone can validate that their implementation is good.
6. Develop your plan for the milestone and write it to a new `docs/planning/PLAN_M{n}.md` file.
   - A great plan is very detailed—intent- and implementation-wise—so that it can be handed to another engineer or agent to be implemented right away. It must be **decision complete**, where the implementer does not need to make any decisions. It must be **self-contained**.
7. Are there better-engineering blockers? If so, bail!
   - The user always wants things done the right way, with clean engineering, good architecture. Never any shortcuts.
   - If the research phase reveals durable wisdom or constraints, update `docs/LEARNINGS.md` immediately.
8. Ask the user any further important clarifying questions you have that arose as a result of your research.
9. Present the plan for user review and signoff.

---

## PLAN_M{n}.md Format

Please use the following format for your `docs/planning/PLAN_M{n}.md` files:

```markdown
# M{n} plan: {title}

## Summary
{brief summary of deliverables+validation for this milestone}

## HOW TO EXECUTE A MILESTONE
{please include verbatim the content of "HOW TO EXECUTE A MILESTONE" section of RENDERING_ROADMAP_V2.md}

## Locked user decisions
{all the decisions that the user made}

## PLAN
{your plan: API, algorithms, files changed, testing}

## BETTER ENGINEERING INSIGHTS + BACKLOG ADDITIONS
{architectural insights, deferred work}

## AI VALIDATION PLAN
{definition of done: tests, cargo check, etc.}

## AI VALIDATION RESULTS
{filled during execution}

## USER VALIDATION SUGGESTIONS
{steps for the user to manually verify}
```

---

## HOW TO EXECUTE A MILESTONE

If the user asks you to execute on a plan, these are the steps to take.

1. Implement the plan.
   - You should check your work with AI autonomous validation and testing.
   - The hope is that implementation can be done with a minimum of user interaction.
   - Once it is complete, fill in the "Validation" section.
2. Perform your testing and validation.
   - Update the "AI VALIDATION RESULTS" section of your `PLAN_M{n}.md` file.
3. Review your own code.
   - Evaluate correctness and style.
   - Run static checking (`cargo check`, `cargo fmt`).
4. After implementation, do a "better engineering" phase.
   - Update `docs/LEARNINGS.md` with new findings and clean up `docs/ARCHITECTURE.md`.
   - Update `docs/CURRENT_STATE.md` and `docs/HANDOFF.md`.
5. Upon completion, ask for user review. Tell the user what to test, what commands to use.

---

## Milestone 1: Spatial Foundation & Scene Management

The goal is to move from simple linear iteration to a high-performance spatial index that can handle a dynamic virtual world.

1. Refine the existing `Octree` in `viewer_core` to support efficient dynamic updates. Objects should be able to move or change bounds without rebuilding the entire tree. We'll use a dirty-flagging system at the `Scene` level.
2. Implement a robust `Frustum` vs `Aabb` intersection system that avoids false positives and handles edge cases (large objects spanning nodes).
3. Introduce a basic Scene Graph hierarchy. A `RenderableInstance` should optionally have a `parent_id`, allowing for nested transforms (e.g., an attachment on an avatar).
4. Validation: Render a stress-test scene with at least 1000 independently moving primitives. Verify that the visibility list returned by the octree perfectly matches the set of objects visible to the camera, with 0% overhead from culled nodes.

## Milestone 2: The Geometry Engine (LLVolume & Mesh)

The goal is to establish the primary geometry pipeline for all world objects, replacing diagnostic proxies with a procedural generator and a mesh loading pipeline.

1. **LLVolume Generator**: Implement pure-Rust logic in `viewer_core` that produces vertex/index buffers from SL prim parameters (Box, Cylinder, Sphere, Torus, Tube, Ring, Prism).
2. **Vertex Modifiers**: Support Path Cut, Hollow, Twist, Taper, Shear, and Slice. Correct UV and Normal generation for every face.
3. **Sculpted Prims**: Add a decoder for sculpt maps (RTT-based displacement) to support legacy content.
4. **Modern Mesh**: Implement a glTF loading pipeline for uploaded mesh assets, including Level of Detail (LOD) selection.
5. **Geometry Caching**: Implement a lookup system for procedurally generated geometry to avoid redundant vertex buffer creation for identical prims.
6. **Validation**: Create a "geometry torture test" scene. Verify that world objects (Prims/Meshes) match benchmarks for vertex count and UV alignment.

## Milestone 3: Material System & Asset-Backed World Rendering

The goal is to move beyond simple colors to a rich material system and transition from diagnostic proxies to full asset-backed world rendering.

1. **Texture Matrix Engine**: Support **UV Animation** (scrolling, rotating, and scaling textures per-face) using a 3x3 texture matrix transform in the shader.
2. **Legacy Materials**: Support for "Bump" (Normal) and "Specular" maps on non-PBR objects, including the "Shiny" environment-mapping path.
3. **PBR Materials**: Implement the Metallic-Roughness workflow for glTF materials, supporting Base Color, Emissive, Metal/Rough, and Occlusion maps.
4. **Asset-Backed Ingestion**: Connect the material engine to the live `viewer_net` ingestion seam. Broad simulator world-state (ObjectUpdates) now drives real material/texture application.
5. **Animated Textures**: Support frame-based flipbook animation for textures.
6. **Validation**: Render a "material showcase" with UV-animated water and PBR-textured machinery, sourced from live simulator object updates. Verify that animation remains smooth.

## Milestone 4: Efficient Draw Submission & VRAM Management

The goal is to optimize the renderer for massive scale and manage limited GPU resources.

1. **Draw Pools**: Group instances by `MeshKind`, `Material`, and `Transparent/Opaque` state.
2. **Instanced Drawing**: Utilize `wgpu` instancing for repeated geometry (e.g., identical prims in a forest).
3. **Occlusion Culling**: Implement hardware occlusion queries or a software Hi-Z pass to skip rendering objects hidden behind walls/terrain.
4. **VRAM Management**: Implement an asset prioritization system that discards low-priority textures (small screen area) or low-LOD meshes when GPU memory is constrained.
5. **Validation**: Benchmark a scene with 5000+ objects. Compare performance with and without occlusion culling. Verify that the viewer remains stable even when VRAM is artificially limited.

## Milestone 5: The Particle & Effects Engine

The goal is to support the rich visual effects that define the SL user experience.

1. **Particle Subsystem**: Implement a CPU-side particle simulator (supporting all SL particle parameters: burst, flow, gravity, color-over-life, etc.) and a GPU billboard renderer.
2. **Point & Spot Lights**: Support dynamic local lights with configurable falloff (Linear/Quadratic) and texture-projecting "Cookies" (Projector lights).
3. **Flexiprims**: Add support for specialized secondary-motion deformation using a spring-mass system for "floppy" attachments (hair, skirts, flags).
4. **Special Effects**: Implement glowing/emissive bloom effects and volumetric light beams (God rays).
5. **Validation**: Stress-test with a "Fire & Fireworks" scene featuring thousands of particles and multiple moving projector lights. Verify visual parity with SL particle behavior.

## Milestone 6: Transparency & Order Independent Rendering

The goal is to solve the "alpha glitch" and provide stable, layered transparency.

1. **Alpha Paths**: Distinguish between **Alpha Masked** (0/1 transparency) and **Alpha Blended** paths.
2. **Z-Sorting & OIT**: Implement per-object Z-sorting for alpha-blended objects. Research and implement a basic Order Independent Transparency (OIT) pass (e.g., Weighted Blended) for complex overlapping transparency.
3. **Water Integration**: Correctly handle the transition between above-water and under-water rendering, including refraction and depth-testing for overlapping alpha.
4. **SSR**: Implement Screen Space Reflections for water and reflective surfaces.
5. **Validation**: Create a "transparency stress test" with multiple layers of tinted glass, water, and alpha-masked foliage. Verify that sorting artifacts are minimized.

## Milestone 7: Avatar Appearance & System Baking

The goal is to render fully customizable avatars with high-performance skinning.

1. **Rigging & Skinning**: Implement a standard SL joint hierarchy and GPU-based vertex skinning.
2. **Animesh & Avatars**: Support rigged bone-based animation for both avatars and "animesh" world objects.
3. **System Layer Baker**: Implement the logic to composite multiple texture layers (skin, tattoos, clothing) into a single "baked" texture for the GPU.
4. **Attachment System**: Support attaching rigged or static meshes to specific bones with correct transform propagation.
4. **Validation**: Render a fully dressed avatar with multiple baked layers and attachments. Verify that skinning remains smooth during simple animations.

## Milestone 8: Environmental Effects & EEP

The goal is to provide full world immersion through the Environmental Enhancement Project (EEP).

1. **EEP Atmospheric Scattering**: Implement a procedural skybox that supports all EEP parameters: Rayleigh/Mie scattering, haze, and fog.
2. **PBR Terrain**: Implement a terrain renderer using heightmaps and paintmap-based multi-texture blending for PBR ground textures.
3. **Celestial Bodies**: Add a dynamic sun, moon (with phases), star maps, and planets.
4. **Clouds & Wind**: Implement volumetric or billboard clouds and wind-based vertex deformation for vegetation (grass/trees).
5. **Validation**: Demonstrate a full day-night cycle with transitioning EEP settings, moving celestial bodies, and wind-blown grass.

## Milestone 9: Cinematic Post-Processing & UI Overlay

The goal is to polish the final image and provide a robust HUD system.

1. **Auto-Exposure**: Implement eye-adaptation using luminance/exposure mapping.
2. **Post-FX Stack**: Implement Tonemapping (ACES/Reinhard), Bloom, Depth of Field (DoF), Vignette, and Film Grain.
3. **HUD & Nametags**: Implement a 2D Overlay layer for 3D HUD objects and world-space UI markers (Nametags, beacons, selection outlines).
4. **Validation**: Showcase a "cinematic" scene with DoF focusing, auto-exposure adjusting to a dark room, and a functional HUD overlay.
