# Root Cause Analysis: Why Live Assets Never Load

## Update (2026-03-30): Preserved PCAP Evidence Refines This RCA

The preserved captures:
- `artifacts/pcaps/firestorm_object_ingress_reference_2026-03-30_firee.pcapng`
- `artifacts/pcaps/app_object_ingress_reference_2026-03-30_App.pcapng`

changed the confidence level of this document.

What is now externally confirmed:
- Firestorm reaches simulator `16.144.39.130:13001` on one coherent local UDP port and receives a large `ObjectUpdateCached` burst after a richer pre-burst startup sequence.
- The app capture reaches that same simulator endpoint but does not receive the corresponding object burst on its handshake/control conversation.
- The app capture also contains a second local UDP port receiving simulator traffic on the same remote endpoint during the capture window.

What this means:
- the older socket-continuity RCA was directionally useful, but it is not complete enough to serve as the final root cause by itself
- runtime local-port behavior is still a live suspect and must be instrumented directly before more message-parity guessing

See:
- `docs/RESEARCH/OBJECT_INGRESS_PCAP_FORENSICS_2026-03-30.md`
- `docs/plans/PLAN_OBJECT_INGRESS_RUNTIME_SOCKET_FORENSICS_2026-03-30.md`

## Problem Statement

The viewer completes login, establishes a simulator connection, and receives ongoing UDP traffic (layer data, ping checks, coarse location), but **zero ObjectUpdate packets** are ever classified or decoded. The object feed remains permanently empty.

## Historical Root Cause Hypothesis

The root cause is a **protocol interplay gap between the initial handshake probe and the persistent social circuit**. There are two distinct issues:

### Issue 1: The initial probe window receives very few packets and the social circuit opens on a DIFFERENT socket

The lifecycle is:

1. `probe_first_simulator_handshake_window_with_policy()` — opens an **ephemeral socket**, sends `UseCircuitCode` + `CompleteAgentMovement`, waits for `AgentMovementComplete`, reads a bounded tail of packets, then **drops the socket**
2. `open_social_circuit()` — opens a **new, different socket** (different ephemeral port) and sends `UseCircuitCode` + `CompleteAgentMovement` again via `ensure_social_circuit_handshake()`

**The simulator sends ObjectUpdate packets to the CLIENT ADDRESS it received `UseCircuitCode` from.** By the time the social circuit is opened on a different port, the simulator has already sent its burst of `ObjectUpdateCached` packets (hundreds, as shown in the Firestorm capture) to the **first, now-closed socket**. Those packets are lost.

The Firestorm capture shows **hundreds of ObjectUpdate packets** arriving in the first 1-2 seconds:
- Lines 4-42: 39 `ObjectUpdateCached` packets immediately after `AgentMovementComplete`
- Lines 43-52: 10 `ObjectUpdate` (full) packets
- This massive burst continues for seconds

### Issue 2: The viewer's `run_probe` flag controls whether the initial handshake probe even runs

Looking at [main.rs:1167-1180](file:///c:/Users/matti/Desktop/Antigravity%20viewer/Vulkan-Viewer/crates/viewer_app/src/main.rs#L1167-L1180):
```rust
if config.run_probe && matches!(result, GridLoginResult::Success(_)) {
    let _ = connection
        .probe_first_simulator_handshake_window_with_policy(...)
        .await;
}
```

Even IF the probe runs, it uses a **separate ephemeral socket** that is dropped before `open_social_circuit()`. The probe's socket is NOT reused.

### Why Other Traffic Still Works

The social circuit socket successfully receives traffic like `StartPingCheck`, `LayerData`, `CoarseLocationUpdate`, and `HealthMessage` because:
- These are **ongoing periodic messages** the simulator sends to whatever circuit address it last saw
- After `ensure_social_circuit_handshake()` sends `UseCircuitCode` + `CompleteAgentMovement` on the new socket, the simulator updates its idea of the client's address
- But by then, the initial **burst of ObjectUpdate/ObjectUpdateCached** that happens at login has already been sent and lost

### Evidence from Logs

**Viewer log** (`rca_after_ping_reply_long_2026-03-29.log`, line 27):
```
inbound kinds: object_updates=0 layer_data=56 start_ping_check=4 movement_complete=1 
  region_handshake=0 enable_simulator=0 coarse_location=17 online_notifications=1 irrelevant=0
```
- **56 LayerData**, **4 StartPingCheck**, **17 CoarseLocationUpdate** — ongoing traffic arrives
- **0 ObjectUpdates** — the initial burst was lost
- **0 RegionHandshake** — also lost with the first socket

**Firestorm capture** (`firestorm_agvproto_capture_2026-03-29_211128.log`):
- Lines 1-800: **Hundreds** of ObjectUpdate packets (cached, full, compressed, terse)
- All arriving within 1-5 seconds of `AgentMovementComplete`
- This is the standard protocol behavior: the sim floods the viewer with world state immediately

## Proposed Fix

> [!IMPORTANT]
> The fix requires ensuring the social circuit reuses the **same socket** that the initial handshake probe used. This preserves the client address the simulator has on file.

### Option A: Socket Handoff (Recommended)

Persist the socket from the initial probe window and hand it to `open_social_circuit()` instead of creating a new one. This is the simplest and most correct fix:

#### [MODIFY] [lib.rs](file:///c:/Users/matti/Desktop/Antigravity%20viewer/Vulkan-Viewer/crates/viewer_net/src/lib.rs)

1. Add an `Option<Arc<UdpSocket>>` field to `Connection` to hold the probe socket
2. At the end of `probe_first_simulator_handshake_window_with_policy()`, stash the socket in the connection instead of dropping it
3. In `open_social_circuit()`, reuse the stashed socket if available, otherwise create a new one

#### [MODIFY] [main.rs](file:///c:/Users/matti/Desktop/Antigravity%20viewer/Vulkan-Viewer/crates/viewer_app/src/main.rs)

4. When `config.run_probe` is false, skip the separate `open_social_circuit()` call and instead have `ensure_social_circuit_handshake()` use the same socket path
5. Remove the redundant `UseCircuitCode` + `CompleteAgentMovement` sends in `ensure_social_circuit_handshake()` when a handshake has already completed on the same socket

### Option B: Always-On Socket (Alternative)

Create a single persistent socket at startup and use it for both the handshake probe and the social circuit.

## Open Questions

> [!IMPORTANT]  
> **Decision needed**: Should we go with Option A (socket handoff, smaller change) or Option B (single persistent socket, cleaner architecture)?

> [!WARNING]
> The `ensure_social_circuit_handshake()` re-sends `UseCircuitCode` and `CompleteAgentMovement` even when the probe already completed these. This causes the simulator to process a second circuit registration. Is this safe, or do we need to skip the re-send when the probe already succeeded?

## Verification Plan

### Automated Tests
- Run `cargo check` and `cargo test -p viewer_net` to confirm no regressions

### Manual Verification
- Run `cargo run -p viewer_app` with live credentials
- Verify the relay log shows `object_feed: startup decode summary: update_messages>0 total_objects>0`
- Verify the `first_sim_ingest` log shows `object_updates>0`

### Firestorm Comparison
- Compare the viewer's inbound traffic counts against a fresh Firestorm capture to confirm order-of-magnitude parity
