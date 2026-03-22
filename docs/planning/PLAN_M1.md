# M1 plan: Spatial Foundation & Scene Management

## Summary
The current rendering state is functional but limited to a flat, static-first scene where dynamic objects (avatars) are fully recreated every frame. This milestone establishes a robust **hierarchy-aware Scene Graph** and a **truly dynamic Octree**. 

**Deliverables:**
1.  **Quaternion Support**: Add orientation to `Transform`.
2.  **Scene Graph**: Hierarchical parenting in `RenderableInstance`.
3.  **Stable ID System**: Persistent instance mapping for dynamic entities (avatars).
4.  **Optimized Octree**: Partial updates for moving objects, avoiding full per-frame churn.
5.  **Stress Test**: Validation with 1,000+ moving objects.

## HOW TO EXECUTE A MILESTONE

If the user asks you to execute on a plan, these are the steps to take.

1. Implement the plan.
   - You should check your work with AI autonomous validation and testing.
   - The hope is that implementation can be done with a minimum of user interaction.
   - Once it is complete, fill in the "Validation" section.
2. Perform your testing and validation.
   - Update the "AI VALIDATION RESULTS" section of your `PLAN_M1.md` file.
3. Review your own code.
   - Evaluate correctness and style.
   - Run static checking (`cargo check`, `cargo fmt`).
4. After implementation, do a "better engineering" phase.
   - Clean up `LEARNINGS.md` and `ARCHITECTURE.md`.
   - Update `docs/CURRENT_STATE.md` and `docs/HANDOFF.md`.
5. Upon completion, ask for user review. Tell the user what to test, what commands to use.

## Locked user decisions
- [Decision] Use Quaternions `[f32; 4]` for all rotations in `Transform`.
- [Decision] Prefer "Stable ID Upkeep" for avatars over "Clear & Re-insert".
- [Decision] Use a dedicated `Scene::sync_spatial()` phase to update Octree from world-space transforms.

## PLAN

### 1. Vector Math & Transform Refinement (`viewer_core/src/lib.rs`)
- **[MODIFY] `Transform`**: Add `rotation: [f32; 4]`.
- **[ADD] `Quat` identity/multiplication/to_mat4**: Basic quaternion math in the math section of `lib.rs`.
- **[MODIFY] `RenderableInstance`**:
    - Add `parent_id: Option<usize>`.
    - Add `stable_id: Option<String>` (to store `agent_id` or similar).

### 2. Scene Graph & Matrix Calculation (`viewer_core/src/lib.rs`)
- **[ADD] `Scene::get_world_matrix(id: usize) -> [[f32; 4]; 4]`**: Recursively walks up `parent_id` chain to calculate the final world matrix.
- **[ADD] `Scene::update_world_bounds(id: usize)`**: Re-calculates instance AABB from the world matrix and mesh local bounds.

### 3. Stable Avatar IDs & Optimized Upkeep (`viewer_core/src/lib.rs`)
- **[ADD] `Scene::instance_map: BTreeMap<String, usize>`**: Maps stable string IDs (like agent UUIDs) to scene instance IDs.
- **[MODIFY] `apply_world_avatar_placeholders`**:
    - Instead of `instances.retain(...)`, search `instance_map` for each incoming avatar.
    - If found, `upsert` only if position/stale changed.
    - If not found, `insert_instance` and record in `instance_map`.
    - Handle removal of avatars that are no longer in the list.

### 4. Dynamic Octree Optimization (`viewer_core/src/spatial.rs`)
- **[MODIFY] `OctreeNode::remove`**: Optimize to return early if AABB is definitely not inside (frustum-like check).
- **[ADD] `Octree::update(id, old_aabb, new_aabb)`**:
    - Direct update. If it hasn't left its current leaf node, just replace the AABB entry.
    - Otherwise, `remove` and `insert`.

### 5. Stress Test Baseline (`viewer_app/src/main.rs`)
- **[ADD] `StressTest` mode**: If environment variable `STRESS_TEST=1` is set, spawn 1000 cubes moving in a random pattern (simulating a busy region).
- **[VERIFY]**: Correct culling behavior and stable frame submission.

## BETTER ENGINEERING INSIGHTS + BACKLOG ADDITIONS
- **Architectural Insight**: To avoid $O(N)$ parent lookups every frame, we could cache the world matrix in `RenderableInstance`, but for M1, recursion is sufficient given the shallow hierarchies of SL (attachments).
- **Backlog**: Milestone 2 will need this for complex Prims, so getting quaternions right now is critical.

## AI VALIDATION PLAN
- `cargo check`: Pass.
- `cargo test`: New tests for nested transform multiplication.
- `cargo run`: Manual visual check of avatar movement stability.

## AI VALIDATION RESULTS
*(Pending Execution)*

## USER VALIDATION SUGGESTIONS
1. Run with `VIEWER_LOGIN_ENDPOINT` to see yourself and others.
2. Observe that avatars no longer "twitch" or disappear/reappear briefly (due to stable ID upkeep).
3. Toggle a debug camera and check that culling is still aggressive and correct.
