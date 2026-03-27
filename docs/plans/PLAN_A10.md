# Plan: A10 Asset Streaming Continuity and Cache Discipline

## Summary
Establish a bounded continuity-aware asset policy so texture/mesh/material assets needed during region handoffs remain stable across short transition windows while cache growth stays deterministic and capped.

## Objective
Implement continuity-focused asset retention/promotion/eviction policy in `viewer_asset`, with typed priority inputs from `viewer_app` and deterministic fallback behavior in `viewer_render`, so visual continuity is preserved during bounded handoff scenarios without unbounded residency.

## Why now
- Roadmap ordering places `A10` after continuity and avatar workflow milestones (`N07`/`R08`/`U09`).
- The viewer now has bounded continuity and richer avatar/attachment surfaces; asset behavior across transition windows is the next primary stability risk.
- Current fixture-driven upload and fallback behavior is deterministic, but continuity-aware retention and prioritization policy is not yet formalized as a first-class contract.

## Scope
- Define and implement bounded continuity-window cache policy in `viewer_asset`:
  - retain active-region critical assets for a short handoff window
  - promote near-neighbor continuity assets into a bounded priority queue
  - evict expired continuity-window assets deterministically under budget pressure
- Add typed continuity-aware request priority inputs from `viewer_app` to `viewer_asset`:
  - active region scope
  - previous region scope
  - bounded neighbor scope
- Preserve deterministic fallback behavior in `viewer_render` for loading/missing continuity assets.
- Add cache metrics and diagnostics fields needed to verify continuity policy behavior (retained/promoted/evicted counts, budget pressure signals).
- Lock decision-complete A10 defaults:
  - `A10_CONTINUITY_WINDOW_MS = 45_000`
  - `A10_MAX_NEIGHBOR_SCOPES = 8` (aligned with N07 bounded continuity neighbors)
  - `A10_REQUESTS_PER_TICK_CAP = 64`
  - `A10_PROMOTIONS_PER_TICK_CAP = 24`
  - scope quotas per continuity window:
    - active scope: up to `96` assets
    - previous scope: up to `48` assets
    - neighbor scope (combined): up to `64` assets
    - hard continuity cap total: `160` assets

## Out of scope
- Unbounded background prefetch.
- Global long-lived residency policies.
- Full CDN/capability parity for every asset class.
- Broad redesign of material/shader model beyond continuity policy needs.

## Current known state
- `viewer_asset` provides deterministic fixture cache behavior and bounded cache budget knobs.
- `viewer_app` already drives deterministic texture requests in bounded modes.
- `viewer_render` has deterministic fallback behavior (`loading`, `missing`, `white`) for texture availability states.
- `viewer_core` and `viewer_app` now carry continuity signals (`active/previous/neighbors`) from N07.
- Deferred scope list already identifies unbounded streaming and global retention as too early; A10 must remain bounded.

## Files and components touched
- `crates/viewer_asset/src/lib.rs`
  - continuity-window cache policy types and enforcement
  - priority request queueing and deterministic eviction behavior
  - cache metrics counters
- `crates/viewer_asset/src/texture_fixture.rs` (and related cache/provider internals as needed)
  - continuity-aware lookup/pin/expire behavior for fixture-backed and provider-backed paths
- `crates/viewer_app/src/main.rs`
  - typed asset-priority request mapping from continuity state (active/previous/neighbor)
  - orchestration wiring only (no asset policy ownership migration)
- `crates/viewer_core/src/lib.rs` (if needed)
  - additive typed continuity asset-priority hints shared across app/asset boundaries
- `crates/viewer_render/src/lib.rs` (bounded)
  - maintain deterministic fallback selection while exposing continuity diagnostics where needed
- `docs/TESTING_REFERENCE.md`
  - update any new continuity verification knobs or policy env vars (if introduced)

## Boundary check
- `viewer_asset` owns fetch/decode/cache/retention/eviction policy.
- `viewer_render` consumes ready assets and fallback states; it does not own asset lifecycle policy.
- `viewer_app` coordinates policy inputs and request dispatch only.
- `viewer_core` owns shared typed contracts but not cache mechanics.
- `viewer_net`/`viewer_grid` provide continuity context signals only; no asset policy implementation should move there.

