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
8. Record any "valid but too early" scope in `docs/plans/DEFERRED_FEATURES.md`.
9. Write plan review in `docs/reviews/`.
10. Obtain user sign-off before implementation.

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

## U04 — Core usability and workflow shell alignment

### Goal

Make startup, session status, social/profile workflow, and diagnostics operable as a coherent daily-use shell without changing architecture boundaries.

### Why now

After `R01`, `A02`, and `N03`, usability and workflow clarity become the main bottleneck for validating richer world behavior safely.

### In scope

* consolidate UI information architecture into clearer runtime/workflow panel grouping
* add explicit startup/session UX states (`disabled`, `starting`, `connected`, `failed`, `reconnecting`)
* tighten chat/IM/profile interaction flow and failure/status messaging
* standardize persisted social/profile cache indicators (fresh/stale/unknown) in workflow-facing UI

### Out of scope

* full parity UX breadth
* new transport/protocol semantics
* broad visual redesign unrelated to workflow clarity

### Boundary check

* `viewer_ui` owns status/workflow rendering and action emission
* `viewer_app` owns orchestration and action dispatch to existing worker/session controls
* `viewer_core` remains source-of-truth owner for social/profile domain state

### Deliverables

* expanded roadmap milestone definition (this section)
* milestone plan artifact `docs/plans/PLAN_U04.md`
* review artifact `docs/reviews/REVIEW_plan_u04.md`

### Validation expectations

* `cargo fmt`
* `cargo check`
* targeted tests in touched UI/app/core modules
* `cargo test` if behavior changes materially
* `cargo run -p viewer_app` workflow smoke covering startup-state transitions and chat/IM/profile interaction loop

### Exit criteria

A deterministic startup-to-social/profile workflow loop is usable in bounded mode, with explicit and stable status/failure presentation.

---

## R05 — Draw submission efficiency and transparency reliability

### Goal

Make renderer submission predictable under growing scene load and ensure transparent content renders in stable, repeatable order.

### Why now

`U04` improves operator workflow; next risk is frame-time instability and visual correctness drift as object/feed richness increases.

### In scope

* split draw submission into explicit pass buckets (`opaque`, `alpha-tested`, `transparent`)
* deterministic transparent ordering (camera-distance back-to-front sort)
* reduce per-frame submission churn via stable batching keys and resource reuse policy
* preserve deterministic fallback behavior for missing/incomplete geometry or material inputs

### Out of scope

* cinematic post-processing overhaul
* full environment/EEP rendering
* broad shader-model redesign

### Boundary check

* `viewer_render` owns pass building, ordering, and GPU submission
* `viewer_app` remains orchestration-only
* `viewer_asset` and `viewer_core` provide inputs/contracts, not render-policy ownership

### Deliverables

* expanded roadmap milestone definition (this section)
* milestone plan artifact `docs/plans/PLAN_R05.md`
* review artifact `docs/reviews/REVIEW_plan_r05.md`

### Validation expectations

* `cargo fmt`
* `cargo check`
* targeted renderer tests (pass bucketing, stable ordering, fallback determinism)
* `cargo test` when rendering behavior changes materially across crate boundaries
* `cargo run -p viewer_app` stress smoke with repeated camera sweeps for transparent-order stability

### Exit criteria

Transparent content ordering is stable across repeated camera movement and bounded stress runs, and submission behavior remains deterministic.

---

## A06 — Material/texture depth expansion

### Goal

Promote existing material scaffolding into an active, bounded multi-texture pipeline supporting legacy and early PBR inputs through typed contracts.

### Why now

`R05` establishes reliable submission lanes needed to scale material complexity safely.

### In scope

* activate `MaterialDescriptor` flow from scene-facing data into renderer binding
* support bounded texture set: base color, normal, metallic/roughness, emissive (where available)
* integrate texture transform/animation matrix path already present in `viewer_core::material::animation`
* extend asset/cache/provider contracts for material-driven texture requests and deterministic `Loading`/`Missing` fallback handling

### Out of scope

* full parity material surface
* avatar baking/system-layer composition
* unbounded live asset streaming

### Boundary check

* `viewer_asset` owns acquisition/cache policy
* `viewer_render` owns bind groups, material uniforms, and GPU-side fallback behavior
* `viewer_core` owns material descriptor semantics and shared typed contracts

