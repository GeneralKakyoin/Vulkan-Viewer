# Implementation Review: U04 (Usability Shell)

## Verdict
**Approved with minor required fix** (one warning + minor docs inconsistencies).

The U04 plan deliverables are implemented: a shared UX session-status contract is owned by `viewer_core`, mapped in `viewer_app`, rendered consistently in `viewer_ui`; UI surfaces are regrouped into Session/Social/Diagnostics; chat + IM sends are gated on session connectivity; profile freshness is explicit (`Fresh`/`Stale`/`Unknown`) with deterministic age formatting; runtime relay has minimal category/message search + min-level filtering.

## Architecture and boundary fit
- **Good boundary discipline:** `viewer_core` owns shared UX types (`SessionUxStatus`, `SessionUxReason`) and the pure freshness helper; `viewer_app` maps its internal state into the bounded UX status; `viewer_ui` renders chips and remains action-emitting presentation logic.
- **Renderer change is scope-adjacent:** `viewer_render` updates `wgpu` copy type aliases. This is not part of U04’s stated scope, but it is small and plausibly justified as “keep build output clean”; treat as incidental cleanup and document it consistently (or revert to keep U04 maximally bounded).

## Plan alignment (what matches)
- **Session status contract is pinned and implemented** exactly as specified in `docs/plans/PLAN_U04.md` “Required plan pinning”.
- **Diagnostics regrouping:** “Runtime Relay” is now under the “Diagnostics” window as a collapsible section.
- **Diagnostics filters:** relay supports min-level selection and text search (category/message substring).
- **Chat + IM guardrails:** send buttons and enter-to-send are disabled unless `SessionUxStatus::Connected`.
- **Profile freshness:** UI shows explicit freshness label + deterministic age string + error line; raw unix-ms display is no longer surfaced as a header “updated: …” field.
- **Logic tests:** new tests cover `compute_profile_freshness` logic + age formatting (`viewer_core`) and session status chip labeling (`viewer_ui`).

## Correctness notes
- **Reconnecting mapping:** current `viewer_app` mapping uses `ChatConnectionState::Reconnecting` as a UX signal. This is acceptable as long as “chat reconnecting” is truly coupled to “session reconnecting”; if those can diverge later, add a dedicated session-level reconnect signal in `viewer_app` and keep `viewer_ui` unaware of the distinction.
- **Relay filter allocations:** the current filter implementation lowercases the filter string inside the per-event predicate; consider hoisting the lowercase conversion once per frame for efficiency.

## Validation (run for this review)
- `cargo fmt`: PASS
- `cargo check`: PASS, **1 warning** in `viewer_ui` (`unused_mut` around relay filtering)
- `cargo test -p viewer_core -p viewer_ui`: PASS (45 + 5 tests)

## Not validated here
- Runtime smoke (`VIEWER_APP_LIVE_STARTUP=off`, `cargo run -p viewer_app`) to visually confirm layout, offline status chip, and gated social controls.

## Documentation / continuity issues
- `docs/CURRENT_STATE.md` currently describes a different set of “UX-facing session states” than the implemented `SessionUxStatus` labels (`disabled/starting/connected/reconnecting/failed`). Update wording to match the actual contract/UI labels.
- `docs/CURRENT_STATE.md` / `docs/HANDOFF.md` claim “0 warnings”, but `cargo check` currently emits one `viewer_ui` warning.

## Required minor fix (before calling “clean”)
1. Remove the `unused_mut` in the relay filtering path (`crates/viewer_ui/src/lib.rs`) so `cargo check` is warning-free, or adjust “0 warnings” claims in continuity docs to be accurate.
