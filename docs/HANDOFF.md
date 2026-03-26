# HANDOFF.md

## What changed

- Replaced `docs/plans/PLAN.md` with a new canonical prefixed-milestone roadmap.
- The roadmap now uses `<Stream><NN>` milestone IDs with stream legend `R/A/N/U` and a dependency-led sequence.
- Legacy completion baseline remains explicit (`M0` through `M5` complete) and now has a legacy-to-new crosswalk table for traceability.
- The next three implementation-ready roadmap milestones are now `R01 -> A02 -> N03`, each with measurable validation expectations and explicit boundary checks.
- Deprecated `docs/plans/RENDERING_ROADMAP_V2.md` and added a redirect to `docs/plans/PLAN.md` plus per-milestone plan files (`PLAN_<ID>.md`).
- Updated continuity docs to reflect the roadmap transition.

## Validation run

- `cargo fmt --all -- --check` (pass)
- `cargo check` (pass)
- consistency checks on `docs/plans/PLAN.md` for:
  - legacy completion statement (`M0` to `M5`)
  - ID format and first three milestones (`R01`, `A02`, `N03`)
  - crosswalk table presence
  - Firestorm reference-only policy language
  - command-level validation expectations (`cargo fmt`, `cargo check`, `cargo test`, `cargo run -p viewer_app`)

## Exact current state

The current repository state should be treated as:

- core runtime, renderer baseline, and crate boundaries are proven
- live Second Life login/bootstrap compatibility is proven
- first-simulator LLUDP handshake is proven
- bounded decode -> snapshot -> seam -> scene mapping is proven
- bounded coarse neighborhood ingestion is viable
- avatar placeholder world presence is viable
- the rendering track is complete through **M2**
- full asset-backed textured/material world rendering is **not yet complete**
- the master roadmap is now prefixed and canonical at `docs/plans/PLAN.md`

More specifically, the rendering side should currently be understood as:

- **M1 complete**: spatial foundation and scene-management groundwork exist
- **M2 complete**: geometry-engine groundwork exists for SL primitives, mesh loading, sculpt support, geometry caching, and early LOD groundwork

## Exact next step

Plan and review `R01` as the first prefixed milestone:

1. author `docs/plans/PLAN_R01.md` as a decision-complete implementation plan
2. write plan review artifact in `docs/reviews/`
3. execute only after user sign-off on `PLAN_R01.md`

Then continue the same flow for `A02`, then `N03`.

## Blockers or risks

- `docs/plans/` currently appears as untracked in this working tree; roadmap docs are updated there but may not show up in tracked diffs until added in normal Git flow.
- If older references to legacy `M6+` sequencing remain in other docs, future agents may still need a final alignment sweep.
- Do not start implementation from roadmap text alone; each milestone still requires its own approved `PLAN_<ID>.md`.

## Constraints

Keep the current architectural boundaries intact:

- `viewer_app` = orchestration only
- `viewer_core` = shared domain state and scene/world mapping
- `viewer_render` = rendering internals and GPU submission
- `viewer_ui` = display/debug UI only
- `viewer_net` = transport/session/codec/diagnostics
- Firestorm is a behavior/protocol reference only, not an architecture template
