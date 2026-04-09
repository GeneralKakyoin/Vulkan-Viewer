# HANDOFF: Object Rotation Ingestion + EventQueue Verify Pass (2026-04-09)

## What Changed
- Added object-feed rotation support from transport decode through scene transform mapping:
  - `viewer_net` decodes compressed object-update packed quaternion xyz and exports optional `rotation_quat_i16` on object feed objects.
  - `viewer_app` bridges `rotation_quat_i16` into `viewer_core::DecodedWorldObjectFeedObject`.
  - `viewer_core` applies decoded object rotation in `world_object_feed_proxy_transform(...)` with axis mapping and identity fallback.
- Retained previously implemented EventQueue seed-session refresh hardening and ran a fresh bounded live verification capture.

## Validation Run
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app -p viewer_core`
- `cargo test -p viewer_net decode_object_update_compressed_extracts_local_ids -- --nocapture`
- `cargo test -p viewer_net decode_object_update_compressed_extracts_nonzero_rotation_quaternion -- --nocapture`
- `cargo test -p viewer_core scene_world_object_feed_maps_decoded_rotation_quaternion_to_scene_axes -- --nocapture`
- `cargo test -p viewer_net`
- `cargo test -p viewer_core`
- Bounded live run:
  - env:
    - `VIEWER_APP_LIVE_STARTUP=on`
    - `VIEWER_APP_LLUDP_STARTUP_PARITY_BUNDLE=on`
    - `VIEWER_APP_CAPABILITY_PROBES_REQUIRE_EVENT_QUEUE_OK=true`
    - `VIEWER_APP_EVENT_QUEUE_CAP_NOT_FOUND_BEFORE_RECONNECT=3`
    - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_event_queue_rotation_gap_closure_2026-04-09.jsonl`
  - command: `cargo run -p viewer_app` (timeout-bounded by harness)

## Exact Current State
- Object render transform correctness is improved: decoded object-feed rotation is now carried and applied (when available) instead of forcing identity rotation.
- EventQueue startup/probe gate remains healthy in latest bounded run (`EventQueueGet:ok` observed).
- In this bounded capture window, cap-rotation `404` did not reoccur, so cap-reprime behavior under active cap invalidation was not re-exercised in this run.

## Exact Next Step
1. Re-run bounded live captures (same env knobs) until cap-rotation `404` is observed, then verify whether current seed-session + fallback improvements recover without reconnect more often than baseline.

## Blockers / Risks
- Cap invalidation timing is simulator/session dependent and may not appear in short bounded runs.
- If all known seed URLs invalidate together, reconnect fallback remains expected behavior.
