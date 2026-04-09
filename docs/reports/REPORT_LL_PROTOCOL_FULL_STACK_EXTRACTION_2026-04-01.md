# Report: LL Protocol Full-Stack Extraction (2026-04-01)

## Executive summary
- The viewer now has broad HTTP capability-lane observability and shaped probes, plus robust EventQueue endpoint parsing (including OpenSimulator binary-IP forms), but LLUDP object ingress is still blocked because `RegionHandshake`/`RegionHandshakeReply` never occur in bounded live runs.
- Login/auth foundation is present and includes LLSD->XML-RPC fallback, redirect handling, and required bootstrap extraction (`agent_id`, `session_id`, `circuit_code`, `seed_capability`) needed for later LLUDP and capability calls.
- Capability semantics are mostly aligned for the currently targeted lanes (`SimulatorFeatures`, `InterestList`, `UntrustedSimulatorMessage`, `ViewerAsset` query-key variants), including EventQueue-gated probe ordering.
- Highest-confidence blocker cluster is handshake eligibility/session-state on child endpoints (not endpoint parsing anymore): runtime shows correct child endpoint targeting and sends of `UseCircuitCode`/`CompleteAgentMovement`, yet still no handshake/object stream unlock.

## Protocol lane inventory matrix
| Lane | Transport | Auth/session dependencies | Request shape requirements | Response shape requirements | Failure modes observed | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| Login (primary) | HTTPS POST LLSD/XML | username + `$1$md5(passwd)` + options + start | `method=login_to_simulator`, `params`, `options` | Must include login result, bootstrap fields on success | request-shape rejection (mitigated via fallback) | `crates/viewer_net/src/lib.rs:299-339`, `crates/viewer_net/src/lib.rs:347-362`, `crates/viewer_grid/src/legacy_login.rs:62-69`, `crates/viewer_grid/src/lib.rs:196-205` |
| Login (fallback) | HTTPS POST XML-RPC | same as above | XML-RPC struct with `first/last/passwd/start/options` | same logical fields (`next_url`, `next_method`, bootstrap fields) | unsupported redirect method, redirect loops | `crates/viewer_net/src/lib.rs:371-410`, `crates/viewer_net/src/lib.rs:458-474`, `crates/viewer_net/src/lib.rs:2359-2376`, `crates/viewer_net/src/lib.rs:2439-2465` |
| Seed capabilities | HTTPS POST LLSD XML | logged-in session + `seed_capability` URL | LLSD string-array of `DEFAULT_SEED_CAPABILITY_REQUEST` | LLSD map capability->URL | non-2xx HTTP | `crates/viewer_net/src/lib.rs:108-130`, `crates/viewer_net/src/lib.rs:154-159`, `crates/viewer_net/src/lib.rs:198-202`, `crates/viewer_net/src/lib.rs:219`, `crates/viewer_net/src/lib.rs:3475-3497` |
| EventQueueGet | HTTPS POST LLSD XML | logged-in session + `EventQueueGet` URL | body includes `ack` + `done` | LLSD map with `events` and `id` | cap-not-found, decode issues, timeout/transient HTTP | `crates/viewer_net/src/lib.rs:5568-5572`, `crates/viewer_net/src/lib.rs:3623-3648`, `crates/viewer_net/src/lib.rs:5458-5473`, `reference/opensimulator/nebadon2025-opensimulator/OpenSim/Region/ClientStack/Linden/Caps/EventQueue/EventQueueGetModule.cs:457-459` |
| Simulator-host caps (`SimulatorFeatures`) | HTTPS GET | logged-in + cap URL | GET, no body | LLSD map | 503 in lane-probe run; 200 in shaped run | `crates/viewer_grid/src/lib.rs:451-467`, `artifacts/logs/network_debug_two_lane_transport_probe_2026-04-01.jsonl:17`, `artifacts/logs/network_debug_lane_request_shaping_2026-04-01.jsonl:42` |
| Simulator-host caps (`InterestList`) | HTTPS POST LLSD XML | logged-in + cap URL + EventQueue readiness gate | POST body `<llsd><map><key>mode</key>...` | LLSD map (`mode`, `stats`) | 404 Agent not found (ungated readiness), 200 when gated | `crates/viewer_grid/src/lib.rs:469-490`, `reference/firestorm/indra/newview/llviewerregion.cpp:3872-3875`, `artifacts/logs/network_debug_capability_readiness_2026-04-01.jsonl:16`, `artifacts/logs/network_debug_capability_probe_event_queue_gating_2026-04-01.jsonl:28` |
| Simulator-host caps (`UntrustedSimulatorMessage`) | HTTPS GET then POST fallback | logged-in + cap URL + gate | GET first; on 405 fallback POST LLSD envelope | LLSD preferred; transport-success fallback accepted | 405 on GET; 200 unparsed-body on POST fallback | `crates/viewer_grid/src/lib.rs:494-509`, `crates/viewer_net/src/lib.rs:4841-4863`, `crates/viewer_net/src/lib.rs:4874-4893`, `crates/viewer_net/src/lib.rs:9057-9073`, `artifacts/logs/network_debug_capability_probe_event_queue_gating_2026-04-01.jsonl:28`, `artifacts/logs/network_debug_untrusted_probe_post_fallback_2026-04-01.jsonl:66` |
| Asset CDN (`ViewerAsset`) | HTTPS GET | capability URL + asset id query key | key-specific query params (`texture_id`, `mesh_id`, `material_id`, `animatn_id`, `sound_id`) and URL variants | binary media or XML error bodies | mixed 200/403 by key | `crates/viewer_grid/src/lib.rs:348-365`, `crates/viewer_grid/src/lib.rs:416-417`, `artifacts/logs/network_debug_lane_request_shaping_2026-04-01.jsonl:2-11` |
| LLUDP root socket | UDP reliable/unreliable + appended ACKs | bootstrap `agent_id/session_id/circuit_code` | `UseCircuitCode`, `CompleteAgentMovement`, startup interest bundle, ACK behavior | inbound handshake/control/object packets | currently no handshake/object ingress | `crates/viewer_net/src/lib.rs:44-53`, `crates/viewer_net/src/lib.rs:79-91`, `crates/viewer_net/src/lib.rs:5831-5844`, `artifacts/logs/network_debug_region_handshake_opensim_endpoint_parity_2026-04-01.jsonl:20-25` |
| LLUDP child/neighbor follow-up | UDP to EventQueue-discovered endpoint | same session/circuit + resolved endpoint | follow-up `UseCircuitCode` + `CompleteAgentMovement` to child endpoint | expected `RegionHandshake` progression | sends occur, but handshake still absent | `crates/viewer_net/src/lib.rs:4096-4113`, `crates/viewer_net/src/lib.rs:4136-4146`, `crates/viewer_app/src/main.rs:4844-4867`, `crates/viewer_app/src/main.rs:4883-4906`, `artifacts/logs/network_debug_region_handshake_opensim_endpoint_parity_2026-04-01.jsonl:51-61` |

