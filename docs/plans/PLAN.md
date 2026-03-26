# PLAN.md

This file is the canonical, end-to-end project roadmap for the viewer.
It remains broader than `CURRENT_STATE.md` and more roadmap-oriented than `MASTER_PLAN.md`.

Use it to understand:

* where the project started
* what is already complete
* what major milestones remain
* what order work should happen in
* how each milestone must be planned before implementation

---

## What this file is for

`PLAN.md` is the full-project roadmap and milestone sequencing source.
It should answer:

* what the project is trying to become
* which milestones are already complete
* which milestone comes next
* why milestone order is intentional
* how to produce implementation-ready milestone plans

It should **not** become:

* a daily status log
* a handoff journal
* a detailed execution report
* a duplicate of `CURRENT_STATE.md`

Current truth belongs in `CURRENT_STATE.md`.
Exact next-step continuity belongs in `HANDOFF.md`.
Durable repository law belongs in `MASTER_PLAN.md` and `AGENTS.md`.

---

## How to use this file

### If you are planning a milestone

1. Read all of `PLAN.md`.
2. Read `MASTER_PLAN.md` for durable mission and boundaries.
3. Read `CURRENT_STATE.md` and `HANDOFF.md`.
4. Read `ARCHITECTURE.md`, `INTERFACES.md`, `LEARNINGS.md`, and `FIELD_GUIDE.md`.
5. Read prior relevant plan/review/report artifacts.
6. Research protocol-sensitive behavior before planning.
7. Write a decision-complete milestone plan in `docs/plans/PLAN_<ID>.md`.
8. Write plan review in `docs/reviews/`.
9. Obtain user sign-off before implementation.

### If you are executing a milestone

1. Read the approved `docs/plans/PLAN_<ID>.md`.
2. Re-check `CURRENT_STATE.md`, `HANDOFF.md`, and `LEARNINGS.md`.
3. Implement only approved scope.
4. Stop if architecture or crate boundaries would change.
5. Validate thoroughly.
6. Write an execution report in `docs/reports/REPORT_<slug>.md`.
7. Update continuity docs.

---

## Planning principles

All roadmap work must preserve:

* compatibility first
* stability second
* maintainability third
* performance fourth
* feature breadth last

All milestones must:

* preserve crate boundaries
* be bounded and reviewable
* define validation up front
* include explicit non-goals

Firestorm policy:

* Firestorm is a behavior/protocol reference
* Firestorm is not an architecture template
* parity is a long-term outcome, not a shortcut justification

---

## Milestone ID system

Milestone IDs now use the prefixed format `<Stream><NN>`.

Examples: `R01`, `A02`, `N03`.

### Stream legend

* `R` = Rendering
* `A` = Asset pipeline and content preparation
* `N` = Network/protocol decode and world-feed shaping
* `U` = UX/UI and workflow usability

Numbering is global by roadmap sequence, while prefix indicates the primary owning stream.

---

## Current roadmap position

The current repo baseline remains complete through legacy milestones **M0 to M5**.
The new prefixed milestone system starts from the next unfinished roadmap work.

### Legacy completion summary (concise)

* `M0` project/workspace foundation: complete
* `M1` runtime and render bootstrap: complete
* `M2` core runtime sandbox: complete
* `M3` real login/bootstrap viability: complete
* `M4` first connected-world slice plus spatial groundwork: complete
* `M5` geometry engine groundwork: complete

### Legacy-to-new crosswalk (traceability)

| Legacy milestone | Legacy meaning | New roadmap dependency anchor |
| --- | --- | --- |
| M0 | workspace/project foundation | prerequisite baseline for all prefixed milestones |
| M1 | runtime/render bootstrap | prerequisite for `R01` and all render-adjacent milestones |
| M2 | core runtime scene/camera sandbox | prerequisite for `R01` and `U` workflow milestones |
| M3 | live login/bootstrap viability | prerequisite for `N03` and later world-feed milestones |
| M4 | connected-world slice and spatial scene groundwork | prerequisite for `N03` and `R01` |
| M5 | geometry engine groundwork | direct prerequisite for `R01` and `A02` |

