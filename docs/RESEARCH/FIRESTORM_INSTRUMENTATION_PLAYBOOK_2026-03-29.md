# Firestorm Instrumentation Playbook (2026-03-29)

Goal: build an instrumented Firestorm and capture authoritative protocol behavior for:
- login handoff completion
- simulator targeting (`EnableSimulator`, `CrossedRegion`)
- ping reliability (`StartPingCheck`/`CompletePingCheck`)
- object update arrival (`ObjectUpdate*`, `ImprovedTerse`, `MultipleObjectUpdate`)

Use logging category `AGVProto` so captures are easy to grep.

## Quick Apply

If your Firestorm repo is separate, run from that Firestorm repo root:
```bash
git apply <path-to-this-repo>/tools/firestorm/firestorm_agvproto_instrumentation.patch
```

## 1) Patch Points

### A. Ping handshake
File: `reference/firestorm/indra/llmessage/message.cpp`
Functions:
- `process_start_ping_check(...)`
- `process_complete_ping_check(...)`

Insert:
```cpp
LL_INFOS("AGVProto") << "start_ping_check from=" << msgsystem->getSender() << " ping_id=" << (S32)ping_id << LL_ENDL;
```
before sending `CompletePingCheck`, and:
```cpp
LL_INFOS("AGVProto") << "complete_ping_check from=" << msgsystem->getSender() << " ping_id=" << (S32)ping_id << LL_ENDL;
```
in `process_complete_ping_check`.

### B. AgentThrottle send
File: `reference/firestorm/indra/newview/llviewerthrottle.cpp`
Function:
- `LLViewerThrottleGroup::sendToSim() const`

Insert after `len = dp.getCurrentSize();`:
```cpp
LL_INFOS("AGVProto") << "agent_throttle_send circuit=" << msg->mOurCircuitCode
                     << " len=" << len
                     << " resend=" << mThrottles[0]
                     << " land=" << mThrottles[1]
                     << " wind=" << mThrottles[2]
                     << " cloud=" << mThrottles[3]
                     << " task=" << mThrottles[4]
                     << " texture=" << mThrottles[5]
                     << " asset=" << mThrottles[6]
                     << LL_ENDL;
```

### C. AgentUpdate send cadence
File: `reference/firestorm/indra/newview/llviewermessage.cpp`
Function:
- `send_agent_update(bool force_send, bool send_reliable)`

Insert right before `_PREHASH_AgentUpdate` build:
```cpp
LL_INFOS("AGVProto") << "agent_update_send reliable=" << (send_reliable ? 1 : 0)
                     << " force=" << (force_send ? 1 : 0)
                     << " tp_state=" << (S32)tp_state
                     << " flags=" << (S32)flags
                     << LL_ENDL;
```

### D. Object update receive handlers
File: `reference/firestorm/indra/newview/llviewermessage.cpp`
Functions:
- `process_object_update(...)`
- `process_compressed_object_update(...)`
- `process_cached_object_update(...)`
- `process_terse_object_update_improved(...)`

Insert at top of each:
```cpp
LL_INFOS("AGVProto") << "recv_object_update kind=<KIND>"
                     << " sender=" << mesgsys->getSender()
                     << " size=" << mesgsys->getReceiveSize()
                     << " csize=" << mesgsys->getReceiveCompressedSize()
                     << LL_ENDL;
```
Replace `<KIND>` with `full|compressed|cached|terse`.

### E. Simulator handoff target
File: `reference/firestorm/indra/newview/llworld.cpp`
Function:
- `process_enable_simulator(...)`

Insert after reading handle/ip/port:
```cpp
LL_INFOS("AGVProto") << "enable_simulator handle=" << handle
                     << " ip=" << LLHost(ip_u32, port)
                     << " port=" << port
                     << LL_ENDL;
```

### F. Region crossing path
File: `reference/firestorm/indra/newview/llviewermessage.cpp`
Functions:
- `process_crossed_region(...)`
- `process_agent_movement_complete(...)`

Insert:
```cpp
LL_INFOS("AGVProto") << "crossed_region sim=" << sim_host << " handle=" << region_handle << LL_ENDL;
```
and in movement complete:
```cpp
LL_INFOS("AGVProto") << "agent_movement_complete sender=" << msg->getSender() << LL_ENDL;
```

### G. Callback registration sanity
File: `reference/firestorm/indra/newview/llstartup.cpp`
Function:
- `register_viewer_callbacks(LLMessageSystem* msg)`

No code change required; verify these remain registered:
- `_PREHASH_ObjectUpdate`
- `ObjectUpdateCompressed`
- `ObjectUpdateCached`
- `_PREHASH_ImprovedTerseObjectUpdate`
- `_PREHASH_EnableSimulator`
- `CrossedRegion`

## 2) Build + Run Capture

1. Build your Firestorm branch with above logs.
2. Log in with same account/location as this viewer tests.
3. Capture at least 120s after `agent_movement_complete` first appears.
4. Save log file as e.g. `artifacts/logs/firestorm_proto_trace.log` in this repo (or point script to external path).

## 3) Compare With Our Viewer

Our latest viewer log examples:
- `artifacts/logs/rca_after_retarget_gate_2026-03-29.log`
- `artifacts/logs/rca_after_ping_reply_long_2026-03-29.log`

Run helper:
```powershell
pwsh ./tools/firestorm/compare_proto_logs.ps1 -FirestormLog <path-to-firestorm-log> -ViewerLog artifacts/logs/rca_after_ping_reply_long_2026-03-29.log
```

## 4) Success Signal

The Firestorm trace should show recurring `recv_object_update` events in same region session where our viewer currently reports zero.
First divergence we care about:
- Firestorm starts receiving object updates but our viewer does not.

That gives us a precise protocol gap to close next.