### Deliverables

* expanded roadmap milestone definition (this section)
* milestone plan artifact `docs/plans/PLAN_A06.md`
* review artifact `docs/reviews/REVIEW_plan_a06.md`

### Validation expectations

* `cargo fmt`
* `cargo check`
* targeted tests for material descriptor mapping and texture-ready/missing transitions
* `cargo test` when material behavior changes materially
* `cargo run -p viewer_app` mixed-material fixture smoke verifying deterministic legacy/PBR fallback paths

### Exit criteria

Bounded mixed legacy/PBR materials render through typed contracts with deterministic loading/missing fallback behavior.

---

## N07 — Region continuity and world scaling

### Goal

Expand from first-region bounded slice to bounded multi-region continuity with explicit typed transition and handoff semantics.

### Why now

After render/material stabilization (`R05` + `A06`), the next safe growth step is continuity breadth without collapsing network/grid boundaries.

### In scope

* type and propagate region-transition control signals (`CrossedRegion`, `ConfirmEnableSimulator`) into bounded continuity state
* add bounded multi-region presence model (active region, neighbor summaries, handoff phase)
* extend ingestion seam with continuity lanes that preserve seam ownership rules
* maintain bounded decode strategy with explicit allowlist and diagnostics-first handling for unknown traffic

### Out of scope

* protocol-everything expansion
* unlimited region streaming
* `viewer_net`/`viewer_grid` boundary collapse

### Boundary check

* `viewer_net` owns transport/decode mechanics and diagnostics
* `viewer_grid` owns semantic interpretation/policy
* `viewer_core::Scene` seam path remains the only create/remove authority for seam-owned roles

### Deliverables

* expanded roadmap milestone definition (this section)
* milestone plan artifact `docs/plans/PLAN_N07.md`
* review artifact `docs/reviews/REVIEW_plan_n07.md`

### Validation expectations

* `cargo fmt`
* `cargo check`
* targeted decode + continuity-lane + seam-ownership tests
* `cargo test` when continuity behavior changes materially
* `cargo run -p viewer_app` bounded transition-control observation runs verifying continuity state and scene behavior

### Exit criteria

Bounded region-transition continuity is visible in diagnostics and scene behavior without boundary violations.

---

## R08 — Avatar appearance and attachment render foundation

### Goal

Establish a bounded avatar-render path that supports deterministic avatar body proxies and first attachment rendering through existing material/render contracts.

### Why now

After `N07`, region continuity is bounded and stable enough to support richer presence visuals without collapsing scene ownership or renderer boundaries.

### In scope

* add bounded avatar appearance payload mapping into scene-facing renderable contracts
* support deterministic attachment proxy rendering (small bounded attachment set)
* preserve A06 material contract usage for avatar/attachment surfaces (no separate ad-hoc path)
* add avatar/attachment lifecycle handling tied to seam-owned presence updates

### Out of scope

* full animation graph parity (skeletal retargeting, AO layers, full blend trees)
* full baked texture pipeline parity
* complete wearable/system-layer parity

### Boundary check

* `viewer_core` owns avatar/attachment scene-domain contracts and lifecycle semantics
* `viewer_render` owns GPU pipelines and draw submission for avatar/attachment paths
* `viewer_app` remains orchestration-only and does not own avatar render policy
* `viewer_net`/`viewer_grid` own transport/semantic inputs, not render implementation details

### Deliverables

* expanded roadmap milestone definition (this section)
* milestone plan artifact `docs/plans/PLAN_R08.md`
* review artifact `docs/reviews/REVIEW_plan_r08.md`

### Validation expectations

* `cargo fmt`
* `cargo check`
* targeted tests in `viewer_core` + `viewer_render` for avatar/attachment lifecycle + mapping
* `cargo test` when cross-crate behavior changes materially
* `cargo run -p viewer_app` bounded visual smoke for avatar/attachment appearance stability

### Exit criteria

Avatar body proxies and bounded attachment proxies render deterministically and update/remove correctly through seam-driven lifecycle behavior.

---

## U09 — Workflow depth and product usability expansion

### Goal

Improve day-to-day operator workflow so world continuity, avatar state, social interaction, and diagnostics can be monitored and acted on without leaving the app.

