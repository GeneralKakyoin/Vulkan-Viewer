# FIELD_GUIDE.md

A map of non-obvious code locations, data structures, and logic in the Vulkan-Viewer codebase.
Every entry here represents a "where is that one thing" answer for parts of the system that are
hard to discover by standard grep or that have non-obvious significance.

**Decision tree for where information goes:**
- **This file** — non-obvious locations of functions, data, or logic within *this* codebase.
- **`LEARNINGS.md`** — durable engineering wisdom and durable lessons, dead ends, recurring traps
- **`ARCHITECTURE.md`** **`INTERFACES.md`** — high-level crate boundaries and system invariants.
- **`docs/RESEARCH/*.md`** — protocol behavior observed from Firestorm.

---

## G01 — LLVolume Face Bitmasks (Procedural Geometry)

**Location:** `crates/viewer_core/src/geometry/llvolume.rs`

**What it is:** The bitmasks used to identify specific faces of a SL primitive (e.g., the "inner" side of a hollowed tube).

**Why it's non-obvious:** These are not named in the same way as the protocol fields. They are used during vertex generation to tag `SubMesh` units. If you are adding a new primitive type or changing how faces are generated, you must use these specific masks to ensure M3+ material targeting works.

---

## G02 — `$1$<md5>` Password Normalization logic

**Location:** `crates/viewer_grid/src/adapter/second_life.rs` (or wherever the `GridLoginAdapter` implementation for SL lives).

**What it is:** The exact string manipulation that transforms a plaintext password into the `$1$<md5>` format required by the SL login endpoint.

**Why it's non-obvious:** It's a grid-specific requirement that is often mistaken for a transmission-layer encryption step. It lives in `viewer_grid` because it's a "meaning/shape" concern, not a transport one.

---

## G03 — Scene Hierarchy "Spatial Sync" boundary

**Location:** `crates/viewer_core/src/scene.rs` -> `Scene::sync_spatial()`

**What it is:** The function that propagates parent transforms to children and updates the Octree.

**Why it's non-obvious:** Individual `Transform` changes do NOT immediately update the Octree or child world matrices. The system assumes a "batch sync" occurs once per frame. If you add a new scene role or move an object and it doesn't appear to move or cull correctly, check if `sync_spatial()` is being called after your change.

---

## G04 — Live Visual Snapshot Internal Serialization

**Location:** `crates/viewer_net/src/worker.rs` (emit path) and `crates/viewer_app/src/main.rs` (receive path).

**What it is:** The logic that determines which fields are serialized to `live_visual_snapshot.json` when running in dev/fallback mode.

**Why it's non-obvious:** Not every field in the `LiveVisualSnapshot` struct is serialized. Secrets are explicitly excluded at the type or serialization layer. If you add a new field to the snapshot and it doesn't show up in the JSON file during debugging, check the `serde` attributes on the struct in `viewer_core`.

---

## G05 — Coarse Location Self-Avatar Fallback ID

**Location:** `crates/viewer_app/src/social.rs` (or adjacent avatar merge logic).

**What it is:** The deterministic UUID used to identify the user's own avatar when the simulator sends a coarse update without an ID block.

**Why it's non-obvious:** When the ID block is missing, the viewer must still know which "point" is the user. We use a stable fallback to prevent the self-avatar from flickering out of existence.

---

## G06 — Material Descriptor Caching & GPU Binding

**Location:** `crates/viewer_render/src/lib.rs` -> `MaterialCache`

**What it is:** The management system for PBR material uniforms and texture bind groups.

**Why it's non-obvious:** It bridges `viewer_core::MaterialDescriptor` to `wgpu::BindGroup`. It handles the mapping of Legacy vs PBR descriptors to a unified shader-side `MaterialUniform`. If you are changing how materials are looked up or how textures are applied, this is the primary ownership point.

---

## G08 — High-Level Horizontal Documentation Headers

**Location:** Top of `lib.rs` in every crate (`viewer_app`, `viewer_core`, etc.)

**What it is:** Standardized documentation blocks that define the "Purpose", "Logic", and "Boundary Rules" for each crate and major subsystem.

**Why it's non-obvious:** These headers serve as the primary on-boarding and continuity mechanism for agents. They explicitly state what a crate *must not* do (e.g., `viewer_net` must not own grid-specific meaning). When starting work in a new crate, always read these headers first to avoid architectural drift.
