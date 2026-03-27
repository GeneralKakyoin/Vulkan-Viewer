# LEARNINGS.md

A living record of durable engineering wisdom accumulated across agent sessions.
Every entry here represents something that **would have changed a plan if known upfront** —
a failed approach, a non-obvious behavior, a hard-won constraint, or a pattern that took
more than one session to establish correctly.

**Decision tree for where learnings go:**
- **This file** — durable engineering wisdom, non-obvious constraints, repeated failure patterns
- **`ARCHITECTURE.md`** — things about the finished system's contracts and invariants (not just wisdom)
- **Task plan files** — milestone-specific notes that apply only to that task

**When to write here:**
- When a user correction or clarification reveals a gap that future plans should not repeat
- When an approach failed and the reason is non-obvious from the codebase
- When a behavior surprised you during implementation or live testing
- After completing any milestone: run through your assumptions and flag which ones were wrong

**When to read here:**
- Before planning any task — validate that your plan doesn't repeat known failures
- During post-implementation review — check your work against each entry

---

## Learnings Decision Tree (Quick Reference)

| Learning type | Goes in |
|---|---|
| Durable wisdom, repeated failure pattern | `LEARNINGS.md` |
| System contract or runtime invariant | `ARCHITECTURE.md` |
| Task-specific note, one-off tradeoff | Task plan file |
| Protocol behavior observed from Firestorm | `docs/RESEARCH/<topic>.md` |

---

## L01 — LLSD login wire format is not reliably accepted; XML-RPC is the safe fallback

**Category:** Protocol / `viewer_net`

**Learned when:** First live SL login attempts using LLSD codec failed despite correct field shapes.

**What happened:** The Second Life login endpoint claims to accept LLSD but in practice rejects it with no meaningful error under certain conditions. Firestorm sends XML-RPC by default. Switching to XML-RPC with a single-step fallback resolved this.

**Rule for future plans:** Any plan touching the login codec path must account for this. Do not assume LLSD is sufficient just because the endpoint advertises it. The LLSD → XML-RPC fallback in `viewer_net` must remain in place.

**Files affected:** `viewer_net` codec selection path.

---

## L02 — `$1$<md5>` credential normalization is required for SL login compatibility

**Category:** Protocol / `viewer_grid`

**Learned when:** Login requests with raw passwords were rejected even when all other fields were correct.

**What happened:** The SL login endpoint expects the password field in Firestorm's `$1$<md5>` format, not a plaintext password. This is not documented in any public API spec but is explicit in Firestorm's login code.

**Rule for future plans:** Any plan that touches `viewer_grid::SecondLifeAdapter` or the login request shape must preserve this normalization. Do not ever pass a raw password string.

**Files affected:** `viewer_grid` login adapter.

---

## L03 — Seam-owned roles must only be created via `apply_world_object_ingestion_seam`

**Category:** Architecture / `viewer_core`

**Learned when:** Early integration attempts created scene roles in the snapshot apply path. This caused roles to persist after seam state changed, because the seam apply path controls the remove lifecycle.

**What happened:** When roles are created outside the seam apply path, they have no lifecycle owner — nothing removes them when the condition that created them disappears. The seam apply path is the only code that has a complete view of "what should exist and what shouldn't."

**Rule for future plans:** Never create or remove seam-owned roles in `apply_live_visual_snapshot`, in startup code, or in UI callbacks. The only valid creation/removal path is `Scene::apply_world_object_ingestion_seam(...)`.

**Files affected:** `viewer_core::Scene`.

---

## L04 — Dirty-only apply is load-bearing, not just an optimization

**Category:** Architecture / `viewer_app`

**Learned when:** Removing the dirty-check on seam/snapshot apply caused subtle rendering artifacts and extra per-frame CPU cost in the adapter path.

**What happened:** The `WorldObjectIngestionAdapter` allocation runs every frame and is not free. Without the dirty check, scene state is reset and re-derived every frame from scratch, which (a) loses any accumulation that the scene was doing internally and (b) causes render flicker on roles that should be stable.

**Rule for future plans:** Never remove the dirty-only guards on seam apply and snapshot apply in `viewer_app`. Any plan that changes the scene update path must explicitly preserve the dirty semantics or document why they're being changed.

