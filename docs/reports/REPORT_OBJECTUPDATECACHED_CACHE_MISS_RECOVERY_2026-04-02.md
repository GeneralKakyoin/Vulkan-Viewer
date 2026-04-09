# Report: ObjectUpdateCached Cache-Miss Recovery (2026-04-02)

## Summary
Implemented and live-validated LLUDP cache-miss recovery parity for `ObjectUpdateCached` by sending `RequestMultipleObjects` on the social circuit when cached local IDs are observed.

## Files Changed
- `crates/viewer_net/src/lib.rs`
- `docs/reviews/REVIEW_IMPL_OBJECTUPDATECACHED_CACHE_MISS_RECOVERY_2026-04-02.md`
- `docs/reports/REPORT_OBJECTUPDATECACHED_CACHE_MISS_RECOVERY_2026-04-02.md`

## Implementation Details
1. Cache-miss request recovery wiring:
- queue local IDs from `ObjectUpdateCached` observations
- flush queued IDs as medium-frequency `RequestMultipleObjects` sends in social poll loop
- preserve bounded batching and queue drain behavior

2. Observability correctness fix:
- corrected `first_simulator_message_label(...)` to map low-frequency `UseCircuitCode` via low-message-number helper
- added regression test:
  - `first_simulator_message_label_maps_use_circuit_code`

## Validation Run
- `cargo fmt --all` -> PASS
- `cargo check -p viewer_net -p viewer_app` -> PASS
- `cargo test -p viewer_net first_simulator_message_label_maps_use_circuit_code -- --nocapture` -> PASS

## Live Validation
- bounded run artifact set:
  - `artifacts/logs/live_mesh_cachemiss_verify_2026-04-02_205620.log`
  - `artifacts/logs/network_debug_mesh_cachemiss_verify_2026-04-02_205620.jsonl`

Key evidence from `after_first_steady_state_window`:
- typed receive includes `ObjectUpdateCached:1` and `ObjectUpdate:1`
- transcript send includes `RequestMultipleObjects(0x0000ff03)#13 ack=0x00000009`
- LLUDP object ingress gate remains healthy (`verdict=PASS`)

## Result Status
- **Cache-miss recovery path is now implemented and live-proven.**
- **Mesh byte/render parity is still gated by downstream mesh capability access and mesh-fetch success, not by missing cached-object follow-up requests.**

## Risks / Follow-up
- Need follow-up run that captures at least one `mesh_fetch: ready` for non-fixture in-scene mesh UUIDs.
- If `403` persists on valid in-scene mesh IDs, compare auth/cookie/request-shaping against Firestorm capture at request-header level.

## Learnings Delta
- `none` — no new durable invariant beyond existing protocol/reference constraints.

## Continuity Updates Performed
- Added implementation review + execution report for this slice.
- Updated `docs/CURRENT_STATE.md` and `docs/HANDOFF.md` with latest live evidence.
