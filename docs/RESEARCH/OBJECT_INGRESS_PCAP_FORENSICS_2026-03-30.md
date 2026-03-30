# Research: Object Ingress PCAP Forensics (2026-03-30)

## Objective
Preserve and summarize the strongest external packet-capture evidence for the blocked object-ingress investigation.

## Preserved Artifacts

### Firestorm reference capture
- repo copy: `artifacts/pcaps/firestorm_object_ingress_reference_2026-03-30_firee.pcapng`
- original source: `C:\Users\matti\Desktop\firee.pcapng`
- sha256: `BA4EE224D69F30244F0700AF69BD03C1EE6D4E21F72C7B55C70CAD0AF43DF91B`

### App reference capture
- repo copy: `artifacts/pcaps/app_object_ingress_reference_2026-03-30_App.pcapng`
- original source: `C:\Users\matti\Desktop\App.pcapng`
- sha256: `C870823C1A0563D2E83D2B0C7F8155C61CD44E931CA5C0D972BE47DE027EE9A1`

## Firestorm Capture Summary

### Primary simulator conversation
- local endpoint: `192.168.1.194:63490`
- simulator endpoint: `16.144.39.130:13001`
- conversation size: `1702` UDP packets

### Confirmed pre-burst sequence
- `3246`: outbound `AgentUpdate`
- `3247`: outbound `AgentAnimation`
- `3248`: outbound `SetAlwaysRun`
- `3249`: outbound `PacketAck`
- `3288`: outbound `MuteListRequest`
- `3289`: outbound `MoneyBalanceRequest`
- `3290`: outbound `AgentDataUpdateRequest`
- `3649`: inbound `PacketAck`
- `3653+`: inbound `ObjectUpdateCached` burst (`High 14`)

### Interpretation
- Firestorm reaches a coherent single simulator conversation on `16.144.39.130:13001`.
- The first large object burst begins immediately after a richer outbound startup/control sequence than the current viewer has been observed to send.

## App Capture Summary

## Captured conversations on the same simulator endpoint

### Conversation A
- local endpoint: `192.168.1.194:51485`
- simulator endpoint: `16.144.39.130:13001`
- conversation size: `12` UDP packets

#### Confirmed sequence
- `711`: outbound `UseCircuitCode`
- `712`: outbound `CompleteAgentMovement`
- `721`: inbound `AgentDataUpdate`
- `722`: inbound `TestMessage`
- `723`: inbound `AgentMovementComplete`
- `724`: inbound `PacketAck`
- `725`: inbound `HealthMessage`
- `726`: inbound `OnlineNotification`
- `738`: inbound `CoarseLocationUpdate`
- `739`: outbound `AgentThrottle`
- `740`: outbound `AgentUpdate`
- `741`: outbound `RetrieveInstantMessages`

#### Key result
- No `ObjectUpdateCached` burst appears on this conversation.

### Conversation B
- local endpoint: `192.168.1.194:61225`
- simulator endpoint: `16.144.39.130:13001`
- conversation size: `14` UDP packets
- direction in capture: inbound-only

#### Confirmed inbound traffic types
- `ViewerEffect`
- `AttachedSound`
- `SimulatorViewerTimeMessage`
- `LayerData`
- `CoarseLocationUpdate`
- `KickUser`

#### Key result
- The app capture contains a second local-port association on the same simulator endpoint during the capture window.

## Interpretation
- Firestorm and the app both target `16.144.39.130:13001`.
- Firestorm receives the expected object burst on a single coherent local port.
- The app capture does not show that burst on its active handshake/control conversation.
- The app capture also shows a second local UDP port receiving simulator traffic on the same remote endpoint.

## Important Uncertainty
- `App.pcapng` begins with inbound traffic on `192.168.1.194:61225`, so the capture window may have started after an earlier port association already existed.
- Because of that, the capture is strong runtime evidence of multi-port behavior during the session, but it does not by itself prove the exact code path that created the earlier port.

## Current Best Inference
- Runtime socket continuity is still not proven in practice, despite the retained-socket code changes and tests.
- Blindly mirroring every extra Firestorm pre-burst request would be premature until the runtime local-port lifecycle is instrumented and verified on the blocked viewer path.

## Why This Matters
- The preserved pcaps are now the best external evidence for the next bounded debugging slice.
- Any next-step plan should use these artifacts first, not rely only on code inspection or older assumptions.
