# Current State: Vulkan-Viewer

## Overview
The Vulkan-Viewer is a high-performance Second Life compatible viewer built in Rust. It currently supports basic region and avatar presence, nearby chat, direct IM, avatar profiles, and a robust diagnostics shell.

## Latest Notable Changes (EventQueue LLSD Root Hardening Live Verify) (2026-04-09)
- Ran a bounded live verification capture using:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_APP_LLUDP_STARTUP_PARITY_BUNDLE=on`
  - `VIEWER_APP_CAPABILITY_PROBES_REQUIRE_EVENT_QUEUE_OK=true`
  - `VIEWER_APP_EVENT_QUEUE_CAP_NOT_FOUND_BEFORE_RECONNECT=3`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_event_queue_llsd_root_hardening_verify_2026-04-09.jsonl`
- The capture showed EventQueue and gate progression without `missing llsd map` decode errors:
  - `EventQueueGet:ok ack_in=0 ack_out=1`
  - `probe_gate:open reason=EventQueueGet:ok`
  - readiness line with `EventQueueGet(...ok=1...)`, `InterestList(...ok=1...)`, `UntrustedSimulatorMessage(...ok=1...)`
- Same run showed LLUDP ingress passing in-window:
  - `lludp_object_gate: verdict=PASS ...`
  - object feed grew to `state_total_objects=982` with sustained `ObjectUpdate`/`ObjectUpdateCompressed` traffic.
- Remaining runtime issue in same capture: late EventQueue `404 cap not found` still appears and can trigger bounded reconnect threshold handling.

## Latest Notable Changes (EventQueue LLSD Root Shape Hardening) (2026-04-09)
- Hardened `viewer_net` LLSD XML root handling for capability decoders that previously assumed top-level `<map>`.
- EventQueue LLSD parse paths now accept valid `<llsd><undef /></llsd>` responses as empty-success instead of `missing llsd map` decode failure.
- Added EventQueue LLSD regression coverage for:
  - undef-root payloads
  - array-root payloads containing a map entry
- Applied the same LLSD root-map helper to other map-assuming LLSD decoders in the same module (`GetDisplayNames`, `SimulatorFeatures`) for consistency.
- Validation: `cargo fmt --all`, `cargo check -p viewer_net`, `cargo test -p viewer_net parse_event_queue_poll_from_llsd_xml -- --nocapture`, `cargo test -p viewer_net parse_event_queue_from_llsd_xml -- --nocapture`.

## Latest Notable Changes (Backfill Missing Impl Reviews and Retire Completed Dated Plans) (2026-04-09)
- Backfilled 21 missing REVIEW_IMPL_* artifacts for completed dated tactical plans (report existed, implementation review missing).
- Retired 21 completed dated tactical plans to docs/plans/Retired/ after backfill.
- Included completed object-ingress plans in this pass (17 retired in this step), including PLAN_OBJECT_INGRESS_REGION_OBJECTS_POSITION_SHAPE_CLARIFICATION_2026-03-31.md.
- Validation: cargo fmt --all, cargo check --workspace.
## Latest Notable Changes (Retire Completed Historical Tactical Plans) (2026-04-09)
- Retired 48 completed dated tactical plans from docs/plans/ to docs/plans/Retired/ using completion-evidence gating (matching report + implementation review).
- Included completed object-ingress branches as requested (14 plans).
- Added retirement artifacts:
  - PLAN_RETIRE_COMPLETED_HISTORICAL_TACTICAL_PLANS_2026-04-09.md
  - REVIEW_PLAN_RETIRE_COMPLETED_HISTORICAL_TACTICAL_PLANS_2026-04-09.md
  - REVIEW_IMPL_RETIRE_COMPLETED_HISTORICAL_TACTICAL_PLANS_2026-04-09.md
  - REPORT_RETIRE_COMPLETED_HISTORICAL_TACTICAL_PLANS_2026-04-09.md
- Validation: cargo fmt --all, cargo check --workspace.
## Latest Notable Changes (Retire Completed Wave Plans) (2026-04-09)
- Retired completed Wave helper-extraction plans by moving them from `docs/plans/` to `docs/plans/Retired/`:
  - `PLAN_WAVE1_VIEWER_NET_HELPER_EXTRACTION_2026-04-04.md`
  - `PLAN_WAVE2_WAVE3_VIEWER_APP_CORE_HELPER_EXTRACTION_2026-04-04.md`
  - `PLAN_WAVE4_VIEWER_UI_HELPER_EXTRACTION_2026-04-04.md`
  - `PLAN_WAVE5_VIEWER_APP_RUNTIME_RELAY_HELPERS_2026-04-04.md`
  - `PLAN_WAVE6_VIEWER_APP_MULTI_CLUSTER_EXTRACTION_2026-04-04.md`
  - `PLAN_WAVE7_VIEWER_RENDER_RESOURCE_HELPER_EXTRACTION_2026-04-05.md`
  - `PLAN_WAVE8_VIEWER_ASSET_HELPER_EXTRACTION_2026-04-08.md`
  - `PLAN_WAVE9_VIEWER_GRID_HELPER_EXTRACTION_2026-04-08.md`
  - `PLAN_WAVE8_WAVE9_REVIEW_CONTINUITY_CLOSURE_2026-04-09.md`
- Added retirement task artifacts:
  - `PLAN_RETIRE_COMPLETED_WAVE_PLANS_2026-04-09.md`
  - `REVIEW_PLAN_RETIRE_COMPLETED_WAVE_PLANS_2026-04-09.md`
  - `REVIEW_IMPL_RETIRE_COMPLETED_WAVE_PLANS_2026-04-09.md`
  - `REPORT_RETIRE_COMPLETED_WAVE_PLANS_2026-04-09.md`
- Validation: `cargo fmt --all`, `cargo check --workspace`.
## Latest Notable Changes (Wave 8/9 Continuity Closure) (2026-04-09)
- Completed missing process artifacts for Wave 8 and Wave 9 by adding the absent plan/implementation reviews under `docs/reviews/`.
- Added a bounded closure plan/review/report set for this continuity pass.
- Replaced `docs/HANDOFF.md` with a single latest handoff that reflects closure status and the unchanged functional next step.
- Validation: `cargo fmt --all`, `cargo check --workspace`.
## Latest Notable Changes (Wave 9 Viewer Grid Helper Extraction) (2026-04-08)
- Extracted `viewer_grid` grid login structs and `SecondLifeAdapter` behavior into `grid_login.rs`.
- Extracted asset capability shape mapping and probe metadata into `asset_capability_policy.rs`.
- Updated `crates/viewer_grid/src/lib.rs` architecture, enforcing strict locality for network capability shapes.
- Validation: `cargo fmt --all`, `cargo check --workspace`, and `cargo test -p viewer_grid` (20/20 passed).

## Latest Notable Changes (Wave 8 Viewer Asset Helper Extraction & Lint Sweep) (2026-04-08)
- Extracted `viewer_asset` mesh and texture decoding utilities into dedicated `mesh_decode_utils.rs` and `texture_decode_utils.rs`.
- Updated `crates/viewer_asset/src/lib.rs` boundaries.
- Executed a comprehensive pass to resolve remaining workspace-level `clippy` warnings in `viewer_app` and `viewer_net` ensuring zero-lint code.
- Workspace now strictly adheres to `-D warnings` enforcement.
- Validation: `cargo fmt --all`, `cargo clippy --workspace -- -D warnings`, and `cargo check --workspace`.

## Latest Notable Changes (Wave 7 Viewer Render Resource Helper Extraction) (2026-04-05)
- Continued next-crate modularization in `viewer_render`.
- Added module:
  - `crates/viewer_render/src/render_resource_utils.rs`
- Moved a bounded render utility cluster out of `crates/viewer_render/src/lib.rs`:
  - alignment helper (`align_up`)
  - fallback/env helpers (`avatar_proxy_fallback_forced`, `texture_budget_mb_from_env`, `create_fallback_texture`, `multiply_rgba`, `blend_clear_color_from_environment`)
  - resource helpers (`create_depth_resources`, `create_mesh_buffers`)
- `viewer_render/src/lib.rs` now wires:
  - `mod render_resource_utils;`
  - `use render_resource_utils::*;`
- `viewer_render/src/lib.rs` reduced to ~1,444 lines in this slice.
- No runtime/protocol behavior change intended; this is structural locality/modularity work.
- validation:
  - `cargo fmt --all`
  - `cargo check -p viewer_render`
  - `cargo test -p viewer_render`

## Latest Notable Changes (Wave 6 Viewer App Multi-Cluster Helper Extraction) (2026-04-04)
- Per bounded modularization work, `viewer_app` extracted three cohesive helper clusters from `main.rs`:
  - object-feed diagnostics helpers -> `crates/viewer_app/src/object_feed_diagnostics_utils.rs`
  - first-simulator diagnostics helpers -> `crates/viewer_app/src/first_sim_diagnostics_utils.rs`
  - capability/protocol diagnostics helpers -> `crates/viewer_app/src/capability_diagnostics_utils.rs`
- `crates/viewer_app/src/main.rs` now wires these modules via local `mod` + `use` imports.
- `viewer_app/src/main.rs` reduced from ~10,526 lines to ~8,978 lines in this wave.
- No runtime/protocol behavior change intended; this is structural locality/modularity work.
- validation:
  - `cargo fmt --all`
  - `cargo check -p viewer_app`
  - `cargo test -p viewer_app`

## Latest Notable Changes (Wave 5 Viewer App Runtime Relay Helper Extraction) (2026-04-04)
- Continued helper extraction in `viewer_app` to reduce `main.rs` monolith size.
- Added module:
  - `crates/viewer_app/src/runtime_relay_utils.rs`
- Moved relay/network-debug helper cluster out of `crates/viewer_app/src/main.rs`:
  - `emit_relay`
  - `is_network_debug_category`
  - `network_debug_log_path`
  - `append_network_debug_log`
  - `append_network_debug_line`
- `viewer_app/src/main.rs` now wires:
  - `mod runtime_relay_utils;`
  - `use runtime_relay_utils::*;`
- `viewer_app/src/main.rs` reduced from ~10,608 lines to ~10,526 lines in this slice.
- No runtime/protocol behavior change intended; this is structural locality/modularity work.
- validation:
  - `cargo fmt --all`
  - `cargo check -p viewer_app`
  - `cargo test -p viewer_app`

## Latest Notable Changes (Wave 4 Viewer UI Helper Extraction) (2026-04-04)
- Continued the crate-local helper extraction sequence with `viewer_ui`.
- Added module:
  - `crates/viewer_ui/src/ui_status_helpers.rs`
- Moved a bounded status/helper cluster out of `crates/viewer_ui/src/lib.rs` into the new module:
  - submit-on-enter helper
  - connection/send/session/probe/recovery/handoff labels/chips
  - friend-id filter sorting helper
- `viewer_ui/src/lib.rs` now wires:
  - `mod ui_status_helpers;`
  - `use ui_status_helpers::*;`
- No runtime/protocol behavior change intended; this is structural locality/modularity work.
- validation:
  - `cargo fmt --all`
  - `cargo check -p viewer_ui`
  - `cargo test -p viewer_ui`

## Latest Notable Changes (Wave 2/3 Viewer App + Core Helper Extraction) (2026-04-04)
- Applied the same crate-local helper extraction pattern to the next two major crates after `viewer_net`.
- `viewer_app`:
  - new module: `crates/viewer_app/src/start_location_utils.rs`
  - moved start-location / SLURL normalization helper cluster out of `main.rs`
- `viewer_core`:
  - new module: `crates/viewer_core/src/math_utils.rs`
  - moved math/matrix/quaternion helper cluster out of `lib.rs`
  - re-exported helpers from `lib.rs` to preserve external call paths
- No runtime/protocol behavior change intended; this is structural locality/modularity work.
- validation:
  - `cargo fmt --all`
  - `cargo check -p viewer_app -p viewer_core`
  - `cargo test -p viewer_app parse_start_location_maps_home_last_and_uri -- --nocapture`
  - `cargo test -p viewer_app parse_start_location_normalizes_supported_slurls -- --nocapture`
  - `cargo test -p viewer_core compute_profile_freshness_logic -- --nocapture`
  - `cargo test -p viewer_app`
  - `cargo test -p viewer_core`

## Latest Notable Changes (Wave 1 Viewer Net Helper Extraction) (2026-04-04)
- Continued cross-crate split roadmap execution with a broader bounded Wave 1 `viewer_net` helper extraction slice.
- Added crate-local helper modules:
  - `crates/viewer_net/src/cookie_utils.rs`
  - `crates/viewer_net/src/login_codec_utils.rs`
  - `crates/viewer_net/src/asset_fetch.rs`
  - `crates/viewer_net/src/object_decode_utils.rs`