## Message handling matrix
| Message | Dir | Trigger/preconditions | Required fields | Expected response/effect | Current app state | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| `UseCircuitCode` | viewer->sim | logged in, circuit ready, target endpoint chosen | `circuit_code`, `session_id`, `agent_id` | server may emit `RegionHandshake` if accepted | sent to root/child | `crates/viewer_net/src/lib.rs:5814-5823`, `crates/viewer_net/src/lib.rs:4096-4113`, `reference/firestorm/scripts/messages/message_template.msg:3127-3133`, `reference/opensimulator/nebadon2025-opensimulator/OpenSim/Region/ClientStack/Linden/UDP/LLUDPServer.cs:1711-1712` |
| `RegionHandshake` | sim->viewer | after accepted `UseCircuitCode` | `RegionFlags`, `SimName`, etc | viewer should capture flags/name and reply | not observed live | `crates/viewer_net/src/lib.rs:6952-6963`, `crates/viewer_net/src/lib.rs:2754-2763`, `artifacts/logs/network_debug_region_handshake_opensim_endpoint_parity_2026-04-01.jsonl:21`, `reference/firestorm/scripts/messages/message_template.msg:3127-3138` |
| `RegionHandshakeReply` | viewer->sim | pending flags set by inbound handshake | `AgentID`, `SessionID`, `Flags` | unlock server object stream gate | send path exists but never armed in run | `crates/viewer_net/src/lib.rs:4200-4213`, `crates/viewer_net/src/lib.rs:5848-5857`, `reference/firestorm/scripts/messages/message_template.msg:3180-3187`, `artifacts/logs/network_debug_region_handshake_opensim_endpoint_parity_2026-04-01.jsonl:22` |
| `CompleteAgentMovement` | viewer->sim | after circuit established | `AgentID`, `SessionID`, `CircuitCode` | server validates and advances movement state | sent to child endpoints | `crates/viewer_net/src/lib.rs:5831-5844`, `crates/viewer_net/src/lib.rs:4146-4156`, `reference/opensimulator/nebadon2025-opensimulator/OpenSim/Region/ClientStack/Linden/UDP/LLClientView.cs:9246-9250`, `artifacts/logs/network_debug_region_handshake_opensim_endpoint_parity_2026-04-01.jsonl:57-61` |
| `AgentMovementComplete` | sim->viewer | server-side move complete | standard low-freq payload | indicates movement completion | present in timeline index | `crates/viewer_net/src/lib.rs:57`, `crates/viewer_net/src/lib.rs:2014-2018`, `reference/firestorm/scripts/messages/message_template.msg:5612` |
| `EnableSimulator` | sim->viewer (via EventQueue and UDP semantics) | neighbor/child activation | endpoint fields (`SimulatorInfo.IP` + `Port`) | viewer should follow child endpoint | parsed and followed | `crates/viewer_net/src/lib.rs:8360-8373`, `crates/viewer_net/src/lib.rs:8422-8430`, `reference/opensimulator/nebadon2025-opensimulator/OpenSim/Region/ClientStack/Linden/Caps/EventQueue/EventQueueGetModule.cs:513-517`, `artifacts/logs/network_debug_region_handshake_opensim_endpoint_parity_2026-04-01.jsonl:51-61` |
| `EstablishAgentCommunication` | sim->viewer (EventQueue) | child setup/control | `sim-ip-and-port`, `seed-capability` | viewer can fetch child seed caps / map endpoint | parsed; repeated details logged | `crates/viewer_net/src/lib.rs:8376-8382`, `crates/viewer_net/src/lib.rs:8399-8403`, `reference/opensimulator/nebadon2025-opensimulator/OpenSim/Region/ClientStack/Linden/Caps/EventQueue/EventQueueGetModule.cs:534-538`, `artifacts/logs/network_debug_region_handshake_opensim_endpoint_parity_2026-04-01.jsonl:64-67` |
| `CrossedRegion` | sim->viewer | crossing transition | includes `SimIP`, `SimPort`, `SeedCapability` | transition phase advances | decode path present | `crates/viewer_net/src/lib.rs:6445-6453`, `reference/firestorm/scripts/messages/message_template.msg:3412-3415`, `reference/opensimulator/nebadon2025-opensimulator/OpenSim/Region/ClientStack/Linden/UDP/LLClientView.cs:1911-1929` |
| `ConfirmEnableSimulator` | sim->viewer | region transition continuity | medium message id 8 | transition to confirming phase | decode path present | `crates/viewer_net/src/lib.rs:91`, `crates/viewer_net/src/lib.rs:6449-6454`, `reference/firestorm/scripts/messages/message_template.msg:3468-3472` |
| `ObjectUpdate*` (`ObjectUpdate`, `Compressed`, `Cached`, `ImprovedTerse`) | sim->viewer | post-handshake object stream | object local IDs and payload variants | object feed update starts | not observed in blocked runs | `crates/viewer_net/src/lib.rs:79-82`, `crates/viewer_net/src/lib.rs:2813-2829`, `reference/firestorm/scripts/messages/message_template.msg:3270-3388`, `artifacts/logs/network_debug_region_handshake_opensim_endpoint_parity_2026-04-01.jsonl:25` |
| `KillObject` | sim->viewer | object lifecycle removal | kill list IDs | remove objects | decode ID mapped, no active ingress due upstream gate | `crates/viewer_net/src/lib.rs:83`, `reference/firestorm/scripts/messages/message_template.msg:3401-3404` |
| `PacketAck` | both | reliable packet acknowledgement | count + packet ids | reliability stabilization | inbound/outbound paths present | `crates/viewer_net/src/lib.rs:36-37`, `crates/viewer_net/src/lib.rs:2917-2927`, `crates/viewer_net/src/lib.rs:5669-5683`, `reference/firestorm/scripts/messages/message_template.msg:46` |

