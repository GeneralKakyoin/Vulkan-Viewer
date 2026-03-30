# Current State: Vulkan-Viewer

## Overview
The Vulkan-Viewer is a high-performance Second Life compatible viewer built in Rust. It currently supports basic region and avatar presence, nearby chat, direct IM, avatar profiles, and a robust diagnostics shell.

## Latest Notable Changes (Object Ingress PCAP Forensics)
- **Preserved runtime evidence added**: the Firestorm and app packet captures are now preserved under `artifacts/pcaps/` with a manifest and stable sha256 hashes.
- **Firestorm pre-burst sequence externally confirmed**: the preserved Firestorm capture on `16.144.39.130:13001` shows `AgentUpdate`, `AgentAnimation`, `SetAlwaysRun`, `PacketAck`, `MuteListRequest`, `MoneyBalanceRequest`, and `AgentDataUpdateRequest` immediately before the first `ObjectUpdateCached` burst.
- **Blocked app path refined by capture evidence**: the preserved app capture reaches the same simulator endpoint but shows no object burst on its handshake/control flow, and it also contains a second local UDP port receiving simulator traffic during the capture window.
- **Current state**: the next bounded slice should verify runtime local-port/socket continuity in practice before adding more Firestorm pre-burst message parity.
- **Planning status**:
  - research note: `docs/RESEARCH/OBJECT_INGRESS_PCAP_FORENSICS_2026-03-30.md`
  - next plan: `docs/plans/PLAN_OBJECT_INGRESS_RUNTIME_SOCKET_FORENSICS_2026-03-30.md`
  - plan review: `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_RUNTIME_SOCKET_FORENSICS_2026-03-30.md`

## Latest Notable Changes (Object Ingress ACK-Trailer Parity)
- **Firestorm-style ACK-trailer transport parity added**: `viewer_net` now parses first-simulator flags/packet IDs/body bounds, queues reliable inbound packet IDs for ACK, and appends bounded ACK trailers onto outbound first-simulator datagrams.
- **Inbound decode is trailer-safe**: first-simulator message decoders now exclude appended ACK trailers from body slices, preventing trailer bytes from corrupting `HealthMessage`, `RegionHandshake`, object-update, and social decode paths.
- **Targeted transport coverage expanded**:
  - ACK-trailer parsing/body-slice test
  - reliable inbound ACK-queue test
  - outbound ACK-trailer send-and-drain test
  - trailer-safe health decode test
- **Connected result remained unchanged**: a bounded `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app` run after the transport change still reported `update_messages=0 total_objects=0 handshake_complete=true traffic_obs=7 region_handshake_updates=0` with the same startup kinds `AgentDataUpdate`, `AgentMovementComplete`, `HealthMessage`, `OnlineNotification`, `PacketAck`, `TestMessage`, and `ViewerEffect`.
- **Current state**: retained socket continuity, startup receive activation, single-social-socket discipline, startup interest sends, recurring `AgentUpdate` cadence, and ACK-trailer parity are all now in place, but object ingress is still blocked. The next likely blocker is a different post-`AgentMovementComplete` control/protocol parity gap.
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_app`
  - connected run: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`

## Latest Notable Changes (Object Ingress AgentUpdate Cadence)
- **Reusable active-circuit `AgentUpdate` helper added**: `viewer_net` now exposes a dedicated `send_agent_update_on_circuit(...)` helper instead of keeping `AgentUpdate` trapped inside startup-only interest sends.
- **Live worker keepalive cadence added**: `viewer_app` now sends bounded recurring non-reliable `AgentUpdate` keepalives on the active retained `SocialCircuit` and re-arms that cadence after social-circuit reopen.
- **Targeted regression coverage added**:
  - `viewer_net` test for non-reliable `AgentUpdate` keepalive send behavior
  - `viewer_app` tests for deterministic keepalive interval/scheduling
- **Connected result remained unchanged**: the bounded live capture in `artifacts/logs/live_agent_update_cadence_2026-03-29_231942.log` still reports `update_messages=0 total_objects=0 handshake_complete=true traffic_obs=7 region_handshake_updates=0` with the same startup kinds `AgentDataUpdate`, `AgentMovementComplete`, `HealthMessage`, `OnlineNotification`, `PacketAck`, `TestMessage`, and `ViewerEffect`.
- **Current state**: retained socket continuity, startup receive activation, single-social-socket discipline, startup parity sends, and recurring `AgentUpdate` cadence are all in place, but none of them have yet restored `RegionHandshake` or `ObjectUpdate*` ingress. The next likely blocker is protocol-accurate first-simulator control/reliability behavior rather than more socket or `AgentUpdate` cadence tuning.
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_app`
  - connected live capture: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app` (bounded 60s capture to `artifacts/logs/live_agent_update_cadence_2026-03-29_231942.log`)