---

# Planned milestones (prefixed system)

## R01 — Post-M5 rendering integration expansion

### Goal

Advance from geometry groundwork to a reliable, bounded render-preparation path for world objects, without broad protocol expansion or full material-system scope.

### Why now

`M5` delivered geometry capability. The next rendering-safe step is to harden render-preparation ownership and visible behavior with existing bounded world feed assumptions.

### In scope

* tighten render-preparation flow from `viewer_core` scene data into `viewer_render`
* enforce renderer ownership of GPU submission internals
* formalize bounded fallback behavior for incomplete geometry/material inputs
* document expected object classes rendered in this milestone
* runtime verification path for mixed primitive/sculpt/mesh proxy behavior in bounded mode

### Out of scope

* broad object decode expansion
* full texture/material parity
* avatar rigging/skinning
* environment/EEP effects

### Boundary check

* `viewer_app` remains orchestration only
* `viewer_core` owns domain scene state and mesh-kind semantics
* `viewer_render` owns GPU/pipeline internals and draw submission
* no grid/protocol policy added to render crates

### Deliverables

* stable bounded render-preparation path for post-M5 geometry content
* clear fallback matrix for missing/partial content
* updated milestone plan artifact `docs/plans/PLAN_R01.md` before implementation

### Validation expectations

* `cargo fmt`
* `cargo check`
* targeted tests for touched crates
* broader `cargo test` when behavior changes materially
* `cargo run -p viewer_app` visual/runtime verification when rendering path changes

### Exit criteria

A bounded, demonstrable set of world objects renders through the intended path with explicit fallback behavior and no boundary violations.

---

## A02 — Asset acquisition, caching, and material input foundation

### Goal

Establish the first robust asset-input layer needed by rendering milestones, including acquisition, cache discipline, and typed handoff contracts.

### Why now

`R01` needs reliable content inputs; without `A02`, rendering progress risks brittle one-off asset handling.

### In scope

* define bounded asset acquisition path in `viewer_asset`
* add cache behavior rules (keying, reuse, invalid/missing handling)
* define typed handoff from asset layer to render-preparation consumers
* support minimal material input foundation needed for near-term rendering milestones

### Out of scope

* full final material system parity
* broad streaming policy across all world content
* avatar baking pipeline

### Boundary check

* `viewer_asset` owns fetch/decode/cache policy
* `viewer_render` does not own asset policy
* `viewer_net` remains transport/protocol mechanics only
* `viewer_core` remains shared type vocabulary

### Deliverables

* bounded asset fetch/cache policy surface
* typed asset-to-render preparation contracts
* milestone plan artifact `docs/plans/PLAN_A02.md` before implementation

### Validation expectations

* `cargo fmt`
* `cargo check`
* targeted asset/cache tests where practical
* broader `cargo test` for behavior-affecting changes
* `cargo run -p viewer_app` for mixed-content runtime verification

### Exit criteria

Asset-backed inputs are available through a stable typed path and can be consumed by rendering without ownership leakage.

---

## N03 — Bounded world/object decode expansion for richer render feed

### Goal

Expand bounded object/world decode just enough to provide richer object feed into the established render and asset pathways.

### Why now

After `R01` and `A02`, the bottleneck is feed richness. `N03` broadens data availability without collapsing seam ownership or protocol boundaries.

### In scope

* bounded `ObjectUpdate` and related object-family decode expansion
* typed payload refinement for seam-owned world object ingestion
* object lifecycle handling improvements (create/update/stale/remove)
* richer transform/scale/shape propagation into scene structures

### Out of scope

* unlimited decode breadth
* direct protocol-to-render shortcuts
* full region continuity/re-handoff breadth

### Boundary check

