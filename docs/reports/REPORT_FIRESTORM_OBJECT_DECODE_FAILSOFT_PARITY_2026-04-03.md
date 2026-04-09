# Report: Firestorm Object Decode Fail-Soft Parity (2026-04-03)

## Summary
Implemented a bounded decoder-hardening pass so valid mesh-ID object data is retained when neighboring LLUDP object entries are malformed/truncated. This matches Firestorm-style tolerant object-by-object progression more closely.

## Files Changed
- `crates/viewer_net/src/lib.rs`
- `docs/plans/PLAN_FIRESTORM_OBJECT_DECODE_FAILSOFT_PARITY_2026-04-03.md`
- `docs/reviews/REVIEW_PLAN_FIRESTORM_OBJECT_DECODE_FAILSOFT_PARITY_2026-04-03.md`
- `docs/reviews/REVIEW_IMPL_FIRESTORM_OBJECT_DECODE_FAILSOFT_PARITY_2026-04-03.md`
- `docs/reports/REPORT_FIRESTORM_OBJECT_DECODE_FAILSOFT_PARITY_2026-04-03.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`

## Implementation Details
1. `decode_mesh_asset_id_from_extra_params(...)`
- changed from strict full-buffer consumption to fail-soft iteration.
- valid mesh IDs are preserved even if trailing entries/bytes are malformed.

2. `decode_object_extra_params_mesh_updates(...)`
- malformed entries no longer abort whole decode.
- decoder now keeps valid following entries.

3. `decode_object_update_compressed_objects(...)`
- malformed/truncated object blocks no longer discard prior valid decoded objects.
- decode returns `None` only when no object block could be read at all.

4. Regression tests added
- `decode_mesh_asset_id_from_extra_params_tolerates_trailing_bytes`
- `decode_object_update_compressed_tolerates_mixed_valid_and_malformed_blocks`
- `decode_object_extra_params_tolerates_invalid_entry_and_keeps_valid_following_entry`

## Reference Alignment
- Firestorm behavior reference:
  - `reference/firestorm/indra/newview/llviewerobject.cpp`
  - `reference/firestorm/indra/newview/llviewerobjectlist.cpp`
- OpenSim payload behavior reference:
  - `reference/opensimulator/nebadon2025-opensimulator/OpenSim/Region/ClientStack/Linden/UDP/LLClientView.cs`
  - `reference/opensimulator/nebadon2025-opensimulator/OpenSim/Framework/PrimitiveBaseShape.cs`

## Validation Run
- `cargo fmt --all` -> PASS
- `cargo check -p viewer_net` -> PASS
- `cargo test -p viewer_net decode_mesh_asset_id_from_extra_params_tolerates_trailing_bytes -- --nocapture` -> PASS
- `cargo test -p viewer_net decode_object_update_compressed_tolerates_mixed_valid_and_malformed_blocks -- --nocapture` -> PASS
- `cargo test -p viewer_net decode_object_extra_params_tolerates_invalid_entry_and_keeps_valid_following_entry -- --nocapture` -> PASS
- `cargo test -p viewer_net` -> PASS

## Live Validation
- `cargo run -p viewer_app` with live env was attempted and failed to rebuild due linker lock:
  - `LNK1104` opening `target\debug\deps\viewer_app.exe`
- direct binary live run succeeded:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_FIXTURE_MESHES=0`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_failsoft_parity_live_2026-04-03.jsonl`
  - command: `.\target\debug\viewer_app.exe`
- observed in-window:
  - startup `lludp_object_gate: FAIL` then steady-state `lludp_object_gate: PASS`
  - `ObjectUpdate`/`ObjectUpdateCached` present in first steady-state receive transcript
  - object feed progressed (`update_messages` up to `585`, `total_objects=128` in bounded window)
  - no `mesh_fetch` relay lines observed in this bounded window

## Result Status
- Decoder parity hardening completed and locally validated.
- This slice improves mesh-ID discovery reliability from live LLUDP updates under imperfect packet conditions.

## Risks / Follow-up
- End-to-end live mesh rendering still additionally depends on:
  - receiving mesh-bearing object updates in-window,
  - and allowed capability access for the resolved mesh UUIDs.

## Learnings Delta
- none — no new durable learning identified; this work operationalizes existing L73/L74/L77 guidance.

## Continuity Updates Performed
- Added plan/reviews/report artifacts for this bounded slice.
- Updated `CURRENT_STATE` and `HANDOFF` to reflect latest validated status and next blocker.
