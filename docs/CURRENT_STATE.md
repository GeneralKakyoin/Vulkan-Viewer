# Current State: Vulkan-Viewer

## Overview
The Vulkan-Viewer is a high-performance Second Life compatible viewer built in Rust. It currently supports basic region and avatar presence, nearby chat, direct IM, avatar profiles, and a robust diagnostics shell.

## Current Position (2026-04-01)
- `RegionObjects` typed-feed ingestion is working and live-validated (`typed_sample=...` appears in bounded connected logs).
- LLUDP world-object ingress remains blocked (`RegionHandshake`, `RegionHandshakeReply`, and `ObjectUpdate*` still absent in bounded runs).
- Reconnect teleport controls are working for evidence capture, and current region identity is surfaced in diagnostics.
- A broader-SLURL capture (`secondlife://Morris/128/128/25`) showed a mixed post-reconnect result: pre-teleport `typed_sample=...` was present, but the first post-reconnect `RegionObjects` startup probe returned `typed_sample=none`.
- A reconnect-only delayed `RegionObjects` re-probe path is now implemented and live-validated via `cargo run -p viewer_app`.
- In the latest authoritative run, post-reconnect delayed re-probe still returned `typed_sample=none` on the same simhost (`simhost-0a962ce03cdb50c3e...`), so timing alone is not yet sufficient to explain emptiness.

## Next Steps (Recommended Order)
1. Treat post-reconnect emptiness as a region/capability-behavior question, not a pure timing question, for this target.
2. Run one bounded capture against a different SLURL target and compare `primary probe` + `re-probe` results side-by-side.
3. If a target yields stable post-reconnect typed samples, continue typed-field promotion; otherwise document this lane as region-dependent and decide whether to prioritize LLUDP branching.

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
  - none for this slice; authoritative `cargo run -p viewer_app` live validation completed

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