**Files affected:** `viewer_app` frame loop, `viewer_core::Scene`.

---

## L05 — `Unknown` traffic in post-AMC diagnostics is intentional signal, not noise

**Category:** Protocol / `viewer_net`

**Learned when:** An early attempt to "clean up" the post-AMC diagnostics by mapping all observed packet IDs to named kinds removed the Unknown bucket. This broke the diagnostic value of the tool.

**What happened:** The `Unknown` bucket is how we detect new traffic patterns. If it's empty, it means everything is typed, which requires live observation evidence. Silently mapping a packet ID to a kind without evidence just hides potential misclassification.

**Rule for future plans:** Do not expand typed LLUDP classification without a live observation record in `docs/RESEARCH/post_amc_bootstrap_boundary_map.md`. Unknown traffic appearing is a signal to investigate, not to suppress.

**Files affected:** `viewer_net` packet classification.

---

## L06 — `viewer_net` ↔ `viewer_grid` boundary collapses under schedule pressure

**Category:** Architecture

**Learned when:** Multiple sessions have shown that when under pressure to get a feature working quickly, logic that interprets what a capability *means* or what a response *implies* ends up in `viewer_net` rather than `viewer_grid`.

**What happened:** The boundary between "send/receive bytes" and "interpret meaning" is conceptually clear but under implementation pressure the temptation is to handle interpretation where the bytes already are. This is the most critical architectural line in the project.

**Rule for future plans:** Any plan that touches both `viewer_net` and `viewer_grid` must explicitly name which files belong to which side of the boundary. If a plan does not do this, reject it and ask for that clarity before implementing.

**Files affected:** Any plan touching login, capability fetch, or bootstrap decode.

---

## L07 — `LiveVisualSnapshot` must never carry sensitive session data

**Category:** Architecture / Security / `viewer_net`

**Learned when:** Early snapshot designs included session IDs and seed capability URLs for "developer convenience." These would have been serialized to `live_visual_snapshot.json` on disk in the fallback path.

**What happened:** Session IDs and capability URLs are not display fields — they are secrets. The snapshot is a communication channel between the worker and the app, but it also has a file serialization fallback which would expose those values to disk.

**Rule for future plans:** The `LiveVisualSnapshot` type must never gain fields for: session ID, secret, seed capability URL, circuit code, or any credential. Display-relevant derived data (e.g. "connected: yes/no", "region name") is fine. Raw secrets are not.

**Files affected:** `viewer_core::LiveVisualSnapshot`, `viewer_net` worker emit path.

---

## L08 — Depth texture must be recreated on resize, not on every frame

**Category:** Rendering / `viewer_render`

**Learned when:** An early renderer attempt recreated the depth texture each frame "to be safe." This caused a GPU memory leak and significant frame-time overhead.

**What happened:** `wgpu` depth textures are GPU resources. Creating one per frame allocates a new resource without releasing the last one until the GPU finishes using it. The correct behavior is: create once at startup, recreate only when the window surface dimensions change.

**Rule for future plans:** Any rendering change that touches the depth texture must only recreate it in the resize handler, never in the frame render function.

**Files affected:** `viewer_render` renderer, depth texture lifecycle.

---

## L09 — Render pipelines are expensive and must be created at startup

**Category:** Rendering / `viewer_render`

**Learned when:** A convenience approach during early MeshKind expansion created pipelines lazily on first use. This caused frame hitches when new mesh kinds appeared in the scene for the first time.

**What happened:** `wgpu` pipeline creation involves shader compilation and can take 10–100ms. Any plan that creates pipelines after startup will produce visible hitches.

**Rule for future plans:** All render pipelines must be created in the renderer initialization path. When adding a new `MeshKind`, add its pipeline at startup alongside the existing ones — never defer it to first use.

**Files affected:** `viewer_render` initialization.

---

## L10 — LLUDP packet IDs must come from the message template, not from memory

**Category:** Protocol / `viewer_net`

**Learned when:** An incorrect packet ID (guessed from memory rather than the template) silently misclassified traffic for several sessions before being caught.

**What happened:** Two packets had similar names and IDs that were easy to confuse. Using the wrong ID caused an entire traffic category to be misclassified as Unknown, hiding real handshake packets.