### Why now

`R08` introduces richer world/avatar output; `U09` is needed to keep debugging and user workflow clarity aligned with that increased runtime complexity.

### In scope

* extend diagnostics workflow for continuity + avatar-state visibility with clear status grouping
* improve social/profile workflow depth (thread handling, profile refresh affordances, clearer stale/fresh semantics)
* add bounded workflow shortcuts for common operator actions (focus/inspect/toggle diagnostics)
* tighten failure-state messaging for reconnect, stale data, and partial-world states

### Out of scope

* full product UX parity sweep
* in-app credential-entry/security redesign
* broad UI framework redesign unrelated to workflow depth

### Boundary check

* `viewer_ui` owns presentation and user action emission
* `viewer_app` owns orchestration and command handling
* `viewer_core` remains source of truth for session/social/avatar/continuity state
* no transport or render-policy ownership shift into UI

### Deliverables

* expanded roadmap milestone definition (this section)
* milestone plan artifact `docs/plans/PLAN_U09.md`
* review artifact `docs/reviews/REVIEW_plan_u09.md`

### Validation expectations

* `cargo fmt`
* `cargo check`
* targeted tests in touched UI/app/core modules
* `cargo test` when workflow behavior changes materially
* `cargo run -p viewer_app` workflow smoke covering continuity + social/profile loops

### Exit criteria

Operators can complete bounded continuity + social/profile workflows with explicit, stable status and failure presentation.

---

## A10 — Asset streaming continuity and cache discipline

### Goal

Strengthen asset continuity across region transitions using bounded streaming/caching policy that preserves determinism and avoids unbounded retention.

### Why now

After `N07` continuity and `R08` avatar/attachment surfaces, asset behavior across handoffs becomes the main stability bottleneck.

### In scope

* add bounded asset continuity policy for region transition windows (retain, promote, evict rules)
* extend typed asset request prioritization for continuity-critical content (active + near-neighbor region scope only)
* add deterministic fallback behavior when continuity assets are loading/missing during handoff
* add cache metrics needed to validate continuity policy behavior

### Out of scope

* unbounded background prefetch
* global long-lived asset residency policy
* full CDN/capability parity for every asset class

### Boundary check

* `viewer_asset` owns fetch/decode/cache/eviction policy
* `viewer_render` consumes asset-ready content but does not own asset policy
* `viewer_app` coordinates policy inputs; it does not become cache owner
* `viewer_net`/`viewer_grid` provide continuity context, not asset lifecycle implementation

### Deliverables

* expanded roadmap milestone definition (this section)
* milestone plan artifact `docs/plans/PLAN_A10.md`
* review artifact `docs/reviews/REVIEW_plan_a10.md`

### Validation expectations

* `cargo fmt`
* `cargo check`
* targeted tests for cache policy + continuity-window retention/eviction behavior
* `cargo test` when cross-crate asset/render behavior changes materially
* `cargo run -p viewer_app` continuity smoke across bounded transition scenarios

### Exit criteria

Continuity-critical assets remain stable through bounded transition windows with deterministic fallback and bounded cache growth.

---

## N11 — Region handoff hardening and transition diagnostics

### Goal

Harden bounded region handoff behavior so transitions are more reliable under packet delay/loss and produce explicit diagnostics for recovery, without expanding into full teleport/session-parity orchestration.

### Why now

`A10` establishes bounded continuity-aware asset policy. The next bottleneck is transition robustness and observability when crossing simulators under real network conditions.

### In scope

* tighten handoff state progression and recovery logic around `CrossedRegion` / `ConfirmEnableSimulator`
* add bounded transition diagnostics with explicit reason classification for stalled or failed handoffs
* add deterministic transition timers/budgets for retry windows and fallback behavior
* improve continuity state reporting so `viewer_ui` can distinguish healthy handoff, delayed handoff, and degraded handoff states
* keep region-neighbor scope bounded and avoid expanding to unlimited graph management

### Out of scope

* full teleport workflow parity
* cross-grid roaming/session migration
* unlimited region graph streaming
* broad protocol expansion outside transition-control packets and diagnostics required by this milestone

### Boundary check

