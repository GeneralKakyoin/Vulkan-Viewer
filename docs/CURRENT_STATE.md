# CURRENT_STATE.md

## Current Phase
Phase E - App-owned live startup orchestration path

Real login compatibility is now proven against the live Second Life endpoint. The active focus has moved to safe post-login bootstrap sequencing, beginning with seed capability handling.
Bootstrap capability probing is now sufficiently characterized to begin first-simulator handshake sequencing research/documentation.

---

## Working

### Runtime / Rendering
- renderer/ui/input/camera stack
- sandbox scene
- multiple objects
- ground plane
- cube
- world-axis marker
- depth buffer
- corrected controls
- viewer_app now supports a bounded in-process live-state feed (primary path) with file fallback:
  - spawns a narrow background live-feed worker when login env vars are present
  - uses real `viewer_net` login/bootstrap/handshake probe state in-process
  - falls back to sanitized snapshot ingest (`live_visual_snapshot.json`) when in-process feed is unavailable
  - startup mode control now exists:
    - `VIEWER_APP_LIVE_STARTUP=auto` (default)
    - `VIEWER_APP_LIVE_STARTUP=on`
    - `VIEWER_APP_LIVE_STARTUP=off`
  - app now surfaces explicit startup status in UI (`starting`, `connected`, `disabled`, `failed`)
  - debug overlay displays live-derived connection/handshake/traffic summary fields
- sandbox cube visual indicator now reflects live state:
    - default/offline: red, scale 1.0
    - logged-in but pre-AMC: yellow, scale 1.1
    - AMC reached: green, scale 1.25
- first world-facing live placeholder marker now renders from the in-process live state bridge:
  - dedicated scene role (`LivePlaceholder`) separate from static sandbox geometry
  - world-space axis marker appears even before broad world/object decoding
  - placeholder transform/color are driven by login/handshake/region-derived snapshot fields
- first bounded world-state bridge beyond placeholders now exists in `viewer_core`:
  - typed region/world-entry-facing model:
    - `FirstRegionPresence`
    - `WorldEntryStage` (`Offline`, `Connected`, `EnteredFirstRegion`)
  - additional world-space diagnostics rendered from typed state:
    - `WorldRegionAnchor` marker
    - `WorldEntryBeacon` marker
  - markers are driven only by already-proven sanitized live inputs (login, AMC, region endpoint/coords)
  - implementation remains diagnostic-first and intentionally stops before broad world/object decode
- world-facing composition is now richer than a single entry marker:
  - simulator-target landmark (`WorldSimTargetMarker`) is derived from first-simulator target identity
  - three bounded traffic pillars are rendered in world space from existing traffic summaries:
    - `WorldTrafficBroaderPillar`
    - `WorldTrafficUnknownPillar`
    - `WorldTrafficRegionControlPillar`
  - the scene now communicates stage + region + target + bounded traffic shape without broad world/object ingestion
- first typed world/object-state ingestion seam now exists in `viewer_core`:
  - seam model:
    - `WorldObjectIngestionSeam`
    - `WorldObjectIngestionItem`
    - `WorldObjectIngestionLane`
  - seam input remains bounded to existing sanitized diagnostics (`WorldDiagnosticSlice`)
  - seam output drives a dedicated scene placeholder role:
    - `WorldIngestionProxy`
  - this creates a clear, testable ingress boundary for future world/object state without broad decode
- seam is now explicitly runtime-fed in normal app flow:
  - `viewer_app` builds seam items each frame via `WorldObjectIngestionAdapter::adapt(snapshot)`
  - `viewer_app` applies seam to `Scene::apply_world_object_ingestion_seam(...)`
  - `WorldIngestionProxy` is no longer implicitly created by snapshot mapping internals; it is fed through the seam path
- app-owned startup orchestration is now explicit and first-class:
  - app owns startup mode selection and config gating
  - app receives startup status + snapshots from worker updates
  - app maps runtime state into seam-fed scene path every frame
- seam now carries one narrow typed payload category beyond generic proxy state:
  - new lane: `TrafficSignalPayload`
  - typed payload fields:
    - `traffic_broader_count`
    - `traffic_unknown_count`
    - `traffic_region_control_count`
  - scene reflection:
    - seam-owned marker `WorldIngestionTrafficPayload`
    - marker transform/scale/color driven by typed traffic payload
- seam now carries a second, more object-facing bounded payload category:
  - lane: `FirstRegionPresenceProxy` remains as entry proxy payload
  - lane: `TrafficSignalPayload` remains as traffic-shaped payload
  - new object-facing seam-owned marker behavior now composes both categories:
    - `WorldIngestionProxy` (entry/presence object-like proxy)
    - `WorldIngestionTrafficPayload` (typed traffic payload marker)
  - seam lanes now represent multi-category payload ingestion behavior while staying pre-decode
- seam now carries a first tiny decoded world/object-facing input lane:
  - new lane: `DecodedSimulatorEndpointPayload`
  - decoded input is bounded and derived only from existing sanitized live state:
    - `first_sim_endpoint` host tail octet
    - `first_sim_endpoint` port
  - scene reflection:
    - seam-owned marker `WorldIngestionDecodedEndpointPayload`
    - marker transform/color are driven by decoded endpoint values
  - this remains diagnostic-first and does not introduce broad object/world decoding