## Latest Notable Changes (Live Texture Capability URL Parity)
- **Firestorm-style texture URL ordering restored**: `viewer_grid::AssetCapabilityPolicy` now tries `/?texture_id=...` first for both `GetTexture` and `ViewerAsset`, then bounded legacy fallbacks.
- **Shared texture fetch helper added**: live scene textures and profile images now reuse the same `viewer_net` HTTP candidate-fetch path with image-oriented `Accept` header handling.
- **Live worker texture requests hardened**: `viewer_app` no longer relies on one exact capability URL shape for scene textures.
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_grid -p viewer_net -p viewer_app`
  - `cargo test -p viewer_grid -p viewer_net -p viewer_app`
- **Current state**: targeted validation passes; connected proof of a real live scene texture is still pending, and the provided `C:\\Users\\matti\\Desktop\\fire.pcapng` did not include transfer-level texture payloads.

## Latest Notable Changes (Startup Protocol And Social Socket Discipline)
- **Startup protocol parity added**: the live worker now sends bounded `RegionHandshakeReply`, `AgentThrottle`, and one-shot `AgentUpdate` messages on the active social circuit, and `RegionHandshake` decode now handles zero-coded payloads.
- **Single-social-socket discipline restored**: nearby-chat polling/sending in the live worker now reuses the active `SocialCircuit` instead of re-handshaking fresh UDP sockets in the steady-state loop.
- **Startup packet mix is now explicit**: the startup relay summary includes the exact first-simulator receive kinds observed on connected runs.
- **Current best live evidence remains blocked**: the best non-regressed connected capture still reports `update_messages=0 total_objects=0 region_handshake_updates=0` with startup kinds `AgentDataUpdate`, `AgentMovementComplete`, `HealthMessage`, `OnlineNotification`, `PacketAck`, `TestMessage`, and `ViewerEffect` in `artifacts/logs/live_single_social_socket_2026-03-29_222138.log`.
- **Reliability-reply experiment reverted**: a bounded attempt to add LLUDP ack/ping replies regressed startup handshake completion and was not kept.
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_app`
  - connected captures:
    - `artifacts/logs/live_startup_protocol_parity_2026-03-29_221717.log`
    - `artifacts/logs/live_single_social_socket_2026-03-29_222138.log`
    - reverted reliability experiment logs:
      - `artifacts/logs/live_lludp_reliability_2026-03-29_222955.log`
      - `artifacts/logs/live_lludp_reliability_2026-03-29_223058.log`
## Latest Notable Changes (Live Object Feed Unblock Verification)
- **`viewer_app` compile compatibility restored**: updated the object-feed snapshot bridge to populate the new `DecodedWorldObjectFeedObject.texture_id` field with the current transport-side `None` placeholder.
- **Live verification relay restored**: re-added bounded `object_feed` startup/tick relay lines in `viewer_app` so connected runs again expose object-feed counters during worker startup.
- **Connected verification result captured**: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app` now reaches `handshake_complete=true` with `traffic_obs=7` on the retained-socket path, but `update_messages=0` and `total_objects=0` remain unchanged in `artifacts/logs/live_socket_continuity_verify_2026-03-29_214711.log`.
- **Current blocker narrowed**: socket continuity is now validated as working, but it is not sufficient by itself to unblock world-object ingress; `RegionHandshake` also remains absent in the captured startup summary (`region_handshake_updates=0`).
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_app`
  - connected live capture: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app` (bounded 60s capture to `artifacts/logs/live_socket_continuity_verify_2026-03-29_214711.log`)

## Latest Notable Changes (First-Simulator Socket Continuity Fix)
- **Probe socket handoff added**: `viewer_net::Connection` now retains the successful first-simulator probe socket and hands it to the first `open_social_circuit()` call instead of binding a second UDP port.
- **Duplicate startup handshake avoided**: the reused-socket path skips redundant `UseCircuitCode` / `CompleteAgentMovement` sends, preserving the simulator address association established during the probe window.
- **Transport regression coverage expanded**: added `viewer_net` tests for retained-socket reuse and fresh-socket fallback behavior.
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net`
  - `cargo test -p viewer_net`
  - `cargo check -p viewer_net -p viewer_app` currently fails in `viewer_app` due an unrelated missing `texture_id` field in `DecodedWorldObjectFeedObject` initialization.
