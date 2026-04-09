# Plan: Object Ingress Lane Request Shaping (2026-04-01)

## Objective
Implement Firestorm-aligned lane request-shaping probes so startup diagnostics answer "what should we send" per lane with bounded, reproducible evidence.

## Scope
- Add typed capability probe request-shape helpers in `viewer_grid`.
- Add generic one-shot shaped probe executor in `viewer_net` with richer metadata.
- Replace startup two-lane generic GET probes in `viewer_app` with a shaped probe matrix:
  - immediate `asset-cdn` `ViewerAsset` query-key probes
  - EventQueue-gated `simhost` probes for `SimulatorFeatures`, `InterestList`, `UntrustedSimulatorMessage`
- Add env knob `VIEWER_APP_LANE_PROBE_ASSET_IDS` for deterministic asset-id probe inputs.

Out of scope:
- LLUDP startup packet changes
- claim of LLUDP ingress unblock
- broad capability-client implementations beyond one-shot diagnostics

## Current known state
- Seed-cap broadening + two-lane transport probe already showed lane reachability, but the old probe path used generic GET and lacked request-shape parity details.
- LLUDP gate remains `FAIL` (`ObjectUpdate*` absent).
- EventQueue gating is already in place for readiness probes.

## Files and components touched
- `crates/viewer_grid/src/lib.rs`
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/TESTING_REFERENCE.md`
- continuity artifacts under `docs/reports/`, `docs/reviews/`, `docs/CURRENT_STATE.md`, `docs/HANDOFF.md`, `docs/LEARNINGS.md`

## Boundary check
- Capability request meaning stays in `viewer_grid`.
- HTTP execution/response classification stays in `viewer_net`.
- Startup orchestration + relay output stays in `viewer_app`.
- No crate boundary changes.

## Step sequence
1. Add `viewer_grid` request-shape catalog:
   - `SimulatorFeatures` GET
   - `InterestList` POST LLSD body (`mode=default`)
   - `UntrustedSimulatorMessage` GET shape
   - `ViewerAsset` query-key URL builders (`texture_id`, `mesh_id`, `material_id`, `animatn_id`, `sound_id`)
2. Add `viewer_net` shaped probe executor accepting method/url-candidates/headers/body and returning richer probe metadata (`selected_url`, `method`, `decode`, `body_preview_hash`, `response_class`).
3. Keep `UntrustedSimulatorMessage` readiness behavior intact while routing through shared probe execution primitives.
4. Add `viewer_app` shaped matrix execution:
   - run `ViewerAsset` query-key probes immediately
   - defer simhost probes until `EventQueueGet:ok`
   - emit `LaneProbeShape:start|ok|err` with host/cap/method/query_key/url_variant/status/decode/body bytes
5. Add env parsing for `VIEWER_APP_LANE_PROBE_ASSET_IDS` with bounded synthetic fallback IDs.
6. Add tests for shape builders, probe executor metadata, and matrix split behavior.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_grid -p viewer_net -p viewer_app`
- `cargo test -p viewer_grid -p viewer_net -p viewer_app`
- bounded live run:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_APP_CAPABILITY_PROBES_REQUIRE_EVENT_QUEUE_OK=true`
  - optional `VIEWER_APP_LANE_PROBE_ASSET_IDS=...`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_lane_request_shaping_2026-04-01.jsonl`
  - `cargo run -p viewer_app` (bounded timeout capture)

## Risks and open questions
- Some lane shapes are expected to return non-2xx; these remain valid evidence.
- Probe decode classification is intentionally bounded and may report `unparsed` for binary bodies.

## Deferred-too-early candidates captured
- None added in this slice; full semantic body decoders remain deferred.

## Learnings pre-check
- L58, L59, L60, L61, L62, L63 constrain this plan directly.

## Completion criteria
- Shaped probe lines appear for both lane families.
- Per-shape outcomes include method/query-key/url-variant + decode classification.
- Validation commands pass and continuity docs are updated.
- LLUDP gate remains tracked independently.
