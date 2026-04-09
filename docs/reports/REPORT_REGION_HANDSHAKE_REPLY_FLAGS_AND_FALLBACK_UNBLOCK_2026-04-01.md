# Report: RegionHandshakeReply Flags and Fallback Unblock (2026-04-01)

## Summary of Implemented Work
- Corrected `RegionHandshakeReply` payload flags to viewer-capability flags instead of echoing simulator `RegionHandshake` region flags.
- Added a bounded fallback send path in `send_pending_region_handshake_reply(...)` so the reply can still be emitted once startup has reached movement-complete stage even if inbound `RegionHandshake` was not observed.
- Added/updated unit tests to verify:
  - reply flags match viewer-derived flags
  - fallback path sends a reply once when stage preconditions are satisfied

## Files Changed
- `crates/viewer_net/src/lib.rs`
- `docs/reports/REPORT_REGION_HANDSHAKE_REPLY_FLAGS_AND_FALLBACK_UNBLOCK_2026-04-01.md`
- `docs/reviews/REVIEW_IMPL_REGION_HANDSHAKE_REPLY_FLAGS_AND_FALLBACK_UNBLOCK_2026-04-01.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/LEARNINGS.md`

## Primary Evidence
- Viewer flags constant and reply flag source:
  - `crates/viewer_net/src/lib.rs:107`
  - `crates/viewer_net/src/lib.rs:4233`
  - `crates/viewer_net/src/lib.rs:5889`
- Fallback gating path:
  - `crates/viewer_net/src/lib.rs:4213`
  - `crates/viewer_net/src/lib.rs:4220`
- Test evidence:
  - `crates/viewer_net/src/lib.rs:14184`
- Firestorm behavior reference (`RegionHandshakeReply` with viewer-composed flags and self-appearance bit):
  - `reference/firestorm/indra/newview/llviewerregion.cpp:3433`
  - `reference/firestorm/indra/newview/llviewerregion.cpp:3440`
  - `reference/firestorm/indra/newview/llviewerregion.cpp:3450`
- OpenSim behavior reference (server stores viewer handshake flags and gates initial data on `0x1000`):
  - `reference/opensimulator/nebadon2025-opensimulator/OpenSim/Region/ClientStack/Linden/UDP/LLClientView.cs:8979`
  - `reference/opensimulator/nebadon2025-opensimulator/OpenSim/Region/ClientStack/Linden/UDP/LLClientView.cs:8990`
  - `reference/opensimulator/nebadon2025-opensimulator/OpenSim/Region/ClientStack/Linden/UDP/LLClientView.cs:15420`
  - `reference/opensimulator/nebadon2025-opensimulator/OpenSim/Region/Framework/Scenes/ScenePresence.cs:4065`
  - `reference/opensimulator/nebadon2025-opensimulator/OpenSim/Region/Framework/Scenes/ScenePresence.cs:4066`

## Validation Run
- `cargo fmt --all` -> PASS
- `cargo check -p viewer_net -p viewer_app` -> PASS (pre-existing warnings only)
- `cargo test -p viewer_net send_pending_region_handshake_reply -- --nocapture` -> PASS (2/2 tests)
- Bounded runtime capture:
  - Command: `VIEWER_APP_LIVE_STARTUP=on`, `VIEWER_APP_CAPABILITY_PROBES_REQUIRE_EVENT_QUEUE_OK=true`, `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_region_handshake_reply_flags_fix_2026-04-01.jsonl`, `cargo run -p viewer_app`
  - Process result: timeout-bounded (expected for long-running app), artifact captured

## Runtime Result Status
- Startup still begins with no observed inbound `RegionHandshake` and initial gate `FAIL`:
  - `artifacts/logs/network_debug_region_handshake_reply_flags_fix_2026-04-01.jsonl:20`
  - `artifacts/logs/network_debug_region_handshake_reply_flags_fix_2026-04-01.jsonl:25`
- Startup now consistently sends `RegionHandshakeReply`:
  - `artifacts/logs/network_debug_region_handshake_reply_flags_fix_2026-04-01.jsonl:22`
  - `artifacts/logs/network_debug_region_handshake_reply_flags_fix_2026-04-01.jsonl:24`
- In the same bounded run, LLUDP object ingress transitions to PASS:
  - `after_first_steady_state_window recv ... ObjectUpdate:2`:
    - `artifacts/logs/network_debug_region_handshake_reply_flags_fix_2026-04-01.jsonl:39`
  - `lludp_object_gate: verdict=PASS ... local_ids=...`:
    - `artifacts/logs/network_debug_region_handshake_reply_flags_fix_2026-04-01.jsonl:45`
- Steady-state object ingestion remains populated:
  - `tick summary: update_messages=164 total_objects=1024 ...`:
    - `artifacts/logs/network_debug_region_handshake_reply_flags_fix_2026-04-01.jsonl:97`

## Risks / Follow-Up
- Inbound `RegionHandshake`/observed `RegionHandshakeReply` counters still show `none` in startup timeline even when object ingress unlocks. This means message classification/observation may still be incomplete for those message kinds.
- Single next probe to close this gap:
  - add one packet-header diagnostic event specifically for low-frequency message `0xffff0091` (RegionHandshake) decode attempts before type classification, then re-run one bounded capture.

## Learnings Delta
- `added`: L71 (handshake-reply viewer-flags + stage-gated fallback can unlock LLUDP object ingress even when inbound `RegionHandshake` is not observed first).

## Continuity Updates Performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/LEARNINGS.md`
