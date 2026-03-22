# M3 plan: Material System & Asset-Backed World Rendering (Expanded)

## Summary
Milestone M3 implements the core material engine, moving beyond simple vertex colors to a rich system supporting legacy SL properties, modern PBR workflows, and dynamic UV animation. It ensures that the viewer can render the complex visual layering (Bump, Specular, PBR, Animated) that defines the Second Life / OpenSim experience.

**Deliverables:**
1.  **Material Data Model**: `LegacyMaterial` and `PbrMaterial` definitions in `viewer_core`.
2.  **UV Matrix Engine**: Support for per-face texture transforms (Offset, Repeat, Rotation) and `llSetTextureAnim` (Flipbook, Smooth Scroll, Scale).
3.  **Unified Material Shader**: A robust `wgpu` shader in `viewer_render` that handles both Legacy and PBR paths with Normal mapping.
4.  **Multi-Texture Binding**: Logic to manage and bind albedo, normal, specular, and PBR textures.
11. **Asset Pipeline Integration**: Connecting `viewer_render` to `viewer_asset` for live texture fetching.
12. **Loading/Error States**: Graceful fallback to "Loading" or "Missing" placeholders.
13. **Validation**:
  - Live asset rendering in a real simulator context.
  - Performance profiling with broad world object counts.

## HOW TO EXECUTE A MILESTONE
If the user asks you to execute on a plan, these are the steps to take.

1. Implement the plan.
   - You should check your work with AI autonomous validation and testing.
   - The hope is that implementation can be done with a minimum of user interaction.
   - Once it is complete, fill in the "Validation" section.
2. Perform your testing and validation.
   - Update the "AI VALIDATION RESULTS" section of your PLAN_M3.md file.
3. Review your own code.
   - Evaluate correctness and style.
   - Run static checking (cargo check, cargo fmt).
4. After implementation, do a "better engineering" phase.
   - Clean up LEARNINGS.md and ARCHITECTURE.md.
   - Update docs/CURRENT_STATE.md and docs/HANDOFF.md.
5. Upon completion, ask for user review. Tell the user what to test, what commands to use.

## Locked user decisions
- [Decision] **Per-Face Granularity**: Full support for independent texture matrices per-face, matching SL protocol.
- [Decision] **Tandem Support**: Legacy (Bump/Specular) and PBR (Metallic-Roughness) will be implemented simultaneously in the same pipeline.
- [Decision] **Animation**: Support standard fixed-grid flipbook animations (`llSetTextureAnim`).
- [Decision] **SubMesh Path**: SubMeshes generated in M2 will be used to switch BindGroups for different textures/materials on the same object.

## PLAN

### 1. Data Model Expansion (`viewer_core/src/lib.rs`)
- **[ADD] `struct TextureEntry`**: Represents a single face's material properties (Legacy).
    - `offset`, `repeat`, `rotation`.
    - `fullbright: bool`.
    - `shiny: u8` (None, Low, Medium, High).
    - `bump: u8` (Normal map type).
    - `address_mode: u8` (Repeat, Mirror, Clamp).
    - `shiny_exponent: f32` (Legacy specular highlight size).
    - `double_sided: bool`.
- **[ADD] `struct PbrDescriptor`**: Represents glTF-compatible PBR properties.
    - `base_color_factor`, `metallic_factor`, `roughness_factor`, `emissive_factor`.
    - `alpha_mode` (Opaque, Mask, Blend).
- [x] **[NEW] `struct FaceGeometry`**: (Produced by M2, consumed by M3).
    - `index_range`: Start/length in global index buffer.
    - `face_id`: u16 identifier matching SL bitflags.
- **[NEW] `ObjectMaterialPayload` Seam Lane**:
    - The ingestion seam now carries live material updates from the simulator (`ObjectUpdate` / `LLSD`).
    - Mapping: `BTreeMap<u16, MaterialDescriptor>` (where u16 is `face_id`).

### 2. UV Animation Engine (`viewer_core/src/material/animation.rs`)
- **[ADD] `struct TextureAnim`**: Stores `llSetTextureAnim` parameters.
- **Math**: Implement the Firestorm-equivalent frame calculation:
    - Mode-based logic (Ping-Pong, Reverse, Loop, Smooth).
    - Centering math: `offset = (-0.5 + 0.5 * scale) + step_offset`.
- **Tandem Support**:
    - Matrix concatenation in vertex shader: `Final_UV = Anim_Matrix * Static_Matrix * Vertex_UV`.
    - Apply same rotation to Tangent space: `Final_Tangent = Anim_Matrix_2x2 * Static_Rotate_2x2 * Vertex_Tangent`.