## Handshake state machine(s)
### Root state machine
1. `LoggedIn` -> fetch seed capabilities.
2. Discover `EventQueueGet` and begin polling with `ack/done` LLSD body.
3. Open social circuit and send root `UseCircuitCode`/`CompleteAgentMovement`.
4. On inbound `RegionHandshake`, capture `RegionFlags` and arm `RegionHandshakeReply`.
5. Send `RegionHandshakeReply` once.
6. Server may then emit object stream (`ObjectUpdate*`).

Evidence: `crates/viewer_net/src/lib.rs:3449-3460`, `crates/viewer_net/src/lib.rs:5568-5572`, `crates/viewer_net/src/lib.rs:4096-4113`, `crates/viewer_net/src/lib.rs:2754-2763`, `crates/viewer_net/src/lib.rs:4200-4213`, `reference/firestorm/scripts/messages/message_template.msg:3186-3187`.

### Child/neighbor state machine
1. EventQueue emits `EnableSimulator` and/or `EstablishAgentCommunication` with child endpoint fields.
2. Viewer resolves endpoint from `sim-ip-and-port` or `SimulatorInfo.IP` (string/binary) + `Port`.
3. Viewer sends `UseCircuitCode` and `CompleteAgentMovement` to child endpoint.
4. Expected server response: `RegionHandshake`; then viewer reply (`RegionHandshakeReply`) and object stream unlock.
5. Current observed branch: step 3 occurs, step 4 absent.