**Rule for future plans:** Every LLUDP message number must be sourced from `reference/firestorm/scripts/messages/message_template.msg`. Never hard-code a message ID from memory. When in doubt, use the `firestorm_research` skill to look it up.

**Files affected:** `viewer_net` packet classification, LLUDP decode.

---

## L11 — Coarse location decode must handle missing ID blocks gracefully

**Category:** Protocol / `viewer_net`

**Learned when:** Live coarse location updates from SL sometimes omit the agent ID block when the coarse count is low. Early code assumed the ID block was always present, causing a decode panic.

**What happened:** The Firestorm-extended coarse payload includes agent IDs, but the original/reduced coarse payload may not. The decode path must treat the ID block as optional and fall back to a deterministic placeholder ID when absent.

**Rule for future plans:** Any plan touching coarse location decode must account for both the reduced payload (no IDs) and the extended payload (with IDs). The self-avatar placeholder must persist using a deterministic fallback when the ID block is absent.

**Files affected:** `viewer_net` coarse location decode, `viewer_app` avatar merge.

---


---

## L12 — Octree must be synchronized via `sync_spatial()` before rendering

**Category:** Architecture / Rendering / `viewer_core`

**Learned when:** Implementing the hierarchical scene graph in Milestone M1.

**What happened:** If `Octree::insert` or `Octree::remove` are called in isolation without a structured synchronization pass, the Octree quickly becomes inconsistent with the scene. Specifically, with a scene graph, moving a parent doesn't immediately update the AABBs of children. The `sync_spatial()` method was introduced to propagate world matrices and update Octree entries in a single, stable pass.

**Rule for future plans:** Always call `scene.sync_spatial()` at the start of the frame (in `viewer_app::redraw`) before passing the scene to the renderer. Do not attempt to update the Octree ad-hoc during individual transform changes unless a very specific use case requires it.

**Files affected:** `viewer_core::Scene`, `viewer_app` frame loop.

---

## L13 — Quaternions must be initialized to Identity [0,0,0,1], never Zero [0,0,0,0]

**Category:** Architecture / Math / `viewer_core`

**Learned when:** Adding the `rotation` field to the `Transform` struct.

**What happened:** Many parts of the codebase were using `Transform` initializers without the new `rotation` field. A common mistake is to zero-initialize it. A zero quaternion is mathematically invalid for rotations and causes `quat_to_mat4` to produce a zero matrix, effectively making objects invisible or infinitely small.

**Rule for future plans:** Always initialize `Transform.rotation` to the identity quaternion `[0.0, 0.0, 0.0, 1.0]`. If a helper function is added for `Transform::default()`, ensure it uses the identity.

**Files affected:** `viewer_core::Transform` and its many initializers.

---

## L14 — Explicit type annotations are required for `gltf` reader iteration

**Category:** Rust / `viewer_asset`

**Learned when:** Resolving persistent build errors in `mesh_loader.rs` during Milestone 2.

**What happened:** The `gltf` crate's reader returned types that were difficult for Rust's type inference to resolve automatically, especially when destructuring or mapping. This led to "type must be known at this point" errors that were not resolved by standard trait imports.

**Rule for future plans:** Any plan involving `gltf` or similar deeply-nested iterator readers must include explicit type annotations for variables receiving the output (e.g., `let (document, buffers, images): (gltf::Document, Vec<gltf::buffer::Data>, Vec<gltf::image::Data>) = ...`). This avoids cascading inference failures.

**Files affected:** `viewer_asset::mesh_loader`.

---

## L15 — `LLVolume` generation must preserve SL-compatible face bitmasks

**Category:** Architecture / Rendering / `viewer_core`

**Learned when:** Implementing the procedural volume generator in `llvolume.rs`.

**What happened:** The Second Life protocol identifies faces using specific bitmasks (e.g., `FACE_PATH_BEGIN`, `FACE_INNER_SIDE`). If the generator produces geometry without these IDs attached to submeshes, future material application (M3) and texture-matrix transforms (M3) will be unable to target specifically-textured faces (like the "inner" side of a hollowed tube).

**Rule for future plans:** All procedural geometry generators must emit `SubMesh` units that map to these canonical SL face IDs. Never merge distinct SL faces into a single mesh without preserving the ID-based look-up path.

