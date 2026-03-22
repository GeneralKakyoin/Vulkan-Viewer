# RENDERING_PHASE_CURRENT.md (Phase E Completion Plan)

## Baseline: Prototype Renderer & Scene Mockup (Phase 0 / Phase E)

### Purpose
This document details the exact state of the rendering pipeline as it exists *right now*, before any major architectural transitions toward standard Firestorm-parity occur. It also defines the explicit steps required to close out the current project phase (Phase E: App-owned live startup orchestration) so that we can safely transition to Rendering Phase 1 (Spatial Partitioning).

---

### Current Capabilities & Proven Elements

1.  **Orchestration Boundary (`viewer_app`)**
    *   Window creation and event loop management (via `winit`).
    *   Input routing (keyboard, mouse tracking for camera controls).
    *   *Proven:* The separation of concerns holds. `viewer_app` manages timing and passes data forward but does no direct rendering itself.

2.  **Shared Domain State (`viewer_core`)**
    *   **Scene Definition:** The world is represented by a simple `Scene` struct containing a flat `Vec<RenderableInstance>`.
    *   **Renderable Instances:** Objects are placed via a struct containing a Transform, a Role (`InstanceRole`), and a Fallback Color.
    *   **Camera System:** A functional perspective camera managing projection/view matrices.
    *   *Proven:* We can successfully pass structured, typed data from the core logic layer to the renderer layer frame-by-frame.

3.  **GPU Backend (`viewer_render`)**
    *   **WGPU Initialization:** Successful creation of `Instance`, `Adapter`, `Device`, `Queue`, and `Surface`.
    *   **Depth Testing:** Z-buffer implementation allows for correct 3D occlusion of overlapping triangles.
    *   **Basic Pipelines:** Hardcoded pipelines to draw a colored cube, a flat ground plane, and axis lines.
    *   *Proven:* We have a stable, cross-platform graphics context that correctly draws depth-tested triangles based on structured input.

4.  **Live State Ingestion Seam**
    *   The application currently renders *diagnostic markers* representing actual network layout (e.g., bounding boxes for parsed avatars or traffic endpoints) through a controlled ingestion seam.
    *   *Constraint:* These are hardcoded proxies; there is *no* mesh decoding or dynamic mesh generation from the network yet.

---

## Action Plan: Completing the Current Phase

To declare this baseline phase 100% complete and clear the way for Rendering Phase 1, the following detailed steps must be executed and verified. The goal here is *not* to add new rendering features, but to ensure the existing "pre-render" ingestion seam is hyper-stable, testable, and strictly bounded.

### Step 1: Finalize the Bounded Object/State Slice
We must complete the multi-entity proxy representation without crossing into broad simulator decoding.
*   [x] **Action:** Implement one additional "lane-local" behavior refinement on the existing world placeholder entities. (For example, introducing a temporal pulse or slight scaling variation driven by a known metric like `viewer_time_updates`).
*   [x] **Validation:** Verify the visual proxy changes correctly without needing new raw network data streams.

### Step 2: Ensure Visual Hierarchy and Readability
The diagnostic markers must be distinctly readable before we start filling the screen with thousands of objects.
*   [x] **Action:** Review the current scale and color-coding of `WorldRegionAnchor`, `WorldEntryBeacon`, and `WorldIngestionProxy` markers.
*   [x] **Action:** If markers overlap heavily or lack clear z-fighting protection, adjust their default transforms slightly so the central entry cluster reads cleanly from a standard camera offset.
*   [x] **Validation:** Launch into the live simulation. Ensure you can visually distinguish the region anchor from the avatar proxy cluster at a glance.

### Step 3: Enforce Seam Reversibility and Hard Bounds
Ensure the current logic parsing the simulator payload shuts down safely if it encounters unexpected data.
*   [x] **Action:** Review `viewer_net`'s early simulator traffic routing. Ensure any unmapped low/medium frequency packets explicitly hit an `Unknown` classification and do *not* crash the feed.
*   [x] **Action:** Implement a hard cutoff check preventing `viewer_grid` or `viewer_net` from attempting to spawn an actual physical `LLVolume` or fetch a mesh yet.
*   [x] **Validation:** Verify the logs gracefully acknowledge `Unknown` or irrelevant packets without attempting to pass garbage geometry sizes to the renderer.

### Step 4: Strengthen Scene Mapping Tests
We must prove that changing network parameters results in the correct scene proxies.
*   [x] **Action:** Write a unit test in `viewer_core` that feeds a mock `WorldDiagnosticSlice` with 3 "known avatars" and 5 "traffic signals".
*   [x] **Action:** Verify the test assertions explicitly count exactly 3 `WorldIngestionProxy` roles and the corresponding traffic markers in the resulting `Scene`.
*   [x] **Validation:** Test passes locally and in CI.

### Step 5: Final UI Debug Exposure
Surface the seam's health explicitly so developers in Phase 1 know if issues are rendering bugs or networking bugs.
*   [x] **Action:** Update the `viewer_ui` debug panel.
*   [x] **Action:** Add explicit readouts for: "Avg Scene Update Time", "Total Scene Instances", and "Visible Proxies".
*   [x] **Validation:** The metrics update reliably in the UI during a live session.

### Conclusion Criteria
Once Steps 1-5 are checked off, **Phase E is closed**. The ingestion seam will be considered a stable, bounded diagnostic bed, providing 100% of the preconditions necessary to begin replacing the flat scene loop with the spatial partitioning required in **Phase 1**.

---
**STATUS: COMPLETED (2026-03-21)**
All steps verified. Diagnostic ingestion seam is stable. Ready for Phase 1.