- **Specular Lookup**: Add `lightFunc` 1D/2D texture for Blinn-Phong glossiness response.
- **[NEW] `compute_texture_matrix(params, anim_state, time) -> mat3x3`**:
    - Converts SL offset/repeat/rotate into an affine 3x3 matrix.
    - Applies `llSetTextureAnim` logic:
        - `SMOOTH`: Linear interpolation of offset/scale over time.
        - `CELLS`: Discrete step calculation for grid-based flipbooks.
        - `ROTATE`: Time-based angle rotation.

### 3. Tangent Space Support (`viewer_core` & `viewer_render`)
- **[PREREQUISITE]**: Ensure M2 generates Tangent attributes in `Vertex` buffer.
- **[MODIFY] `Vertex`**: Add `tangent: [f32; 4]` to support normal mapping.

### 4. Advanced Shader Implementation (`viewer_render/src/lib.rs`)
- **[MODIFY] `standard.wgsl`**:
    - **Uniforms**:
        - `MaterialUniform`: `material_type` flag (0=None, 1=Legacy, 2=Pbr), `emissive_color`, `alpha_cutoff`, `fullbright_mode`, `specular_exponent`.
        - `FaceUniform`: Array of `mat3x3<f32>` texture matrices for all faces in the draw call (Roadmap Requirement).
    - **Vertex Shader**:
        - `v_uv = (u_face_data[in_face_id] * vec3(in_uv, 1.0)).xy`.
        - Derive Bitangent: `v_bitangent = cross(in_normal, in_tangent.xyz) * in_tangent.w`.
    - **Fragment Shader**:
        - **Gamma**: Ensure vertex colors are converted to Linear before math.
        - **Legacy Path**: `Diffuse * Light + Specular(N, L, H, shiny_exponent)`.
        - **PBR Path**: Cook-Torrance BRDF. Sample `metallic_roughness` (B=Metal, G=Rough).
        - **Env Mapping**: Sample global reflection cubemap based on `shiny` level for Legacy, or `roughness` for PBR.
        - **Alpha Cutout**: `if (color.a < 0.5) discard;`.

### 5. Render Backend Updates (`viewer_render/src/lib.rs`)
- **[ADD] `BindGroupLayout` for Materials**:
    - Binding 0: BaseColor/Diffuse Texture.
    - Binding 1: Normal Texture.
    - Binding 2: MetallicRoughness (PBR) or Specular/Shiny (Legacy) Texture.
    - Binding 3: Emissive Texture.
- **[NEW] `PbrMaterial.wgsl`**:
    - Supports 4 slots: Base Color, Normal, ORM (Occlusion/Roughness/Metallic), Emissive.
    - Packing: `R=Occlusion`, `G=Roughness`, `B=Metallic`.
    - Tangent-space bitangent derivation: `B = sign * cross(N, T)`.
    - Double-sided flip: `N *= gl_FrontFacing ? 1.0 : -1.0`.
- **[NEW] `LegacyMaterial.wgsl`**:
    - Supports 17 bump types via procedural logic or specialized normals.
    - `lightFunc` sampling for specular response.
    - Diffuse alpha maps to emissive in `ALPHA_MODE_EMISSIVE`.
- **[ADD] `MaterialCache`**:
    - Deduplicate `BindGroups` for identical material parameter sets.
    - **[NEW] `TextureProvider`**: Interface between `viewer_render` and `viewer_asset`.
    - Handles "Placeholder -> Loaded" texture swaps without rebuilding pipelines.
- **[NEW] `AssetBoundMaterial` (Async Streaming Foundation)**:
    - A wrapper that tracks `AssetID` status and binds the correct GPU texture handle (Proxy or Final).
    - Ensures the UI/Render loop never blocks on disk/network I/O.

### 6. Integration & Validation
- **[ADD] `STRESS_TEST=3`**: Material Showcase.
    - 5x5 grid of objects demonstrating:
        - Smooth UV scrolling (Water/Conveyor belt).
        - Flipbook animation (Fire/Smoke).
        - PBR Metal/Rough variations.
        - Legacy "Shiny" reflection.
        - Normal mapped stone/brick.

## BETTER ENGINEERING INSIGHTS + BACKLOG ADDITIONS
- **Insight**: Alpha Blending (originally deferred to M6) is being actively researched for inclusion in M3/M4 now that scope guards are removed.
- **Backlog**: VRAM pressure will increase significantly with normal maps; M4 must handle texture downscaling.

## AI VALIDATION PLAN
- `cargo check`: Pass.
- `cargo test`: Unit tests for `compute_texture_matrix` verifying correct UV wrapping at boundaries.
- `cargo run`: Visual check of UV rotation and flipbook timing.
- **WGPU Debug**: Verify no `BindGroup` leaks or redundant descriptor sets per frame.

## AI VALIDATION RESULTS
*(Pending Execution)*

## USER VALIDATION SUGGESTIONS
1. Run with `STRESS_TEST=3`.
2. Inspect animated water: verify it scrolls smoothly without stuttering.
3. Compare PBR metallic surface vs Legacy shiny surface side-by-side.