- **Current state**: the transport-side socket continuity fix is implemented and validated in `viewer_net`; broader app/live validation remains blocked until the unrelated `viewer_app` compile error is resolved.

## Latest Notable Changes (N15 Completed)
- **Recovery probe command path completed**: `Retry Continuity Probe` now executes through the live worker command lane instead of remaining deferred/unavailable.
- **Bounded guard semantics enforced**: retry behavior now applies both cooldown and explicit single-flight protection for in-flight probe requests.
- **Operator status messaging improved**: diagnostics now show explicit recovery action status text (`accepted`, `cooldown`, `completed`, `unavailable`) for probe and asset actions.
- **Deferred promotion synced**: the U14 deferred probe-command candidate is now marked `promoted` in `docs/plans/DEFERRED_FEATURES.md`.
- **Validation**:
  - `cargo fmt --all`
  - `cargo check --workspace`
  - `cargo test -p viewer_core -p viewer_ui -p viewer_app -p viewer_net`
  - `cargo test --workspace`
  - `VIEWER_APP_LIVE_STARTUP=off STRESS_TEST=screenshot VIEWER_TEST_SCREENSHOT_DIR=artifacts/screenshots_n15_smoke VIEWER_TEST_SCREENSHOT_EVERY_N_FRAMES=1 VIEWER_TEST_SCREENSHOT_MAX_FRAMES=1 cargo run -p viewer_app`
  - manual screenshot review: `artifacts/screenshots_n15_smoke/viewer_test_0001.png`

## Latest Notable Changes (Workspace Parity Repair N11-R16)
- **Cross-crate parity restored**: repaired API drift between `viewer_app` and sibling crates (`viewer_core`, `viewer_grid`, `viewer_net`, `viewer_render`, `viewer_ui`, `viewer_asset`) so the workspace builds again.
- **Recovery + probe path reconnected**: added/verified recovery contracts, continuity probe execution entrypoint, and cache failure-reset hook used by operator recovery controls.
- **Transition cue wiring restored**: transition cue contracts and render uniform cue params are now aligned with app/UI wiring, including diagnostics display.
- **Validation**:
  - `cargo fmt --all`
  - `cargo check --workspace`
  - `cargo test -p viewer_core -p viewer_grid -p viewer_net -p viewer_render -p viewer_ui -p viewer_asset -p viewer_app`
  - offline screenshot smoke under `artifacts/screenshots_parity_repair_smoke/`
- **Current state**: app startup and rendering smoke run passes again on this branch; feature surfaces for `N11`, `R12`, `A13`, `U14`, and `R16` are present in code and validated via targeted tests.

## Latest Notable Changes (R16 Fix Pass)
- **Recovering Cue Recency Fix**: `viewer_app::derive_transition_visual_cue(...)` now requires recent probe success before applying `Recovering`, preventing stale-success misclassification during later degraded windows.
- **Cue Mapping Regression Coverage**: Added stale-probe cue tests in `viewer_app` to lock degraded/healthy fallback behavior when probe success is old.
- **Expanded R16 Visual Evidence**: Added degraded and stalled screenshot captures under `artifacts/screenshots_r16_smoke/` and updated R16 completion/review artifacts.

## Latest Notable Changes (R16 Completed)
- **Transition Visual Cue Contract**: Added `TransitionVisualCue` enum and `TransitionVisualState` struct to `viewer_core` with sanitized-intensity baseline and 4 unit tests.
- **Renderer Cue Integration**: Extended `EnvironmentUniform` with `cue_params` vec4 (112→128 bytes), added `cue_tint_color()` WGSL helper and bounded fragment tinting (≤18% max blend) in `viewer_render`. No new pipelines, bind groups, or shader files.
- **Deterministic Mapping**: Added `derive_transition_visual_cue()` in `viewer_app` mapping `HandoffOutcome` + `last_probe_result` → `TransitionVisualState` with per-frame dirty-only update.
- **UI Cue Diagnostics**: Added read-only "Render Cue" status line (colored label + intensity) to the Environment diagnostics panel in `viewer_ui`.
- **R16 Validation**: `cargo fmt`, `cargo check --workspace`, `cargo test --workspace`, and screenshot smoke test all passed. Stable scene confirmed with no tint regression on healthy baseline.