Evidence: `crates/viewer_net/src/lib.rs:8360-8430`, `crates/viewer_app/src/main.rs:4844-4906`, `artifacts/logs/network_debug_region_handshake_opensim_endpoint_parity_2026-04-01.jsonl:51-61`, `artifacts/logs/network_debug_region_handshake_opensim_endpoint_parity_2026-04-01.jsonl:21-22`, `reference/opensimulator/nebadon2025-opensimulator/OpenSim/Region/Framework/Scenes/ScenePresence.cs:4049-4060`.

## Capability invocation matrix (method/body/query/header exactness)
| Capability | Method | Body | Query | Headers | Required ordering | Runtime status |
| --- | --- | --- | --- | --- | --- | --- |
| `SimulatorFeatures` | GET | none | none | optional accept | after EventQueue gate in startup probe path | 200 in shaped lane run |
| `InterestList` | POST | LLSD map `{mode: default}` | none | `Content-Type: application/llsd+xml`, `Accept: application/llsd+xml` | should wait until first `EventQueueGet:ok` | 200 when gated; 404 ungated runs |
| `UntrustedSimulatorMessage` | GET -> POST fallback on 405 | fallback LLSD envelope body | none | LLSD headers for POST | gated preferred | 405 GET; 200 transport-ready with `probe_decode=unparsed_body` after fallback |
| `ViewerAsset` | GET | none | one of `texture_id`,`mesh_id`,`material_id`,`animatn_id`,`sound_id` | accept varies by endpoint | immediate lane probe allowed | mixed by key: texture/sound 200, others 403 |
| `EventQueueGet` | POST | LLSD `ack`,`done` | none | LLSD headers | looped | operational with ack increments |

Evidence: `crates/viewer_grid/src/lib.rs:451-509`, `crates/viewer_grid/src/lib.rs:348-365`, `crates/viewer_net/src/lib.rs:4841-4893`, `crates/viewer_net/src/lib.rs:5568-5572`, `crates/viewer_app/src/main.rs:3852-3867`, `crates/viewer_app/src/main.rs:1369-1394`, `artifacts/logs/network_debug_lane_request_shaping_2026-04-01.jsonl:2-11`, `artifacts/logs/network_debug_lane_request_shaping_2026-04-01.jsonl:42`, `artifacts/logs/network_debug_capability_probe_event_queue_gating_2026-04-01.jsonl:28`, `artifacts/logs/network_debug_capability_readiness_2026-04-01.jsonl:16`.