- `viewer_net/src/lib.rs` now wires these modules and re-exports asset fetch API functions from `asset_fetch`.
- `viewer_net/src/lib.rs` reduced from ~715 KB to ~651 KB in this slice.
- No protocol/runtime behavior change intended; this is organization-only.
- validation:
  - `cargo fmt --all`
  - `cargo check -p viewer_net`
  - `cargo test -p viewer_net merge_cookie_header_prefers_new_values_and_preserves_existing -- --nocapture`
  - `cargo test -p viewer_net llsd_codec_decodes_simple_login_response -- --nocapture`
  - `cargo test -p viewer_net decode_object_update_compressed_extracts_mesh_id_from_extra_params -- --nocapture`
  - `cargo test -p viewer_net fetch_mesh_asset_bytes_reuses_set_cookie_between_probe_and_followup -- --nocapture`
  - `cargo test -p viewer_net`

## Latest Notable Changes (Repo-Wide Crate-Local Editing Policy) (2026-04-04)
- Repository policy now explicitly requires future implementation changes to be made in the owning crate and nearest behavior-focused subfile/module.
- `AGENTS.md` now includes a dedicated "Code placement and locality rule" under scope control.
- `docs/MASTER_PLAN.md`, `docs/ARCHITECTURE.md`, `docs/INTERFACES.md`, and `docs/TASKS.md` now reinforce crate-local/subfile-local placement and planning discipline.
- `docs/agents/CODEX.md` now aligns default writer behavior with crate-local modular placement.
- This change is documentation/process only; no runtime behavior or crate ownership changed.
- validation:
  - `cargo fmt --all`
  - `cargo check`

## Current Position (2026-04-03)
- Face-accurate texture + material parity slice implemented and validated (2026-04-03):
  - `viewer_net` decoded object-feed exports now carry additive face material payload:
    - `default_face_material`
    - `face_material_overrides`
  - `TextureEntry` decode now parses default + per-face exceptions and preserves fail-soft behavior for truncated/malformed payload tails.
  - `viewer_app` now propagates face material payload into core ingestion and expands texture scheduling extraction to include face default/override and normal/specular references.
  - `viewer_core` now prefers decoded face material payload for world object-feed materials, populates `MaterialSet.by_face`, and updates alpha mode from decoded material alpha hints.
  - new diagnostics/checklist signals:
    - object-feed relay line `face_materials: objects_with_default=... objects_with_overrides=... override_faces=...`
    - checklist artifact `artifacts/logs/object_face_checklist_autodiscover.jsonl`
  - validation:
    - `cargo fmt --all`
    - `cargo check -p viewer_net -p viewer_app -p viewer_core -p viewer_render`
    - `cargo test -p viewer_net`
    - `cargo test -p viewer_app`
    - `cargo test -p viewer_core`
    - `cargo test -p viewer_render`
  - bounded live evidence:
    - `artifacts/logs/network_debug_face_material_parity_2026-04-03.jsonl`
    - steady-state sample: `tick face_materials: objects_with_default=89 objects_with_overrides=89 override_faces=577`
  - current diagnosis:
    - face-level texture/material data is now retained and applied end-to-end.
    - remaining appearance gap is full RenderMaterials extension resolution for complete normal/specular/PBR parity.
- World-object-feed true placement + scale slice implemented and validated (2026-04-03):
  - `viewer_core::world_object_feed_proxy_transform(...)` now uses decoded in-region object-feed position in meters instead of compressing into a bounded debug cluster.
  - axis mapping is now explicit for scene `Y-up` convention:
    - position: decoded `X,Y,Z` -> world `X,Z,Y`
    - scale: decoded `X,Y,Z` -> world `X,Z,Y`
  - fallback behavior for missing decoded position remains deterministic ring placement.
  - regression tests added:
    - `scene_world_object_feed_maps_decoded_position_to_in_region_world_space`
    - `scene_world_object_feed_positionless_objects_keep_fallback_ring_transform`
  - validation:
    - `cargo fmt --all`
    - `cargo check -p viewer_core`
    - `cargo test -p viewer_core`
    - `cargo check -p viewer_app`
  - bounded live launch evidence:
    - `artifacts/logs/network_debug_world_object_true_placement_2026-04-03.jsonl`
    - in-window signals remained healthy (`lludp_object_gate: PASS`, object-feed tick summaries present).
  - current diagnosis:
    - placement/scaling semantics are now aligned to decoded object-feed coordinates.
    - remaining visual parity work is downstream (rotation/material/texture and camera/framing ergonomics).
- Live SL mesh header compatibility slice implemented and live-validated (2026-04-03):
  - `viewer_asset` binary-LLSD parsing now accepts Firestorm-compatible notation-style string tokens in binary payloads:
    - quoted value tokens `'` and `"`
    - quoted map keys `'` and `"`
    - binary date token `d`
  - targeted parser regression tests added:
    - `parses_binary_llsd_map_with_quoted_keys_and_values`
    - `parses_binary_llsd_date_token_without_failing_header_walk`
    - `parses_binary_llsd_delimited_string_escapes`
  - validation:
    - `cargo fmt --all`
    - `cargo check -p viewer_asset -p viewer_app`
    - `cargo test -p viewer_asset`
    - `cargo test -p viewer_app`
    - `cargo build -p viewer_app`
  - bounded live verification:
    - stdout: `artifacts/logs/live_sl_mesh_header_fix_2026-04-03.out.log`
    - network debug: `artifacts/logs/network_debug_live_sl_mesh_header_fix_2026-04-03.jsonl`
    - decisive result:
      - `mesh_fetch` continues succeeding with `206` -> `200` -> `ready`
      - live `mesh_asset: decode_failed ... failed to parse SL mesh LLSD header` no longer appears
      - real live assets now decode, for example:
        - `ac27119d-c8fd-2edc-6c28-049f70ec462c` -> `vertices=30566 submeshes=3`
        - `320c4867-7720-1094-1667-a6dab6943476` -> `vertices=307 submeshes=8`
        - `09c1fea9-a268-e57d-12ce-379d00da3a75` -> `vertices=3896 submeshes=3`
  - current diagnosis:
    - the live mesh header parser blocker is cleared.
    - the next remaining work is downstream visual verification and appearance parity, not live SL mesh header compatibility.
- Mesh fetch to visible-object pipeline slice implemented (2026-04-03):
  - `viewer_asset` now detects Second Life mesh assets and decodes binary-LLSD + zlib mesh payloads into `ProcessedMesh`.
  - `viewer_asset::GeometryCache::get_mesh_with_status(...)` now reports:
    - `EmptyData`
    - `CachedReady`
    - `Decoded`
    - `DecodeFailed`
  - `viewer_app` now stages live mesh assets as:
    - `Requested`
    - `Fetched`
    - `Decoded`
    - `Failed`
  - screenshot torture mode now seeds a deterministic synthetic SL mesh asset (`debug-secondlife-mesh`) so decode/upload/render can be verified offline.
  - validation:
    - `cargo fmt --all`
    - `cargo check -p viewer_asset -p viewer_app`
    - `cargo test -p viewer_asset`
    - `cargo test -p viewer_app`
  - bounded offline verification:
    - screenshot captured at `artifacts/screenshots_mesh_visibility_offline_verify_2026-04-03/viewer_test_0001.png`
    - manual review verdict: non-placeholder mesh geometry is visible in the torture scene
  - remaining follow-up from this slice:
    - compact mesh-verification JSONL artifact did not materialize in the bounded offline run
    - full live object material/texture parity remains deferred after geometry-first stabilization
- Failproof mesh-ID retention and export unblock slice implemented (2026-04-03):
  - `viewer_net` now distinguishes transport-side object-feed state from exported scheduler-visible slice:
    - `object_feed_total_objects`
    - `object_feed_export_objects`
    - `object_feed_state_mesh_objects`
    - `object_feed_export_mesh_objects`
    - `object_feed_object_update_mesh_hits`
    - `object_feed_object_update_compressed_mesh_hits`
    - `object_feed_object_extra_params_mesh_hits`
  - object-feed export truncation is now deterministic and mesh-first:
    - `has_mesh_id desc`
    - `last_seen_tick desc`
    - `local_id asc`
  - `viewer_app` relay summaries now show both state and export views explicitly:
    - `state_total_objects`
    - `export_objects`
    - `state_mesh_objects`
    - `export_mesh_objects`
    - `export_truncated`
    - `mesh_hits=ObjectUpdate:...,ObjectUpdateCompressed:...,ObjectExtraParams:...`
  - replay-grade tests added/passing:
    - `decode_real_firestorm_object_update_extracts_mesh_id_from_extra_params`
    - `object_feed_replay_preserves_real_firestorm_mesh_id_across_cached_and_terse_updates`
    - `object_feed_export_prioritizes_mesh_objects_when_truncated`
    - `update_live_visual_from_connection_preserves_transport_total_object_count`
  - validation:
    - `cargo fmt --all`
    - `cargo check -p viewer_net -p viewer_app`
    - `cargo test -p viewer_net`
    - `cargo test -p viewer_app`
    - `cargo build -p viewer_app`
  - bounded live decoded-only verification with rebuilt executable:
    - `artifacts/logs/live_mesh_retention_decoded_only_2026-04-03_rerun.out.log`
    - `artifacts/logs/network_debug_live_mesh_retention_decoded_only_2026-04-03_rerun.jsonl`
    - decisive result:
      - `state_total_objects=674`
      - `export_objects=128`
      - `state_mesh_objects=305`
      - `export_mesh_objects=128`
      - `mesh_id_count=101`
      - `mesh_hits=ObjectUpdate:265,ObjectUpdateCompressed:54,ObjectExtraParams:0`
    - live mesh fetch is now proven active and succeeding in-window:
      - repeated `mesh_fetch: attempt ... status=206`
      - followed by `mesh_fetch: attempt ... status=200`
      - then `mesh_fetch: ready ... attempt=1`
  - current diagnosis:
    - the discovery/export/scheduler blocker is cleared on the current route/window.
    - the earlier April 3 `mesh_id_count=0` diagnosis was produced from a stale executable and is superseded by the rebuilt-binary live run.
- Targeted code cleanup warning-hygiene pass completed (2026-04-03):
  - removed stale unused render constant in `viewer_render`.
  - `viewer_app` now emits startup diagnostics including `event_queue_poll_every_ticks` and `event_queue_poll_timeout_ms`, eliminating unread-field warning without behavior change.
  - validation:
    - `cargo fmt --all`
    - `cargo check -p viewer_render -p viewer_app`
    - `cargo test -p viewer_render -- --nocapture`
    - `cargo test -p viewer_app in_process_live_feed_config_from_lookup -- --nocapture`
- Firestorm fail-soft object decode parity hardening implemented (2026-04-03):
  - `viewer_net` now preserves valid mesh-ID/object decode from LLUDP packets when neighboring entries are malformed/truncated.
  - hardened decode paths:
    - `decode_mesh_asset_id_from_extra_params(...)` now tolerates trailing malformed bytes after valid entries.
    - `decode_object_extra_params_mesh_updates(...)` now skips invalid entries and keeps valid following entries.
    - `decode_object_update_compressed_objects(...)` now keeps valid blocks and ignores malformed/truncated neighbors.
  - regression tests added/passing:
    - `decode_mesh_asset_id_from_extra_params_tolerates_trailing_bytes`
    - `decode_object_update_compressed_tolerates_mixed_valid_and_malformed_blocks`
    - `decode_object_extra_params_tolerates_invalid_entry_and_keeps_valid_following_entry`
  - validation:
    - `cargo fmt --all`
    - `cargo check -p viewer_net`
    - `cargo test -p viewer_net decode_mesh_asset_id_from_extra_params_tolerates_trailing_bytes -- --nocapture`
    - `cargo test -p viewer_net decode_object_update_compressed_tolerates_mixed_valid_and_malformed_blocks -- --nocapture`
    - `cargo test -p viewer_net decode_object_extra_params_tolerates_invalid_entry_and_keeps_valid_following_entry -- --nocapture`
    - `cargo test -p viewer_net`
  - bounded live evidence:
    - `cargo run -p viewer_app` attempted but rebuild blocked by `LNK1104` linker lock on `viewer_app.exe`.
    - direct binary run (`.\target\debug\viewer_app.exe`) with `VIEWER_APP_LIVE_STARTUP=on`, `VIEWER_FIXTURE_MESHES=0`:
      - `artifacts/logs/network_debug_failsoft_parity_live_2026-04-03.jsonl`
      - startup gate `FAIL` then steady-state gate `PASS`, `ObjectUpdate*` observed.
      - object feed progressed (`update_messages` up to `585`, `total_objects=128`), but no in-window `mesh_fetch` relay lines.
