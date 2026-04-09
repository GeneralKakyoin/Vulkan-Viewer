# Plan: Object Ingress LLUDP Startup Parity Bundle (2026-04-01)

## Scope
- Add a runtime-gated LLUDP startup parity bundle path in `viewer_app` for bounded live experimentation.
- Keep the existing default startup path unchanged when the new flag is disabled.
- Add explicit pass/fail startup gate relay output for first `ObjectUpdate*` evidence with local-id signal.

Out of scope:
- New LLUDP startup message families
- Additional simulator-host capability clients
- Transport architecture changes across crate boundaries
- UI feature expansion

## Current known state
- LLUDP world-object ingress remains blocked (`RegionHandshake`, `RegionHandshakeReply`, `ObjectUpdate*` absent in bounded runs).
- The startup control/request subset and EventQueue work are already in place.
- `viewer_net` already exposes startup timeline/transcript summaries and object-feed counters with local IDs.
- Existing startup priming logic in `viewer_app` already sends:
  - pending `RegionHandshakeReply`
  - startup interest messages
  - startup request parity messages
  - `RetrieveInstantMessages`
  - startup social drain

## Files and components touched
- `crates/viewer_app/src/main.rs`
- `docs/TESTING_REFERENCE.md`
- continuity and execution artifacts under:
  - `docs/reports/`
  - `docs/CURRENT_STATE.md`
  - `docs/HANDOFF.md`
  - `docs/OBJECT_INGRESS_STATUS.md`
  - `docs/LEARNINGS.md`

## Boundary check
- `viewer_app` owns startup orchestration and runtime env-gated behavior selection.
- `viewer_net` behavior is consumed through existing public APIs only.
- No ownership migration between `viewer_net` and `viewer_grid`.
- No renderer/asset/UI policy changes.

## Step sequence
1. Add `VIEWER_APP_LLUDP_STARTUP_PARITY_BUNDLE` config parsing in `viewer_app` (`bool`, default `off`).
2. Split startup social prime into two explicit modes:
   - baseline mode (current behavior)
   - parity-bundle mode (same bounded startup cluster plus explicit startup ACK flush inside prime).
3. Keep the existing post-prime ACK flush helper call so non-bundle behavior remains unchanged and relay output stays comparable.
4. Add one explicit startup gate relay line that reports:
   - `first_object_update_index`
   - `decoded_object_feed_update_messages`
   - `decoded_object_feed_total_objects`
   - a bounded preview of first local IDs
   - pass/fail verdict based on `ObjectUpdate*` index plus non-empty local-id evidence.
5. Add/update targeted unit tests for new env parsing and gate formatting behavior.
6. Update `docs/TESTING_REFERENCE.md` with the new runtime flag.
7. Run required validation and capture results in report + continuity docs.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_app`
- `cargo test -p viewer_net`

Live validation command (bounded, evidence run target for follow-up):
- `VIEWER_APP_LIVE_STARTUP=on`
- `VIEWER_APP_LLUDP_STARTUP_PARITY_BUNDLE=on`
- `VIEWER_NETWORK_DEBUG_LOG_PATH=<artifact-path>`
- `cargo run -p viewer_app`

## Risks and open questions
- The parity bundle may still fail to produce `ObjectUpdate*`, which is expected and must trigger immediate capability-readiness branching.
- Additional ACK flushing could affect startup timing; bundle must remain runtime-gated and default-off.
- Pass/fail output must avoid false positives by requiring both update-index evidence and non-empty local IDs.

## Deferred-too-early candidates captured
- No new deferred candidates added in this slice.
- Existing deferrals for broad ACK/control mirroring and wider capability expansion remain in force.

## Learnings pre-check
- L33: same-port simulator traffic is not object-ingress success.
- L39/L40: observability and EventQueue behavior can progress while LLUDP object ingress remains zero.
- L54: runtime conclusions require fresh built binaries and explicit evidence markers.
- L55/L56/L57: route-dependent `RegionObjects` behavior should not be conflated with LLUDP unblock.

## Completion criteria
- New runtime flag exists and defaults off.
- Startup prime behavior is clearly mode-gated without boundary drift.
- Startup pass/fail gate line appears with local-id evidence fields.
- Required static/tests pass.
- Continuity artifacts are updated with exact validation status.