## ACK/reliability matrix
| Mechanism | Required behavior | Current implementation | Observed status | Evidence |
| --- | --- | --- | --- | --- |
| Reliable detection | ACK reliable inbound packets except `PacketAck` itself | checks `LLUDP_RELIABLE_FLAG`, excludes `PacketAck`, queues packet ids | implemented | `crates/viewer_net/src/lib.rs:36`, `crates/viewer_net/src/lib.rs:2917-2927` |
| Appended ACK trailer | append ack ids when MTU budget allows and set ACK flag | computes size budget and appends big-endian ack trailer | implemented | `crates/viewer_net/src/lib.rs:2944-2963`, `crates/viewer_net/src/lib.rs:6170-6181` |
| Explicit `PacketAck` datagram | send low-freq `PacketAck` with ack count + ids | `encode_packet_ack_payload` + flush loop | implemented | `crates/viewer_net/src/lib.rs:4243-4268`, `crates/viewer_net/src/lib.rs:5669-5683` |
| ACK trailer decode | parse appended ack trailer length from inbound flags/count | decodes when `LLUDP_ACK_FLAG` set | implemented | `crates/viewer_net/src/lib.rs:6158-6167` |
| Runtime forensics | expose pending/appended/explicit ACK stats | first-sim forensics emits ack lines and gate lines | implemented | `crates/viewer_app/src/main.rs:3429-3447`, `crates/viewer_app/src/main.rs:3650-3655` |

## Server gate matrix
| Gate | What must be true before server emits next data family | Evidence source | Current status |
| --- | --- | --- | --- |
| Login bootstrap gate | successful login with valid `agent_id/session_id/circuit_code/seed_capability` | login decode + bootstrap extraction | PASS |
| EventQueue control gate | `EventQueueGet` polling active and acknowledged | EventQueue request shape and poll loop | PASS |
| Capability readiness gate (InterestList/Untrusted) | first `EventQueueGet:ok` should occur before probe execution to avoid agent-not-found | probe gate logic + gated run outcomes | PASS |
| Child endpoint resolution gate | parse `EnableSimulator`/`EstablishAgentCommunication` endpoint fields, including binary IP forms | extractor supports `sim-ip-and-port` and binary `SimulatorInfo.IP` | PASS |
| Child handshake eligibility gate | after `UseCircuitCode` + `CompleteAgentMovement`, child should emit `RegionHandshake` | Firestorm/OpenSim handshake semantics | FAIL |
| Object stream unlock gate | simulator starts object data after `RegionHandshakeReply` progression | Firestorm comment + OpenSim `NeedInitialData` flow | FAIL (no handshake/reply observed) |

Evidence: `crates/viewer_net/src/lib.rs:2581-2590`, `crates/viewer_net/src/lib.rs:265-301`, `crates/viewer_app/src/main.rs:1369-1394`, `artifacts/logs/network_debug_capability_readiness_2026-04-01.jsonl:16`, `artifacts/logs/network_debug_capability_probe_event_queue_gating_2026-04-01.jsonl:28`, `crates/viewer_net/src/lib.rs:8411-8450`, `artifacts/logs/network_debug_region_handshake_opensim_endpoint_parity_2026-04-01.jsonl:56-61`, `reference/firestorm/scripts/messages/message_template.msg:3127-3133`, `reference/firestorm/scripts/messages/message_template.msg:3186-3187`, `reference/opensimulator/nebadon2025-opensimulator/OpenSim/Region/Framework/Scenes/ScenePresence.cs:4049-4060`.

