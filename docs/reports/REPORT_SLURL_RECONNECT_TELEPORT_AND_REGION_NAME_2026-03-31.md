# Report: SLURL Reconnect Teleport And Current Region Name (2026-03-31)

## Summary of implemented work
- Added a bounded reconnect-based teleport action to the existing `Network Debug` workflow.
- Added Firestorm-style SLURL normalization so supported SLURL inputs are converted into login start strings of the form `uri:Region&x&y&z`.
- Surfaced the current region name as a display-safe live field and threaded it through the app/debug path.

## Files changed
- `crates/viewer_core/src/lib.rs`
- `crates/viewer_ui/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/plans/DEFERRED_FEATURES.md`
- `docs/plans/PLAN_SLURL_RECONNECT_TELEPORT_AND_REGION_NAME_2026-03-31.md`
- `docs/reviews/REVIEW_PLAN_SLURL_RECONNECT_TELEPORT_AND_REGION_NAME_2026-03-31.md`
- `docs/reports/REPORT_SLURL_RECONNECT_TELEPORT_AND_REGION_NAME_2026-03-31.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/LEARNINGS.md`
- `docs/OBJECT_INGRESS_STATUS.md`

## Validation run
- `cargo fmt --all`
  - PASSED
- `cargo check -p viewer_core -p viewer_ui -p viewer_app`
  - PASSED
- `cargo test -p viewer_core -p viewer_ui -p viewer_app`
  - PASSED
- `cargo check -p viewer_net --example llsd_login_attempt`
  - PASSED
- bounded offline runtime smoke:
  - `VIEWER_APP_LIVE_STARTUP=off cargo run -p viewer_app`
  - PASSED for startup smoke and intentionally stopped after ~8 seconds
  - artifacts:
    - `artifacts/logs/live_slurl_reconnect_teleport_offline_2026-03-31.out.log`
    - `artifacts/logs/live_slurl_reconnect_teleport_offline_2026-03-31.err.log`

## Result status
- The app now exposes current region name in the live/debug path.
- The `Network Debug` window now accepts:
  - `secondlife://Region/x/y/z`
  - `secondlife:///app/teleport/Region/x/y/z`
  - `https://maps.secondlife.com/secondlife/Region/x/y/z`
  - direct `uri:Region&x&y&z`
- Submitted SLURL targets are normalized to Firestorm-style login start strings and queued through the existing worker command lane.
- Worker handling now performs a bounded reconnect using the requested target rather than attempting in-session teleport parity.

## Risks or follow-up items
- Manual live validation of the reconnect teleport flow remains unvalidated in this slice because it requires a credentialed in-world run.
- This is intentionally not true in-session teleport parity; that remains deferred.
- Region name may still be bootstrap-derived until a decoded simulator name appears on the current blocked path.

## Learnings delta
- added
- Added L51 to `docs/LEARNINGS.md`.

## Continuity updates performed
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/LEARNINGS.md`
- Updated `docs/OBJECT_INGRESS_STATUS.md`
- Added the new plan/review/report set for the reconnect-based SLURL slice
