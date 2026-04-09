# Plan: RegionHandshakeReply Viewer-Flags Correction and Fallback Unblock (2026-04-01)

## Objective
Unblock LLUDP object ingress by correcting `RegionHandshakeReply` semantics and adding a bounded fallback reply path when `RegionHandshake` is not observed, using OpenSim and Firestorm evidence.

## Scope
- Correct `RegionHandshakeReply` `RegionInfo.Flags` to viewer-capability flags (not echoed simulator region flags).
- Add one bounded fallback send path for `RegionHandshakeReply` after startup social drain when no handshake has been observed.
- Keep changes within `viewer_net` (protocol mechanics) and `viewer_app` (startup orchestration).

## Current known state
- Live runs show `AgentMovementComplete` and control traffic, but no `RegionHandshake`/`RegionHandshakeReply` and no `ObjectUpdate*`.
- Existing code currently stores simulator `RegionFlags` from `RegionHandshake` and reuses that value in `RegionHandshakeReply`.
- Firestorm constructs viewer handshake flags independently (includes self-appearance support bit).
- OpenSim `RegionHandshakeReply` handler gates initial data progression and stores viewer flags.

Evidence:
- `artifacts/logs/network_debug_region_handshake_opensim_endpoint_parity_2026-04-01.jsonl:19-25`
- `crates/viewer_net/src/lib.rs:2754-2763`
- `crates/viewer_net/src/lib.rs:4200-4213`
- `crates/viewer_net/src/lib.rs:4321-4334`
- `reference/firestorm/indra/newview/llviewerregion.cpp:3433-3451`
- `reference/opensimulator/nebadon2025-opensimulator/OpenSim/Region/ClientStack/Linden/UDP/LLClientView.cs:8979-8990`
- `reference/opensimulator/nebadon2025-opensimulator/OpenSim/Region/Framework/Scenes/ScenePresence.cs:4049-4060`
- `reference/opensimulator/nebadon2025-opensimulator/OpenSim/Region/Framework/Scenes/ScenePresence.cs:4065-4067`

## Files and components touched
- `crates/viewer_net/src/lib.rs`
  - handshake-reply flag composition
  - pending handshake-reply gating semantics
  - bounded fallback API for startup path
  - unit tests updated/added
- `crates/viewer_app/src/main.rs`
  - startup social prime path calls fallback handshake-reply once when needed

## Boundary check
- `viewer_net`: protocol/message mechanics only.
- `viewer_app`: startup orchestration trigger only.
- No `viewer_grid` or render/UI changes.

## Step sequence
1. Replace handshake-reply payload flag source with viewer-capability flags (base self-appearance support bit).
2. Keep pending-reply behavior for normal handshake path but stop using simulator region flags as reply flags.
3. Add `send_fallback_region_handshake_reply_if_missing(...)` in `viewer_net`:
   - send once only when reply not yet sent
   - require startup handshake stage has reached at least waiting-for-AMC / AMC observed
4. Invoke fallback once in `viewer_app::prime_startup_social_circuit(...)` after startup drain.
5. Add/update unit tests for:
   - reply payload flags use viewer capability flags
   - fallback path sends exactly once and sets sent-state
6. Run validation + bounded runtime capture to check for first `RegionHandshakeReply` send evidence and object ingress movement.

## Validation plan
1. `cargo fmt --all`
2. `cargo check -p viewer_net -p viewer_app`
3. `cargo test -p viewer_net -p viewer_app`
4. bounded runtime:
   - `VIEWER_APP_LIVE_STARTUP=on`
   - `VIEWER_APP_CAPABILITY_PROBES_REQUIRE_EVENT_QUEUE_OK=true`
   - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_region_handshake_reply_flags_fix_2026-04-01.jsonl`
   - `cargo run -p viewer_app` (timeout-bounded)

## Risks and open questions
- If server requires prior inbound `RegionHandshake` before accepting reply, fallback may be ignored.
- If additional viewer-flag bits are required beyond self-appearance support, object unlock may remain blocked.

## Deferred-too-early candidates captured
- None for this bounded protocol-correction slice.

## Learnings pre-check
- L10 (message IDs from template)
- L68 (handshake progression and initial-data gating)
- L69 (endpoint parity alone does not unblock handshake)
- L70 (use full-stack map to target highest-confidence blocker)

## Completion criteria
- `RegionHandshakeReply` flags are viewer-derived, not echoed simulator flags.
- Startup path can emit one bounded fallback `RegionHandshakeReply` when handshake is absent.
- Runtime evidence clearly shows whether fallback send occurs and whether `lludp_object_gate` changes.