- LLUDP mesh discovery + debug declutter slice implemented (2026-04-03):
  - `viewer_app` mesh scheduling now uses decoded object-feed mesh IDs (real LLUDP object updates) plus explicit fixture IDs; default visible-scene/proxy mesh discovery was removed.
  - Network Debug window decluttered:
    - `Session` summary now emphasizes object-ingress and mesh-queue signals.
    - section defaults now open only high-signal groups (`Session`, `LLUDP`, `RegionObjects`).
    - per-section lines are tail-capped; `Recent Network Events` now starts collapsed.
  - validation:
    - `cargo fmt --all`
    - `cargo check -p viewer_app -p viewer_ui -p viewer_net`
    - `cargo test -p viewer_app test_extract_decoded_object_feed_mesh_ids_is_deterministic_and_capped -- --nocapture`
    - `cargo test -p viewer_ui -- --nocapture`
    - screenshot smoke validated via direct binary run:
      - `artifacts/screenshots_mesh_discovery_debug_declutter_2026-04-03/viewer_test_0001.png` (manual visual review: PASS).
    - bounded live decoded-only run (`VIEWER_FIXTURE_MESHES=0`) captured:
      - `artifacts/logs/live_mesh_discovery_decoded_only_2026-04-03.log`
      - `artifacts/logs/network_debug_mesh_discovery_decoded_only_2026-04-03.jsonl`
      - object feed progressed (`update_messages` up to `855`, `total_objects=128`) with no `mesh_fetch` events in-window.
- Startup receive-first + immediate ACK flush parity slice implemented (2026-04-03):
  - `viewer_app` startup prime now performs a bounded prelude social receive drain before startup-interest sends.
  - `viewer_net` social polling now flushes pending ACK IDs immediately after each inbound receive.
  - bounded non-strict Fidelis run now shows object ingress unlock:
    - `after_first_steady_state_window lludp_object_gate: verdict=PASS`
    - `ObjectUpdate` observed in first steady-state receive transcript.
  - artifacts:
    - `artifacts/logs/live_startup_parity_receivefirst_ackflush_2026-04-03_175048.log`
    - `artifacts/logs/network_debug_startup_parity_receivefirst_ackflush_2026-04-03_175048.jsonl`
    - `artifacts/logs/startup_first_divergence_diff_receivefirst_ackflush_2026-04-03_175048.json`
- Fallback `RegionHandshakeReply` ordering parity slice implemented (2026-04-03):
  - removed pre-send fallback reply from startup-prime prelude and pre-receive social poll.
  - fallback reply now evaluates after inbound social receive observations.
  - bounded non-strict rerun evidence:
    - startup transcript no longer sends `RegionHandshakeReply` at index 3.
    - first-divergence moved to index 3 `AgentThrottle` vs Firestorm `ViewerEffect`.
    - `RegionHandshakeReply` now appears later (`#12`) after inbound packets.
    - LLUDP object gate still `FAIL` in this bounded Fidelis window.
  - artifacts:
    - `artifacts/logs/live_startup_parity_non_strict_post_reply_order_2026-04-03_174328.log`
    - `artifacts/logs/network_debug_startup_parity_non_strict_post_reply_order_2026-04-03_174328.jsonl`
    - `artifacts/logs/startup_first_divergence_diff_post_reply_order_2026-04-03_174328.json`
- Firestorm-vs-viewer startup first-divergence diff completed (2026-04-03):
  - comparison artifact: `artifacts/logs/startup_first_divergence_diff_2026-04-03_172944.json`
  - first outbound divergence is at index `3`:
    - Firestorm: `ViewerEffect (0x0000ff11)`
    - viewer (non-strict): `RegionHandshakeReply (0xffff0095)`
  - implication: fallback `RegionHandshakeReply` ordering is likely too early versus Firestorm behavior on this route; next patch should target fallback reply ordering, not strict inbound-handshake chasing.
- Endpoint-aware LLUDP handshake reprime + reply-targeting slice implemented (2026-04-03):
  - `viewer_net` now tracks observed first-simulator sender endpoints for bootstrap/handshake traffic and uses prioritized reply targeting for `RegionHandshakeReply`:
    - `last_region_handshake_sender` -> `last_bootstrap_sender` -> `circuit.target`
  - `viewer_net` now exposes bounded `send_handshake_reprime_bundle(...)` that sends `UseCircuitCode` + `CompleteAgentMovement` across deduped candidate endpoints.
  - `viewer_app` strict-mode reprime now invokes the handshake bundle before startup-interest reprime and reports sent datagram count.
  - new tests:
    - `send_pending_region_handshake_reply_prefers_observed_bootstrap_sender_endpoint`
    - `send_handshake_reprime_bundle_sends_to_observed_and_primary_endpoints`
  - validation:
    - `cargo fmt --all`
    - `cargo check -p viewer_net -p viewer_app`
    - `cargo test -p viewer_net send_pending_region_handshake_reply -- --nocapture`
    - `cargo test -p viewer_net send_handshake_reprime_bundle -- --nocapture`
    - `cargo test -p viewer_app -- --nocapture`
- Full-region-handshake strictness slice implemented (2026-04-03):
  - `viewer_net` now supports strict handshake-reply policy via `set_require_observed_region_handshake_for_reply(...)`.
  - `viewer_app` adds env knob `VIEWER_APP_REQUIRE_REGION_HANDSHAKE_REPLY` (default `false`) and bounded strict-mode re-prime attempts.
  - live A/B evidence on Fidelis:
    - default mode: `region_handshake_observed=none`, fallback reply sent once, `lludp_object_gate: PASS`.
    - strict mode: `region_handshake_observed=none`, reply send count stays `0`, bounded re-prime attempts fire, `lludp_object_gate: FAIL` in-window.
  - artifacts:
    - `artifacts/logs/live_region_handshake_default_2026-04-03_021321.log`
    - `artifacts/logs/network_debug_region_handshake_default_2026-04-03_021321.jsonl`
    - `artifacts/logs/live_region_handshake_strict_2026-04-03_021506.log`
    - `artifacts/logs/network_debug_region_handshake_strict_2026-04-03_021506.jsonl`
- Mesh fetch cookie-state parity hardening implemented + live-tested (2026-04-03):
  - `viewer_net` mesh staged fetch now preserves `Set-Cookie` state across probe/fallback HTTP calls.
  - added helpers:
    - `extract_cookie_header_from_set_cookie(...)`
    - `merge_cookie_header(...)`
    - mesh staged fetch wrapper with cookie carry-over
  - new tests:
    - `fetch_mesh_asset_bytes_reuses_set_cookie_between_probe_and_followup`
    - `merge_cookie_header_prefers_new_values_and_preserves_existing`
  - bounded live Fidelis fixture run still reports:
    - `mesh_fetch: failed ... 403 Forbidden ... <Code>AccessDenied</Code>`
  - artifacts:
    - `artifacts/logs/live_mesh_cookie_parity_2026-04-03_015824.log`
    - `artifacts/logs/network_debug_mesh_cookie_parity_2026-04-03_015824.jsonl`
- ObjectUpdateCached cache-miss recovery is now live-validated (2026-04-02):
  - `viewer_net` now issues medium-frequency `RequestMultipleObjects` for queued `ObjectUpdateCached` local IDs on the active social circuit.
  - bounded Fidelis run shows direct causal evidence in one steady-state window:
    - typed receive includes `ObjectUpdateCached:1` and `ObjectUpdate:1`
    - transcript send includes `RequestMultipleObjects(0x0000ff03)#13 ack=0x00000009`
  - artifacts:
    - `artifacts/logs/live_mesh_cachemiss_verify_2026-04-02_205620.log`
    - `artifacts/logs/network_debug_mesh_cachemiss_verify_2026-04-02_205620.jsonl`
  - diagnostics cleanup: low-frequency `UseCircuitCode` now labels correctly instead of `Unknown(0xffff0003)`.
- Mesh ingress + 403 classification hardening slice implemented (2026-04-02):
  - `viewer_net` now classifies low-frequency `ObjectExtraParams` (`99`) and decodes mesh IDs from sculpt/mesh extra-param entries into object-feed state.
  - `viewer_app` mesh-failure relays now include explicit 403 buckets (for example `bucket=AccessDenied`) while preserving `MissingCapability` retry semantics.
  - bounded Fidelis live fixture run now shows explicit root cause:
    - `mesh_fetch: failed ... reason=MissingCapability ... bucket=AccessDenied`
    - artifact: `artifacts/logs/live_mesh_fidelis_fixture_2026-04-02_200524.log`
  - bounded Fidelis no-fixture run confirms LLUDP object ingress still reaches `lludp_object_gate: PASS`:
    - artifact: `artifacts/logs/live_mesh_fidelis_object_extra_params_2026-04-02_200245.log`
- Firestorm mesh parity hardening slice implemented (2026-04-02):
  - `viewer_net` now parses `ObjectUpdateCompressed` optional fields up to `ExtraParams` and decodes mesh IDs from explicit sculpt/mesh entries.
  - `viewer_grid` now exposes Firestorm-style mesh URL helper with cap preference `ViewerAsset -> GetMesh2 -> GetMesh` and query-only `mesh_id` URLs.
  - `viewer_app` mesh request lane now uses the policy helper instead of ad-hoc cap-loop URL generation.
  - `viewer_net` mesh fetch now performs range-first (`Range: bytes=0-`) then plain fallback.
  - validation: `cargo fmt --all`, `cargo check -p viewer_net -p viewer_grid -p viewer_app`, `cargo test -p viewer_net`, `cargo test -p viewer_grid`, `cargo test -p viewer_app test_extract_decoded_object_feed_mesh_ids_is_deterministic_and_capped -- --nocapture`.
  - bounded live evidence (`artifacts/logs/live_mesh_firestorm_parity_2026-04-02_192934.log`) still shows `ViewerAsset` mesh probes at `403` and no in-window `mesh_fetch` queue events.
- Object-feed mesh decode + scheduling parity (Firestorm + OpenSim referenced) is now implemented (2026-04-02):
  - `viewer_net` decoded object feed now retains/exports optional `mesh_id` from LLUDP `ExtraParams` (sculpt/mesh param IDs) with bounded parsing.
  - `viewer_core::DecodedWorldObjectFeedObject` now carries optional `mesh_id`.
  - `viewer_app` now includes decoded object-feed mesh IDs in `tick_scene_meshes` request scheduling (alongside fixture + visible scene meshes).
  - validation: `cargo fmt --all`, `cargo check -p viewer_net -p viewer_core -p viewer_app`, `cargo test -p viewer_net`, `cargo test -p viewer_core`, `cargo test -p viewer_app`.
- New implementation artifacts:
  - `docs/plans/PLAN_OBJECT_FEED_MESH_ID_DECODE_SCHEDULING_2026-04-02.md`
  - `docs/reviews/REVIEW_PLAN_OBJECT_FEED_MESH_ID_DECODE_SCHEDULING_2026-04-02.md`
  - `docs/reviews/REVIEW_IMPL_OBJECT_FEED_MESH_ID_DECODE_SCHEDULING_2026-04-02.md`
  - `docs/reports/REPORT_OBJECT_FEED_MESH_ID_DECODE_SCHEDULING_2026-04-02.md`
- Object data ingestion/decoding parity (Firestorm + OpenSim referenced) is now implemented (2026-04-02):
  - `viewer_net` object-feed decode now exports bounded `position_centi` across `ObjectUpdate`, `ObjectUpdateCompressed`, and `ImprovedTerseObjectUpdate` where payloads carry spatial data.
  - `viewer_app` now maps `position_centi` into `viewer_core` object-feed snapshots.
  - `viewer_core` world object-feed proxies now prefer decoded position data and only fall back to local-id ring placement when decoded position is absent.
  - validation: `cargo fmt --all`, `cargo check -p viewer_net -p viewer_core -p viewer_app`, `cargo test -p viewer_net`, `cargo test -p viewer_core`, `cargo test -p viewer_app`.
