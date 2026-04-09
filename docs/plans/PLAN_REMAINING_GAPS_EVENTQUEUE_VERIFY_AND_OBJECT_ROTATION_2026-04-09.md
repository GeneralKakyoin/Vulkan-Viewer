# Plan: Remaining Gaps Closure - EventQueue Live Verify + Object Rotation Ingestion (2026-04-09)

## Objective
Close the currently actionable remaining gaps by (1) verifying EventQueue cap re-prime behavior live after recent seed-session hardening and (2) fixing object render orientation correctness by propagating decoded object rotation through ingest to scene transforms.

## Scope
- In scope:
  - Add bounded object-rotation payload support in object feed path (`viewer_net` -> `viewer_app` -> `viewer_core`).
  - Apply decoded object rotation in world-object transform mapping with safe fallback behavior.
  - Add targeted tests for rotation decode/mapping and regression safety.
  - Run bounded live verification capture for EventQueue cap re-prime behavior.
  - Update continuity/review/report artifacts.
- Out of scope:
  - Full protocol redesign for seed-cap invalidation.
  - Broad rendering/parity overhaul beyond bounded object-feed rotation correctness.
  - Architecture or crate-boundary changes.

## Current known state
- EventQueue cap re-prime fallback/session-seed promotion was implemented and compile-tested, but live verification for the latest patch set is pending.
- Object-feed transforms currently apply decoded position/scale but use identity rotation; compressed object-update decode path currently skips packed rotation bytes.
- Quaternion normalization hardening exists in math utils, so downstream matrix generation is now robust against non-unit quaternions.

## Files and components touched
- `crates/viewer_net/src/object_decode_utils.rs`
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `crates/viewer_core/src/lib.rs`
- `crates/viewer_core/src/math_utils.rs` (tests only if needed)
- Continuity artifacts in `docs/reviews/`, `docs/reports/`, `docs/CURRENT_STATE.md`, `docs/HANDOFF.md`

## Boundary check
- `viewer_net` remains transport/decode owner.
- `viewer_app` remains snapshot-bridge/orchestration only.
- `viewer_core` remains domain transform owner.
- No crate ownership drift and no cross-boundary semantic collapse.

## Step sequence
1. Add decoded rotation field to net object feed ingress/export structs.
2. Parse compressed object-update packed quaternion xyz, reconstruct w, and preserve bounded quantized rotation payload.
3. Propagate rotation through app snapshot mapping to core object-feed payload.
4. Apply object rotation in `world_object_feed_proxy_transform` using mapped axis convention and safe identity fallback.
5. Add targeted tests for decode/mapping and transform behavior.
6. Run `fmt`, `check`, targeted tests.
7. Run bounded live capture focused on EventQueue cap re-prime behavior and inspect log evidence.
8. Write report/review + continuity updates.

## Validation plan
1. `cargo fmt --all`
2. `cargo check -p viewer_net -p viewer_app -p viewer_core`
3. `cargo test -p viewer_net` (or targeted new tests + existing relevant ones)
4. `cargo test -p viewer_core` (or targeted new tests + existing relevant ones)
5. bounded live run:
   - `VIEWER_APP_LIVE_STARTUP=on`
   - `VIEWER_APP_LLUDP_STARTUP_PARITY_BUNDLE=on`
   - `VIEWER_APP_CAPABILITY_PROBES_REQUIRE_EVENT_QUEUE_OK=true`
   - `VIEWER_APP_EVENT_QUEUE_CAP_NOT_FOUND_BEFORE_RECONNECT=3`
   - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_event_queue_rotation_gap_closure_2026-04-09.jsonl`
   - `cargo run -p viewer_app` (timeout-bounded)

## Risks and open questions
- Packed quaternion sign for w is not explicitly encoded in compressed payload path; choose deterministic positive-w reconstruction and rely on quaternion equivalence where sign inversion is equivalent orientation.
- Live cap rotation behavior may still reconnect when all candidate seed URLs are simultaneously invalidated server-side.

## Deferred-too-early candidates captured
- No new deferred candidate in this bounded slice.

## Learnings pre-check
- L13: preserve valid identity quaternion behavior.
- L22: live/screenshot evidence requires explicit artifact inspection.
- L76: object-ingest placement should use decoded payloads where present.
- L82: keep changes crate-local and behavior-local.

## Completion criteria
- Object-feed rotation is decoded (where available), propagated, and applied in scene transforms.
- Tests covering new rotation path pass.
- Bounded live EventQueue capture executed and inspected for cap re-prime behavior.
- Continuity docs updated with exact validation outcomes and remaining external-risk items.