## Step sequence
1. Define typed continuity asset-priority contract and policy constants listed in this plan (`A10_*` defaults above).
2. Implement continuity-window retention/promotion/eviction policy in `viewer_asset` with deterministic ordering:
   - priority rank (highest to lowest): active -> previous -> neighbor -> non-continuity
   - first evict expired window entries
   - then evict by lowest priority rank
   - tie-break by oldest `last_touched_tick`
   - final tie-break by stable `AssetID` ascending
3. Wire continuity-priority request inputs from `viewer_app` using N07 continuity state (active/previous/neighbors), bounded by explicit caps.
4. Add required continuity cache metrics surfaced for diagnostics and verification:
   - `continuity_retained_active`
   - `continuity_retained_previous`
   - `continuity_retained_neighbor`
   - `continuity_promoted_neighbor`
   - `continuity_evicted_expired`
   - `continuity_evicted_budget`
   - `continuity_requests_enqueued`
   - `continuity_requests_dropped_cap`
   - `continuity_budget_pressure_events`
5. Validate fallback continuity behavior in `viewer_render` remains deterministic during loading/missing transitions.
6. Add targeted tests for policy behavior and determinism.
7. Run runtime continuity smoke and deterministic screenshot checks.
8. Update continuity docs/review/report artifacts after implementation.

## Validation plan
- Always:
  - `cargo fmt --all -- --check`
  - `cargo check --workspace`
- Targeted:
  - `cargo test -p viewer_asset`
  - `cargo test -p viewer_app`
  - `cargo test -p viewer_render`
  - `cargo test -p viewer_core` (if shared contract fields are added)
  - targeted deterministic policy tests to add and run:
    - `cargo test -p viewer_asset continuity_window_policy_enforces_scope_quotas`
    - `cargo test -p viewer_asset continuity_eviction_order_is_deterministic`
    - `cargo test -p viewer_app continuity_priority_mapping_respects_active_previous_neighbor_caps`
- Broader:
  - `cargo test --workspace` when cross-crate behavior changes materially
- Runtime smoke:
  - `VIEWER_APP_LIVE_STARTUP=off VIEWER_FIXTURE_TEXTURES=1 STRESS_TEST=screenshot VIEWER_TEST_SCREENSHOT_DIR=artifacts/screenshots_a10_smoke VIEWER_TEST_SCREENSHOT_EVERY_N_FRAMES=1 VIEWER_TEST_SCREENSHOT_MAX_FRAMES=2 cargo run -p viewer_app`
  - `cargo test -p viewer_asset continuity_window_policy_enforces_scope_quotas -- --exact`
  - `cargo test -p viewer_asset continuity_eviction_order_is_deterministic -- --exact`

## Risks and open questions
- Risk: continuity retention windows can accidentally pin too much if caps are underspecified; hard caps and deterministic eviction are mandatory.
- Risk: request-priority breadth can blur crate boundaries if policy logic leaks into app/render.
- Open question: whether continuity metrics should be surfaced in existing diagnostics panel or a dedicated asset continuity subsection.

## Deferred-too-early candidates captured
- Predictive multi-hop region prefetch beyond active/previous/neighbor bounded scope.
- Cross-session persistent warm cache policy with long-lived residency.
- Full asset-class parity (all mesh/texture variants via live capability/CDN transport) beyond bounded A10 verification scope.

## Learnings pre-check
- `L03`: Keep seam-owned lifecycle boundaries intact while consuming continuity signals for asset priority.
- `L04`: Preserve dirty-only apply behavior in `viewer_app` while adding continuity-priority request mapping.
- `L06`: Maintain `viewer_net`/`viewer_grid` semantic boundary; asset policy belongs in `viewer_asset`.
- `L08`: Avoid per-frame GPU resource churn while validating fallback behavior.
- `L09`: Do not add lazy per-frame pipeline construction for continuity checks.
- `L14`: If A10 touches glTF/mesh-reader integration paths, include explicit type annotations to avoid inference failures.
- `L16`: Preserve `white_view` semantics for empty texture IDs and reserve loading/missing for true availability states.
- `L17`: Prefer seam-side derived payload handling when data is app-local rather than worker-owned.

## Completion criteria
- Continuity-critical assets are retained/promoted/evicted via explicit bounded policy with deterministic ordering.
- Asset request priority mapping from active/previous/neighbor continuity scopes is implemented and capped.
- Required continuity metrics listed in this plan are available and verifiable under budget pressure.
- Renderer fallback behavior remains deterministic during continuity-window loading/missing states.
- Validation commands pass (or blockers are explicitly documented), and continuity docs/handoff are updated.
