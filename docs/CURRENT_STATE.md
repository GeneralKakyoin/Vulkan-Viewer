# CURRENT_STATE.md

A living state snapshot. This file describes what is true now.
Update it when behavior changes materially.
Do not use it as a roadmap, milestone history log, or planning journal.

---

## Latest notable changes

* R01 execution: mesh loading/caching is hardened so empty/invalid glTF bytes cannot poison the mesh cache; dynamic geometry upload skips zero-index submeshes and only updates AABBs when upload succeeds.
* A02 execution: fixture-backed texture acquisition + bounded CPU cache foundation is in place (`AssetStatus` + `FixtureTextureCache`), and `viewer_app` can upload fixture textures to the renderer when `VIEWER_FIXTURE_TEXTURES` is set.
* New flag-driven runtime verification modes are available:
  * `STRESS_TEST=camera` for deterministic auto-camera orbit validation
  * `STRESS_TEST=screenshot` for automated PNG capture (`artifacts/screenshots` default) to support model/agent visual verification
* A canonical test/verification reference now exists at `docs/TESTING_REFERENCE.md` (commands, flags, and `VIEWER_*` env vars).
* New prefixed milestone plans are now authored:
  * `docs/plans/PLAN_R01.md`
  * `docs/plans/PLAN_A02.md`
* Matching plan reviews are now authored:
  * `docs/reviews/REVIEW_plan_r01.md`
  * `docs/reviews/REVIEW_plan_a02.md`
* `N03` execution: bounded object-update classification + minimal decode now exports a capped object feed through `LiveVisualSnapshot` and the ingestion seam, and the scene now renders deterministic world-object feed proxy instances with lifecycle-safe removal behavior.
* Deferred-feature capture policy is now propagated across workflow docs (`AGENTS.md`, `docs/TASKS.md`, and planner/reviewer runbook prompts), not just `docs/plans/PLAN.md`.
* `docs/plans/PLAN.md` now includes explicit instructions for expanding future concise milestones (`U04+`) into implementation-ready milestone plans.
* A canonical deferred-feature parking list now exists at `docs/plans/DEFERRED_FEATURES.md`; planning-time "too early" features must be recorded there.
* The master roadmap in `docs/plans/PLAN.md` now uses prefixed milestone IDs (`R/A/N/U`) and treats the next milestone sequence as `R01 -> A02 -> N03` (with `N03` now next).
* `docs/plans/RENDERING_ROADMAP_V2.md` is now deprecated and redirects planning to the master roadmap plus per-milestone `PLAN_<ID>.md` files.
* The rendering track should currently be treated as complete through **M2**.
* The M2 geometry torture test (`STRESS_TEST=2`) now uploads and renders real procedural/sculpt geometry (dynamic meshes) instead of falling back to diagnostic cubes.
* `LLVolume` circle-path extrusion now produces 3D geometry (closed paths stitched; no planar end caps on integer-revolution circles).
* The repo is beyond the original Phase E baseline and now includes:

  * bounded live-world ingestion seams
  * avatar placeholder world presence
  * spatial scene-management groundwork
  * geometry-engine groundwork for SL primitives, mesh, and sculpt support
* Continuity cleanup is in progress so this file reflects present truth only.

---

## Current overall state

The viewer has a working runtime, proven live login/bootstrap compatibility, a bounded first connected-world slice, avatar placeholder presence, and a rendering track that has advanced through **M2**.

The current repo state should be understood as:

* **Core runtime and networking foundation:** proven
* **First connected world slice:** substantially proven
* **Rendering track:** complete through **M2**
* **Full asset-backed world rendering:** not yet complete
* **Broad viewer parity features:** still future work

---

## What is proven

### Runtime and app foundation

The following are established baseline behavior:

* stable `wgpu` / `winit` / `egui` runtime foundation
* camera, scene, and typed renderable-instance flow
* renderer architecture viability
* crate boundaries are holding
* `viewer_app` owns orchestration rather than renderer internals

### Networking and login

The following are proven:

* real Second Life login succeeds
* login transport covers LLSD / XML-RPC compatibility cases
* seed capability fetch and EventQueue bootstrap are viable
* first-simulator LLUDP handshake succeeds in live runs
* early post-login traffic is typed and classified

### Live ingestion seam and first world slice

The following are established:

* bounded decode -> snapshot -> seam -> scene mapping is viable
* bounded coarse neighborhood ingestion is viable
* pre-world object/state composition is viable
* avatar placeholder presence is viable
* chat / IM / nearby / profile UI shell exists

### Rendering track

The rendering side should currently be treated as complete through **M2**.

#### M1 — Spatial foundation and scene management

The repo state includes spatial and scene-management groundwork beyond the original flat diagnostic baseline, including:

* hierarchical scene graph support via `parent_id`
* quaternion-based rotation support in `Transform`
* `Scene::sync_spatial()` as the batch spatial-update boundary
* world-matrix propagation for parent/child transforms
* dirty-flag spatial update flow and stable object-ID handling
* renderer consumption of precomputed world matrices
* hierarchical stress-test coverage for dynamic spatial behavior

#### M2 — Geometry engine groundwork

The repo state includes geometry-engine groundwork for asset-backed rendering:

* procedural `LLVolume` generation in `viewer_core`
* `gltf`-based mesh loading groundwork in `viewer_asset`
* sculpt decode support for legacy content
* geometry caching and early LOD-selection groundwork
* preserved SL-compatible face bitmasks for future material alignment

This means the repo is **past** the original “diagnostic proxies only” rendering state, but it is **not yet** at full textured/material world rendering.

---

## Current known limits

### Rendering limits

These are still not complete:

* full texture and material pipeline
* final asset-backed world-object rendering path across the live world
* avatar rigging and skinned rendering
* environmental / EEP rendering
* polished post-processing and mature overlay systems

### Networking and protocol limits

These remain incomplete or only partially bounded:

* broad world/object decode beyond the current bounded slice
* region/world streaming beyond the current bounded diagnostic seam
* full `RegionHandshake` payload interpretation
* robust sim-name extraction and crossed-region handling across the whole runtime

### Viewer and product limits

These remain future work:

* complete login UI workflow
* inventory and map shells
* broader usability and parity expansion

---

## Current focus

The repo should not describe rendering as “not started.”
The last reliable rendering milestone should be treated as **M2 complete**.

The next work should therefore be framed as **post-M2 rendering expansion**, not as if geometry generation still has not happened.

That means planning should start from a repo state that already includes:

* runtime foundation
* login/bootstrap viability
* bounded live ingestion seam
* avatar placeholder world slice
* M1 spatial groundwork
* M2 geometry-engine groundwork

---

## Continuity rule for this file

`CURRENT_STATE.md` must describe present truth.
Keep history minimal.
A tiny “latest notable changes” section is allowed, but detailed milestone plans, execution history, and future sequencing belong in:

* `docs/plans/`
* `docs/reviews/`
* `docs/reports/`
* `docs/TASKS.md`
* `docs/HANDOFF.md`