- Object UUID focus ingest/render path is now implemented (2026-04-02):
  - `viewer_net` now retains/exports LLUDP `ObjectUpdate` `FullID` as optional decoded `object_id`.
  - `viewer_app` now supports `VIEWER_APP_OBJECT_UUID_FOCUS=<uuid>` to forward only matching object-feed entries into scene ingestion.
  - `docs/TESTING_REFERENCE.md` documents the new focus knob.
- Bounded Puppy live evidence with focus UUID (`10930d3b-1821-c584-a0c7-28a34999800d`) shows:
  - target UUID present in RegionObjects typed sample
  - LLUDP object ingress remains healthy (`lludp_object_gate: PASS`)
  - focused object-feed export remained `total_objects=0` in the bounded window (no matching `FullID` observed yet)
  - artifacts:
    - `artifacts/logs/live_object_uuid_focus_puppy_2026-04-02.log`
    - `artifacts/logs/network_debug_object_uuid_focus_puppy_2026-04-02.jsonl`
- RegionObjects mesh-candidate diagnostic extraction is now implemented:
  - `RegionObjectsInspection` includes bounded `candidate_mesh_asset_ids`.
  - runtime `region_objects` summaries now include `mesh_candidates=...`.
- Bounded live evidence confirms field emission on Ahern route:
  - `artifacts/logs/live_region_objects_mesh_candidates_2026-04-02.log:39`
  - observed value on this sample: `mesh_candidates=none`.
- Companion mesh probe in same run remains:
  - `mesh_fetch: queued id=947d4505-eb76-2ef5-c049-e7882881d689 ...`
  - `mesh_fetch: failed ... 403 Forbidden ...`
- New implementation artifacts:
  - `docs/reports/REPORT_REGION_OBJECTS_MESH_CANDIDATE_EXTRACTION_2026-04-02.md`
  - `docs/reviews/REVIEW_IMPL_REGION_OBJECTS_MESH_CANDIDATE_EXTRACTION_2026-04-02.md`
- Live mesh ingest pipeline is now wired end-to-end in code:
  - `LiveFeedCommand::RequestMesh` worker handling implemented.
  - mesh URL candidates built from `GetMesh2`/`GetMesh`/`ViewerAsset` with `mesh_id` shaping.
  - fetched bytes now feed `geometry_cache.get_mesh(uuid, lod, bytes)`.
- Bounded live runtime evidence confirms mesh lane execution:
  - `mesh_fetch: queued id=00000000-0000-0000-0000-000000000001 lod=0 candidates=5`
  - `mesh_fetch: failed ... reason=MissingCapability detail=http status 403 Forbidden ...`
  - artifact: `artifacts/logs/live_mesh_fixture_probe_2026-04-01.log`
- `VIEWER_FIXTURE_MESHES` is now documented in `docs/TESTING_REFERENCE.md` for deterministic mesh-lane probing.
- New implementation artifacts:
  - `docs/reports/REPORT_LIVE_MESH_INGEST_PIPELINE_2026-04-01.md`
  - `docs/reviews/REVIEW_IMPL_LIVE_MESH_INGEST_PIPELINE_2026-04-01.md`
- Added a dedicated single-object live-texture verification mode:
  - `STRESS_TEST=live_texture` (aliases: `live-texture`, `single_live_texture`, `7`)
  - inserts one center cube and applies the first fixture/live texture ID.
- Bounded live verification confirms end-to-end ingest on this mode:
  - `texture_fetch: queued id=00000000-0000-0000-0000-000000000001 ...`
  - `texture_fetch: ready id=00000000-0000-0000-0000-000000000001 attempt=1 ...`
  - artifact: `artifacts/logs/live_single_live_texture_center_test_2026-04-01.log`
- Testing reference now documents the new stress-test mode:
  - `docs/TESTING_REFERENCE.md`
- RegionHandshakeReply viewer-flag correction + bounded fallback is now implemented and live-validated:
  - reply flags are viewer-derived (`0x00001000` self-appearance support), not simulator region-flag echoes.
  - one-shot fallback reply can send once startup reaches movement-complete stage even if inbound `RegionHandshake` is not observed.
- New bounded live artifact shows LLUDP object-ingress unlock in the same run:
  - startup still starts at `lludp_object_gate: FAIL` with `region_handshake=none`.
  - after first steady-state window, `ObjectUpdate:2` appears and gate flips to `PASS`.
  - steady-state object feed remains populated (`update_messages=164 total_objects=1024`).
- Primary evidence artifact:
  - `artifacts/logs/network_debug_region_handshake_reply_flags_fix_2026-04-01.jsonl`
- New execution artifacts:
  - `docs/reports/REPORT_REGION_HANDSHAKE_REPLY_FLAGS_AND_FALLBACK_UNBLOCK_2026-04-01.md`
  - `docs/reviews/REVIEW_IMPL_REGION_HANDSHAKE_REPLY_FLAGS_AND_FALLBACK_UNBLOCK_2026-04-01.md`
- LL full-stack protocol extraction is now documented in execution-grade form:
  - lane inventory matrix
  - message handling matrix
  - handshake state machines (root + child)
  - capability invocation exactness matrix
  - ACK/reliability matrix
  - server gate matrix
  - PASS/FAIL/PARTIAL gap table and ranked blockers
  - minimal patch sequence with measurable acceptance signals
- New closure artifacts were added for the next implementation branch:
  - `docs/reports/REPORT_LL_PROTOCOL_FULL_STACK_EXTRACTION_2026-04-01.md`
  - `docs/plans/PLAN_LL_PROTOCOL_GAP_CLOSURE_2026-04-01.md`
  - `docs/reviews/REVIEW_PLAN_LL_PROTOCOL_GAP_CLOSURE_2026-04-01.md`
- Runtime branch conclusion remains unchanged from latest bounded captures:
  - endpoint-shape parity is proven
  - child follow-up sends are proven
  - `RegionHandshake`/`RegionHandshakeReply` are still absent
  - LLUDP object ingress gate is still `FAIL`
- `RegionObjects` typed-feed ingestion is working and live-validated (`typed_sample=...` appears in bounded connected logs).
- The typed sample now includes `landimpact` when present, and this is live-validated on the Ahern baseline (`landimpact=1` / `landimpact=20` observed).
- LLUDP world-object ingress remains blocked (`RegionHandshake`, `RegionHandshakeReply`, and `ObjectUpdate*` still absent in bounded runs).
- OpenSimulator-parity endpoint extraction is now live:
  - EventQueue `EnableSimulator` binary-IP fields are decoded to explicit endpoint targets.
  - follow-up sends now target resolved child `ip:port` endpoints instead of port-only fallback.
- Despite endpoint-parity improvements, latest bounded run still reports:
  - `region_handshake=none`
  - `region_handshake_reply=none`
  - `lludp_object_gate: FAIL`
- A runtime-gated LLUDP startup parity-bundle path is now implemented in `viewer_app` (`VIEWER_APP_LLUDP_STARTUP_PARITY_BUNDLE=on|off`, default `off`).
- First-simulator forensics now emit an explicit LLUDP object-ingress gate line with pass/fail verdict and local-id evidence preview.
- A bounded live parity-bundle run was executed with the flag enabled; the gate stayed `FAIL` (`object_update=none`, `update_messages=0`, `total_objects=0`, `local_ids=none`).
- Capability-readiness invocation instrumentation is now live (`startup readiness: ...` matrix in `parallel_protocol` relays).
- Capability readiness now has EventQueue-gated ordering control (`VIEWER_APP_CAPABILITY_PROBES_REQUIRE_EVENT_QUEUE_OK`, default `true`).
- Latest bounded EventQueue-gated run surfaced concrete outcomes:
  - `InterestList` probe => `ok` (`keys=mode,stats`) after first `EventQueueGet:ok`
  - `UntrustedSimulatorMessage` probe => `ok` via GET->POST fallback (`probe_transport_status=200`, `probe_decode=unparsed_body`)
- EventQueue still degrades on this path (`cap not found`) and now triggers bounded reconnect via a dedicated threshold.
- Reconnect teleport controls are working for evidence capture, and current region identity is surfaced in diagnostics.
- A broader-SLURL capture (`secondlife://Morris/128/128/25`) showed a mixed post-reconnect result: pre-teleport `typed_sample=...` was present, but the first post-reconnect `RegionObjects` startup probe returned `typed_sample=none`.
- A reconnect-only delayed `RegionObjects` re-probe path is now implemented and live-validated via `cargo run -p viewer_app`.
- The latest target-comparison run confirms route dependence:
  - initial simhost (`simhost-0a962ce03cdb50c3e...`) still returned `typed_sample=none`
  - reconnect target `secondlife://Ahern/50/60/70` on `simhost-04e63a701b66ed282...` returned rich `typed_sample=...` on both primary probe and delayed re-probe
- New paired A/B evidence confirms this split is reproducible:
  - Morris target run: rich pre-reconnect, empty post-reconnect
  - Ahern target run: empty pre-reconnect, rich post-reconnect
- `RegionObjects` transcript lines now include `host_family=...` tags, so route shifts are explicit in both relay and protocol-event views.
- Two-lane non-baseline transport probes are now live in startup diagnostics (one probe per host family, bounded to two families).
- Latest bounded evidence captured concrete lane responses:
  - `asset-cdn` (`ViewerAsset`) => `403 application/xml` (`body_bytes=275`)
  - `simhost-...` (`SimulatorFeatures`) => `503 text/plain` (`body_bytes=13`)
- Firestorm-aligned lane request-shaping matrix is now implemented and live-validated:
  - immediate `asset-cdn` `ViewerAsset` query-key probes (`texture_id`, `mesh_id`, `material_id`, `animatn_id`, `sound_id`)
  - EventQueue-gated simhost shaped probes (`SimulatorFeatures` GET, `InterestList` POST, `UntrustedSimulatorMessage` GET)
  - shaped probe relay/protocol markers: `LaneProbeShape:start|ok|err`
- Strategic direction has pivoted: further incremental `RegionObjects` continuation is paused; active branch is now hard-gated LLUDP ingress (`first ObjectUpdate* decoded`).

## Latest Notable Changes (RegionHandshake OpenSimulator Endpoint Parity)
- **EventQueue simulator-target parsing now handles OpenSimulator-style endpoint shapes**:
  - `SimulatorInfo.IP` binary payloads are preserved and decoded from base64 bytes.
  - resolved endpoint metadata is now emitted (`endpoint_ip`, `endpoint_port`, `endpoint_source`).
- **EnableSimulator follow-up routing now uses resolved endpoint metadata**:
  - live evidence shows explicit child-target sends, for example:
    - `endpoint=34.211.187.220:13022 source=simulatorinfo_binary_ip_port`
    - `endpoint=35.91.216.188:13028 source=simulatorinfo_binary_ip_port`
    - `endpoint=54.186.233.232:13032 source=simulatorinfo_binary_ip_port`
- **Handshake progression observability is now explicit**:
  - `region_handshake_observed ...`
  - `region_handshake_reply_send ...`
- **Current handshake/object status remains blocked** in the same bounded run:
  - `RegionHandshake = none`
  - `RegionHandshakeReply = none`
  - `ObjectUpdate* = none`
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net -p viewer_app`
  - bounded live capture:
    - `artifacts/logs/network_debug_region_handshake_opensim_endpoint_parity_2026-04-01.jsonl`

## Latest Notable Changes (LLUDP 4-Step Startup Path)
- **Startup interest invariant is now explicit and first-class**:
  - `first_sim_forensics` emits `startup_interest_gate: verdict=PASS|FAIL required=... missing=... classification=...`
  - gate evidence includes required sends (`AgentThrottle`, `AgentUpdate`, `AgentHeightWidth`) with order index and packet ids.
- **Startup transcript visibility is expanded**:
  - first-simulator transcript output now uses a wider tail window to avoid hiding startup send evidence.
- **AgentUpdate keepalive path is now inspectable and tunable**:
  - periodic relay line: `agent_update_keepalive: period_ticks=... camera_center=... far=... control_flags=... reliable=...`
  - new env knobs:
    - `VIEWER_APP_AGENT_UPDATE_FAR`
    - `VIEWER_APP_AGENT_UPDATE_KEEPALIVE_TICKS`
- **Session residency classifier is now emitted each protocol window**:
  - `parallel_protocol` emits `residency_state=unknown|root_likely|child_likely|degraded evidence=...`
  - classifier uses startup timeline + region-control counters + EventQueue health/simulator-target evidence.
- **Bounded live matrix executed**:
  - baseline: `artifacts/logs/network_debug_lludp_unblock_4step_baseline_2026-04-01.jsonl`
  - far override: `artifacts/logs/network_debug_lludp_unblock_4step_far_2026-04-01.jsonl`
  - keepalive ticks override: `artifacts/logs/network_debug_lludp_unblock_4step_keepalive_2026-04-01.jsonl`
  - alternate route (`VIEWER_LOGIN_START=uri:Ahern&50&60&70`):
    `artifacts/logs/network_debug_lludp_unblock_4step_alternate_route_2026-04-01.jsonl`
- **Current gate status remains unchanged**:
  - `lludp_object_gate` still `FAIL` in all bounded runs (`ObjectUpdate*` absent, no local-id evidence).

## Latest Notable Changes (Non-Retryable 4xx Texture Fetch)
- **Live texture scheduler now treats deterministic denied/not-found HTTP statuses as non-retryable**:
  - HTTP `401/403/404` now classify as `MissingCapability` in `viewer_app` texture failure mapping.
  - This prevents bounded scheduler retry/backoff loops on deterministic `AccessDenied` responses.
- **Bounded live validation confirms behavior**:
  - `texture_fetch: queued ...` lines still appear as expected.
  - `403 Forbidden` outcomes now surface as:
    - `texture_fetch: failed ... reason=MissingCapability attempt=1 ...`
  - no `retry` lines are emitted for those same IDs.
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_app`
  - `cargo test -p viewer_app`
  - bounded live run artifact:
    - `artifacts/logs/live_texture_scheduler_runtime_4xx_nonretry_2026-04-01.log`

