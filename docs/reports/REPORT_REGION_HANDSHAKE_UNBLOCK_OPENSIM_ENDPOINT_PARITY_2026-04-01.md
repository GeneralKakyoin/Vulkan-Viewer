# Report: RegionHandshake Unblock via OpenSimulator Endpoint/Handshake Parity (2026-04-01)

## Summary of implemented work
- Added OpenSimulator-aligned EventQueue endpoint extraction in `viewer_net`:
  - preserves LLSD `<binary>` scalar values
  - decodes base64 binary IP values into IPv4/IPv6 strings
  - resolves authoritative endpoint tuple metadata on simulator targets:
    - `endpoint_ip`
    - `endpoint_port`
    - `endpoint_source` (`sim_ip_and_port`, `ip_port_fields`, `simulatorinfo_binary_ip_port`)
- Added runtime diagnostics in `viewer_app` for simulator-target EventQueue messages:
  - emits resolved endpoint metadata
  - emits raw field summary for each target event (`raw=...`)
- Updated `EnableSimulator` follow-up routing in `viewer_app` to prefer resolved endpoint metadata from `viewer_net`.
- Added handshake progression relay evidence in `first_sim_forensics`:
  - `region_handshake_observed index=... updates=... sim_name=...`
  - `region_handshake_reply_send count=... first_send_idx=... observed_reply_idx=...`

## Files changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `reference/opensimulator/README.md`
- `docs/plans/PLAN_REGION_HANDSHAKE_UNBLOCK_OPENSIM_ENDPOINT_PARITY_2026-04-01.md`
- `docs/reports/REPORT_REGION_HANDSHAKE_UNBLOCK_OPENSIM_ENDPOINT_PARITY_2026-04-01.md`
- `docs/reviews/REVIEW_PLAN_REGION_HANDSHAKE_UNBLOCK_OPENSIM_ENDPOINT_PARITY_2026-04-01.md`
- `docs/reviews/REVIEW_IMPL_REGION_HANDSHAKE_UNBLOCK_OPENSIM_ENDPOINT_PARITY_2026-04-01.md`

## Validation run
- `cargo fmt --all` -> PASSED
- `cargo check -p viewer_net -p viewer_app` -> PASSED
- `cargo test -p viewer_net -p viewer_app` -> PASSED
- bounded live capture (timeout-bounded):
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_APP_CAPABILITY_PROBES_REQUIRE_EVENT_QUEUE_OK=true`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_region_handshake_opensim_endpoint_parity_2026-04-01.jsonl`
  - `cargo run -p viewer_app` -> EXECUTED (timeout-bounded, evidence captured)

## Result status
- Endpoint extraction now surfaces OpenSimulator-style binary-IP event shapes:
  - `EnableSimulator detail ... endpoint_source=simulatorinfo_binary_ip_port ip_raw=...`
- Follow-up sends now target resolved explicit endpoints from binary-IP decode:
  - `EnableSimulator follow-up sent UseCircuitCode endpoint=34.211.187.220:13022 source=simulatorinfo_binary_ip_port`
  - similar for `35.91.216.188:13028` and `54.186.233.232:13032`
- `EstablishAgentCommunication` details now repeatedly show resolved `sim-ip-and-port` endpoints and seed-cap host families.
- LLUDP handshake gate remains blocked in this run:
  - `region_handshake=none`
  - `region_handshake_reply=none`
  - `region_handshake_reply_send count=0`
  - `lludp_object_gate: verdict=FAIL`

## Risks / follow-up items
- Endpoint-shape ambiguity is no longer the primary unknown.
- Remaining likely branch is handshake eligibility/session-state semantics on child endpoints (for example, whether additional circuit/session progression is required before `RegionHandshake` emission on these child routes).
- Next patch should stay small and evidence-first (no broad startup-packet expansion).

## Learnings delta
- added (L69): endpoint-shape completeness can be fixed without restoring `RegionHandshake`; once binary-IP + `sim-ip-and-port` routing is proven, pivot from endpoint parsing to handshake eligibility/session-state gating.

## Continuity updates performed
- Added new plan review and implementation review artifacts for this slice.
- Updated `CURRENT_STATE.md`, `HANDOFF.md`, and `LEARNINGS.md`.
