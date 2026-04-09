# Review: Implementation ObjectUpdateCached Cache-Miss Recovery (2026-04-02)

## Verdict
- Approved.

## Architecture / Boundary Fit
- Change remains inside `viewer_net` transport/decode behavior and first-simulator social-circuit send path.
- No crate boundary expansion.

## Correctness Concerns
- Live run now shows the intended causal chain in one window:
  - inbound `ObjectUpdateCached`
  - outbound `RequestMultipleObjects`
  - subsequent `ObjectUpdate` presence in the same steady-state transcript.
- Diagnostic labeling bug on low-frequency `UseCircuitCode` has been corrected, preventing misleading `Unknown(0xffff0003)` output.

## Modularity / Maintainability
- Cache-miss queue/flush follows existing pending-ACK batching pattern.
- Added focused regression test for message labeling to avoid future observability drift.

## Validation Adequacy
- `cargo fmt --all` PASS
- `cargo check -p viewer_net -p viewer_app` PASS
- `cargo test -p viewer_net first_simulator_message_label_maps_use_circuit_code -- --nocapture` PASS
- bounded live runtime capture confirms `RequestMultipleObjects` emission:
  - `artifacts/logs/live_mesh_cachemiss_verify_2026-04-02_205620.log`
  - `artifacts/logs/network_debug_mesh_cachemiss_verify_2026-04-02_205620.jsonl`

## Risks / Open Questions
- This slice verifies object cache-miss recovery behavior, not full mesh byte acquisition; no `mesh_fetch: ready` occurred in this bounded run.
- Access control (`403`) on mesh capability lanes may still block live render of real meshes.

## Learnings Delta Verdict
- none: existing learnings already cover protocol ID sourcing and bounded live-evidence discipline.

## Approval Status
- Implementation accepted for this slice.