## Current app vs required behavior gap table
| Area | Required behavior | Current behavior | Verdict | Evidence |
| --- | --- | --- | --- | --- |
| Login wire compatibility | tolerate LLSD request-shape mismatch and complete via XML-RPC | LLSD->XML-RPC fallback implemented | PASS | `crates/viewer_net/src/lib.rs:2359-2376` |
| Redirect handling | honor `next_url` + POST-only `next_method` | redirect chain supported, non-POST rejected | PASS | `crates/viewer_net/src/lib.rs:2439-2455` |
| Credential normalization | send `$1$<md5>` if raw password | normalization enforced | PASS | `crates/viewer_grid/src/legacy_login.rs:62-69`, `crates/viewer_grid/src/lib.rs:203-209` |
| Seed lane breadth | request broad capability inventory for lane mapping | broad seed request list present | PASS | `crates/viewer_net/src/lib.rs:108-130`, `crates/viewer_net/src/lib.rs:3475-3479` |
| EventQueue semantics | `ack/done` LLSD request and event/id parse | implemented | PASS | `crates/viewer_net/src/lib.rs:5568-5572`, `reference/opensimulator/nebadon2025-opensimulator/OpenSim/Region/ClientStack/Linden/Caps/EventQueue/EventQueueGetModule.cs:457-459` |
| EventQueue endpoint discovery | parse `SimulatorInfo.IP` binary + `Port` and `sim-ip-and-port` | implemented and live-observed | PASS | `crates/viewer_net/src/lib.rs:8411-8450`, `artifacts/logs/network_debug_region_handshake_opensim_endpoint_parity_2026-04-01.jsonl:51-54`, `artifacts/logs/network_debug_region_handshake_opensim_endpoint_parity_2026-04-01.jsonl:64-67` |
| Child follow-up sends | send `UseCircuitCode`/`CompleteAgentMovement` to resolved endpoint | sent to explicit child endpoints | PASS | `crates/viewer_app/src/main.rs:4864-4906`, `artifacts/logs/network_debug_region_handshake_opensim_endpoint_parity_2026-04-01.jsonl:56-61` |
| Handshake progression | observe `RegionHandshake`, send `RegionHandshakeReply`, then object stream | handshake and reply absent | FAIL | `artifacts/logs/network_debug_region_handshake_opensim_endpoint_parity_2026-04-01.jsonl:21-22`, `artifacts/logs/network_debug_region_handshake_opensim_endpoint_parity_2026-04-01.jsonl:25` |
| Object ingress families | decode and ingest `ObjectUpdate*` (+ kill/update control flows) | decode paths exist; no live ingress on blocked branch | PARTIAL | `crates/viewer_net/src/lib.rs:2813-2829`, `reference/firestorm/scripts/messages/message_template.msg:3270-3404`, `artifacts/logs/network_debug_region_handshake_opensim_endpoint_parity_2026-04-01.jsonl:25` |
| ACK/reliability | collect reliable ids, append/sent acks, parse ack trailers | implemented with forensics | PARTIAL (needs verification under active object traffic) | `crates/viewer_net/src/lib.rs:2917-2927`, `crates/viewer_net/src/lib.rs:2944-2963`, `crates/viewer_net/src/lib.rs:5669-5683`, `crates/viewer_app/src/main.rs:3429-3447` |
| Capability method/body/query exactness | Firestorm-like shaping across key caps | implemented for priority caps | PASS | `crates/viewer_grid/src/lib.rs:451-509`, `crates/viewer_grid/src/lib.rs:348-365`, `artifacts/logs/network_debug_lane_request_shaping_2026-04-01.jsonl:2-11`, `artifacts/logs/network_debug_lane_request_shaping_2026-04-01.jsonl:42` |
| Asset retry policy | do not retry deterministic 4xx | 401/403/404 classified non-retryable | PASS | `crates/viewer_app/src/main.rs:1074-1093` |
| Region crossing control intake | handle `CrossedRegion` / `ConfirmEnableSimulator` transitions | decode and continuity transitions implemented | PARTIAL (observability present, blocked by upstream handshake) | `crates/viewer_net/src/lib.rs:2889-2908`, `crates/viewer_net/src/lib.rs:6445-6454` |

## Top 10 blockers (ranked by impact then confidence)
| Rank | Blocker | Impact | Confidence | Why |
| --- | --- | --- | --- | --- |
| 1 | Child handshake eligibility/session-state mismatch | Very high | High | Correct child endpoint targeting and sends occur, but no `RegionHandshake`. |
| 2 | Missing server unlock sequence (`RegionHandshakeReply` never armed) | Very high | High | Reply path depends on inbound handshake flags; no inbound handshake means no unlock. |
| 3 | Incomplete proof of circuit acceptance per child endpoint | High | Medium-high | No explicit accepted-state indicator between sends and expected handshake. |
| 4 | Potential mismatch in child endpoint socket/circuit reuse semantics | High | Medium | Code reuses social circuit socket; may differ from required child acceptance behavior. |
| 5 | ACK timing under blocked startup may hide first handshake packets | Medium-high | Medium | ACK stack exists, but no positive-object/handshake traffic to validate ordering assumptions. |
| 6 | EventQueue degradation (`cap not found`) may interrupt sustained control flow | Medium | Medium | Reconnect thresholds exist; repeated degradation seen historically. |
| 7 | Untrusted capability payload-class uncertainty | Medium | Medium | Transport-ready fallback in place, but semantic decode still unknown. |
| 8 | Region-crossing control path unvalidated under successful object ingress | Medium | Medium | decode exists, but blocked branch prevents end-to-end validation. |
| 9 | Missing direct child seed-cap follow-up diagnostics for handshake eligibility | Medium | Medium | seed-cap follow-up exists but needs tighter eligibility evidence correlation. |
| 10 | Lack of packet-level acceptance probes for rejected child handshakes | Medium | Medium-low | Current forensics emphasize timeline/gates but not explicit acceptance predicates. |