## Latest Notable Changes (U14 Completed)
- **Bounded Recovery Controls**: Added `RecoveryAction`, `RecoveryResultCode`, and `RecoveryActionResult` to `viewer_core` for deterministic mapping of recovery operations. 
- **Deterministic Action Debounce**: Implemented `compute_recovery_action` and `dispatch_recovery_action` over `last_probe_retry_ms` and `last_asset_refresh_ms` using standard boundaries in `viewer_app` (15s probe, 5s asset refresh).
- **Explicit Visibility Statuses**: `viewer_ui` now reports and warns on failed assets (`failed_transport/decode/timeout`) mapping live bridge diagnostic failures into actionable operator statuses.
- **Fixture Rejection Flushing**: Added `clear_failures` to `FixtureTextureCache` allowing active caches to immediately re-try assets after a failed fetch. 
- **Continuity Probe Deferral**: The "Retry Continuity Probe" pathway is explicitly deferred to N11 (returns `Unavailable` and disabled in UI) until network probe features are wired.
- **Validation passing**: `cargo fmt --all`, `cargo check --workspace`, `cargo test --workspace` all run optimally without impacting boundaries. 

## Latest Notable Changes (A13 Completed)
- **Typed Live Fetch Contracts**: Added `AssetFetchRequest` and wired `LiveTextureProvider` to return `AssetFetchOutcome<DecodedRgbaImage>` so live status/failure semantics stay explicit through the cache boundary.
- **Source Mode Controls**: Added `VIEWER_ASSET_SOURCE_MODE=fixture|auto|live` handling in `viewer_app` startup wiring to control live-provider attachment without crossing crate boundaries.
- **Live Timeout Control**: Added `VIEWER_ASSET_LIVE_TIMEOUT_MS` (bounded) and worker wiring so texture fetch timeout policy is configurable and deterministic.
- **Failure Classification Path**: Added typed failure propagation from worker (`TextureAssetFailed`) through app/provider/cache to metrics (`transport`, `decode`, `timeout`, `other`) and bounded fallback accounting.
- **A13 Orchestration Path**: Renamed app texture tick lane to `tick_scene_textures(...)` and retained continuity-priority-driven request flow.
- **A13 Validation**: Confirmed with `cargo fmt --all`, targeted A13 crate tests, `cargo check --workspace`, `cargo test --workspace`, and offline screenshot smoke (`VIEWER_APP_LIVE_STARTUP=off VIEWER_FIXTURE_TEXTURES=1 STRESS_TEST=screenshot ... cargo run -p viewer_app`).

## Latest Notable Changes (R12 Gap Fix)
- **Environment Contract Hardening**: Extended `viewer_core::EnvironmentState` with additive, serde-defaulted controls (`time_of_day_normalized`, `sky_enabled`, `fog_enabled`) and added `EnvironmentState::sanitized()` clamping.
- **Fog Correctness Fix**: Updated `viewer_render` fog depth source to use camera-to-fragment world-space distance instead of `clip_position.w`.
- **Fog Density Activation**: Wired `fog.density` into the shader fog factor so the field is no longer inert.
- **Sky Baseline Improvement**: Added bounded sky-top/sky-bottom influence to both clear-color derivation and fragment tinting so both sky colors are used.
- **App Mapping Path**: Added `derive_environment_from_snapshot(...)` in `viewer_app` and per-frame environment refresh from live snapshot continuity state.
- **Diagnostics Expansion**: `viewer_ui` environment panel now shows time-of-day and sky/fog enabled flags plus sky top/bottom and fog values.
- **R12 Gap-Fix Validation**: Verified with `cargo fmt --all`, `cargo check -p viewer_core -p viewer_render -p viewer_app -p viewer_ui`, `cargo test -p viewer_core -p viewer_render -p viewer_app -p viewer_ui`, and screenshot smoke (`VIEWER_APP_LIVE_STARTUP=off STRESS_TEST=screenshot ... cargo run -p viewer_app`).

