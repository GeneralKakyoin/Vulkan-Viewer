# Plan: Startup Receive-First + Immediate ACK Flush Parity Slice (2026-04-03)

## Scope
- Adjust startup ordering to process initial inbound LLUDP before sending the startup-interest bundle.
- Add immediate pending-ACK flush in social poll receive loop.
- Validate by bounded non-strict live run and startup sequence diff.

## Current Known State
- Latest parity diff: first divergence is Firestorm `ViewerEffect` vs viewer `AgentThrottle` at send index 3.
- Object gate remains FAIL in bounded Fidelis run.

## Files / Components Touched
- `crates/viewer_app/src/main.rs`
- `crates/viewer_net/src/lib.rs`
- docs plan/review/report + continuity docs.

## Boundary Check
- No architecture changes; startup orchestration and transport timing only.

## Step Sequence
1. In startup prime, perform a bounded prelude drain before startup interest sends.
2. In `poll_social_events`, flush pending LLUDP ACK IDs immediately after each receive.
3. Run fmt/check/tests.
4. Run bounded non-strict live capture and regenerate first-divergence diff.

## Validation Plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net send_pending_region_handshake_reply -- --nocapture`
- `cargo test -p viewer_app -- --nocapture`
- bounded live run + diff artifact

## Risks / Open Questions
- Could increase early startup traffic volume; must remain bounded by existing packet budgets.
- May still not unlock object ingress if another upstream gate remains.

## Learnings Pre-Check
- L38/L39/L71: evidence-led bounded ordering/timing changes only.

## Completion Criteria
- Startup transcript shows changed early ordering (prelude receive before interest bundle effects).
- ACK flush appears in early window when pending ACK IDs exist.
- Validation commands pass.
