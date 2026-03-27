# Plan: A13 Live Asset Transport Bridge and Cache Integration Slice

## Summary
Implement a bounded live asset bridge that routes selected texture/mesh asset fetches from SL capabilities into `viewer_asset` while preserving A10 cache discipline, deterministic fallback rendering, and strict transport/policy boundaries.

## Objective
- Move from fixture-first rendering inputs toward bounded live asset sourcing for selected classes.
- Integrate live source outputs into existing asset cache and priority policy (including A10 continuity priorities).
- Preserve deterministic `Loading` / `Missing` / `Ready` behavior in renderer-facing contracts.
- Add explicit observability of asset source and failure classification.

## Why now
- Current scene assets are largely fixture-backed; live fetch exists only for narrow profile-image use cases.
- `A10` created bounded continuity caching that can now be leveraged for live content.
- `R12` visual baseline work increases dependency on real asset fidelity.

## In scope
- `viewer_asset`:
  - Add bounded source abstraction for fixture vs live provider.
  - Add live texture ingestion path (first asset class slice) with deterministic decode/admission rules.
  - Reuse A10 quotas/priorities/eviction for live-fed entries.
- `viewer_net`:
  - Expose required transport methods for bounded live asset requests (no policy ownership).
- `viewer_grid`:
  - Own capability meaning/policy mapping where required (`GetTexture`, `ViewerAsset`, and bounded fallback policy semantics).
- `viewer_app`:
  - Wire live source availability into request flow.
  - Keep orchestration-only role and avoid cache-policy leakage.
- `viewer_render`:
  - No policy ownership changes; continue consuming ready/fallback states.
- Diagnostics:
  - add source visibility (`fixture`, `live`, `missing`) and bounded failure reason counters.

## Out of scope
- Full asset-class parity across all object/avatar/environment classes.
- Unbounded background prefetch or aggressive speculative download.
- Persistent cross-session cache residency policy.
- Broad network transport refactor.

## Current known state
- Fixture-based texture cache exists and is integrated in app render loop.
- A10 added continuity priorities, quotas, deterministic eviction, and metrics.
- SL capability discovery already includes `GetTexture` / `ViewerAsset` in worker path, and profile image fetch uses capability transport.
- Full scene asset streaming from SL is not yet wired.

## Files and components touched
- `crates/viewer_asset/src/lib.rs`
  - provider abstraction additions
  - live source integration and cache admission path
  - tests
- `crates/viewer_asset/src/texture_fixture.rs` (or split modules as needed)
  - shared cache behavior across source types
  - metrics/source counters
  - tests
- `crates/viewer_net/src/lib.rs`
  - bounded asset-fetch transport hooks (if not already exposed)
  - tests
- `crates/viewer_grid/src/lib.rs` (or adapter modules)
  - capability meaning mapping for live asset fetch policy
  - tests
- `crates/viewer_app/src/main.rs`
  - request wiring and source selection orchestration
  - tests
- `crates/viewer_ui/src/lib.rs` (bounded diagnostics)
  - display source/failure metrics
  - tests
- `docs/TESTING_REFERENCE.md`
  - add any new verification knobs/env vars introduced.

## Boundary check
- `viewer_asset` owns source selection, decode, cache admission, retention/eviction policy.
- `viewer_net` owns HTTP/capability transport mechanics only.
- `viewer_grid` owns capability semantics and policy interpretation.
- `viewer_render` consumes `AssetStatus` outputs only.
- `viewer_app` orchestrates requests and mappings only.
- `viewer_ui` is display-only for diagnostics.

## Step sequence
1. Define bounded source abstraction
   - fixture/live provider contract
   - stable source-state enum for diagnostics.
2. Implement live fetch path in asset crate
   - request pipeline with bounded concurrency/caps
   - decode + cache admission using existing A10 policy.
3. Wire transport/policy boundary
   - `viewer_net`: expose fetch method(s)
   - `viewer_grid`: semantic selection/meaning for capability usage.
4. Integrate app orchestration
   - route eligible requests through live path when capability is available
   - retain fixture fallback path for deterministic offline mode.
5. Preserve renderer fallback behavior
   - ensure no semantic change to `Loading`/`Missing` color/fallback conventions.
6. Add diagnostics
   - source counts and bounded failure reasons.
7. Add tests
   - source selection determinism
   - live->cache admission correctness
   - fallback behavior when live source unavailable or failing
   - boundary tests for net/grid ownership split.
8. Runtime verification
   - offline fixture-only baseline
   - bounded live run when credentials/caps exist.
9. Closeout docs/review/report.

## Validation plan
- Always:
  - `cargo fmt --all -- --check`
  - `cargo check --workspace`
- Targeted:
  - `cargo test -p viewer_asset`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_grid`
  - `cargo test -p viewer_app`
  - `cargo test -p viewer_render`
  - `cargo test -p viewer_ui` (if touched)
- Broader:
  - `cargo test --workspace`
- Runtime smoke:
  - Offline baseline:
    - `VIEWER_APP_LIVE_STARTUP=off VIEWER_FIXTURE_TEXTURES=1 cargo run -p viewer_app`
  - Live bounded:
    - `cargo run -p viewer_app` (with `VIEWER_LOGIN_*` configured)

## Risks and open questions
- Risk: source-selection ambiguity may leak policy into app layer unless abstractions are tight.
- Risk: live fetch latency could cause cache churn if admission/retry policy is not bounded.
- Risk: transport failures can overwhelm diagnostics without bounded aggregation.
- Open question: first-class live scope should start with texture only, or include bounded mesh in same milestone.
- Open question: whether per-source budget partitioning is needed in A13 or deferred.

## Deferred-too-early candidates captured
- Full multi-class live streaming parity (all textures/meshes/material variants).
- Persistent warm cache with disk invalidation/versioning policy.
- Predictive multi-hop prefetch with movement modeling.

## Learnings pre-check
- `L06`: enforce transport vs semantic split (`viewer_net` vs `viewer_grid`).
- `L07`: never expose sensitive capability/session data in snapshot/contracts.
- `L14`: explicit typing in glTF/mesh decode paths to avoid inference failures.
- `L16`: preserve renderer fallback semantics for empty/loading/missing textures.
- `L19`: deterministic eviction/tie-breaking must remain intact under live ingestion.
- `L04`: preserve dirty-only apply semantics in app path.

## Completion criteria
- A bounded live source path feeds selected scene assets into `viewer_asset`.
- A10 cache policy remains enforced for live-fed entries.
- Renderer fallback semantics remain deterministic.
- Source/failure diagnostics are visible and bounded.
- Validation commands pass (or blockers are documented).
- Continuity artifacts updated with current state and next step.