### Architecture
- clear crate boundaries
- `viewer_app` remains orchestration-only
- `viewer_render` owns rendering
- `viewer_ui` owns egui/debug UI
- `viewer_net` owns transport/session
- `viewer_grid` owns grid-specific semantics

### Login Stack
- HTTP login transport
- redirect handling
- typed login request/result/bootstrap models
- login diagnostics/trace
- JSON codec
- minimal LLSD codec
- experimental XML-RPC `login_to_simulator` codec path in `viewer_net`
- LLSD `passwd` normalization to legacy `$1$<md5>` form
- XML-RPC credential shaping normalized to legacy `first`/`last` semantics
- configurable wire format
- manual login attempt example
- successful real live login achieved
- minimal seed capability fetch transport path implemented in `viewer_net`
- one-shot EventQueueGet inspection path implemented in `viewer_net`
- one-shot SimulatorFeatures inspection path implemented in `viewer_net`
- one-shot MapLayer inspection path implemented in `viewer_net`
- capability one-shot requests now send explicit LLSD `Accept` headers
- MapLayer HTTP 405 is now classified explicitly as likely non-HTTP/legacy-UDP semantics in this one-shot transport path
- typed first-simulator handshake scaffold is now implemented in `viewer_net`:
  - bootstrap prerequisites ready
  - first region target known
  - `UseCircuitCode` stage
  - `CompleteAgentMovement` stage
  - waiting-for-`AgentMovementComplete` stage
  - terminal `AgentMovementComplete` stage
- strict ordered handshake-stage transition API exists with focused test coverage
- first-simulator handshake stages are now transport-bound in `viewer_net`:
  - real `UseCircuitCode` send action (UDP datagram transport scaffold)
  - real `CompleteAgentMovement` send action (UDP datagram transport scaffold)
  - ordered send flow enforced by stage-aware send APIs
  - per-send diagnostics captured for attempts/success/failure
- receive-side handshake observation/classification slice is now implemented in `viewer_net`:
  - one-shot UDP receive hook for handshake datagrams
  - typed inbound message classification (`AgentMovementComplete`, `RegionHandshake`, `EnableSimulator`, `Irrelevant`)
  - typed receive diagnostics (classification signal, stage before/after, stage advancement flag)
  - explicit `AgentMovementComplete` receive-side stage confirmation when waiting
- receive-side handshake classification now includes a minimal typed LLUDP packet-header/message-number decoder:
  - decodes high/medium/low message-number forms from simulator UDP packet shape
  - classifies handshake-relevant low-frequency messages by packet message number:
    - `RegionHandshake` (low 148)
    - `EnableSimulator` (low 151)
    - `AgentMovementComplete` (low 250)
  - keeps JSON/text matching only as compatibility fallback (no longer primary path)
- receive-side diagnostics now include decode evidence details:
  - decode source (`PacketMessageNumber`, `JsonField`, `TextScan`, `Unknown`)
  - optional decoded packet message number for unknown packet-shaped traffic
- receive-side diagnostics now include explicit traffic-scope classification:
  - `BootstrapRelevant`
  - `TransportControl`
  - `LikelyBroaderTraffic`
  - `Unknown`
- small typed early-simulator-traffic scaffold now exists in `viewer_net`:
  - `EarlySimulatorTrafficKind`
  - `EarlySimulatorTrafficObservation`
  - currently routed kinds:
    - `HealthMessage`
    - `SimulatorViewerTimeMessage`
    - `OnlineNotification`
    - `ViewerEffect`
    - `CoarseLocationUpdate`
    - `AttachedSound`
  - available via `Connection::early_simulator_traffic_observations()`
  - summary available via `Connection::summarize_early_simulator_traffic()`
- bounded region-transition control visibility diagnostics now exist:
  - dedicated scope: `RegionTransitionControl`
  - run-level summary available via `Connection::summarize_region_transition_control()`
  - post-boundary summary includes:
    - `region_transition_control` count
    - `crossed_region` count
    - `confirm_enable_simulator` count
    - `watched_region_transition_control_not_seen` flag
- handshake-stage confirmation was tightened:
  - `AgentMovementComplete` stage advancement now requires packet-message decode evidence (not JSON/text fallback alone)
- one-shot probe path now exists to send `UseCircuitCode` + `CompleteAgentMovement` and wait on the same UDP socket:
  - `Connection::probe_first_simulator_handshake_once(...)`
  - manual example wiring added for live receive validation
- bounded receive-window probe now exists on the same socket:
  - `Connection::probe_first_simulator_handshake_window(...)`
  - bounded packet count + timeout with ordered observation reporting
  - receive diagnostics now include `observation_index`
- bounded post-movement tail probing now exists (opt-in):
  - `Connection::probe_first_simulator_handshake_window_with_tail(...)`
  - default bounded-window behavior remains unchanged (`tail=0`)
  - probe report now includes first `AgentMovementComplete` index and post-movement observation count
