# Plan: Cross-Crate Behavior/Function Split (Staged) 2026-04-04

## Objective
Create a bounded, reviewable refactor track that decomposes oversized crate entry files into behavior-focused modules across all crates, without doing a single broad rewrite.

## Scope
- In scope:
  - Staged file/module splits across all workspace crates (`viewer_app`, `viewer_asset`, `viewer_core`, `viewer_grid`, `viewer_net`, `viewer_platform`, `viewer_render`, `viewer_ui`).
  - Internal organization only (module extraction, function relocation, internal visibility cleanup).
  - No intended runtime behavior changes.
- Out of scope:
  - Feature expansion.
  - Cross-crate ownership changes.
  - Protocol behavior changes.
  - Large naming/style-only rewrites.

## Current known state
- The largest risk surfaces are monolithic files:
  - `crates/viewer_net/src/lib.rs` (~715 KB)
  - `crates/viewer_app/src/main.rs` (~439 KB)
  - `crates/viewer_core/src/lib.rs` (~280 KB)
- `viewer_ui` and `viewer_render` still have concentrated single-file ownership but are smaller and already partially modularized.
- Repo law and boundary docs require preserving crate ownership and avoiding opportunistic architecture drift.

## Files and components touched
- Planning artifact:
  - `docs/plans/PLAN_CROSS_CRATE_BEHAVIOR_FUNCTION_SPLIT_2026-04-04.md`
- Planned implementation targets (future waves):
  - `crates/viewer_net/src/*`
  - `crates/viewer_app/src/*`
  - `crates/viewer_core/src/*`
  - `crates/viewer_render/src/*`
  - `crates/viewer_ui/src/*`
  - `crates/viewer_asset/src/*`
  - `crates/viewer_grid/src/*`
  - `crates/viewer_platform/src/*`

## Boundary check
- `viewer_net` remains transport/decode mechanics; no grid semantic migration into `viewer_net`.
- `viewer_grid` remains semantic/policy shaping; no transport ownership move.
- `viewer_app` remains orchestration; no absorption of renderer/protocol/asset internals.
- `viewer_core` remains shared domain contract owner.
- `viewer_render` remains sole GPU internals owner.
- `viewer_ui` remains presentation-only.
- `viewer_asset` remains asset fetch/decode/cache owner.
- `viewer_platform` remains minimal platform boundary.

## Step sequence
1. Wave 0: Guardrails and inventory (all crates, no behavior edits)
- Add per-crate split maps (source file -> target module groups).
- Define wave-scoped no-regression acceptance checks.
- Produce a review artifact for the split maps before code extraction starts.

2. Wave 1: `viewer_net` extraction (highest risk, highest payoff)
- Extract into behavior modules while keeping crate API stable from callers:
  - login/session transport
  - LLUDP send/receive helpers
  - object-feed decode/state
  - mesh/asset fetch helpers
  - diagnostics/summary helpers
- Keep protocol semantics unchanged; only move code and tighten internal visibility.

3. Wave 2: `viewer_app` extraction
- Split orchestration lanes into modules:
  - startup configuration and mode parsing
  - live worker command and update handling
  - scene/snapshot/seam mapping
  - input/hotkey routing
  - diagnostics relay formatting
- Preserve `main.rs` as thin composition root.

4. Wave 3: `viewer_core` extraction
- Split domain-heavy sections:
  - scene lifecycle/apply paths
  - world object ingestion seam contracts
  - live visual snapshot contracts
  - social/presence contracts
- Preserve seam ownership invariants and existing public type paths.

5. Wave 4: `viewer_render` + `viewer_ui` extraction
- `viewer_render`: isolate pipeline setup, frame submission, material cache/bind group handling, and environment uniforms into focused modules.
- `viewer_ui`: isolate diagnostics panels, social/profile panels, and command emission helpers.
- No cross-layer ownership changes.

6. Wave 5: `viewer_asset` + `viewer_grid` + `viewer_platform` cleanup pass
- `viewer_asset`: split decode/cache/fetch policy modules.
- `viewer_grid`: split adapter, continuity shaping, and legacy compatibility helpers.
- `viewer_platform`: keep minimal, but align structure with workspace conventions.

7. Wave 6: Workspace consistency pass
- Normalize module naming patterns and visibility rules.
- Remove temporary compatibility shims introduced in early waves.
- Run workspace-level validation and produce final implementation report.

## Validation plan
- Per wave (mandatory):
  - `cargo fmt --all`
  - `cargo check -p <touched crates>`
  - `cargo test -p <touched crates>`
- Additional when runtime/orchestration/render paths are touched:
  - `cargo run -p viewer_app` (bounded smoke)
- End-of-track gate:
  - `cargo check --workspace`
  - `cargo test --workspace`
  - `cargo run -p viewer_app` (bounded runtime smoke)

## Risks and open questions
- Risk: hidden behavior drift during large function moves.
  - Mitigation: one crate group per wave, strict before/after targeted tests, no mixed feature work in same wave.
- Risk: boundary erosion under convenience edits.
  - Mitigation: wave review checklist explicitly verifies crate ownership constraints.
- Risk: merge friction if implementation overlaps active live-protocol work.
  - Mitigation: keep waves short and independently reviewable; avoid broad rename-only churn.
- Open question: whether to preserve legacy entrypoints exactly or introduce compatibility re-exports during each wave.
  - Current assumption: use temporary re-exports where needed, remove only in Wave 6.

## Deferred-too-early candidates captured
- No new deferred feature candidate identified in this planning slice.
- `docs/plans/DEFERRED_FEATURES.md` unchanged.

## Learnings pre-check
- Applicable learnings:
  - L03: seam-owned role lifecycle must stay in seam apply path.
  - L04: dirty-only apply path is load-bearing.
  - L06: enforce `viewer_net` vs `viewer_grid` boundary explicitly.
  - L07: do not leak sensitive session fields across snapshot boundaries.
  - L10: protocol identifiers must remain template-sourced; avoid accidental semantic rewrites during moves.
  - L22: visual/runtime smoke must include manual screenshot review when screenshot mode is used.
- Plan implication: this is a structural refactor plan only; no protocol/semantic behavior changes are allowed inside extraction waves.

## Completion criteria
- Every crate in the workspace has been included in at least one approved extraction wave.
- Monolithic entry files are reduced to composition roots plus module declarations.
- No crate ownership boundary violations are introduced.
- Required validations pass per wave and at workspace end gate.
- Plan review artifacts and execution reports exist for each implemented wave.
