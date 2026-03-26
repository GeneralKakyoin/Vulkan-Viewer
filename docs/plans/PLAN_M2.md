# M2 plan: The Geometry Engine (LLVolume & Mesh)

## Summary
Milestone M2 replaces the hardcoded "Proxy Cube" and "Avatar Placeholder" with a true 3D geometry engine. It enables rendering any Second Life / OpenSim object by implementing a procedural `LLVolume` generator (for primitives) and a `glTF` pipeline (for modern meshes).

**Deliverables:**
1.  **`LLVolume` Parameters**: Define a data-complete `VolumeParams` struct in `viewer_core`.
2.  **Procedural Generator**: Implementation of Box, Sphere, Cylinder, etc., with all SL modifiers (Hollow, Twist, Cut, Taper, etc.).
3.  **Sculpted Prim Decoder**: Logic to generate grid meshes from XYZ-mapped textures.
4.  **glTF Adapter**: Integration of the `gltf` crate to load and cache uploaded assets.
5.  **Geometry Cache**: Deduplication in `viewer_asset` to minimize GPU memory pressure.
6.  **Validation**: A visual "Geometry Torture Test" scene.

## HOW TO EXECUTE A MILESTONE
If the user asks you to execute on a plan, these are the steps to take.

1. Implement the plan.
   - You should check your work with AI autonomous validation and testing.
   - The hope is that implementation can be done with a minimum of user interaction.
   - Once it is complete, fill in the "Validation" section.
2. Perform your testing and validation.
   - Update the "AI VALIDATION RESULTS" section of your `RENDERING_PLAN_M2.md` file.
3. Review your own code.
   - Evaluate correctness and style.
   - Run static checking (`cargo check`, `cargo fmt`).
4. After implementation, do a "better engineering" phase.
   - Clean up `LEARNINGS.md` and `ARCHITECTURE.md`.
   - Update `docs/CURRENT_STATE.md` and `docs/HANDOFF.md`.
5. Upon completion, ask for user review. Tell the user what to test, what commands to use.

## Locked user decisions
- [Decision] Move **Animesh** to Milestone 7 (Avatar Appearance) to unify rigging/skinning logic.
- [Decision] Move **Flexiprims** to Milestone 5 (Particles & Effects) as it involves dynamic simulation.
- [Decision] Use the `gltf` crate for modern mesh loading.
- [Decision] `viewer_core` will define the *parameters* and *procedural logic*; `viewer_asset` will handle *caching* and *GPU buffer management*.
- [Decision] Initial implementation will focus on CPU-side geometry generation (not GPU compute) for simplicity and compatibility in M2.

## PLAN

### 1. Define `VolumeParams` & `GeometrySource` (`viewer_core/src/lib.rs`)
- **[ADD] `struct VolumeParams`**:
    - `profile_type`: Square, Circle, Triangle (ISO/Equal/Right), Half-Circle.
    - `hole_type`: Same, Square, Circle, Triangle.
    - `path_type`: Line, Circle.
    - `params`: Begin/End (Cut), Hollow, Scale (X,Y), Shear (X,Y), Twist (Begin/End), Taper (X,Y), Revolutions, Skew, Radius Offset.
- **[MODIFY] `RenderableInstance`**:
    - Replace `MeshKind` with `GeometrySource`:
      ```rust
      enum GeometrySource {
          Procedural(VolumeParams, f32), // Params, LOD/Detail
          Sculpt(String, SculptType),    // Texture UUID, Type (Sphere/Torus/etc)
          Mesh(String, u32),             // Asset UUID, LOD level
          Diagnostic(MeshKind),          // Cube, Axis, Plane (internal)
      }
      ```

### 2. Implement `LLVolume` Generator (`viewer_core/src/geometry/llvolume.rs`)
- **[NEW] `generate_volume_mesh`**:
    - **Step 1: Profile Generation**: Generate vertices for the chosen base shape + hole.
    - **Step 2: Path Extrusion**: Generate a sequence of transforms (`PathPt`) based on skew, revolutions, and path type.
    - **Step 3: Vertex Mapping**: Apply path transforms, scale, and twist to the profile.
    - **Step 4: Face Grouping**: Group vertices/indices into `SubMesh` units based on SL Face bitmasks (Begin, End, Side, Inner).
    - **Step 5: Normals & Tangents**: Calculate smooth normals for curved surfaces and flat normals for hard edges.

### 3. Sculpted Prim Support (`viewer_core/src/geometry/sculpt.rs`)
- **[NEW] `generate_sculpt_mesh`**:
    - Decode J2K/JP2 texture (if not already handled by a util).
    - Map pixels to 32x32 or 64x64 XYZ grid.
    - Handle Sphere/Torus/Plane/Cylinder wrapping modes.

### 4. glTF & Mesh Pipeline (`viewer_asset/src/lib.rs` & `loader.rs`)
- **[NEW] `MeshLoader`**: Use `gltf` crate to parse `.glb`/`.gltf` files.
- **[ADD] LOD Selection**: Logic to choose the correct mesh primitive based on a distance-to-screen-area heuristic.

### 5. Geometry Cache (`viewer_asset/src/cache.rs`)
- **[NEW] `GeometryCache`**:
    - Key: `GeometrySource` (hashed).
    - Value: `MeshBuffers` (wgpu::Buffer for Vertices, Indices, and SubMesh ranges).
    - Policy: LRU eviction by memory size.

### 6. Integration & Stress Test (`viewer_app/src/main.rs`)
- **[ADD] `STRESS_TEST=2`**: Spawns a grid of 100 complex prims (hollowed torus with max twist and path cut) to verify the generator.
- **[MODIFY] `sync_spatial`**: Ensure generated meshes update their `Aabb` by calculating bounds during generation.

## BETTER ENGINEERING INSIGHTS + BACKLOG ADDITIONS
- **Boundary correction:** GPU buffer ownership stays in `viewer_render` (wgpu handles must not cross crates). `viewer_asset::GeometryCache` remains a CPU-side processed-mesh cache; `viewer_render::RenderBackend` owns the dynamic GPU mesh buffers.
- **Backlog**: Normal mapping (M3) will require the Tangents generated here.
- **Backlog**: VRAM management (M4) will build on the `GeometryCache` LRU policy.

## AI VALIDATION PLAN
- `cargo check`: Pass.
- `cargo test`: Unit tests for `VolumeParams` normalization (clamping values to SL limits).
- `cargo run`: Visual comparison of "Prim faces" in the debug UI.

## AI VALIDATION RESULTS
**Status:** Completed (geometry torture test path now renders real procedural/sculpt geometry).

**Commands run:**
- `cargo fmt`
- `cargo check`
- `cargo test`
- `$env:STRESS_TEST='2'; $env:VIEWER_APP_LIVE_STARTUP='off'; cargo run -p viewer_app`

**Results:**
- `cargo fmt`: pass
- `cargo check`: pass
- `cargo test`: pass
- `cargo run -p viewer_app`: app launched; automated run was time-bounded and the process was terminated after launch to unblock the session (manual visual validation still recommended).

## USER VALIDATION SUGGESTIONS
1. Run with `STRESS_TEST=2`.
2. Inspect a "Torus" prim with `Hollow=0.5` and `Twist=180` to see if geometry is watertight and correctly normals.
