# TASKS.md

## Current Focus
Only work on the smallest steps that advance bounded world-facing live diagnostics while preserving crate boundaries and stopping before broad object/world decoding.

Latest priority shift:
- avatar placeholder slice is now in place (coarse + self + projected labels)
- next smallest step is improving placeholder fidelity (asset-backed proxies) without widening transport scope

Status note:
- bounded pre-world object/state phase is now substantially complete
- bounded broader-ingestion phase is now substantially complete:
  - coarse-neighborhood sampling now carries bounded second+third coarse samples
  - mapping is in place and test-protected across decode -> snapshot -> seam (`DecodedCoarseNeighborhoodPayload`) -> scene neighborhood-family roles
- next phase should be a narrower bounded world/object refinement step that stays below broad object/world decoding

---

## Active Tasks

### T0 - Avatar Placeholder V1 stabilization
Why it matters:
The first in-world avatar visibility path now exists and should be stabilized before broader world decoding.

Dependencies:
- current coarse decode summary in `viewer_net`
- current app merge/supervision and label projection behavior

Done when:
- placeholder lifecycle (seen/updated/stale/removed) remains stable through reconnect cycles
- self placeholder remains present even when coarse ID blocks are missing
- label projection remains readable while camera moves
- no regressions in chat/IM/profile windows

---

### T1 - Seed capability bootstrap kickoff
Why it matters:
Live login is proven; seed capability bootstrap is now the critical path.

Dependencies:
- successful real login path
- existing transport/session diagnostics

Done when:
- first seed capability request is issued safely
- response is captured and sanitized
- response is parseable and classified at the correct boundary

---

### T2 - Define capability boundary ownership
Why it matters:
Bootstrap logic can easily erode `viewer_net` vs `viewer_grid` boundaries.

Dependencies:
- T1
- Firestorm behavior reference for ordering only

Done when:
- transport mechanics stay in `viewer_net`
- capability meaning/typing stays in `viewer_grid`
- boundary is documented clearly

---

### T3 - Bootstrap diagnostics and fixtures
Why it matters:
Post-login work needs reproducible traces and tests.

Dependencies:
- T1/T2 findings

Done when:
- capability/bootstrap traces are sanitized and useful
- focused tests/fixtures cover current bootstrap assumptions
- non-viable HTTP probe paths are explicitly classified (for example, MapLayer likely legacy/non-HTTP in this phase)

---

### T4 - Continuity updates for bootstrap phase
Why it matters:
Milestone transition must be obvious to future agents.

Dependencies:
- every meaningful bootstrap change

Done when:
- `docs/CURRENT_STATE.md` updated
- `docs/HANDOFF.md` updated
- `docs/MASTER_PLAN.md` updated if project priority/milestone status changes
- relevant `docs/RESEARCH/*` updated when protocol understanding changes

### T5 - First-simulator handshake research kickoff
Why it matters:
Bootstrap inspection is now characterized enough that the next risk is handshake sequencing ambiguity.

Dependencies:
- bootstrap capability findings
- successful real login/session bootstrap

Done when:
- first-region handshake ordering is documented from Firestorm behavior reference
- dependencies between seed grant, circuit setup, and movement completion are explicit
- crate-boundary ownership is mapped without starting simulator transport implementation
- typed handshake-stage scaffold exists in `viewer_net` with focused transition tests

---

## Next Tasks After Bootstrap Credibility

### T6 - Event queue startup
Done when:
- minimal `EventQueueGet` polling works
- early live events can be received and classified

### Near-term blocker focus (within current bootstrap inspection)
- stabilize one-shot `SimulatorFeatures` fetch against current live `503 Service Unavailable`
- stabilize one-shot `EventQueueGet` against current retryable upstream/proxy failures
- treat `MapLayer` HTTP one-shot as classified blocker (likely legacy/non-HTTP path) unless new protocol evidence emerges

### T7 - Simulator handshake preparation
Done when:
- simulator handshake requirements are documented
- typed first-simulator handshake stages are bound to minimal transport send/ack actions in `viewer_net`
- send-side handshake diagnostics exist for attempts/success/failure
- receive-side handshake observation/classification exists and can drive `AgentMovementComplete` stage confirmation
- receive-side handshake identity classification is driven by typed UDP message-number decode (not heuristic-only matching)
- handshake work is scoped without collapsing crate boundaries

### T7.1 - Handshake payload decode fidelity
Done when:
- `AgentMovementComplete` receive handling includes minimal typed block/field extraction (beyond message identity)
- receive diagnostics can surface typed decode success/failure for handshake payload fields
- out-of-order/irrelevant message handling remains stable with focused tests

### T7.2 - Live handshake probe fidelity
Done when:
- first-simulator one-shot probe sends handshake messages and listens on the same socket/port
- live manual probe outcome is captured with explicit diagnostics
- if no inbound packets are observed, blocker is classified clearly (transport continuity vs outbound payload fidelity)

### T7.3 - Handshake progression observation
Done when:
- first live inbound handshake-relevant packet after wire-fidelity pass is classified (currently `AgentDataUpdate`)
- receive-side typed classification covers observed early packets without heuristic dependence
- next blocker from `AgentDataUpdate` to `AgentMovementComplete` is explicitly characterized

