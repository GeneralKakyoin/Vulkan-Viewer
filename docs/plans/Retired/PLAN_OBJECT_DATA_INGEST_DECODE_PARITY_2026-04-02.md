# Plan: Object Data Ingestion/Decoding Parity (Firestorm + OpenSim) 2026-04-02

## Objective
Implement a fuller, bounded object-feed ingestion/decode path so live object proxies are driven by decoded LLUDP object spatial data (not local-id ring placement), while preserving crate boundaries and existing seam ownership rules.

## Scope
- Extend `viewer_net` object-feed decode/export to include decoded object position metadata from:
  - `ObjectUpdate`
  - `ObjectUpdateCompressed`
  - `ImprovedTerseObjectUpdate`
- Keep `ObjectUpdateCached` support as ID-level refresh only (packet does not carry full spatial payload).
- Propagate new decoded fields through `viewer_app` to `viewer_core` snapshot/seam paths.
- Update world-object proxy transform logic to consume decoded position when present.
- Add/expand targeted tests for decode correctness and ingestion mapping.

## Current Known State
- `viewer_net` object feed currently exports local id, optional scale, optional full object UUID.
- `viewer_core` world-object transform is primarily local-id ring-based and only uses optional scale.
- Existing decode for compressed/terse paths currently extracts local IDs only.
- Existing bounded references already available in repo:
  - Firestorm message layout and object-update handling
  - OpenSim packet construction for compressed/terse updates

## Files And Components Touched
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `crates/viewer_core/src/lib.rs`
- `docs/reviews/REVIEW_PLAN_OBJECT_DATA_INGEST_DECODE_PARITY_2026-04-02.md`
- `docs/reports/REPORT_OBJECT_DATA_INGEST_DECODE_PARITY_2026-04-02.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/LEARNINGS.md` (delta review outcome recorded)

## Boundary Check
- `viewer_net`: transport/decode mechanics only; no UI/render policy.
- `viewer_app`: mapping/orchestration only.
- `viewer_core`: seam-owned world proxy lifecycle and transform rules.
- No `viewer_grid` semantic ownership changes.
- Firestorm/OpenSim used as protocol behavior references only, not architecture templates.

## Step Sequence
1. Add bounded spatial fields to `DecodedObjectFeedObject`/internal feed state in `viewer_net`.
2. Expand decode helpers:
   - `ObjectUpdate`: extract position from `ObjectData` fixed payload when present.
   - `ObjectUpdateCompressed`: extract local id + scale + position from packed compressed block.
   - `ImprovedTerseObjectUpdate`: extract local id + position from terse block.
3. Update object-feed upsert/export to retain and surface newly decoded fields.
4. Map new fields in `viewer_app` into `viewer_core::DecodedWorldObjectFeedObject`.
5. Extend seam item payload fields in `viewer_core` and propagate through adapter.
6. Update `world_object_feed_proxy_transform(...)` to prefer decoded position when available, with existing bounded fallback behavior when absent.
7. Add/adjust targeted tests in `viewer_net`, `viewer_app`, and `viewer_core` for decode and transform pathways.
8. Run required validation commands.
9. Write execution report + continuity updates.

## Validation Plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_core -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_core`
- `cargo test -p viewer_app`

## Risks And Open Questions
- Compressed and terse payload shapes are format-sensitive; malformed length handling must remain fail-safe (`None` decode on invalid blocks).
- Position mapping into bounded diagnostic scene must avoid extreme jumps while still reflecting decoded spatial ordering.
- Existing focus UUID filtering behavior must remain unchanged.

## Deferred-Too-Early Candidates Captured
- None identified for this slice (work is directly on current object-ingress path).

## Learnings Pre-Check
- L10: packet/message IDs must come from template source.
- L15: protocol-sensitive behavior must be source-anchored, not memory-based.
- L75: object UUID focus is LLUDP FullID-based; preserve behavior.
- L68/L69/L71: handshake-side unblock is already handled; this slice remains decode/ingest quality only.

## Completion Criteria
- Object-feed export includes decoded position metadata where packets carry it.
- `viewer_core` world-object proxies use decoded position when available and fall back safely otherwise.
- Tests and static checks pass for touched crates.
- Execution report, CURRENT_STATE, HANDOFF updated.