## Latest Notable Changes (Live Texture Fetch Scheduler Pipeline)
- **`viewer_app` live texture fetches now use a bounded scheduler pipeline** instead of ad-hoc per-command spawn:
  - queued + deduplicated requests by `AssetID`
  - priority-aware dispatch (`Active` > `Previous` > `Neighbor` > `Normal`)
  - bounded in-flight cap (`4`)
  - bounded retry/backoff for retryable failures (`Timeout` / `Transport`, max attempts `3`)
- **Live texture ingest decode now uses shared decoder**:
  - `LiveFeedUpdate::TextureAsset` now decodes with `viewer_asset::decode_texture_rgba8(...)` (J2C + PNG), replacing prior PNG-only decode.
- **New scheduler helper tests in `viewer_app`** cover retry reason/attempt boundaries and monotonic backoff behavior.
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_app -p viewer_asset -p viewer_net -p viewer_grid`
  - `cargo test -p viewer_app -p viewer_asset`
  - bounded runtime screenshot smoke:
    - `artifacts/screenshots_live_texture_scheduler_smoke_2026-04-01/viewer_test_0001.png` reviewed (visual pass)

## Next Steps (Recommended Order)
1. Investigate and stabilize EventQueue decode/readiness behavior on simulator-host `:12043` (current frequent `missing llsd map` errors).
2. Use the new LLUDP 4-step run artifacts as baseline:
   - `artifacts/logs/network_debug_lludp_unblock_4step_baseline_2026-04-01.jsonl`
   - `artifacts/logs/network_debug_lludp_unblock_4step_far_2026-04-01.jsonl`
   - `artifacts/logs/network_debug_lludp_unblock_4step_keepalive_2026-04-01.jsonl`
3. Keep LLUDP acceptance criterion unchanged: first decoded `ObjectUpdate*` with non-empty local-id evidence.

## Latest Notable Changes (Lane Request Shaping)
- **Capability request-shape modeling is now explicit in `viewer_grid`**:
  - `SimulatorFeatures` => GET
  - `InterestList` => POST LLSD (`mode=default`)
  - `UntrustedSimulatorMessage` => GET shape
  - `ViewerAsset` => Firestorm-style query-key URL shaping (`/?<key>=...` first)
- **`viewer_net` now has a generic shaped probe executor** with bounded metadata:
  - `status`, `content_type`, `body_bytes`
  - `selected_url`, `method`, `decode`, `body_preview_hash`, `response_class`
- **`viewer_app` startup probes now run as a shaped matrix**:
  - immediate asset-cdn `ViewerAsset` key probes
  - simhost probes deferred until `EventQueueGet:ok`
- **New env control**:
  - `VIEWER_APP_LANE_PROBE_ASSET_IDS` (optional CSV IDs for deterministic shaped viewer-asset probes)
- **Latest bounded live evidence** (`artifacts/logs/network_debug_lane_request_shaping_2026-04-01.jsonl`):
  - `asset-cdn` shaped outcomes vary by query key:
    - `texture_id` => `200 image/x-j2c`
    - `mesh_id` => `403 application/vnd.ll.mesh`
    - `material_id` => `403 application/vnd.ll.material`
    - `animatn_id` => `403 application/vnd.ll.animation`
    - `sound_id` => `200 application/ogg`
  - gated simhost shaped probes:
    - `SimulatorFeatures` GET => `200 application/llsd+xml` (`llsd_map`)
    - `InterestList` POST => `200 application/llsd+xml` (`llsd_map`)
    - `UntrustedSimulatorMessage` GET => `405 text/plain`
  - LLUDP gate in same run: `FAIL` (`ObjectUpdate*` absent)
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_grid -p viewer_net -p viewer_app`
  - `cargo test -p viewer_grid -p viewer_net -p viewer_app`
  - bounded `cargo run -p viewer_app` live capture (timeout-bounded)

## Latest Notable Changes (Two-Lane Transport Probe)
- **Bounded non-baseline lane probing is now implemented** across two host families in startup diagnostics:
  - deterministic target selection from non-baseline seed capabilities
  - one-shot probe per selected host lane
- **New protocol/relay markers are now present**:
  - `LaneProbe:start ...`
  - `LaneProbe:ok ...` / `LaneProbe:err ...`
- **Latest live evidence from `artifacts/logs/network_debug_two_lane_transport_probe_2026-04-01.jsonl`**:
  - `asset-cdn` probe target `ViewerAsset` => `status=403`, `content_type=application/xml`, `body_bytes=275`
  - `simhost-0eec...` probe target `SimulatorFeatures` => `status=503`, `content_type=text/plain`, `body_bytes=13`
- **Interpretation impact**:
  - both lanes are now concretely observable at transport level
  - non-2xx responses are informative lane behavior, not evidence of lane absence
  - LLUDP object ingress remained blocked in the same run
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_app`
  - bounded live run:
    - `VIEWER_APP_LIVE_STARTUP=on`
    - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_two_lane_transport_probe_2026-04-01.jsonl`
    - `cargo run -p viewer_app` (bounded capture)

## Latest Notable Changes (Seed-Cap Broadening Host-Lane Discovery)
- **Seed capability discovery is now Firestorm-aligned and broad** in `viewer_net` (`DEFAULT_SEED_CAPABILITY_REQUEST` expanded from minimal probes to a much wider capability name set).
- **New startup and follow-up host-lane relay lines are now emitted** in `viewer_app`:
  - `seed_caps:non_baseline_by_host ...`
  - `seed non-baseline capability names by host: ...`
- **Host-grouped evidence now explicitly shows multi-lane discovery** in bounded live logs:
  - `asset-cdn:count=4 names=GetMesh,GetMesh2,GetTexture,ViewerAsset`
  - `simhost-...:count=102 ...`
- **Interpretation impact**:
  - capability discovery was previously too narrow for lane forensics
  - alternate HTTP lanes are now visible, but this alone did not unblock LLUDP object ingress
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_app`
  - bounded live run:
    - `VIEWER_APP_LIVE_STARTUP=on`
    - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_seed_cap_broadening_host_lane_discovery_2026-04-01.jsonl`
    - `cargo run -p viewer_app` (bounded capture)

## Latest Notable Changes (Untrusted Probe POST Fallback)
- **`UntrustedSimulatorMessage` probe now has method fallback** in `viewer_net`:
  - tries GET first
  - on `405 Method Not Allowed`, retries via Firestorm-style LLSD POST envelope (`message`, `body`)
- **Transport-success responses now surface as readiness `ok`** even when payload is non-LLSD/unparsed:
  - `probe_transport_status=200`
  - `probe_decode=unparsed_body`
- **Latest bounded live evidence confirms readiness `ok`** for both `InterestList` and `UntrustedSimulatorMessage` after EventQueue gate-open.
- **LLUDP gate remained failing in the same run**:
  - `lludp_object_gate: verdict=FAIL ... object_update=none ... local_ids=none`
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_app`
  - bounded live run with:
    - `VIEWER_APP_LIVE_STARTUP=on`
    - `VIEWER_APP_LLUDP_STARTUP_PARITY_BUNDLE=on`
    - `VIEWER_APP_EVENT_QUEUE_CAP_NOT_FOUND_BEFORE_RECONNECT=3`
    - `VIEWER_APP_CAPABILITY_PROBES_REQUIRE_EVENT_QUEUE_OK=true`
    - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_untrusted_probe_post_fallback_2026-04-01.jsonl`
    - `cargo run -p viewer_app` (bounded capture)

## Latest Notable Changes (Capability Probe EventQueue Gating)
- **Startup capability probe ordering is now explicitly gated** behind first successful EventQueue poll by default:
  - `VIEWER_APP_CAPABILITY_PROBES_REQUIRE_EVENT_QUEUE_OK` (default `true`)
- **Protocol evidence now shows deferred probe lifecycle**:
  - `probe_gate:wait EventQueueGet:ok`
  - `InterestList:deferred waiting_for=EventQueueGet:ok`
  - `UntrustedSimulatorMessage:deferred waiting_for=EventQueueGet:ok`
  - `probe_gate:open reason=EventQueueGet:ok`
- **Outcome changed materially for `InterestList`** in bounded live evidence:
  - prior ungated run: `InterestList => 404 Agent not found`
  - gated run: `InterestList => ok keys=mode,stats`
- **`UntrustedSimulatorMessage` now reports readiness `ok`** in the latest run:
  - `probe_transport_status=200`
  - `probe_decode=unparsed_body`
- **LLUDP gate remained failing in the same run**:
  - `lludp_object_gate: verdict=FAIL ... object_update=none ... local_ids=none`
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_app`
  - bounded live run with:
    - `VIEWER_APP_LIVE_STARTUP=on`
    - `VIEWER_APP_LLUDP_STARTUP_PARITY_BUNDLE=on`
    - `VIEWER_APP_EVENT_QUEUE_CAP_NOT_FOUND_BEFORE_RECONNECT=3`
    - `VIEWER_APP_CAPABILITY_PROBES_REQUIRE_EVENT_QUEUE_OK=true`
    - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_capability_probe_event_queue_gating_2026-04-01.jsonl`
    - `cargo run -p viewer_app` (timeout-bounded capture)

## Latest Notable Changes (Capability Readiness Invocation Investigation)
- **Capability readiness matrix is now emitted** in `parallel_protocol` relays for:
  - `EventQueueGet`
  - `InterestList`
  - `UntrustedSimulatorMessage`
  - `RegionObjects`
- **Bounded one-shot probes now run at startup** when caps are available:
  - `InterestList:start ...` / `InterestList:ok|err ...`
  - `UntrustedSimulatorMessage:start ...` / `UntrustedSimulatorMessage:ok|err ...`
- **Latest bounded evidence captured concrete outcomes**:
  - `InterestList:err http status 404 Not Found ... Agent not found`
  - `UntrustedSimulatorMessage:err http status 405 Method Not Allowed`
- **EventQueue degradation now has bounded cap-not-found reconnect handling**:
  - `event queue cap-not-found threshold reached (3) ; reconnecting`
- **LLUDP gate remained failing in the same run**:
  - `lludp_object_gate: verdict=FAIL ... object_update=none ... local_ids=none`
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_app`
  - bounded live run with:
    - `VIEWER_APP_LIVE_STARTUP=on`
    - `VIEWER_APP_LLUDP_STARTUP_PARITY_BUNDLE=on`
    - `VIEWER_APP_EVENT_QUEUE_CAP_NOT_FOUND_BEFORE_RECONNECT=3`
    - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_capability_readiness_2026-04-01.jsonl`
    - `cargo run -p viewer_app` (timeout-bounded capture)

## Latest Notable Changes (LLUDP Startup Parity Bundle Runtime Gate)
- **New runtime flag in `viewer_app` startup config**: `VIEWER_APP_LLUDP_STARTUP_PARITY_BUNDLE` (bool-like, default `false`).
- **Startup prime mode is now explicit in relays**:
  - `startup prime mode=lludp_parity_bundle:on`
  - `startup prime mode=lludp_parity_bundle:off`