## Latest Notable Changes (R12)
- **Environment Rendering Baseline**: Introduced `EnvironmentState` (ambient, sky, fog) to `viewer_core` and `viewer_render` to improve scene readability and visual continuity.
- **GPU Environment Uniforms**: Added `EnvironmentUniform` (Group 3, Binding 0) to `SCENE_SHADER` to drive ambient modulation and linear fog parameters on the GPU.
- **Deterministic Atmospheric clear**: Updated `RenderBackend` to clear the color attachment using the `sky_bottom_color`, ensuring a stable horizon background.
- **Linear Fog Integration**: Implemented distance-based linear fog in the fragment shader, using `clip_position.w` as a depth proxy for R12.
- **Environment Diagnostics**: Integrated environment parameter visualization (ambient, sky, fog) into the `viewer_ui` diagnostics panel.
- **R12 Validation**: Verified with `cargo fmt`, `cargo check --workspace`, and full test passes for `viewer_core` and `viewer_render`.

## Latest Notable Changes (Clippy Workspace Cleanup)
- **Workspace Clippy Cleanup**: Resolved warning classes across `viewer_ui`, `viewer_app`, `viewer_core`, `viewer_asset`, `viewer_render`, `viewer_grid`, and `viewer_net` so `cargo clippy --workspace --all-targets -- -D warnings` now passes.
- **UI Render API Hardening**: Replaced `UiSystem::render`’s long argument list with `RenderInput` to remove argument-count lint pressure and reduce callsite fragility.
- **Deterministic Idiomatic Pass**: Applied lint-safe refactors (`collapsible_if`, `manual_is_multiple_of`, `field_reassign_with_default`, `new_without_default`, `redundant_closure`, `manual_ignore_case_cmp`, etc.) without changing behavior.
- **Validation**: Confirmed `cargo fmt --all`, `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` pass. `cargo run -p viewer_app` was started successfully and timed out after launch during smoke attempt.

## Latest Notable Changes (Deferred Promotions)
- **Screenshot Diff Harness**: Added `viewer_app` binary `screenshot_diff` for baseline-vs-candidate PNG comparison with configurable mean-absolute-error threshold.
- **Camera Waypoint Scripts**: Added `VIEWER_TEST_CAMERA_PATH_FILE` JSON waypoint support for deterministic scripted camera paths in `STRESS_TEST=camera|screenshot` modes.
- **Deferred Sync**: Marked the corresponding deferred entries as `promoted` in `docs/plans/DEFERRED_FEATURES.md`.
- **Verification**: Confirmed `cargo fmt --all`, `cargo check --workspace`, and `cargo test -p viewer_app` pass with new script + harness paths.

## Latest Notable Changes (A10)
- **Continuity-Aware Asset Streaming**: Implemented `AssetPriority` (Active, Previous, Neighbor, Normal) and priority-based cache eviction in `viewer_asset`.
- **Deterministic Cache Discipline**: Refactored `FixtureTextureCache` to use metadata-based priority queuing with deterministic tie-breaking (`last_touched_tick`, `AssetID`).
- **Cache Metrics & UI**: Added `CacheMetrics` tracking (budget, pressure, counts) and integrated it into the `viewer_ui` Diagnostics panel.
- **Visibility-Driven Priority**: `viewer_app` now automatically promotes visible scene textures to `Active` priority during the streaming tick.
- **A10 Validation**: Verified with workspace-wide test pass, targeted cache eviction unit tests, and compilation of the full Vulkan-Viewer suite.

## Latest Notable Changes (R08)
- **Avatar Appearance Contracts**: Added `AvatarAppearanceSummary` and bounded `AvatarAttachmentProxy` scene-facing contracts in `viewer_core`.
- **Attachment Lifecycle**: Added `WorldAvatarAttachmentProxy` scene roles plus seam-owned attachment proxy lifecycle management in `Scene::apply_world_object_ingestion_seam(...)`.
- **Deterministic Attachment Projection**: `viewer_app` now projects bounded attachment proxies from existing avatar samples and feeds them into the seam path.
- **Attachment Diagnostics**: `viewer_ui` now shows avatar and attachment proxy counts in the diagnostics panel.
- **R08 Coverage**: Added deterministic attachment projection and seam create/remove tests in `viewer_core`; validated app, UI, renderer, and workspace test passes plus offline screenshot smoke.

