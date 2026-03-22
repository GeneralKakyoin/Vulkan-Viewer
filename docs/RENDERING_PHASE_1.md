# RENDERING_PHASE_1.md — Spatial Partitioning

## Summary

Replace `viewer_core::Scene`'s flat `Vec<RenderableInstance>` with an Octree + frustum culling
system so the scene can scale beyond diagnostic markers without GPU performance collapse.
All existing Phase E seam-owned diagnostic roles must continue to function exactly as before.

## Owning Crate

- **Changes**: `viewer_core` (Octree, Aabb, Frustum, Scene refactor), `viewer_render` (VisibilityList handoff)
- **Must not change**: `viewer_net`, `viewer_grid`, `viewer_ui`, `viewer_app` (except the Scene frustum query call)
- **Crate rule**: `viewer_render` receives a flat `VisibilityList`, not a reference to the Octree itself.

---

## Phase 1: Spatial Partitioning &amp; Scene Graph

### Goal Definition
By the end of Phase 1, the GPU must not receive draw calls for objects behind the camera or outside the viewing frustum. The spatial data must remain strictly in `viewer_core`; `viewer_render` only consumes the visible result list. The diagnostic seam's `Scene::apply_world_object_ingestion_seam` must continue to function exactly as before — it merely inserts into a tree instead of a flat `Vec`.

---

## Action Plan: Completing Phase 1


### Pre-Phase 1: Diagnostic Preservation & Behavioral Reference
Before we build tree structures, we must establish spatial boundaries for the existing Phase E diagnostic markers and ensure we understand how Firestorm structures its spatial data.
*   [ ] **Step 0.0 (Firestorm Spatial Layout Audit):** Inspect `reference/firestorm/indra/newview/llspatialpartition.h` and `lLoctree.h` to identify Firestorm's behavioral layout (e.g., how it structures octree nodes, bounds storage, and culling state). Use this to inform the Rust structures.
*   [ ] **Step 0.1 (Bounds Math):** Implement a foundational `Aabb` (Axis-Aligned Bounding Box) struct in `viewer_core`, mirroring Firestorm's center/extents math behavior where relevant.
*   [ ] **Step 0.2 (Instance Adaptation):** Update `RenderableInstance` (or create a wrapper) so every instance can report its `Aabb` in world space based on its transform scale and mesh kind. Ensure this maps seamlessly to Phase E's diagnostic proxies.
*   [ ] **Step 0.3 (Baseline Verification):** Integrate these boundaries inertly. Ensure the code compiles and Phase E diagnostic markers still render correctly before migrating to trees.

### Phase 1 Execution: The Spatial Tree
This represents the core engine migration.
*   [ ] **Step 1.1 (Frustum Math):** Implement a `Frustum` struct (6 planes) in `viewer_core`. Implement `Frustum::contains_aabb()` intersection logic (returning completely Inside, Completely Outside, or Intersecting).
*   [ ] **Step 1.2 (Octree Implementation):** Implement a dynamic `viewer_core::spatial::Octree` data structure. It must support insertion, removal, and automatic node splitting/merging based on capacity.
*   [ ] **Step 1.3 (Octree Querying):** Implement `Octree::query_frustum(&Frustum)` which walks the tree and returns a flat list (e.g., `Vec<InstanceId>`) of visible objects.
*   [ ] **Step 1.4 (Scene Migration):** Refactor `Scene` to use the `Octree` as its primary storage instead of a flat `Vec<RenderableInstance>`. Update all diagnostic marker insertion/removal logic (from Phase E) to sync with the Octree.
*   [ ] **Step 1.5 (Camera Integration):** Update the `Camera` system to calculate and export a `Frustum` based on its view-projection matrix every frame.
*   [ ] **Step 1.6 (Renderer Handoff):** Update `viewer_app` to query the `Scene` with the `Camera` frustum, producing a `VisibilityList`. Modify the handoff to `viewer_render` so it only iterates and draws instances present in the `VisibilityList`.

### Validation & Metrics
Prove the system works mathematically and visually.
*   [ ] **Step 1.7 (Mathematical Unit Tests):** 
    * Write tests proving `Octree` insertion splits nodes correctly.
    * Write tests proving `query_frustum` explicitly rejects `Aabb`s outside the view planes.
*   [ ] **Step 1.8 (UI Debug Telemetry):**
    * Update the `viewer_ui` "Performance" panel. 
    * Add a new readout separating the existing "Total Scene Instances" into: "Total Instances (Octree)" and "Visible Instances (Culled)".

### Conclusion Criteria
Once Steps 0.1 through 1.8 are checked off, **Phase 1 is closed**. We will know it works because spinning the camera 180 degrees away from the diagnostic proxy cluster will drop the "Visible Instances" count in the UI to near-zero, proving the GPU is saved from rendering off-screen geometry. This clears the path for **Phase 2 (LLVolume Core Geometry Generation)**.