Status update:
- materially advanced: bounded probe now observes `AgentDataUpdate` -> `TestMessage` -> `AgentMovementComplete` and stage advancement is confirmed in live runs

### T7.4 - Early post-movement inbound typing
Done when:
- the next inbound packets after initial `AgentMovementComplete` are captured with bounded diagnostics
- typed classification is expanded only for the small set repeatedly observed in this stage
- progression remains handshake/bootstrap-focused (no world-state subsystem implementation)

Status update:
- materially advanced: bounded post-movement tail capture now exists (`probe_first_simulator_handshake_window_with_tail`)
- live tail run (`tail=2`) observed post-movement packets beyond handshake completion, including:
  - `PacketAck` (`0xfffffffb`) now typed as transport-control
  - `HealthMessage` (typed, likely broader traffic)
- diagnostics now separate:
  - bootstrap-relevant traffic
  - transport-control traffic
  - likely broader traffic
- additional repeated post-AMC IDs are now typed where justified:
  - `OnlineNotification` (`0xffff0142`) as likely broader traffic
  - `ViewerEffect` (`0x0000ff11`) as likely broader traffic
- `CoarseLocationUpdate` (`0x0000ff06`) is now typed as likely broader traffic
- probe reports now include post-boundary summary counts + ordered post-boundary kinds for durable diagnostics
- small typed early-simulator-traffic scaffold now exists in `viewer_net` and captures observed broader packet kinds separately from bootstrap/control diagnostics
- scaffold diagnostics now include per-kind early-traffic summary counts
- phase status: early simulator traffic consolidation is done enough for current scope
- repeated unknown medium `0x0000ff0d` is now typed as `AttachedSound` (likely broader traffic)
- post-boundary diagnostics now include repeated unknown packet-number reporting
- region-transition control visibility diagnostics now exist with explicit `not_seen_in_run` reporting
- message-ID typing rails are ready for:
  - `CrossedRegion` (`0x0000ff07`)
  - `ConfirmEnableSimulator` (`0x0000ff08`)
- regression durability is now materially improved with deterministic fixture tests covering:
  - boundary scope separation (`BootstrapRelevant` / `TransportControl` / `LikelyBroaderTraffic` / `RegionTransitionControl` / `Unknown`)
  - post-boundary repeated unknown packet-number reporting
  - region-transition summary behavior when watched IDs are observed
- targeted live-observation controls now exist for region-transition visibility:
  - optional post-AMC timeout override in bounded probe policy path
  - optional early-stop when first region-transition control packet is observed
  - this enables longer/sparser control-signal capture without introducing polling loops
- next phase: type only targeted medium handoff IDs (`CrossedRegion` / `ConfirmEnableSimulator`) if repeatedly observed, without broad world/object decoding

### T8 - First connected world slice
Done when:
- world-derived placeholder state can be rendered from live connection data

Status update:
- materially advanced with first live visual slice bridge:
  - shared minimal live visual state model now exists (`LiveVisualSnapshot`)
  - manual `viewer_net` example emits sanitized live snapshot JSON after real login/probe
  - `viewer_app` + `viewer_ui` consume and display this live-derived state
  - sandbox visual indicator now reflects live handshake status (no world/object decoding)
- remaining to complete T8:
  - replace file-based bridge with narrow in-process live-state feed
  - keep scope diagnostic-first and stop before broad object/world-state rendering

Status update (latest):
- in-process live-state feed is now the primary app path when login env vars are present
- file-based snapshot bridge remains as bounded fallback/dev path
- connected-view diagnostics are richer and test-protected (app/core/ui)
- first world-facing placeholder bridge is now implemented:
  - dedicated scene-level `LivePlaceholder` role
  - world-space marker driven by real live-derived state (login/AMC/region-coordinates)
  - focused `viewer_core` tests protect placeholder behavior/mapping
- first bounded world-state bridge beyond placeholders is now implemented in `viewer_core`:
  - typed `FirstRegionPresence` + `WorldEntryStage` model
  - bounded world-space diagnostic roles:
    - `WorldRegionAnchor`
    - `WorldEntryBeacon`
  - focused tests now cover typed mapping and marker behavior
- first meaningful live world-facing diagnostic slice is now implemented:
  - bounded typed `WorldDiagnosticSlice` + `WorldTrafficSummary` model
  - world composition expanded with:
    - `WorldSimTargetMarker`
    - `WorldTrafficBroaderPillar`
    - `WorldTrafficUnknownPillar`
    - `WorldTrafficRegionControlPillar`
  - composition remains diagnostic-first and reversible
  - focused tests now protect traffic-slice mapping and pillar scaling behavior
- first typed world/object-state ingestion seam is now implemented:
  - `WorldObjectIngestionSeam`
  - `WorldObjectIngestionItem`
  - `WorldObjectIngestionLane`
  - seam-driven scene placeholder role: `WorldIngestionProxy`
  - focused tests now protect seam mapping and seam-driven scene behavior