**Files affected:** `viewer_core::geometry::llvolume`.

---

## L16 — `white_view` fallback for empty texture IDs ensures valid bind groups

**Category:** Rendering / `viewer_render`

**Learned when:** Implementing Milestone A06 (Texture & Material Integration).

**What happened:** When a `MaterialDescriptor` has an empty `AssetID` for a texture slot, the renderer must still provide a valid `wgpu::TextureView` to the bind group. Using the loading (yellow) or missing (magenta) views for "legally empty" slots (like an unassigned normal map) would produce visual noise. An opaque white 1x1 texture is used instead to ensure the shader's multiplication (`base_color * mesh_color`) remains identity when no texture is intended.

**Rule for future plans:** Always use `white_view` as the fallback for empty or unassigned texture slots in `create_material_bind_group`. Reserve `loading_view` and `missing_view` for cases where an ID is present but the asset is not yet available.

**Files affected:** `viewer_render::RenderBackend`.

---

## L17 — Attachments that are derived from app-local avatar samples fit better as seam payloads than snapshot fields

**Category:** Architecture / `viewer_core` / `viewer_app`

**Learned when:** Implementing R08 attachment proxies on top of the existing avatar sample path.

**What happened:** The attachment data was derived from the app's current avatar sample list, not from the live worker snapshot itself. Extending `WorldObjectIngestionItem` would have forced every snapshot lane constructor to grow new fields even though the payload was orthogonal to the existing snapshot decode data.

**Rule for future plans:** When a bounded visual payload is derived from app-side state rather than the live snapshot, prefer a seam-side payload collection on `WorldObjectIngestionSeam` and keep the snapshot item constructors unchanged unless the worker truly owns the new data.

**Files affected:** `viewer_core::WorldObjectIngestionSeam`, `viewer_app` avatar-to-seam mapping.

---

## L18 — Egui window visibility state must be updated outside the window closure

**Category:** Rust / `viewer_ui`

**Learned when:** Implementing U09 visibility toggles for Diagnostics and Social windows.

**What happened:** Attempting to update `self.show_diagnostics = open_state` inside the `egui::Window::show` closure caused a borrow checker error because the closure borrows `self` (or the builder borrows it) and you cannot mutably borrow `self` while the closure is active.

**Rule for future plans:** Use a local mutable boolean for the `open` parameter, and assign its value back to the `self` field *after* the window's `show` call has completed.

**Files affected:** `viewer_ui/src/lib.rs`.

---

## L19 — Priority-based cache eviction requires deterministic tie-breaking for stability

**Category:** Architecture / Asset / `viewer_asset`

**Learned when:** Refactoring `FixtureTextureCache` from LRU to a priority-metadata queue in Milestone A10.

**What happened:** When multiple assets have the same priority (e.g., all `Normal` or all `Active`), a non-deterministic eviction choice can cause "asset thrashing" where the same set of textures are repeatedly loaded and evicted every frame. By strictly ordering by `(priority, last_touched_tick, asset_id)`, we ensure that if someone must be evicted, it's always the same candidate until state changes, providing frame-to-frame stability.

**Rule for future plans:** Any cache eviction policy must include a deterministic fallback (like `AssetID` or a creation-sequencer) to break ties between items of equal priority or age.

**Files affected:** `viewer_asset::texture_fixture`.
---
 
 ---
 
## L21 — Fragment fog should use explicit camera-to-world distance, not `clip_position.w`
 
 **Category:** Rendering / `viewer_render`
 
 **Learned when:** Implementing Milestone R12 (Environment and Atmospheric Baseline).
 
**What happened:** The initial R12 implementation used `clip_position.w` in the fragment path as a fog-depth proxy. In practice, this produced weak/incorrect fog behavior against baseline start/end defaults and made tuning less predictable. Switching to `distance(in.world_pos, camera.camera_position.xyz)` restored deterministic, physically-intuitive fog control while remaining bounded and inexpensive.
 
**Rule for future plans:** For bounded distance fog in this codebase, prefer explicit camera-to-world distance in shader math. Treat `clip_position.w` as an optimization candidate only if validated with render tests and default-environment visual checks.
 
 **Files affected:** `viewer_render` SCENE_SHADER WGSL.
