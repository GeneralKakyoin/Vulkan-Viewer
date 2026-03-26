# Plan: A02 Asset Acquisition and Cache Foundation (Local+Stubbed First)

## Objective
Establish a typed, bounded, and deterministic asset acquisition + cache foundation that supports rendering consumers without introducing live capability-fetch coupling in this milestone.

## Scope
In scope:
- Define the asset-facing contract for `AssetID` lookups and pending states (`Loading`, `Ready`, `Missing`).
- Implement local+stubbed acquisition path suitable for deterministic tests and bounded runtime smoke.
- Define and enforce bounded LRU cache policy with default 512 MB budget and env override.
- Require deterministic fixture assets (`test_assets/*`) as milestone validation inputs.
- Define renderer-consumer boundary behavior for pending/missing assets.

Out of scope:
- Immediate live capability-backed fetch integration.
- Full material parity implementation.
- Avatar baking/system-layer composition pipeline.
- Protocol transport/policy expansion outside existing boundaries.

## Current known state
- `viewer_asset` contains a geometry cache for procedural/sculpt/mesh processed geometry.
- `viewer_render/src/texture_provider.rs` defines `PendingTexture` and provider scaffolding, including bounded-VRAM structures.
- `viewer_core/src/material/mod.rs` contains material descriptor scaffolding but is not yet fully integrated as a stable cross-crate contract surface.
- Deterministic fixture images already exist under `test_assets/`.
- Current roadmap/handoff requires `R01` completion first; this milestone is serially downstream.

## Files and components touched
- `crates/viewer_asset/src/lib.rs`
  Asset acquisition/caching ownership and typed provider contract implementation.
- `crates/viewer_asset/src/mesh_loader.rs`
  Deterministic local fixture loading behavior and error handling consistency.
- `crates/viewer_render/src/texture_provider.rs`
  Consumer-side pending-state semantics and bounded cache budget handling.
- `crates/viewer_app/src/main.rs`
  Orchestration wiring for stubbed/local provider usage in bounded runtime mode.
- `test_assets/`
  Deterministic fixture corpus used by tests and smoke scenarios.
- `crates/viewer_core/src/material/mod.rs` (if needed for contract finalization)
  Shared descriptor alignment only; no high-level policy migration.

## Boundary check
- `viewer_asset` owns acquisition policy, cache keying, and eviction policy.
- `viewer_render` consumes typed provider results; it does not own asset acquisition policy.
- `viewer_app` wires providers at orchestration level; no renderer internals migrate upward.
- `viewer_net` and `viewer_grid` remain out of scope except for explicit future handoff points.
- No cross-crate `wgpu` handle leakage beyond renderer ownership rules.

## Step sequence
1. Serial gate check:
   - Confirm `R01` is approved and closed before beginning `A02` execution.
2. Contract lock:
   - Finalize asset lookup contract around `AssetID` + `PendingTexture` states.
   - Define exact fallback behavior for missing/invalid fixture lookups.
3. Local+stub acquisition path:
   - Implement deterministic local fixture source and stub provider behavior.
   - Avoid live capability/network fetch integration in this milestone.
4. Bounded cache policy:
   - Implement/lock LRU eviction semantics with default `512 MB` budget.
   - Add env override (for example `VIEWER_ASSET_CACHE_BUDGET_MB`) with sane bounds.
5. Renderer consumer integration:
   - Ensure renderer consumes pending-state contract consistently (`Loading`/`Ready`/`Missing`).
   - Prevent undefined behavior when assets are late/unavailable.
6. Fixture-driven test coverage:
   - Add deterministic tests using `test_assets/*`.
   - Validate hit/miss behavior, eviction behavior, and fallback behavior.
7. Runtime smoke:
   - Run bounded runtime smoke in offline/local mode to confirm end-to-end wiring.
8. Milestone closeout:
   - Document cache policy, defaults, override behavior, and deferred live-fetch follow-up.

## Validation plan
- Static and build checks:
  - `cargo fmt`
  - `cargo check`
- Tests:
  - Targeted tests for provider contract behavior and cache keying/eviction.
  - Deterministic fixture-based tests (`test_assets/*`) are mandatory.
  - Broader `cargo test` when cross-crate behavior changes materially.
- Runtime smoke (required):
  - `cargo run -p viewer_app` in bounded/offline mode with fixture-backed provider path.
  - Verify stable behavior for `Loading`, `Ready`, and `Missing` paths.
- Cache policy checks:
  - Verify default budget is 512 MB.
  - Verify env override is honored and eviction still deterministic.

## Risks and open questions
Risks:
- Contract drift between `viewer_asset` and `viewer_render` may produce ambiguous pending-state handling.
- Fixture-only acquisition can diverge from eventual live-fetch behavior if interfaces are under-specified.
- Cache budget defaults may mask pressure unless tests intentionally force eviction.

Open questions:
- None for this milestone plan. Live capability-backed acquisition is explicitly deferred and does not block `A02`.

## Completion criteria
- Typed asset contract is implemented and documented for `AssetID` + pending states.
- Local+stubbed acquisition path works deterministically with required fixture assets.
- Bounded LRU cache policy is in place with default 512 MB budget and env override.
- Renderer consumer behavior is defined and stable for `Loading`/`Ready`/`Missing`.
- Required validation checks pass (or blockers are explicitly documented with remaining unvalidated items).
