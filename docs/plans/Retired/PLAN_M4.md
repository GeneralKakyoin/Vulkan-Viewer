# Milestone M4: Efficient Draw Submission & VRAM Management

## Summary
Milestone M4 focuses on optimizing the rendering pipeline for scale and resource efficiency. It transitions the viewer from a naive "one draw call per face" model to a batch-oriented architecture using **Draw Pools** and **Instancing**. It also introduces **Occlusion Culling** to avoid drawing obscured geometry and **VRAM Management** to proactively handle GPU memory limits through prioritization and eviction.

**Deliverables:**
1.  **Draw Pools**: A sorting and grouping phase that clusters instances by `MeshBuffers`, `Pipelines`, and `BindGroups`.
2.  **Instanced Drawing**: Refactor `SceneShader` and `RenderBackend` to use `draw_indexed_instanced` for repetitive geometry.
3.  **Hardware Occlusion Queries**: Implement a two-pass system using `wgpu::QuerySet` to cull hidden objects.
4.  **VRAM Budgeting & LRU**: Implement active tracking of GPU-side `TextureView` and `BindGroup` usage with a prioritization-aware eviction policy.
5.  **Targeted Asset Bridge**: Implement a narrow "fetch-per-frame" listener that picks a single random Asset ID from the discovered world state and attempts to resolve its mesh/textures.
6.  **Validation**: `STRESS_TEST=4` with 5000+ objects, comparing FPS with and without optimizations.

## HOW TO EXECUTE A MILESTONE
The agent must:
1.  Read the current milestone documentation.
2.  Identify technical unknowns and eliminate them via research/code-discovery.
3.  Draft a `RENDERING_PLAN_Mx.md` (decision-complete) and get user approval.
4.  Implement in small, verifiable increments.
5.  Perform a final stress test and document throughput/performance gains.

## Technical Unknowns & Fact Discovery
- **Fact**: `viewer_core::TextureAnim` is per-instance. To instance animated objects, the `uv_matrix` must move from `MaterialUniform` to `InstanceData`.
- **Fact**: Opaque and Alpha-Masked objects can be aggressively instanced. Alpha-Blended transparency requires per-instance sorting and will likely bypass instancing or use smaller cohorts to maintain Back-to-Front order.
- **Fact**: `wgpu` occlusion queries have result readback latency. We will use results from Frame N-1 to decide visibility in Frame N to avoid pipeline stalls.

## Proposed Changes

### 1. Draw Pools & Batching (`viewer_render/src/lib.rs`)
- **[ADD] `DrawPoolManager`**:
    - Groups instances into `DrawCallBucket` based on `(MeshBuffers, Pipeline, MaterialSet)`.
    - **Sorting**:
        1.  **Opaque Pass**: Front-to-Back (maximizes early-Z).
        2.  **Alpha Masked Pass**: Front-to-Back.
        3.  **Alpha Blended Pass**: Back-to-Front (required for correctness). Instancing is restricted here to ensure correct painters-algorithm order.

### 2. Instanced Rendering with UV Support (`viewer_render/src/lib.rs` & WGSL)
- **[MODIFY] `standard.wgsl`**:
    - `struct InstanceData { model_matrix: mat4x4<f32>, color: vec4<f32>, uv_matrix: mat4x4<f32> }`
    - UV transformation logic remains in the vertex shader but pulls from `InstanceData`.
- **[MODIFY] `RenderBackend`**:
    - Replace dynamic-offset uniform binding for objects with a `wgpu::Buffer` (Storage or large Uniform).
    - Batch-upload instance data per pool.

### 3. Hardware Occlusion Queries (`viewer_render/src/lib.rs`)
- **[NEW] `OcclusionScanner`**:
    - Manages a `wgpu::QuerySet` and a result buffer.
    - **Pass 1 (Occlusion Probe)**: Render visibility AABBs for all candidates for culling.
    - **Readback**: Use `wgpu::CommandEncoder::copy_query_results_to_buffer`.
    - **Latency**: Results from Frame N-1 are applied to Frame N to avoid stalling the pipeline.
- **Occluder Selection**: Start by marking `MeshKind::GroundPlane` and large static meshes as "Occluders".

### 4. VRAM Management & Prioritization (`viewer_render/src/lib.rs`)
- **[ADD] `VramTracker`**: Tracks estimated VRAM usage for `TextureView` and `BindGroup` objects.
- **Priority Heuristic**: `Score = (AreaOnScreen / Distance^2) * (1.0 + is_last_seen_bonus)`.
- **Eviction**: When over budget (e.g., >80% of reported GPU memory or a fixed 1GB limit), evict lowest-score textures.
- **Integration**: Signal `viewer_asset` to drop CPU-side textures if they've been evicted from VRAM for too long.

### 5. Targeted Asset Bridge (`viewer_app/src/main.rs` & `viewer_net/src/lib.rs`)
- **[ADD] `AssetDiscoveryBuffer`**:
    - A ring-buffer (e.g., `VecDeque<AssetID>`) in `AppState` that remembers the last 100 mesh/texture IDs seen in `ObjectUpdate` packets.
- **[ADD] `AssetClient` (`viewer_net`)**:
    - Implements a minimal `fetch_asset(url, uuid)` using `reqwest`.
- **[ADD] `VIEWER_FETCH_RANDOM_OBJECT=1` Toggle**:
    - If set, once every few seconds, the app picks a random ID from the discovery buffer and attempts to fetch it.
    - Successfully fetched assets are binned into the `AssetStore` and immediately rendered in place of their diagnostic placeholder.

### 6. Validation Plan
- **Performance**: Benchmark ~5000 objects. Target: <5ms frame time for draw submission.
- **Resource Management**: Run with `MAX_VRAM_MB=128` and verify that the scene still renders.
- **Asset Bridge**: Log into a real region, enable `VIEWER_FETCH_RANDOM_OBJECT=1`, and verify that at least one "Random Object" transitions from a diagnostic cube to a real mesh/texture.

## AI Validation Results
*(To be completed during execution)*