* `viewer_net` owns transition-control transport/decode mechanics and retry/timer plumbing
* `viewer_grid` owns semantic interpretation/classification of transition outcomes
* `viewer_core` owns typed continuity state and seam-facing contracts
* `viewer_ui` presents transition diagnostics only; no transition-policy ownership
* `viewer_app` orchestrates and maps typed state only; no transport-policy ownership

### Deliverables

* expanded roadmap milestone definition (this section)
* milestone plan artifact `docs/plans/PLAN_N11.md`
* review artifact `docs/reviews/REVIEW_plan_n11.md`

### Validation expectations

* `cargo fmt`
* `cargo check`
* targeted tests for transition-state progression and bounded-recovery behavior
* targeted tests for transition diagnostic classification and continuity reporting
* `cargo test` when cross-crate transition behavior changes materially
* `cargo run -p viewer_app` bounded transition smoke with diagnostics verification

### Exit criteria

Region handoffs remain bounded, deterministic, and diagnosable under degraded conditions, with clear typed outcome reporting and no boundary violations.

---

## R12 — Environment and atmospheric baseline

### Goal

Introduce a bounded environment rendering baseline (sky/ambient/fog/time-of-day influence) that improves world readability without committing to full EEP parity.

### Why now

After `N11` transition hardening, visual continuity quality becomes the next usability bottleneck. A bounded environment baseline increases readability while preserving renderer stability.

### In scope

* add a typed environment state contract in `viewer_core` (ambient, sky tint, fog parameters, bounded time-of-day scalar)
* implement renderer support for environment-driven lighting/fog in `viewer_render`
* wire bounded environment state from app/runtime inputs into render path
* add deterministic fallback environment profile when environment inputs are missing

### Out of scope

* full EEP parity (day cycles, water/sky shader parity, advanced atmosphere scattering)
* cinematic post-processing suites
* broad material model redesign beyond environment baseline needs

### Boundary check

* `viewer_core` owns environment domain contract
* `viewer_render` owns shader/pipeline behavior for environment effects
* `viewer_app` maps/wires environment state only
* no environment-policy ownership leakage into `viewer_ui`/`viewer_net`

### Deliverables

* expanded roadmap milestone definition (this section)
* milestone plan artifact `docs/plans/PLAN_R12.md`
* review artifact `docs/reviews/REVIEW_plan_r12.md`

### Validation expectations

* `cargo fmt`
* `cargo check`
* targeted renderer tests for environment uniform mapping/fallback determinism
* `cargo test` when render behavior changes materially
* `cargo run -p viewer_app` bounded visual smoke for fog/ambient/sky transitions

### Exit criteria

Environment baseline renders deterministically through typed contracts, improves scene readability, and preserves renderer boundary discipline.

---

## A13 — Live asset transport bridge and cache integration slice

### Goal

Expand from fixture-first asset behavior to a bounded live asset bridge for selected texture/mesh classes while preserving A10 cache discipline and deterministic fallback behavior.

### Why now

`R12` increases dependency on stable live visual inputs. The next highest leverage step is a bounded live asset bridge to reduce fixture dependence without overreaching into full parity.

### In scope

* add bounded live fetch path for selected asset classes (starting with texture IDs already present in scene/material contracts)
* integrate live fetch results into existing `viewer_asset` cache policy and priority model
* preserve deterministic `Loading`/`Missing` fallback behavior in `viewer_render`
* add explicit diagnostics for source (`fixture` vs `live`) and bounded failure classification

### Out of scope

* full asset-class parity across all content types
* unbounded prefetch or persistent cross-session cache policy
* invasive transport redesign

### Boundary check

* `viewer_asset` owns asset-source selection, decode, cache admission, and eviction policy
* `viewer_net` owns capability transport mechanics only
* `viewer_grid` owns capability semantics/policy interpretation
* `viewer_render` consumes ready assets/fallback states only
* `viewer_app` orchestrates request wiring only

### Deliverables

* expanded roadmap milestone definition (this section)
* milestone plan artifact `docs/plans/PLAN_A13.md`
* review artifact `docs/reviews/REVIEW_plan_a13.md`

### Validation expectations

