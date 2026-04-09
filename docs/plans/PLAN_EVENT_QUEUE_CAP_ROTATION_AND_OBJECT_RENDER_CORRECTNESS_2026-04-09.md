# Plan: EventQueue Cap-Rotation Recovery + Object Render Correctness (2026-04-09)

## Objective
Implement a bounded reliability and visual-correctness slice that (1) stabilizes late-session EventQueue polling after cap rotation and (2) improves object render correctness evidence for transform/material parity under live ingress.

## Scope
- In scope:
  - EventQueue cap-rotation re-prime branch after bounded `404 cap not found` threshold.
  - Bounded live reconnect/capability refresh behavior to rebind EventQueue URL and resume poll cadence.
  - Object-render correctness hardening focused on existing decoded inputs (transform mapping checks, material/alpha propagation checks, and diagnostics/test coverage) without architecture changes.
  - Validation and continuity updates (report + state + handoff + review artifact).
- Out of scope:
  - Broad protocol redesign.
  - Unlimited retry loops or unbounded reconnect strategies.
  - New rendering architecture, new shader system, or large refactors.

## Current known state
- Latest bounded run (2026-04-09) confirms EventQueue LLSD root-shape decode hardening is effective; prior `missing llsd map` decode blocker is not reproduced.
- Remaining instability is late-session EventQueue `404 cap not found`, currently triggering bounded reconnect behavior without guaranteed cap re-prime success.
- LLUDP object ingress can reach `PASS` in-window; object feed reaches non-trivial counts.
- Object rendering pipeline has major parity groundwork in place (position/scale and face-material retention), while downstream correctness confidence still needs bounded verification hardening.

## Files and components touched
- Primary implementation:
  - `crates/viewer_net/src/lib.rs`
  - `crates/viewer_net/src/worker.rs` (if cap-refresh ownership requires worker-lane change)
  - `crates/viewer_app/src/main.rs` (only bounded diagnostics/relay wiring if needed)
  - `crates/viewer_core/src/lib.rs` and/or `crates/viewer_core/src/math_utils.rs` (only if transform parity tests/helpers need bounded updates)
  - `crates/viewer_render/src/lib.rs` (only if bounded material/alpha correctness bug is found)
- Tests (targeted):
  - `viewer_net` EventQueue/capability reconnect tests
  - `viewer_core` transform mapping tests (if touched)
  - `viewer_render` material/alpha tests (if touched)
- Continuity artifacts:
  - `docs/reviews/REVIEW_PLAN_EVENT_QUEUE_CAP_ROTATION_AND_OBJECT_RENDER_CORRECTNESS_2026-04-09.md`
  - `docs/reviews/REVIEW_IMPL_EVENT_QUEUE_CAP_ROTATION_AND_OBJECT_RENDER_CORRECTNESS_2026-04-09.md`
  - `docs/reports/REPORT_EVENT_QUEUE_CAP_ROTATION_AND_OBJECT_RENDER_CORRECTNESS_2026-04-09.md`
  - `docs/CURRENT_STATE.md`
  - `docs/HANDOFF.md`

## Boundary check
- `viewer_net` owns transport/capability polling/retry mechanics.
- `viewer_grid` meaning/policy boundary remains unchanged.
- `viewer_app` remains orchestration/relay-only; no protocol semantics added.
- `viewer_core` remains domain-state/math ownership only.
- `viewer_render` remains GPU/render behavior owner only.
- No crate-boundary changes and no cross-crate policy collapse.

## Step sequence
1. Add bounded EventQueue cap-rotation re-prime logic:
   - Detect `404 cap not found` in EventQueue poll path.
   - When threshold is reached, run bounded cap refresh/rebind path.
   - Resume EventQueue polling with refreshed URL and preserve existing backoff/guard semantics.
2. Add or update diagnostics to make re-prime transitions explicit:
   - include pre-threshold count, threshold-trigger, re-prime result, and resumed poll outcome.
3. Add targeted `viewer_net` tests:
   - threshold-triggered re-prime path
   - successful rebind resumes poll
   - fail-safe behavior when re-prime fails (bounded/reported, not infinite loop)
4. Perform bounded object-render correctness pass (only for concrete regressions found during validation):
   - verify transform parity invariants remain correct (rotation identity defaults preserved; mapping tests green)
   - verify material/alpha application remains deterministic for decoded face/default materials
   - add/update targeted tests if a bug is fixed.
5. Run validation ladder.
6. Run bounded live verification with explicit env knobs and capture fresh artifact.
7. Write implementation review + execution report + continuity updates.

## Validation plan
1. `cargo fmt --all`
2. `cargo check --workspace`
3. `cargo test -p viewer_net`
4. `cargo test -p viewer_core`
5. `cargo test -p viewer_render`
6. Bounded live run:
   - `cargo run -p viewer_app` with
     - `VIEWER_APP_LIVE_STARTUP=on`
     - `VIEWER_APP_LLUDP_STARTUP_PARITY_BUNDLE=on`
     - `VIEWER_APP_CAPABILITY_PROBES_REQUIRE_EVENT_QUEUE_OK=true`
     - `VIEWER_APP_EVENT_QUEUE_CAP_NOT_FOUND_BEFORE_RECONNECT=3`
     - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_event_queue_cap_rotation_and_render_correctness_2026-04-09.jsonl`
7. Manual log/screenshot review where applicable (L22 compliance).

## Risks and open questions
- Simulator-side cap lifecycle may still produce unavoidable reconnect churn; success criteria must be bounded and evidence-based.
- Re-prime logic can regress startup gate behavior if not carefully isolated from first-success flow.
- Render-correctness pass must avoid scope creep into full parity roadmap.

## Deferred-too-early candidates captured
- No new deferred candidate added in this plan.
- Potential full RenderMaterials parity expansion remains deferred to a dedicated later milestone slice if uncovered by this bounded pass.

## Learnings pre-check
- L13: preserve identity quaternion defaults and avoid invalid rotation initialization.
- L22: screenshot/live visual verification requires manual artifact review, not command success alone.
- L64: lane-specific capability behavior must be validated with lane-accurate requests/evidence.
- L68/L69/L71/L77: startup/object-ingress gating is stage-sensitive; rely on explicit gate evidence.
- L78/L81: protocol decode correctness depends on exact payload layout and parser ownership boundaries.
- L82: keep changes crate-local and behavior-local; avoid monolith growth.

## Completion criteria
- EventQueue cap-rotation re-prime path implemented and covered by targeted tests.
- Fresh bounded live artifact demonstrates explicit re-prime diagnostics and no regression of startup/object-ingress baseline behavior.
- Object-render correctness checks completed with any discovered bounded fixes plus targeted tests.
- Validation commands and outcomes fully recorded.
- Continuity docs updated with exact current state and exact next step.