- **First-simulator forensics now include strict LLUDP gate output**:
  - `lludp_object_gate: verdict=PASS|FAIL`
  - `object_update=<index|none>`
  - `update_messages=<count>`
  - `total_objects=<count>`
  - `local_ids=<preview|none>`
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_app`
  - `cargo test -p viewer_net`
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_app`
  - `cargo test -p viewer_net`
  - bounded live run:
    - `VIEWER_APP_LIVE_STARTUP=on`
    - `VIEWER_APP_LLUDP_STARTUP_PARITY_BUNDLE=on`
    - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_lludp_startup_parity_bundle_2026-04-01.jsonl`
    - `cargo run -p viewer_app` (timeout-bounded capture)
- **Observed gate result**:
  - `startup lludp_object_gate: verdict=FAIL object_update=none update_messages=0 total_objects=0 local_ids=none`
  - `after_first_steady_state_window lludp_object_gate: verdict=FAIL object_update=none update_messages=0 total_objects=0 local_ids=none`
- **Current blocker**:
  - LLUDP object ingress remains zero after parity-bundle run; next branch is simulator-host capability-readiness checks.

## Latest Notable Changes (Object Ingress Host-Family Transcript Tag)
- **RegionObjects lines now expose explicit route identity** with `host_family=...` tags.
- **Tag coverage includes**:
  - primary probe success/error
  - post-reconnect re-probe success/error
  - matching `RegionObjects:*` protocol-event entries
- **Live validation confirms tag emission**, for example:
  - `host_family=simhost-0629fe9f6de4b8693`
  - `host_family=simhost-0eec03118f78cfe1f`
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_app`
  - bounded connected run:
    - `VIEWER_APP_LIVE_STARTUP=on`
    - `VIEWER_APP_AUTO_TELEPORT_SLURL=secondlife://Ahern/50/60/70`
    - `VIEWER_APP_AUTO_TELEPORT_DELAY_TICKS=40`
    - `VIEWER_APP_REGION_OBJECTS_REPROBE_DELAY_TICKS=80`
    - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_region_objects_host_family_tag_2026-04-01.jsonl`
    - `cargo run -p viewer_app`
  - artifacts:
    - `artifacts/logs/live_region_objects_host_family_tag_2026-04-01.out.log`
    - `artifacts/logs/live_region_objects_host_family_tag_2026-04-01.err.log`
    - `artifacts/logs/network_debug_region_objects_host_family_tag_2026-04-01.jsonl`

## Latest Notable Changes (Object Ingress Reconnect A/B Compare)
- **Paired comparison is complete and reproducible** with identical knobs across Morris and Ahern targets.
- **Morris run**:
  - pre-reconnect (`simhost-04e63a...`) showed rich typed samples with `landimpact`
  - post-reconnect (`simhost-0a962c...`) stayed `typed_sample=none` on primary and delayed re-probe
- **Ahern run**:
  - pre-reconnect (`simhost-0a962c...`) showed `typed_sample=none`
  - post-reconnect (`simhost-04e63a...`) showed rich typed samples with `landimpact=1|20` on primary and delayed re-probe
- **Decision impact**: branch choice should be based on paired evidence; single reconnect captures are insufficient.
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - bounded Morris and Ahern `cargo run -p viewer_app` captures with dedicated artifacts under `artifacts/logs/`

## Latest Notable Changes (Object Ingress Typed Field Promotion: landimpact)
- **One additional typed field is now promoted**: `typed_sample=...` includes `landimpact` when present.
- **Live evidence is explicit on Ahern baseline**:
  - `landimpact=1`
  - `landimpact=20`
- **Reconnect stability remained intact**: primary probe and delayed reconnect re-probe continued to surface rich typed samples in the same run.
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net -p viewer_app`
  - bounded connected run:
    - `VIEWER_APP_LIVE_STARTUP=on`
    - `VIEWER_APP_AUTO_TELEPORT_SLURL=secondlife://Ahern/50/60/70`
    - `VIEWER_APP_AUTO_TELEPORT_DELAY_TICKS=40`
    - `VIEWER_APP_REGION_OBJECTS_REPROBE_DELAY_TICKS=80`
    - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_region_objects_typed_landimpact_2026-04-01.jsonl`
    - `cargo run -p viewer_app`
  - artifacts:
    - `artifacts/logs/live_region_objects_typed_landimpact_2026-04-01.out.log`
    - `artifacts/logs/live_region_objects_typed_landimpact_2026-04-01.err.log`
    - `artifacts/logs/network_debug_region_objects_typed_landimpact_2026-04-01.jsonl`

## Latest Notable Changes (Object Ingress Reprobe Target Comparison)
- **Cross-target evidence is now explicit**: one bounded run contained both outcomes in sequence.
- **Before reconnect**, `RegionObjects` on `simhost-0a962ce03cdb50c3e...` returned `<root>` and `typed_sample=none`.
- **After auto-teleport reconnect** to `secondlife://Ahern/50/60/70`, `RegionObjects` on `simhost-04e63a701b66ed282...` returned UUID-keyed map payloads with typed samples (`Object`, `bamboo`) on:
  - primary probe
  - delayed post-reconnect re-probe (`delay_ticks=80`)
- **Decision impact**: timing-only is not a universal explanation and emptiness is not universal either; branch selection now needs explicit target/route comparison.
- **Validation**:
  - bounded connected run:
    - `VIEWER_APP_LIVE_STARTUP=on`
    - `VIEWER_APP_AUTO_TELEPORT_SLURL=secondlife://Ahern/50/60/70`
    - `VIEWER_APP_AUTO_TELEPORT_DELAY_TICKS=40`
    - `VIEWER_APP_REGION_OBJECTS_REPROBE_DELAY_TICKS=80`
    - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_region_objects_post_reconnect_reprobe_target_ahern_2026-04-01.jsonl`
    - `cargo run -p viewer_app`
  - artifacts:
    - `artifacts/logs/live_region_objects_post_reconnect_reprobe_target_ahern_2026-04-01.out.log`
    - `artifacts/logs/live_region_objects_post_reconnect_reprobe_target_ahern_2026-04-01.err.log`
    - `artifacts/logs/network_debug_region_objects_post_reconnect_reprobe_target_ahern_2026-04-01.jsonl`

## Latest Notable Changes (Object Ingress Post-Reconnect Reprobe Timing)
- **Bounded reconnect-only re-probe logic is now implemented**: `viewer_app` can arm a one-shot delayed `RegionObjects` re-probe during reconnect sessions.
- **A new runtime knob is available**: `VIEWER_APP_REGION_OBJECTS_REPROBE_DELAY_TICKS` (default `80`) controls the re-probe delay window.
- **Log/protocol markers are explicit**: `RegionObjects:reprobe_armed`, `RegionObjects:reprobe_ok`, and `RegionObjects:reprobe_err` were added for unambiguous timeline reading.
- **Static/unit validation passed**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `CARGO_INCREMENTAL=0 cargo test -p viewer_net`
  - `CARGO_INCREMENTAL=0 cargo test -p viewer_app`
- **Current blocker**:
  - none for this slice; authoritative `cargo run -p viewer_app` live validation completed, and follow-up target comparison shows mixed (target-dependent) reconnect outcomes

## Latest Notable Changes (Object Ingress RegionObjects Broader SLURL Capture)
- **A broader reconnect target was executed live**: `VIEWER_APP_AUTO_TELEPORT_SLURL=secondlife://Morris/128/128/25`.
- **Pre-teleport behavior remained healthy**: `typed_sample=...` surfaced as expected with `Object`/`bamboo` pathfinding-linkset records.
- **Post-reconnect behavior changed materially**: the first bounded probe on the new simhost returned `keys=<root>` and `typed_sample=none`.
- **New direction is now timing-focused**: next branch is bounded post-reconnect `RegionObjects` re-probe timing, not immediate field-promotion.
- **Validation**:
  - bounded direct-binary live run with artifacts:
    - `artifacts/logs/live_region_objects_typed_feed_broader_slurl_capture_2026-04-01.out.log`
    - `artifacts/logs/live_region_objects_typed_feed_broader_slurl_capture_2026-04-01.err.log`
    - `artifacts/logs/network_debug_region_objects_typed_feed_broader_slurl_capture_2026-04-01.jsonl`

## Latest Notable Changes (Object Ingress RegionObjects Typed Feed Live Validation)
- **The bounded `RegionObjects` typed-feed promotion is now live-validated**: the current build emits the new `typed_sample=...` relay on-wire during a bounded connected run.
- **The relay stays compact and readable in the live JSONL transcript**: trusted fields such as `profile`, `name`, `linkset_use`, `walkability`, `position`, `description_shape`, and `owner` now appear in the existing `region_objects` summary line.
- **This confirms the typed-feed lane works end-to-end across the active reconnect flow**: the live pass used the current auto-teleport reconnect controls and still surfaced the promoted object sample data.
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_app`
  - bounded direct-binary live run with:
    - `artifacts/logs/live_region_objects_typed_feed_live_validation_rerun_2026-03-31.out.log`
    - `artifacts/logs/live_region_objects_typed_feed_live_validation_rerun_2026-03-31.err.log`
    - `artifacts/logs/network_debug_region_objects_typed_feed_live_validation_rerun_2026-03-31.jsonl`
- **Current nuance**:
  - the live rerun validated the new `typed_sample=...` summary on-wire
  - the reconnect did not broaden into a clearly different visible object family in this pass, so the next broader-sampling run should use a different reconnect target if sample diversity matters

## Latest Notable Changes (SLURL Auto-Teleport Capture Control)
- **Terminal-driven reconnect teleport capture is now available**: `viewer_app` can auto-queue the existing reconnect-based SLURL teleport path from env-configured startup controls.
- **The auto-teleport path is bounded and one-shot**: `VIEWER_APP_AUTO_TELEPORT_SLURL` arms the target and `VIEWER_APP_AUTO_TELEPORT_DELAY_TICKS` controls when it fires after connected steady state; the worker will not loop teleports across reconnects in the same run.
- **This reuses the existing reconnect teleport semantics**: no new transport behavior was added; the worker still normalizes supported SLURLs into Firestorm-style `uri:Region&x&y&z` and reconnects with that start location.
- **Testing/docs now include the capture knob**: `docs/TESTING_REFERENCE.md` documents the new env vars and includes a bounded example command for the next teleport evidence pass.
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_app`
  - `cargo test -p viewer_app`
  - bounded direct-binary smoke with env-configured auto-teleport target:
    - `artifacts/logs/live_slurl_auto_teleport_capture_control_binary_2026-03-31.out.log`
    - `artifacts/logs/live_slurl_auto_teleport_capture_control_binary_2026-03-31.err.log`

## Latest Notable Changes (SLURL Reconnect Teleport And Current Region Name)
- **A bounded in-app SLURL teleport control now exists**: the `Network Debug` window can now queue a reconnect-based teleport request using supported SLURL forms.
- **Supported SLURL inputs are normalized to Firestorm-style login start strings**: `secondlife://...`, `secondlife:///app/teleport/...`, and maps-style SLURLs now normalize to `uri:Region&x&y&z`.
- **Current region name is now surfaced as a first-class live/debug value**: the app exposes it through `LiveVisualSnapshot`, the diagnostics lines, and the `Network Debug` session summary.
- **This intentionally does not claim in-session teleport parity**: the worker performs a reconnect with the new start location instead of widening into full session-handoff orchestration.
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_core -p viewer_ui -p viewer_app`
  - `cargo test -p viewer_core -p viewer_ui -p viewer_app`
  - bounded offline smoke:
    - `artifacts/logs/live_slurl_reconnect_teleport_offline_2026-03-31.out.log`
    - `artifacts/logs/live_slurl_reconnect_teleport_offline_2026-03-31.err.log`

## Latest Notable Changes (Object Ingress RegionObjects Tuple Multi-Region Capture)
- **A longer same-region evidence pass is now complete**: the existing tuple-analysis surface was left unchanged and used for a 110-second bounded live run.
- **Time alone did not broaden the tuple family**: the latest longer capture still showed `samples=2` and `names=DSS Candlier Frame`.
- **The tuple pattern stayed unchanged across the longer window**:
  - `s0=const:0`
  - `s1=const:10.000000`
  - `s2=const:30`
  - `s3=const:0`
  - `s4=var:2|3`
  - `s5=const:0`
- **This moves the next branch from longer same-region capture to actual region change evidence**: the next active plan is now `docs/plans/PLAN_OBJECT_INGRESS_REGION_OBJECTS_TUPLE_TELEPORT_CAPTURE_2026-03-31.md`.
- **Validation**:
  - bounded connected run with captured logs:
    - `artifacts/logs/live_region_objects_tuple_multi_region_capture_2026-03-31.out.log`
    - `artifacts/logs/live_region_objects_tuple_multi_region_capture_2026-03-31.err.log`
    - `artifacts/logs/network_debug_region_objects_tuple_multi_region_capture_2026-03-31.jsonl`

## Latest Notable Changes (Object Ingress RegionObjects Tuple Sample Widening)
- **Code-side tuple widening is now in place**: the `RegionObjects` inspection path can now summarize more child maps and reports bounded distinct tuple-object names.
- **The current region/window remained narrow even after widening**: the latest bounded live run still showed `samples=2` and `names=DSS Candlier Frame`.
- **The current tuple pattern is unchanged**:
  - `s0=const:0`
  - `s1=const:10.000000`
  - `s2=const:30`
  - `s3=const:0`
  - `s4=var:2|3`
  - `s5=const:0`
- **This shifts the next step from code widening to evidence widening**: the next active plan is now `docs/plans/PLAN_OBJECT_INGRESS_REGION_OBJECTS_TUPLE_MULTI_REGION_CAPTURE_2026-03-31.md`.
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net`
  - bounded connected run with captured logs:
    - `artifacts/logs/live_region_objects_tuple_sample_widening_2026-03-31.out.log`
    - `artifacts/logs/network_debug_region_objects_tuple_sample_widening_2026-03-31.jsonl`