* `viewer_net` handles transport/decode mechanics, not grid meaning
* `viewer_grid` retains semantic interpretation/policy
* seam ownership in `viewer_core::Scene` remains the only create/remove path for seam-owned roles

### Deliverables

* bounded richer world-object feed supporting render preparation
* lifecycle-safe seam updates with typed payload contracts
* milestone plan artifact `docs/plans/PLAN_N03.md` before implementation

### Validation expectations

* `cargo fmt`
* `cargo check`
* targeted decode and seam-ownership tests
* broader `cargo test` for material behavior impact
* `cargo run -p viewer_app` bounded live-world verification

### Exit criteria

A materially richer bounded world-object set reaches scene/render preparation through typed, boundary-compliant paths.

---

## U04 — Core usability and workflow shell alignment (concise)

### Dependency notes

Depends on stable outcomes from `R01`, `A02`, and `N03`.

### Non-goals

Not full parity UX breadth; no invasive architecture changes.

---

## R05 — Draw submission efficiency and transparency reliability (concise)

### Dependency notes

Depends on feed maturity from `N03` and asset discipline from `A02`.

### Non-goals

Not cinematic post-FX overhaul; not full environment rendering.

---

## A06 — Material/texture depth expansion (concise)

### Dependency notes

Depends on `R05` submission patterns and validated cache behavior from `A02`.

### Non-goals

Not broad avatar baking or full parity material feature set.

---

## N07 — Region continuity and world scaling (concise)

### Dependency notes

Depends on stable bounded decode discipline from `N03`.

### Non-goals

Not protocol-everything milestone; no boundary collapse.

---

## R08 — Avatar appearance and attachment render foundation (concise)

### Dependency notes

Depends on `A06` asset/material inputs and `N07` continuity robustness.

### Non-goals

Not complete animation/parity stack in one milestone.

---

## U09 — Workflow depth and product usability expansion (concise)

### Dependency notes

Depends on credible world/avatar rendering outcomes from prior milestones.

### Non-goals

Not giant catch-all parity bucket.

---

## R10+ — Environment, polish, and parity expansion (concise)

### Dependency notes

Begins only after stable world/object/avatar/workflow layers are proven.

### Non-goals

Do not treat parity as a single milestone; split into bounded, reviewed milestones.

---

## Ordering constraints

The following order is intentional unless explicitly revised:

1. completed legacy baseline (`M0` to `M5`) remains fixed
2. `R01` before any broad render-feature expansion
3. `A02` before deeper asset-backed rendering
4. `N03` before broad world-feed rendering expectations
5. `U04` after first stable render+asset+feed triad
6. later milestones continue dependency-led interleaving, not strict round-robin

If a proposal violates order, the planner must justify and obtain explicit approval.

---

## Milestone planning requirement

No meaningful implementation may begin from roadmap text alone.
Each milestone must first be broken into an approved plan document:

* `docs/plans/PLAN_R01.md`
* `docs/plans/PLAN_A02.md`
* `docs/plans/PLAN_N03.md`
* and so on for each subsequent milestone

---

## Standard milestone plan template

Use this structure for each `docs/plans/PLAN_<ID>.md`:

```md
# Plan: <ID> <title>

## Summary
## Objective
## Why now
## In scope
## Out of scope
## Current known state
## Files and components touched
## Boundary check
## Step sequence
## Validation plan
## Risks and open questions
## Completion criteria
```

Review artifacts live in `docs/reviews/`.
Execution reports live in `docs/reports/REPORT_<slug>.md`.

---

## Continuity rules for this file

Update `PLAN.md` only when one of these changes:

* a milestone is completed
* milestone ordering changes materially
* roadmap constraints change materially
* stream definitions or ID conventions change

Do not update this file for routine implementation churn.

---

## Current next planning targets

The next planning sequence is:

1. `R01`
2. `A02`
3. `N03`

Each requires its own approved `docs/plans/PLAN_<ID>.md` before implementation.