## Latest Notable Changes (U09)
- **UI Shortcuts (F1-F3)**: Implemented F1 (toggle diagnostics), F2 (focus diagnostics/continuity), and F3 (toggle social) toggles in `viewer_app` event routing.
- **Diagnostics Grouping**: Reorganized the diagnostics panel into "Performance & Metrics" and "Presence & Continuity" collapsing headers in `viewer_ui`.
- **Profile Refresh Cooldown**: Added a 10-second refresh cooldown policy in `viewer_ui` with a countdown timer, backed by `last_refresh_unix_ms` in `viewer_core::AvatarProfileState`.
- **U09 Validation**: Verified with workspace tests, `cargo check`, and borrow-checker fixes for egui window state updates.

## Latest Notable Changes (N07)
- **Bounded Region Continuity Model**: Added typed continuity state (`None`, `Crossed`, `Confirming`, `Completed`) in `viewer_net` and propagated it through `viewer_app` into `LiveVisualSnapshot`.
- **Continuity Snapshot Contract**: Extended `viewer_core::LiveVisualSnapshot` with `RegionContinuitySummary` (active/previous region coords + bounded neighbors, serde-defaulted).
- **Continuity Seam Lane**: Added `WorldObjectIngestionLane::ContinuityPayload` and seam->scene lifecycle wiring so continuity visualization remains seam-owned.
- **Continuity Diagnostics UI**: Added read-only continuity lines in `viewer_ui` diagnostics (`phase`, active/previous region, neighbor count).
- **N07 Test Coverage**: Added targeted tests for continuity mapping, continuity seam payload emission, and seam-owned continuity role lifecycle removal.

## Latest Notable Changes (R05)
- **R05 Review Fixes**: Resolved vertex layout mismatch in `viewer_render` pipelines and reconciled alpha/capping logic.
- **RGBA Rendering Contract**: Standardized all color handling to RGBA `[f32; 4]` across `viewer_core` and `viewer_render`.
- **AlphaMode Support**: Integrated `Opaque`, `AlphaTest`, and `Blend` modes into the `Scene` and `RenderableInstance` types.
- **Pass Bucketing & Sorting**: Implemented a pass-based rendering system with deterministic front-to-back sorting for opaque/alpha-tested items and back-to-front sorting for transparent items.
- **Stable Draw Item Model**: Created `DrawItem` and `draw_helpers.rs` to ensure consistent frame-to-frame draw submission.
- **Fragment Alpha Discard**: Added alpha-testing logic directly to the GPU shader for performance.

## Latest Notable Changes (A06)
- **Texture & Material Integration**: Fully integrated `MaterialSet` and `TextureAnim` into `RenderableInstance` for per-instance and per-face overrides.
- **Wired UV Matrix Pipeline**: Connected `RenderableInstance::texture_anim` to the `RenderBackend` uniform upload, enabling scrolling and flipbook animations on the GPU.
- **Ergonomic Instance API**: Added `with_texture_anim` builder and automated default initialization to `RenderableInstance`.
- **Verified Animation Logic**: Added comprehensive unit tests for UV matrix math and a visual verification case in the Geometry Torture stress test.
- **Deterministic Texture Fallbacks**: Robust handling of loading (yellow) and missing (magenta) states integrated into the descriptor binding.

## Latest Notable Changes (U04)
- **Unified Session Status**: Standardized UX-facing session states (`disabled`, `starting`, `connected`, `reconnecting`, `failed`) implemented across the codebase.
- **Improved UI Shell**: Reorganized into Session, Social, and Diagnostics panels.
- **Diagnostics Relay Filters**: Added category and level filtering to the integrated Runtime Relay.
- **Enhanced Profile Headers**: Implemented cache freshness labels (`Fresh`/`Stale`) and human-readable age readout.
- **Crate Modernization**: Resolved all `wgpu` deprecation warnings in `viewer_render`.

## Active Milestone
**Finalizing U14 Operator Resilience**
Focus: Wrap U14 workflow resilience and move dynamically forwards (the transition parity).

## System Components
- `viewer_app`: Orchestration and worker state mapping (UI-agnostic).
- `viewer_core`: Shared domain types, session status contract, and deterministic logic.
- `viewer_ui`: Egui-based presentation layer (decoupled from app internals).
- `viewer_render`: Wgpu-driven rendering backend.
- `viewer_net`/`viewer_grid`: Protocol and asset transport layers.

- **Verification Status**: `cargo fmt --all`, `cargo check --workspace`, targeted crate tests, `cargo test --workspace`, and offline screenshot smoke all pass on the current R08 baseline.