## Latest Notable Changes (Object Ingress RegionObjects Tuple Slot Analysis)
- **The tuple-like `RegionObjects` descriptions now have bounded slot analysis**: the relay/log surfaces `tuple_analysis=...` in addition to the raw tuple strings.
- **The current live pattern is now explicit**:
  - `samples=2`
  - `slots=6`
  - `s0=const:0`
  - `s1=const:10.000000`
  - `s2=const:30`
  - `s3=const:0`
  - `s4=var:2|3`
  - `s5=const:0`
- **The current tuple sample is narrow but useful**: both surfaced tuple samples belong to `DSS Candlier Frame`, which is enough to prove a repeatable slot pattern but not enough to assign semantics yet.
- **Next active plan**: `docs/plans/PLAN_OBJECT_INGRESS_REGION_OBJECTS_TUPLE_SAMPLE_WIDENING_2026-03-31.md`
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_app`
  - bounded connected run with captured logs:
    - `artifacts/logs/live_region_objects_tuple_slot_analysis_2026-03-31.out.log`
    - `artifacts/logs/network_debug_region_objects_tuple_slot_analysis_2026-03-31.jsonl`

## Latest Notable Changes (Object Ingress RegionObjects Position Shape Clarification)
- **The `RegionObjects` pathfinding lane now surfaces live positions directly**: the bounded probe now records `position`, `position_key=present`, and `position_shape=llsd_array_len3` for the first UUID-keyed child maps.
- **The earlier “no position” interpretation has been corrected**: the currently surfaced tuple-description and placeholder-description records both classify as `...with_position`, not `...no_position`.
- **The current live records now show concrete object-space coordinates**, for example:
  - `39.21141815185546875|68.02368927001953125|2999.260009765625`
  - `61.998996734619140625|87.66783905029296875|2962.088134765625`
- **The remaining open question is narrower**: the unresolved part is now the tuple-like `description` content itself, not whether those records carry position.
- **Next active plan**: `docs/plans/PLAN_OBJECT_INGRESS_REGION_OBJECTS_TUPLE_CONTENT_INTERPRETATION_2026-03-31.md`
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_app`
  - bounded connected run with captured logs:
    - `artifacts/logs/live_region_objects_position_shape_clarification_2026-03-31.out.log`
    - `artifacts/logs/network_debug_region_objects_position_shape_clarification_2026-03-31.jsonl`

## Latest Notable Changes (Object Ingress RegionObjects Pathfinding Variant Discrimination)
- **The typed `RegionObjects` lane now distinguishes live record variants**: the bounded probe surfaces `variant`, `description_shape`, and `description_tuple` hints when the first UUID-keyed child maps do not follow one uniform shape.
- **One concrete variant difference is now explained**:
  - placeholder-text records surface `description_shape=placeholder_text`
  - tuple-like records surface `description_shape=comma_numeric_tuple6`
  - tuple-like records now expose the raw tuple payload in bounded form, such as `0|10.000000|30|0|2|0`
- **A dedicated status tracker now exists**: `docs/OBJECT_INGRESS_STATUS.md` separates what is proven working, what is proven not working, and the current object-ingress questions.
- **LLUDP object ingress is still blocked**:
  - `RegionHandshake = none`
  - `RegionHandshakeReply = none`
  - `ObjectUpdate* = none`
  - `update_messages=0`
- **Next active plan**: `docs/plans/PLAN_OBJECT_INGRESS_REGION_OBJECTS_PATHFINDING_VARIANT_DISCRIMINATION_2026-03-31.md` remains the active investigation frame until the tuple-like variant is understood more deeply or superseded by a tighter follow-up.
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_app`
  - bounded connected run with captured logs:
    - `artifacts/logs/live_region_objects_pathfinding_field_promotion_2026-03-31.out.log`
    - `artifacts/logs/network_debug_region_objects_pathfinding_field_promotion_2026-03-31.jsonl`

## Latest Notable Changes (Object Ingress RegionObjects Pathfinding Field Promotion)
- **The `RegionObjects` opening now has typed named fields**: the bounded probe now emits compact per-object pathfinding summaries instead of only semantic strings.
- **First typed live fields are now visible**:
  - `profile=pathfinding_linkset`
  - `linkset_use=dynamic_phantom`
  - `walkability=100/100/100/100`
  - concrete `name=...`
  - concrete `owner=...`
  - `description=(No Description)` on some records
- **A new live nuance surfaced**: not every first-object record looks identical; some entries still surface comma-like data in `description`, and a separate `position` field did not appear in the bounded window.
- **LLUDP object ingress is still blocked**:
  - `RegionHandshake = none`
  - `RegionHandshakeReply = none`
  - `ObjectUpdate* = none`
  - `update_messages=0`
- **Next active plan**: `docs/plans/PLAN_OBJECT_INGRESS_REGION_OBJECTS_PATHFINDING_VARIANT_DISCRIMINATION_2026-03-31.md`
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_app`
  - bounded connected run with captured logs:
    - `artifacts/logs/live_region_objects_pathfinding_field_promotion_2026-03-31.out.log`
    - `artifacts/logs/network_debug_region_objects_pathfinding_field_promotion_2026-03-31.jsonl`

## Latest Notable Changes (Object Ingress RegionObjects Semantic Mapping)
- **The `RegionObjects` opening is now evidence-backed semantically**: the bounded probe now classifies the first UUID-keyed child maps as Firestorm-style `pathfinding_linkset` payloads rather than only exposing raw nested fields.
- **The first surfaced pathfinding semantics are now visible**:
  - `A-D` map to walkability coefficients
  - `can_be_volume` and `phantom` are now normalized from `0/1` to booleans in the relay
  - the current live objects classify as `linkset_use=dynamic_phantom`
  - concrete object names like `DSS Candlier Frame` and `Trance  Chair: Rope Bondage` are now surfaced in the same summary
- **The object-data opening is now more usable**: the viewer is ingesting object-related simulator payloads and can explain what the first bounded field family means.
- **LLUDP object ingress is still blocked**:
  - `RegionHandshake = none`
  - `RegionHandshakeReply = none`
  - `ObjectUpdate* = none`
  - `update_messages=0`
- **Next active plan**: `docs/plans/PLAN_OBJECT_INGRESS_REGION_OBJECTS_PATHFINDING_FIELD_PROMOTION_2026-03-30.md`
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_app`
  - bounded connected run with captured logs:
    - `artifacts/logs/live_region_objects_semantic_mapping_2026-03-30.out.log`
    - `artifacts/logs/network_debug_region_objects_semantic_mapping_2026-03-30.jsonl`

## Latest Notable Changes (Object Ingress RegionObjects Capability Probe)
- **The first object-related simulator data opening is now proven**: the primary simulator `RegionObjects` capability returns a UUID-keyed top-level map on the simulator-host `:12043` lane.
- **The response is structurally real, not empty**: the bounded probe now shows the first surfaced UUID keys classifying as nested `map` values, proving the simulator is returning structured per-object payloads.
- **This satisfies the immediate object-ingress goal**: the viewer is now ingesting object-related simulator data, even though it is not yet LLUDP `ObjectUpdate*`.
- **LLUDP object ingress is still blocked**:
  - `RegionHandshake = none`
  - `RegionHandshakeReply = none`
  - `ObjectUpdate* = none`
  - `update_messages=0`
  - `total_objects=0`
- **Next active plan**: `docs/plans/PLAN_OBJECT_INGRESS_REGION_OBJECTS_FIELD_EXTRACTION_2026-03-30.md`
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_app`
  - bounded connected runs with captured logs:
    - `artifacts/logs/live_region_objects_probe_2026-03-30.out.log`
    - `artifacts/logs/network_debug_region_objects_probe_2026-03-30.jsonl`
    - `artifacts/logs/live_region_objects_probe_2026-03-30_v2.out.log`
    - `artifacts/logs/network_debug_region_objects_probe_2026-03-30_v2.jsonl`

## Latest Notable Changes (Object Ingress Post-EnableSimulator Seed-Cap Follow-Up)
- **Primary simulator region-cap parity moved forward**: the default seed-cap request now returns `InterestList`, `RegionObjects`, and `UntrustedSimulatorMessage` on the primary simulator-host `:12043` lane in addition to the previously surfaced caps.
- **Explicit seed-cap follow-up support now exists**: `viewer_net` can now fetch seed capabilities from an explicit URL, and `viewer_app` can perform bounded follow-up if EventQueue ever surfaces `EstablishAgentCommunication` seed-cap targets.
- **The current live gap is now sharper**:
  - `EstablishAgentCommunication` still did not appear
  - EventQueue still only surfaced `AgentGroupDataUpdate`, `AgentStateUpdate`, `EnableSimulator`, and `ParcelProperties`
  - `RegionHandshake`, `RegionHandshakeReply`, and `ObjectUpdate*` still remained absent
- **New best next branch**: `docs/plans/PLAN_OBJECT_INGRESS_REGION_OBJECTS_CAP_PROBE_2026-03-30.md`
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_app`
  - bounded connected run with captured logs:
    - `artifacts/logs/live_post_enable_seedcap_2026-03-30.out.log`
    - `artifacts/logs/live_post_enable_seedcap_2026-03-30.err.log`
    - `artifacts/logs/network_debug_post_enable_seedcap_2026-03-30.jsonl`

## Latest Notable Changes (Object Ingress Network Debug And Decision Acceleration)
- **A dedicated in-app `Network Debug` window now exists**: `viewer_ui` renders bounded sections for session state, capabilities/EventQueue, LLUDP forensics, follow-up activity, and recent network events.
- **Network-debug state is now typed and shared cleanly**: `viewer_core` exposes `NetworkDebugState`, and `viewer_app` owns aggregation so the UI stays decoupled from transport internals.
- **Network-debug events now persist to JSONL**: network-focused relay categories append to `logs/network_debug.jsonl` by default, or to `VIEWER_NETWORK_DEBUG_LOG_PATH` when overridden.
- **The bounded live run verified the backing feed**: `artifacts/logs/network_debug_2026-03-30.jsonl` captured `39` entries showing retained first-simulator socket continuity, capability inventory, EventQueue responses through `ack_out=4`, and repeated `EnableSimulator` details/follow-up.
- **The object-ingress blocker is unchanged**:
  - `RegionHandshake = none`
  - `RegionHandshakeReply = none`
  - `ObjectUpdate* = none`
  - `update_messages=0`
  - `total_objects=0`
- **Next active plan**: `docs/plans/PLAN_OBJECT_INGRESS_POST_ENABLE_SIMULATOR_SEEDCAP_2026-03-30.md`
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_core -p viewer_app -p viewer_ui -p viewer_net`
  - `cargo test -p viewer_core`
  - `cargo test -p viewer_ui`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_app`
  - bounded connected run with captured logs:
    - `artifacts/logs/live_network_debug_window_2026-03-30.out.log`
    - `artifacts/logs/live_network_debug_window_2026-03-30.err.log`
    - `artifacts/logs/network_debug_2026-03-30.jsonl`

## Latest Notable Changes (Object Ingress EventQueue And EnableSimulator Port Follow-Up)
- **A real simulator/world-data opening now exists**: the viewer now ingests structured simulator-host EventQueue data, not just message-name counts.
- **Nested EventQueue body preservation is now live**: bounded runs now surface nested `ParcelProperties` fields and port-only `EnableSimulator` details rather than dropping those bodies during parse.
- **`EnableSimulator` is now concrete on our path**: the bounded March 30, 2026 run surfaces repeated `SimulatorInfo[0].Port` values `13013`, `13000`, and `13001`.
- **Bounded port follow-up is now implemented**: the worker sends one-shot `UseCircuitCode` follow-up to newly seen EventQueue `EnableSimulator` ports on the current simulator host.
- **LLUDP object ingress is still blocked after that follow-up**:
  - `RegionHandshake = none`
  - `RegionHandshakeReply = none`
  - `ObjectUpdate* = none`
  - `update_messages=0`
  - `total_objects=0`
- **Next active plan**: `docs/plans/PLAN_OBJECT_INGRESS_POST_ENABLE_SIMULATOR_SEEDCAP_2026-03-30.md`
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_app`
  - bounded connected runs:
    - `artifacts/logs/live_event_queue_control_consumption_2026-03-30_bounded.out.log`
    - `artifacts/logs/live_enable_simulator_port_followup_2026-03-30.out.log`

