# Post-AMC Bootstrap Boundary Map

## Question
After `AgentMovementComplete`, which inbound packets should still be treated as bootstrap-relevant versus transport-control versus likely broader simulator traffic?

## Scope
- Live bounded-tail probe observations from `viewer_net` manual example.
- Firestorm message-template cross-check for packet identity only.
- No world-state subsystem implementation.

## Early Post-AMC Protocol Map

| Message | Packet Number | Scope | Why |
|---|---|---|---|
| `AgentDataUpdate` | `0xFFFF0183` (low 387) | `BootstrapRelevant` | Appears in immediate startup window and remains part of early session bring-up context. |
| `TestMessage` | `0xFFFF0001` (low 1) | `TransportControl` | Transport/protocol sanity/control signal, not a bootstrap gate. |
| `AgentMovementComplete` | `0xFFFF00FA` (low 250) | `BootstrapRelevant` | First strong completion checkpoint for first-region movement startup. |
| `PacketAck` | `0xFFFFFFFB` (fixed low `0xFFFB`) | `TransportControl` | Reliable packet ack/control-plane traffic. |
| `HealthMessage` | `0xFFFF008A` (low 138) | `LikelyBroaderTraffic` | Simulator liveness/environment telemetry, not a bootstrap gate. |
| `OnlineNotification` | `0xFFFF0142` (low 322) | `LikelyBroaderTraffic` | Social/world notification traffic after initial bootstrap. |
| `ViewerEffect` | `0x0000FF11` (medium 17) | `LikelyBroaderTraffic` | Viewer effect broadcast traffic, not bootstrap sequencing. |
| `CoarseLocationUpdate` | `0x0000FF06` (medium 6) | `LikelyBroaderTraffic` | Coarse avatar/presence location traffic; useful for diagnostics but not bootstrap-gating in this phase. |
| `AttachedSound` | `0x0000FF0D` (medium 13) | `LikelyBroaderTraffic` | Audio/world side-effect traffic; not bootstrap/handoff control. |
| unknown medium example | varies | `Unknown` | Some medium IDs can still appear in short tails; keep explicit until repeated and justified. |

## Boundary Rule For This Phase
Treat the bootstrap boundary as consolidated once:
1. `AgentMovementComplete` is observed and stage-confirmed, and
2. subsequent traffic in the bounded tail is either:
   - transport-control (`PacketAck`, `TestMessage`), or
   - likely broader traffic (`HealthMessage`, `OnlineNotification`, `ViewerEffect`), or
   - explicitly unknown/non-bootstrap.

Do not start broad world-state decoding from this map alone.

## Diagnostic Preservation
- Probe report now includes a post-boundary summary:
  - per-scope counts
  - ordered post-boundary kinds
- This is intended to preserve early non-bootstrap observation evidence without repeated ad hoc manual parsing.
- `viewer_net` now also has a typed early-traffic scaffold for this phase:
  - `EarlySimulatorTrafficKind`
  - `EarlySimulatorTrafficObservation`
  - scoped to currently observed non-bootstrap early traffic only (no broad world/object decode).
- bounded simulator-payload decode now exists for this phase:
  - source: `CoarseLocationUpdate` body
  - currently decoded minimally:
    - location block count
    - first coarse XYZ sample (when present)
  - this decode is intentionally tiny and is used as a seam-fed diagnostic input, not as broad world/object ingestion.
- bounded simulator-payload decode has been extended with a second tiny source:
  - source: `HealthMessage` body
  - currently decoded minimally:
    - health scalar normalized to basis points
  - this remains diagnostic-first and is routed through a distinct seam lane/scene role, not broad world/object ingestion.
- bounded multi-input composition note:
  - current decoded lanes (`CoarseLocationUpdate` + `HealthMessage`) now drive one seam-owned composite beacon role
  - composition is gated on both decoded lanes being present
  - this is treated as object-like diagnostic composition, not broad object/world-state decoding

## Practical Stop Line
- Continue typing only if a repeated post-AMC packet is clearly startup-gating/bootstrap-relevant.
- Otherwise keep it in `LikelyBroaderTraffic` or `Unknown` diagnostics and stop at boundary documentation.

## Consolidation Status
- Current consolidation is "done enough" for this phase:
  - repeated observed early non-bootstrap packet set is typed in the early-traffic scaffold
  - diagnostics preserve scope separation and per-kind summaries
- Hard stop remains:
  - do not start broad object/world-state decoding from this slice.

## Handoff/Control Watch List (Next Phase)
- `0x0000FF07` (`CrossedRegion`) and `0x0000FF08` (`ConfirmEnableSimulator`) are high-value region-transition control IDs to type once repeatedly observed.
- Keep them as next-phase targets; do not pre-emptively expand payload decode beyond message-ID visibility.
- Current status:
  - message-ID typing rails and dedicated diagnostics now exist
  - bounded high-tail live runs in current phase did not observe these IDs yet
  - diagnostics now report this absence explicitly (`not_seen_in_run`) instead of leaving it implicit.