- policy-based bounded receive probing now exists for targeted handoff/control visibility:
  - `Connection::probe_first_simulator_handshake_window_with_policy(...)`
  - supports optional post-AMC timeout override (for sparse control packets)
  - supports optional early-stop when first `RegionTransitionControl` signal is seen
  - keeps probing bounded by packet cap and timeout controls (no polling loop)
- outbound handshake send packets are now protocol-shaped binary LLUDP datagrams:
  - reliable LLUDP flags + packet-id header
  - low-frequency message numbers for `UseCircuitCode` (3) and `CompleteAgentMovement` (249)
  - field ordering/layout aligned to template expectations (UUID + U32 blocks)
  - per-send diagnostics now include packet id and packet message number
- live same-socket probe now receives parseable real inbound simulator traffic:
  - inbound low message `387` classified as `AgentDataUpdate`
  - inbound low message `1` classified as `TestMessage`
  - inbound low message `250` classified as `AgentMovementComplete`
  - inbound classification source is typed packet message-number decode
  - handshake stage now advances to `AgentMovementComplete` in live bounded-window probe runs
  - typed inbound classification now also includes:
    - `HealthMessage` (low 138)
    - `SimulatorViewerTimeMessage` (low 150)
- live bounded post-movement tail probe now captures immediate post-movement traffic:
  - with bounded post-AMC tails (`tail=2..4`), observed sequence examples include:
    - `AgentDataUpdate` -> `TestMessage` -> `AgentMovementComplete` -> `PacketAck` -> `HealthMessage`
    - additional early post-AMC traffic now typed as likely broader traffic:
      - `OnlineNotification` (`0xFFFF0142`, low 322)
      - `ViewerEffect` (`0x0000FF11`, medium 17)
      - `CoarseLocationUpdate` (`0x0000FF06`, medium 6)
      - `AttachedSound` (`0x0000FF0D`, medium 13)
    - remaining medium unknowns can still appear in this window and remain explicitly `Unknown`
  - `0xfffffffb` is now typed as `PacketAck` (transport-control) instead of unmapped traffic
  - known typed packets are now explicitly separated from likely broader traffic in diagnostics
  - bounded probe report now includes post-boundary summary counters and ordered post-boundary kinds
  - post-boundary summary now also captures unknown packet message numbers when unknown traffic appears
  - post-boundary summary now captures repeated unknown packet numbers for faster stabilization checks
  - early-traffic observations are now captured as a separate typed layer (beyond bootstrap/control)
- deterministic regression coverage now protects the traffic-phase boundary model:
  - fixture-backed probe-window test verifies `RegionTransitionControl` separation from broader traffic
  - repeated unknown post-boundary packet-number reporting is now asserted (including repeated-ID collapse)
  - region-transition summary `not_seen_in_run` vs observed-path behavior is covered
  - scope-stop behavior remains explicit by keeping unmapped packet IDs in `Unknown` (no world/object decode expansion)
  - policy probe behavior is covered for:
    - post-AMC timeout override
    - optional early-stop on first observed region-transition control packet
- sanitized live visual snapshot production remains in the manual `viewer_net` path (fallback/dev tool):
  - `llsd_login_attempt` writes `LiveVisualSnapshot` JSON after successful login/probe
  - output path: `VIEWER_LIVE_VISUAL_SNAPSHOT_PATH` or default `live_visual_snapshot.json`
  - snapshot excludes sensitive session IDs/seed URLs

### Research / Continuity
- Firestorm login flow documented
- Firestorm first-simulator handshake sequence documented
- continuity-stack docs established
- scope model established
- workflow discipline defined in AGENTS
- autonomous milestone-driven execution policy clarified in AGENTS
- repo Codex config and low-cost subagent profiles added for bounded worker/explorer use
- skills policy now explicitly checks `.agents/skills/viewer/` first (then `.agents/skills/` fallback)

---

## Current Blocker

The immediate blocker is no longer login/bootstrap transport viability. The current blocker is phase control:

- we now have connected live diagnostics, a meaningful world-facing composition, a typed ingestion seam, and explicit app-owned startup orchestration
- seam feed is now explicit in orchestration/runtime path (app -> seam adapter -> scene seam application)
- first tiny decoded seam input is now implemented; next steps must decide between:
  - adding one more bounded decoded payload category, or
  - starting the first minimal real simulator payload decode into the same seam
- either path must still avoid broad object/world decoding
- hard stop remains: no broad object/world-state protocol ingestion in this scope

---

## Most Likely Immediate Work

- extend seam payload categories beyond decoded endpoint input while preserving current bounded seam ownership
- keep the seam reversible and testable before introducing any broader decode paths
- keep deriving state only from already-proven sanitized live fields
- strengthen scene mapping tests for role/marker stability
- optionally expose the new bounded state summary in debug UI without adding product UI features
- preserve strict stop-line before broad world/object protocol ingestion

---

## Current Next Step

The smallest correct next step is:

**add one more bounded decoded payload through the existing seam (or first tiny simulator-payload decode), while keeping broad object/world decode out of scope**

---

## Do Not Do Yet

- simulator connection
- event queue
- world streaming
- login UI integration
- broad viewer feature work