## Latest Notable Changes (Object Ingress Parallel Protocol Startup Investigation)
- **Cross-protocol startup diagnostics are now live**: the bounded worker now reports seed-cap fetches, capability-family inventory, simulator-host `EventQueueGet` scheduling, and the existing LLUDP startup timeline in one place.
- **The current capability lane is now explicit**: the March 30, 2026 bounded run shows `EventQueueGet`, `AgentProfile`, `GetDisplayNames`, `SimulatorFeatures`, and `MapLayer` all resolving to simulator-host HTTPS on `:12043`, while `GetTexture` and `ViewerAsset` resolve to asset CDN URLs.
- **The current gap is narrower than “missing all caps”**: the same run shows `EventQueueGet:start ack=0` on the simulator-host URL, but no surfaced `EventQueueGet` completion before shutdown, while `RegionHandshake`, `RegionHandshakeReply`, and `ObjectUpdate*` all remain absent.
- **Firestorm evidence now points at the next branch**: source and `Firestorms.pcapng` confirm substantial simulator-host `:12043` activity in parallel with LLUDP, with Firestorm’s `EventQueueGet` implemented as a persistent `LLEventPoll`.
- **Next active plan**: `docs/plans/PLAN_OBJECT_INGRESS_PERSISTENT_EVENT_QUEUE_2026-03-30.md`
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_app`
  - bounded connected run with captured logs:
    - `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`
    - `artifacts/logs/live_parallel_protocol_2026-03-30.out.log`
    - `artifacts/logs/live_parallel_protocol_2026-03-30.err.log`
  - `tshark` extraction:
    - `artifacts/logs/firestorm_parallel_protocol_timing_2026-03-30.csv`

## Latest Notable Changes (Object Ingress ACK Flush Timing)
- **Explicit ACK flush path added**: `viewer_net` now sends bounded explicit `PacketAck` datagrams on the active first-simulator `SocialCircuit`, and `viewer_app` triggers that flush at the approved startup and first steady-state checkpoints.
- **The ACK queue now drains**: the bounded March 30, 2026 run showed startup `pending=0`, a first steady-state flush of `3` ACK IDs, and first steady-state `pending=0`.
- **Object ingress remained blocked**: even after the queue drain, the run still ended with `update_messages=0 total_objects=0 region_handshake_updates=0`.
- **New clue surfaced after the flush**: previously unnamed packets `0x00000016` and `0xffff0105` appeared in the first steady-state window; Firestorm message-template lookup identifies them as `CameraConstraint` and `GenericMessage`.
- **Next active plan**: `docs/plans/PLAN_OBJECT_INGRESS_POST_ACK_UNKNOWN_CLASSIFICATION_2026-03-30.md`
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_app`
  - bounded connected run with captured log: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`

## Latest Notable Changes (Object Ingress ACK/Receive Forensics Completed)
- **Bounded startup forensics are now live**: `viewer_net` and `viewer_app` now emit ACK queue, appended-ACK usage, raw packet-number, unclassified-packet, first-observation timeline, and ordered transcript summaries for the first-simulator startup path.
- **The live bounded run narrowed the next branch**: the March 30, 2026 connected run showed `unclassified=none` at startup and first steady state, while `RegionHandshake` and `ObjectUpdate*` still remained absent.
- **ACK timing is now the selected next behavior-change target**: the same run showed pending ACK IDs growing from `4` to `8` while appended ACK usage occurred only once, so the next active plan is `docs/plans/PLAN_OBJECT_INGRESS_ACK_FLUSH_TIMING_2026-03-30.md`.
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_app`
  - bounded connected run: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`

## Latest Notable Changes (Gemini CLI Repo Integration)
- **Gemini CLI is now a documented second-eye tool**: `docs/agents/GEMINI.md` now describes the local CLI workflow, verified smoke command, recommended headless usage, and explicitly forbids Gemini from being the primary planner or decision-maker.
- **Codex discovery path is explicit**: `docs/agents/CODEX.md` and `docs/agents/RUNBOOK.md` now point Codex toward Gemini CLI only for second-eye review and bounded repo-risk analysis.
- **Repo-local Gemini skill added for Codex**: `.agents/skills/use_gemini_cli/SKILL.md` now gives Codex a concise triggerable path for invoking Gemini CLI without inventing ad hoc prompts.
- **Validation**:
  - `gemini -p "Reply with exactly the single word OK."`

## Latest Notable Changes (Object Ingress Next-Step Roadmap)
- **Roadmap artifact added for the current blocker**: `docs/plans/PLAN_OBJECT_INGRESS_NEXT_STEP_ROADMAP_2026-03-30.md` turns the object-ingress investigation into an explicit decision tree.
- **The roadmap did its job**: the bounded ACK/control/receive observability slice is now complete and used to choose the next active branch.
- **Selected branch is explicit**: the next behavior-change plan is `docs/plans/PLAN_OBJECT_INGRESS_ACK_FLUSH_TIMING_2026-03-30.md`.
- **Plan audit added to the roadmap**: older startup/socket plans with unmatched report names were reconciled as completed or superseded, and the current active unfinished implementation plan is now `PLAN_OBJECT_INGRESS_ACK_FLUSH_TIMING_2026-03-30.md`.

## Latest Notable Changes (Object Ingress ACK/Receive Planning)
- **Remaining concerns are now unified in one plan**: `docs/plans/PLAN_OBJECT_INGRESS_ACK_AND_RECEIVE_FORENSICS_2026-03-30.md` addresses ACK timing, receive-path observability gaps, `RegionHandshake` progression, and startup ordering as one staged investigation surface.
- **Next step is observability, not another startup message**: after `AgentHeightWidth`, `SetAlwaysRun`, and the observed startup `AgentAnimation` all failed independently, the next approved direction is bounded ACK/control/receive transcript instrumentation on the same one-port path.

## Latest Notable Changes (Object Ingress AgentAnimation)
- **Third control-block promotion added**: `viewer_net` now sends the observed startup `AgentAnimation` packet in the control flow after `AgentUpdate` and before `SetAlwaysRun`.
- **Targeted coverage updated**: startup interest tests now lock the ordered quintet `AgentThrottle` -> `AgentHeightWidth` -> `AgentUpdate` -> `AgentAnimation` -> `SetAlwaysRun`.
- **Connected result remained blocked**: the bounded March 30, 2026 run stayed on one port (`59553`, `split=false`), but `update_messages=0 total_objects=0 region_handshake_updates=0` still did not change.
- **Current state**: the staged control-block message promotions are now exhausted without restoring object ingress. The next likely blocker is ACK/control-reply behavior rather than another single startup message.
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_app`
  - bounded connected run with captured log: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`

## Latest Notable Changes (Object Ingress SetAlwaysRun)
- **Second control-block promotion added**: `viewer_net` now sends `SetAlwaysRun` (`Low 88`) in the startup control flow after `AgentUpdate`.
- **Targeted coverage updated**: startup interest tests now lock the ordered quartet `AgentThrottle` -> `AgentHeightWidth` -> `AgentUpdate` -> `SetAlwaysRun`.
- **Connected result remained blocked**: the bounded March 30, 2026 run stayed on one port (`49475`, `split=false`) and showed `SetAlwaysRun` on-wire (`0xffff0058`), but `update_messages=0 total_objects=0 region_handshake_updates=0` still did not change.
- **Current state**: `SetAlwaysRun` alone is not sufficient. The later `AgentAnimation` promotion is now also ruled out as a sufficient fix.
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_app`
  - bounded connected run: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`

## Latest Notable Changes (Object Ingress AgentHeightWidth)
- **First control-block promotion added**: `viewer_net` now sends `AgentHeightWidth` (`Low 83`) in startup interest flow, placed after `AgentThrottle` and before `AgentUpdate`.
- **Targeted coverage updated**: startup interest tests now lock the ordered trio `AgentThrottle` -> `AgentHeightWidth` -> `AgentUpdate`.
- **Connected result remained blocked**: the bounded March 30, 2026 run stayed on one port (`63178`, `split=false`), but `update_messages=0 total_objects=0 region_handshake_updates=0` still did not change.
- **Current state**: `AgentHeightWidth` alone is not sufficient. The later `SetAlwaysRun` promotion is now also ruled out as a sufficient fix.
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_app`
  - bounded connected run with captured log: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`

## Latest Notable Changes (Object Ingress Control-Block Planning)
- **New Firestorm pcap refined the working pre-burst block**: `Test.pcapng` on `16.144.39.130:13001` shows a richer working sequence than the earlier captures, including `AgentHeightWidth` between `AgentThrottle` and `AgentUpdate`.
- **Next plan is deliberately incremental**: `docs/plans/PLAN_OBJECT_INGRESS_CONTROL_BLOCK_PARITY_2026-03-30.md` stages the control-block work so we add one packet at a time instead of mirroring the whole block.
- **First promotion candidate is explicit**: `AgentHeightWidth` (`Low 83`) is the next smallest newly confirmed missing packet on the working Firestorm path.

## Latest Notable Changes (Object Ingress Post-AMC Request Parity)
- **Bounded startup request subset added**: `viewer_net` and `viewer_app` now send startup `MuteListRequest`, `MoneyBalanceRequest`, and `AgentDataUpdateRequest` on the active retained first-simulator circuit.
- **`LayerData` is now a named inbound kind**: first-simulator classification can now surface `LayerData` in startup receive summaries instead of leaving it implicit in packet captures.
- **Connected result remained blocked**: the bounded March 30, 2026 run on local port `53392` showed the new startup requests on-wire (`0xffff0106`, `0xffff0139`, `0xffff0182`) but still reported `update_messages=0 total_objects=0 region_handshake_updates=0`.
- **Current state**: the narrowed startup request subset is not sufficient by itself. The first staged control-block promotion (`AgentHeightWidth`) is now also ruled out as a sufficient fix.
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_app`
  - bounded connected run: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`

## Latest Notable Changes (Object Ingress Runtime Socket Forensics)
- **Bounded first-simulator socket diagnostics added**: `viewer_net` now records first-simulator probe bind, fresh bind, retain, reuse, send, and receive events with local address/port context.
- **Worker relay summaries added**: `viewer_app` now emits startup and first-steady-state socket summaries so connected runs expose the actual local UDP port path.
- **Live runtime question answered**: the bounded March 30, 2026 connected run showed a single local port `65241` across `after_probe`, `after_open_social_circuit`, `after_startup_social_prime`, and `after_first_steady_state_window`, all with `split=false`.
- **Blocked state remained unchanged**: even on that one-port path, `object_feed` stayed `update_messages=0 total_objects=0 region_handshake_updates=0`.
- **Wire evidence refined the blocker further**: `improved.pcapng` confirms the app receives same-port simulator traffic including repeated `LayerData`, `CoarseLocationUpdate`, `AttachedSound`, `ViewerEffect`, `TestMessage`, `SimulatorViewerTimeMessage`, `ChatFromSimulator`, `AgentDataUpdate`, and `AgentMovementComplete`, but still no `ObjectUpdate*`.
- **Current state**: runtime split-port/socket continuity is no longer the best live explanation for missing object ingress on the current code path. The completed next slice was `docs/plans/PLAN_OBJECT_INGRESS_POST_AMC_REQUEST_PARITY_2026-03-30.md`.
- **Validation**:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_app`
  - bounded connected run: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`

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







