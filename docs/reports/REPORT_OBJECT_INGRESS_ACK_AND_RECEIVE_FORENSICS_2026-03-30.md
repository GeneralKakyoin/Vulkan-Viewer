# Report: Object Ingress ACK And Receive Forensics (2026-03-30)

## Summary of Implemented Work
- Added bounded first-simulator forensic summaries in `viewer_net` for:
  - ACK queue state
  - appended ACK-trailer send usage
  - raw packet message-number counts
  - unclassified packet-number counts
  - first-observation indices for `RegionHandshake`, `AgentMovementComplete`, `PacketAck`, and object-update-family packets
  - ordered startup send/receive transcript tails
- Surfaced those summaries in `viewer_app` relay output during:
  - startup decode summary
  - first steady-state socket summary window
- Audited open plan artifacts and updated the roadmap so only genuinely active unfinished plans remain on the current schedule.

## Files Changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_OBJECT_INGRESS_NEXT_STEP_ROADMAP_2026-03-30.md`
- `docs/plans/PLAN_OBJECT_INGRESS_ACK_FLUSH_TIMING_2026-03-30.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_ACK_FLUSH_TIMING_2026-03-30.md`
- `docs/reports/REPORT_OBJECT_INGRESS_ACK_AND_RECEIVE_FORENSICS_2026-03-30.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/LEARNINGS.md`

## Validation Run
- `cargo fmt --all`: PASSED
- `cargo check -p viewer_net -p viewer_app`: PASSED
- `cargo test -p viewer_net`: PASSED
- `cargo test -p viewer_app`: PASSED
- bounded connected run `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`: PASSED

## Result Status
- Observability goal completed.
- The bounded live run produced the missing discriminators:
  - startup `unclassified=none`
  - first steady-state `unclassified=none`
  - `region_handshake=none`
  - `object_update=none`
  - explicit inbound `PacketAck` observations are present
  - pending ACK queue grew from `4` at startup summary to `8` by the first steady-state summary
  - outbound appended ACK usage was observed only once on the blocked path
- That evidence narrows the next behavior-change branch to ACK/control timing rather than receive surfacing/classification.

## Risks or Follow-up Items
- The selected next slice is `docs/plans/PLAN_OBJECT_INGRESS_ACK_FLUSH_TIMING_2026-03-30.md`.
- The reverted broad reliability-reply experiment remains out of bounds unless this smaller ACK-flush plan fails and new evidence justifies widening scope.
- If explicit ACK flush timing still leaves `RegionHandshake` absent, the next plan will need to reassess a later control-reply or ordering nuance.

## Learnings Delta
- added
  - bounded forensics can prove that receive surfacing is not the primary blocker even when object ingress is still zero

## Continuity Updates Performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/LEARNINGS.md`
- Added this report
