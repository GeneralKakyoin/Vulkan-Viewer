# Report: Object Ingress Post-AMC Request Parity (2026-03-30)

## Summary of Implemented Work
- Added first-simulator startup request helpers in `viewer_net` for:
  - `MuteListRequest`
  - `MoneyBalanceRequest`
  - `AgentDataUpdateRequest`
- Wired those one-shot requests into `viewer_app` startup prime on the active retained `SocialCircuit`, immediately after the existing startup interest messages and before `RetrieveInstantMessages`.
- Added `LayerData` first-simulator inbound classification so named receive summaries can surface it when present.
- Added targeted `viewer_net` coverage for:
  - `LayerData` classification
  - startup request parity message IDs and body shapes

## Files Changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_OBJECT_INGRESS_POST_AMC_REQUEST_PARITY_2026-03-30.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_POST_AMC_REQUEST_PARITY_2026-03-30.md`
- `docs/reports/REPORT_OBJECT_INGRESS_POST_AMC_REQUEST_PARITY_2026-03-30.md`
- `docs/reviews/REVIEW_IMPL_OBJECT_INGRESS_POST_AMC_REQUEST_PARITY_2026-03-30.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/LEARNINGS.md`
- `docs/CLUES.md`
- `docs/plans/DEFERRED_FEATURES.md`

## Validation Run
- `cargo fmt --all`: PASSED
- `cargo check -p viewer_net -p viewer_app`: PASSED
- `cargo test -p viewer_net`: PASSED
- `cargo test -p viewer_app`: PASSED
- bounded connected run `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`: PASSED

## Result Status
- The bounded live run confirmed the viewer now sends the targeted startup requests on the same retained first-simulator port:
  - `0xffff0106` (`MuteListRequest`)
  - `0xffff0139` (`MoneyBalanceRequest`)
  - `0xffff0182` (`AgentDataUpdateRequest`)
- The one-port runtime path remained intact:
  - port `53392`
  - `split=false`
- Object ingress remained blocked:
  - `update_messages=0`
  - `total_objects=0`
  - `region_handshake_updates=0`
- The startup receive summary remained:
  - `AgentDataUpdate`
  - `AgentMovementComplete`
  - `HealthMessage`
  - `OnlineNotification`
  - `PacketAck`
  - `TestMessage`
  - `ViewerEffect`
- The bounded live run did not surface `LayerData` in the startup/first-steady-state summaries, even though earlier pcap evidence had shown it on a different run.

## Risks or Follow-up Items
- This slice rules out the narrowed startup request subset as a sufficient fix by itself.
- The next likely blocker is a later control/reply gap or a still-missing inbound decode/classification surface after startup prime.
- The next plan should consider:
  - bounded `SetAlwaysRun` / `AgentAnimation` evidence-led parity
  - or a deeper receive-path comparison for packets observed in pcap but not yet in live summaries

## Learnings Delta
- `added`: `L34` documenting that the startup request subset (`MuteListRequest`, `MoneyBalanceRequest`, `AgentDataUpdateRequest`) does not restore object ingress on the current one-port path.

## Continuity Updates Performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/LEARNINGS.md`
- Updated `docs/CLUES.md`
- Synced deferred scope in `docs/plans/DEFERRED_FEATURES.md`
