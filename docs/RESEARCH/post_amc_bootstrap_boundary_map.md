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

## Practical Stop Line
- Continue typing only if a repeated post-AMC packet is clearly startup-gating/bootstrap-relevant.
- Otherwise keep it in `LikelyBroaderTraffic` or `Unknown` diagnostics and stop at boundary documentation.