- first bounded real ingestion adapter feed is now implemented:
  - `WorldObjectIngestionAdapter` maps runtime `LiveVisualSnapshot` -> seam
  - `viewer_app` applies seam each frame through explicit seam API
  - seam now owns proxy injection/removal path in scene flow
- first narrow typed payload path through seam is now implemented:
  - new seam lane: `TrafficSignalPayload`
  - seam now carries typed traffic payload counts
  - seam-owned marker `WorldIngestionTrafficPayload` reflects this payload in scene space
- first tiny decoded payload path through seam is now implemented:
  - new seam lane: `DecodedSimulatorEndpointPayload`
  - bounded decode from `first_sim_endpoint` now feeds typed seam fields:
    - decoded host-tail octet
    - decoded port
  - seam-owned marker `WorldIngestionDecodedEndpointPayload` reflects this decoded payload in scene space
- first minimal real simulator-payload decode path is now implemented:
  - new seam lane: `DecodedCoarseLocationPayload`
  - transport decode source: inbound `CoarseLocationUpdate` body in `viewer_net`
  - bounded decoded fields propagated via live snapshot:
    - decoded location count
    - decoded first coarse XYZ sample
  - seam-owned marker `WorldIngestionDecodedCoarseLocationPayload` reflects this simulator-derived decoded payload in scene space
- bounded multi-input simulator-derived seam ingestion is now complete:
  - second seam lane added: `DecodedHealthPayload`
  - transport decode source: inbound `HealthMessage` body in `viewer_net`
  - bounded decoded field propagated via live snapshot:
    - normalized health basis points
  - seam-owned marker `WorldIngestionDecodedHealthPayload` reflects this decoded payload in scene space
- first bounded object-like decoded composition is now complete:
  - seam-owned composite role: `WorldIngestionDecodedCompositeBeacon`
  - composition inputs: `DecodedCoarseLocationPayload` + `DecodedHealthPayload`
  - composition appears only when both decoded lanes are present
- first bounded object/state slice is now complete:
  - seam payload category: `ObjectStateEntitySeedPayload`
  - seam-owned composition roles:
    - `WorldObjectStateEntityBody`
    - `WorldObjectStateEntityAura`
  - payload/category and roles are gated by decoded coarse + health availability
- bounded object/state slice now supports a first narrow multi-entity family:
  - deterministic entity-count gating from decoded coarse count:
    - low coarse count -> primary body+aura
    - medium coarse count -> adds wing body+aura
    - higher coarse count -> adds guard body+aura
  - seam-owned cluster role added:
    - `WorldObjectStateEntityClusterCore`
  - one tiny supporting decoded input added to seam payload:
    - `decoded_coarse_updates` for deterministic bounded spatial variation
- app runtime now applies seam updates on dirty-only changes:
  - seam adaptation still runs in app-owned startup/redraw flow
  - scene seam application is skipped when seam content is unchanged
- bounded object/state slice now includes lifecycle semantics as a dedicated seam lane:
  - new lane: `ObjectStateEntityLifecyclePayload`
  - lane carries bounded counters:
    - `decoded_coarse_updates`
    - `decoded_health_updates`
  - seam-owned lifecycle roles:
    - `WorldObjectStateEntityPulse`
    - `WorldObjectStateEntityStability`
  - deterministic lifecycle phase mapping is now explicit:
    - `Dormant`, `Warming`, `Active`, `Strained`
- app runtime now also applies live snapshot scene updates on dirty-only changes:
  - snapshot apply is skipped when `LiveVisualSnapshot` content is unchanged
- bounded live scene composition now has a first visually meaningful tiny-cluster hierarchy pass:
  - coherent cluster-base layout now improves anchor/center/satellite readability
  - decoded endpoint/coarse/health, seam proxy/composite, traffic payload, and entity family now read as one grouped slice
  - composition is materially less marker-scattered at normal camera distance
- lifecycle readability is now materially improved:
  - lifecycle phase influences visible spacing and intensity
  - active phase increases pulse and satellite spread
  - strained phase increases stability-stress profile
  - focused tests now lock this progression behavior
- bounded seam ingestion now includes one additional tightly-related decoded lane for live progression signal:
  - new lane: `DecodedViewerTimePayload`
  - decoded source: inbound `SimulatorViewerTimeMessage` (tiny body/signature slice + update counter)
  - seam-owned role: `WorldIngestionDecodedViewerTimePayload`
  - decode -> snapshot -> seam -> scene path is test-protected, including presence/absence behavior
- viewer_app-owned live startup orchestration is now first-class:
  - startup mode control (`VIEWER_APP_LIVE_STARTUP`: auto/on/off)
  - app-owned startup status mapping and diagnostics
  - app receives worker status + snapshot updates and feeds seam/scene path
- T8 status:
  - substantially complete for the current bounded pre-world object/state phase
  - next phase should begin the first richer bounded world/object ingestion slice through the existing seam,
    while preserving the hard stop before broad object/world decoding

---

## Explicitly Deferred

Do not work on these yet unless the plan changes:
- simulator/world integration implementation beyond bootstrap staging
- viewer login UI integration
- asset-backed world rendering
- inventory/chat/map shells
- media/voice
- broad OpenSim divergence support
