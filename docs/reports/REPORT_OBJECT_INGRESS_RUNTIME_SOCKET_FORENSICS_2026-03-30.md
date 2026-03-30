# Report: Object Ingress Runtime Socket Forensics (2026-03-30)

## Summary of Implemented Work
- Added bounded first-simulator socket diagnostics in `viewer_net` for probe bind, fresh bind, retain, reuse, send, and receive activity.
- Threaded those diagnostics through the retained-probe path, fresh social-circuit handshake path, steady-state social polling, nearby-chat polling, and legacy profile receive path.
- Added `viewer_app` relay summaries at probe, social-circuit open, startup prime, reopen, and the first steady-state window.
- Expanded `viewer_net` tests to lock one-port reuse/fresh-bind behavior through the active social circuit.

## Files Changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/reviews/REVIEW_IMPL_OBJECT_INGRESS_RUNTIME_SOCKET_FORENSICS_2026-03-30.md`
- `docs/reports/REPORT_OBJECT_INGRESS_RUNTIME_SOCKET_FORENSICS_2026-03-30.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/LEARNINGS.md`
- `docs/CLUES.md`

## Validation Run
- `cargo fmt --all` — PASSED
- `cargo check -p viewer_net -p viewer_app` — PASSED
  - note: existing `viewer_render` dead-code warning for `DEBUG_CLIP_SPACE_TRIANGLE` remained a warning, not a failure
- `cargo test -p viewer_net` — PASSED
- `cargo test -p viewer_app` — PASSED
- `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app` — PASSED as a bounded connected run

## Result Status
- Implementation status: complete for the approved runtime socket forensics scope.
- Live outcome: runtime first-simulator traffic stayed on one local UDP port end to end.
- Key bounded live relay lines:
  - `after_probe`: `ports=65241 split=false`
  - `after_open_social_circuit`: `ports=65241 split=false`
  - `after_startup_social_prime`: `ports=65241 split=false`
  - `after_first_steady_state_window`: `ports=65241 split=false events=21 sends=5 receives=13`
- Blocked state remained unchanged:
  - `update_messages=0`
  - `total_objects=0`
  - `region_handshake_updates=0`
  - startup kinds still began with `AgentDataUpdate`, `AgentMovementComplete`, `CoarseLocationUpdate`, `HealthMessage`, `OnlineNotification`, `PacketAck`, and `TestMessage`
- The first steady-state tail also showed same-port inbound traffic beyond the original startup mix, including repeated `0xffff0001` and one-byte message `0x0b`, but still no `ObjectUpdate*`.

## Risks or Follow-up Items
- The live run falsified the current split-port hypothesis for the code now in the repo.
- The next bounded slice should compare missing Firestorm pre-burst/control messages against this now-confirmed one-port runtime path.
- The same-port `0x0b` inbound traffic may also indicate a decode/classification gap that should be checked before assuming every missing behavior is outbound-only.

## Learnings Delta
- added: this slice produced a new durable learning that once live diagnostics show `split=false` on one shared local port, socket continuity should stop being treated as the primary blocker for missing object ingress.

## Continuity Updates Performed
- Added implementation review: `docs/reviews/REVIEW_IMPL_OBJECT_INGRESS_RUNTIME_SOCKET_FORENSICS_2026-03-30.md`
- Updated `docs/CURRENT_STATE.md`
- Updated `docs/HANDOFF.md`
- Updated `docs/LEARNINGS.md`
- Updated `docs/CLUES.md`
