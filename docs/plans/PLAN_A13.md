# Plan: A13 Live Asset Transport Bridge and Cache Integration Slice

## Status
- Completed on 2026-03-27 after implementation, validation, and continuity updates.

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
  - add source typing:
    - `enum AssetSourceKind { Fixture, Live }`
    - `enum AssetFetchFailureReason { Transport, Decode, Unsupported, MissingCapability, Timeout }`
  - add request/result contracts:
    - `struct AssetFetchRequest { id: AssetID, priority: AssetPriority }`
    - `struct AssetFetchOutcome { status: AssetStatus<Arc<DecodedRgbaImage>>, source: AssetSourceKind, failure: Option<AssetFetchFailureReason> }`
  - add bounded live bridge trait:
    - `trait LiveTextureProvider { fn request_texture(&mut self, id: &AssetID, priority: AssetPriority) -> anyhow::Result<()>; fn poll_texture(&mut self, id: &AssetID) -> anyhow::Result<Option<DecodedRgbaImage>>; }`
  - integrate live ingestion into existing cache admission + A10 retention/eviction flow
  - tests for source selection, failure classification, and deterministic fallback
- `crates/viewer_asset/src/texture_fixture.rs` (or split modules as needed)
  - keep fixture loader as first provider implementation
  - add source/failure counters to cache metrics update points
  - tests verifying metrics increment correctness
- `crates/viewer_net/src/lib.rs`
  - expose transport-only helper for capability asset GET with timeout budget
  - no cache policy, no source selection logic
  - tests for request construction + bounded timeout handling
- `crates/viewer_grid/src/lib.rs` (or adapter modules)
  - add capability semantic chooser:
    - prefer `GetTexture`, fallback `ViewerAsset`, else report unavailable
  - expose typed policy result for app/asset orchestration
  - tests for mapping order and fallback semantics
- `crates/viewer_app/src/main.rs`
  - replace fixture-only tick path with source-aware path:
    - rename/expand `tick_fixture_textures` to `tick_scene_textures`
    - pass `AssetPriorityHint`s from existing continuity mapper
  - add explicit offline guard: when live startup disabled, force fixture source
  - tests for deterministic source choice and fallback behavior
- `crates/viewer_ui/src/lib.rs` (bounded diagnostics)
  - diagnostics lines:
    - live asset requests
    - live asset successes
    - live asset failures by reason
    - fixture fallback count
  - tests for deterministic diagnostics formatting
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
1. **Create typed source and failure contracts (`viewer_asset`)**
   - Add `AssetSourceKind` and `AssetFetchFailureReason`.
   - Keep them diagnostics-facing only; do not expose transport internals or credentials.
2. **Add source-aware request/outcome structs (`viewer_asset`)**
   - Introduce `AssetFetchRequest` and `AssetFetchOutcome`.
   - Ensure `AssetFetchOutcome.status` remains `AssetStatus` so renderer behavior stays unchanged.
3. **Implement live provider boundary (`viewer_asset`)**
   - Add `LiveTextureProvider` trait.
   - Keep provider strictly async-poll style compatible with current frame tick loop (no blocking calls in render/update loop).
4. **Keep fixture provider as deterministic fallback (`viewer_asset`)**
   - Adapt existing fixture cache path to implement/compose with new source contracts.
   - Preserve existing A10 queue limits and eviction invariants.
5. **Transport hook (`viewer_net`)**
   - Add one bounded fetch helper for raw asset bytes through capability URL.
   - No decode, no retry policy beyond simple bounded transport retry/timeout.
6. **Capability semantics (`viewer_grid`)**
   - Add helper that maps available caps to fetch strategy:
     - `GetTexture` first
     - `ViewerAsset` second
     - unavailable state if none
   - Return typed semantic result; do not call transport directly from `viewer_grid`.
7. **App orchestration swap (`viewer_app`)**
   - Replace fixture-only callsite with source-aware `tick_scene_textures`.
   - Keep existing `build_asset_priority_hints(...)` output as the only priority input.
   - Rule set:
     - offline/no caps => fixture only
     - caps available => try live first, fallback fixture on failure/unavailable
     - renderer sees only `AssetStatus` behavior as today
8. **Metrics + diagnostics integration (`viewer_asset` + `viewer_ui`)**
   - Add counters for:
     - live enqueued
     - live ready
     - live failed transport/decode/timeout
     - fixture fallback used
   - Surface counters in Diagnostics panel using existing metrics style.
9. **Unit/integration tests**
   - `viewer_asset`: source selection deterministic across repeated frames.
   - `viewer_asset`: live failure transitions to fixture fallback without permanent poison.
   - `viewer_grid`: capability mapping preference order.
   - `viewer_app`: offline mode never attempts live provider.
   - `viewer_ui`: diagnostics lines include new counters.
10. **Runtime verification matrix**
   - Offline run: confirm only fixture counters move.
   - Live run: confirm live counters move and fallback remains bounded on failures.
   - Screenshot smoke: confirm no regression to Loading/Missing color semantics.
11. **Documentation and continuity**
   - Update testing reference with any new env knobs:
     - `VIEWER_ASSET_SOURCE_MODE=fixture|auto|live`
     - `VIEWER_ASSET_LIVE_TIMEOUT_MS=<u64>`
   - Record final ownership boundaries in report/handoff.

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
