# REPORT: Object Data Ingestion/Decoding Parity (Firestorm + OpenSim) 2026-04-02

## Summary Of Implemented Work
- Extended object-feed decode/export in `viewer_net` to carry bounded decoded position metadata (`position_centi`) in addition to existing `local_id`, `scale_centi`, and optional `object_id`.
- Expanded LLUDP object-family parsing:
  - `ObjectUpdate` now extracts positional payload from `ObjectData` fixed blocks when present.
  - `ObjectUpdateCompressed` now decodes per-object packed block fields (UUID/local-id/scale/position) instead of local-id only.
  - `ImprovedTerseObjectUpdate` now decodes local-id + position from terse data blocks instead of local-id only.
- Threaded new decoded position field through:
  - `viewer_app` snapshot mapping (`viewer_net` -> `viewer_core` object feed)
  - `viewer_core` seam payload (`decoded_object_position_centi`)
  - scene proxy transform path.
- Updated world object proxy placement behavior:
  - when decoded position exists, proxy placement now reflects decoded spatial ordering in bounded cluster coordinates;
  - fallback ring placement remains for packets without usable position data.
- Added/updated unit tests covering compressed + terse decode paths with position assertions and export compatibility.

## Firestorm/OpenSim Reference Sources Used
- `reference/firestorm/scripts/messages/message_template.msg` (ObjectUpdate family packet block layout)
- `reference/firestorm/indra/newview/llviewerobject.cpp` (compressed + terse unpack expectations, object data map)
- `reference/firestorm/indra/newview/llviewermessage.cpp` (ObjectUpdate family processing entry points)
- `reference/opensimulator/nebadon2025-opensimulator/OpenSim/Region/ClientStack/Linden/UDP/LLClientView.cs`:
  - `CreateCompressedUpdateBlockZC(...)`
  - `CreateImprovedTerseBlock(...)`
  - object update packet header/build flows

## Files Changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `crates/viewer_core/src/lib.rs`
- `docs/plans/PLAN_OBJECT_DATA_INGEST_DECODE_PARITY_2026-04-02.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_DATA_INGEST_DECODE_PARITY_2026-04-02.md`
- `docs/reports/REPORT_OBJECT_DATA_INGEST_DECODE_PARITY_2026-04-02.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/LEARNINGS.md`

## Validation Run
- `cargo fmt --all` -> PASS
- `cargo check -p viewer_net -p viewer_core -p viewer_app` -> PASS
- `cargo test -p viewer_net` -> PASS
- `cargo test -p viewer_core` -> PASS
- `cargo test -p viewer_app` -> PASS

## Result Status
- PASS (implemented and validated for scoped crates and tests)

## Risks / Follow-Up Items
- Current scene mapping intentionally keeps decoded positions in a bounded diagnostic cluster scale, not 1:1 world-meter placement.
- Rotation decode is not yet surfaced through object-feed proxy orientation; this slice focused on ingestion/decode + position-driven placement.

## Learnings Delta
- added
- Added a durable learning entry on compressed/terse positional payload handling and bounded fallback expectations.

## Continuity Updates Performed
- Updated `docs/CURRENT_STATE.md` with latest object decode parity status.
- Replaced `docs/HANDOFF.md` with this task's latest handoff state and next step.
- Updated `docs/LEARNINGS.md` with new durable learning entry.