* `cargo fmt`
* `cargo check`
* targeted tests for live-vs-fixture source selection and cache policy invariants
* targeted tests for deterministic fallback under live fetch failure/latency
* `cargo test` when cross-crate asset behavior changes materially
* `cargo run -p viewer_app` bounded live-asset smoke (with explicit diagnostics assertions)

### Exit criteria

A bounded set of live assets flows through `viewer_asset` and existing render contracts with deterministic cache/fallback behavior and explicit source diagnostics.

---

## U14 — Workflow resilience and operator recovery controls

### Goal

Improve runtime operator control during degraded states (transition delay, stale assets, reconnect loops) so workflow remains predictable without introducing architecture drift.

### Why now

After `A13`, operators need stronger runtime control and clearer recovery affordances to validate broader live behavior safely.

### In scope

* add bounded recovery controls (retry handoff diagnostics probes, reset bounded asset queues, clear stale workflow indicators)
* improve degraded-state messaging and callouts for transition/asset health
* extend diagnostics grouping for transition + asset source health summaries
* tighten keyboard/action shortcuts for recovery paths where appropriate

### Out of scope

* full UX redesign
* account/session security workflow overhaul
* embedding transport or cache policy logic in UI

### Boundary check

* `viewer_ui` owns presentation/action emission
* `viewer_app` owns action dispatch/orchestration
* `viewer_core` owns typed state contracts for diagnostics/recovery indicators
* transport/cache policy remains owned by `viewer_net`/`viewer_asset`

### Deliverables

* expanded roadmap milestone definition (this section)
* milestone plan artifact `docs/plans/PLAN_U14.md`
* review artifact `docs/reviews/REVIEW_plan_u14.md`

### Validation expectations

* `cargo fmt`
* `cargo check`
* targeted UI/app/core tests for recovery controls and status messaging
* `cargo test` when workflow behavior changes materially
* `cargo run -p viewer_app` degraded-state workflow smoke (transition + asset failure scenarios)

### Exit criteria

Operators can diagnose and recover from bounded degraded states through clear, deterministic controls and messaging without boundary violations.

---

## Ordering constraints

The following order is intentional unless explicitly revised:

1. completed legacy baseline (`M0` to `M5`) remains fixed
2. `R01` before any broad render-feature expansion
3. `A02` before deeper asset-backed rendering
4. `N03` before broad world-feed rendering expectations
5. `U04` after first stable render+asset+feed triad
6. `R05` -> `A06` -> `N07` remains fixed as the first continuity/stability arc
7. `R08` -> `U09` -> `A10` must complete before expanding `N11`
8. `N11` -> `R12` -> `A13` -> `U14` is the next fixed bounded arc unless explicitly revised
9. later milestones continue dependency-led interleaving, not strict round-robin

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

## How to plan future concise milestones (`N11+`)

Milestones still listed as "(concise)" are roadmap placeholders, not implementation-ready plans.

Before planning or implementing one of these milestones, the planner must:

1. confirm predecessor dependencies are complete (or explicitly document any accepted partial dependency risk)
2. expand the milestone in this file to the full milestone shape:
   * `Goal`
   * `Why now`
   * `In scope`
   * `Out of scope`
   * `Boundary check`
   * `Deliverables`
   * `Validation expectations`
   * `Exit criteria`
3. split the milestone into additional prefixed IDs if the scope is too broad for one bounded plan
4. update ordering constraints if dependency order changes
5. create `docs/plans/PLAN_<ID>.md` and a review artifact in `docs/reviews/`
6. obtain user sign-off before implementation starts

If this conversion is not done, the milestone is not ready for implementation work.

---

## Deferred-too-early feature capture (mandatory)

When planning identifies a feature that is useful but too early for the current milestone, it must not be lost.

Record each deferred item in `docs/plans/DEFERRED_FEATURES.md` with:

* candidate feature
* source milestone/plan where it was considered
* exact reason it was deferred as "too early"
* earliest milestone where it may be reconsidered
* dependency/trigger needed before reconsideration
* status (`open`, `promoted`, `dropped`)

Do not delete deferred items when promoted. Mark them `promoted` and reference the milestone plan that picked them up.

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
## Deferred-too-early candidates captured
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

1. `N11`
2. `R12`
3. `A13`
4. `U14`

Each requires its own approved `docs/plans/PLAN_<ID>.md` before implementation.