Evidence anchors: `artifacts/logs/network_debug_region_handshake_opensim_endpoint_parity_2026-04-01.jsonl:21-25`, `artifacts/logs/network_debug_region_handshake_opensim_endpoint_parity_2026-04-01.jsonl:56-61`, `crates/viewer_net/src/lib.rs:4200-4213`, `crates/viewer_net/src/lib.rs:4096-4113`, `crates/viewer_net/src/lib.rs:3501-3525`, `crates/viewer_net/src/lib.rs:2917-2927`.

## Minimal patch sequence (smallest first)
1. Patch: Add child-endpoint handshake eligibility probe lines (single-source diagnostics in `viewer_net` + relay in `viewer_app`) capturing per-endpoint send packet IDs, first inbound low-freq message number, and timeout classification.
   - Expected observable evidence: new per-endpoint lines showing whether any inbound low-freq packet follows child `UseCircuitCode` within bounded window.
   - Acceptance signal: ability to classify each child endpoint as `accepted`, `silent`, or `rejected` in one bounded run.

2. Patch: Correlate EventQueue target -> child socket send -> first inbound packet timeline in one structured event record.
   - Expected observable evidence: one joined record per endpoint with `endpoint_source`, `use_circuit_packet_id`, `cam_packet_id`, `first_inbound_kind`.
   - Acceptance signal: eliminate ambiguity about socket/endpoint mismatch.

3. Patch: Add bounded alternate child send mode (same messages, alternate target socket policy) behind env flag only.
   - Expected observable evidence: A/B logs where only send mode differs.
   - Acceptance signal: one mode yields first `RegionHandshake` index != none.

4. Patch: If handshake appears, immediately verify `RegionHandshakeReply` emission and object gate transition.
   - Expected observable evidence: `region_handshake_observed index=...`, `region_handshake_reply_send count>0`, `lludp_object_gate verdict=PASS`.
   - Acceptance signal: first decoded `ObjectUpdate*` with non-empty local ID evidence.

5. Patch: If still no handshake, add strict credential/session parity check instrumentation at child send boundary (verify agent/session/circuit bytes used in payload).
   - Expected observable evidence: debug line with deterministic hash of serialized IDs per child send.
   - Acceptance signal: prove payload identity parity with root prerequisites; if mismatch, fix and rerun.

6. Patch: Only after handshake success, validate ACK/reliability under active object traffic and tune flush timing if needed.
   - Expected observable evidence: rising object updates with stable ACK stats and no decode drops spike.
   - Acceptance signal: sustained `lludp_object_gate PASS` across first steady-state window.

## Risks and unknowns
- Missing evidence: definitive server-side reason for silent child endpoint after valid-looking sends.
  - Single next probe: per-endpoint acceptance classification (patch step 1) with first inbound low-freq message number and timeout reason.
- Missing evidence: whether socket reuse policy prevents child handshake in this route.
  - Single next probe: A/B mode with alternate child send socket policy behind one env flag (patch step 3).
- Missing evidence: whether session/circuit bytes differ on child sends.
  - Single next probe: serialized payload identity hash logging for child `UseCircuitCode` and `CompleteAgentMovement` (patch step 5).

## Runtime artifacts used
- `artifacts/logs/network_debug_region_handshake_opensim_endpoint_parity_2026-04-01.jsonl`
- `artifacts/logs/network_debug_lane_request_shaping_2026-04-01.jsonl`
- `artifacts/logs/network_debug_capability_probe_event_queue_gating_2026-04-01.jsonl`
- `artifacts/logs/network_debug_untrusted_probe_post_fallback_2026-04-01.jsonl`
- `artifacts/logs/network_debug_two_lane_transport_probe_2026-04-01.jsonl`
- `artifacts/logs/network_debug_capability_readiness_2026-04-01.jsonl`
