# Plan: LLUDP Object-Update Unblock via 4-Step Startup Path (2026-04-01)

## Objective
Implement the 4-step LLUDP unblock slice with bounded diagnostics and runtime knobs so we can move from uncertain startup behavior to decision-complete evidence.

## Scope
- Step 1: verify + expose startup interest send invariant (no blind re-adding).
- Step 2: make AgentUpdate keepalive behavior inspectable and tunable.
- Step 3: add explicit root/child/degraded residency classifier output.
- Step 4: execute bounded live capture matrix and report LLUDP gate verdict.

## Current known state
- LLUDP object gate was still failing (`ObjectUpdate*` absent).
- Startup interest sends were implemented in `viewer_net`, but app-side transcript output was tail-truncated and could hide evidence.
- Keepalive sends existed but lacked structured runtime evidence and A/B knobs.

## Files and components touched
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/TESTING_REFERENCE.md`
- continuity artifacts (report/reviews/current state/handoff/learnings)

## Boundary check
- `viewer_net`: transport mechanics + typed startup/forensics summaries.
- `viewer_app`: orchestration, diagnostics emission, runtime env controls.
- No `viewer_grid` ownership expansion; no crate-boundary shifts.

## Step sequence
1. Add typed startup-interest gate summary in `viewer_net` over full send diagnostics.
2. Add configurable AgentUpdate path in `viewer_net` (`far`, `control_flags`) while preserving default wrapper behavior.
3. Add `VIEWER_APP_AGENT_UPDATE_FAR` and `VIEWER_APP_AGENT_UPDATE_KEEPALIVE_TICKS` in `viewer_app` config parsing.
4. Emit explicit keepalive diagnostics on each periodic send attempt.
5. Expand startup transcript visibility window and emit `startup_interest_gate` line.
6. Add session residency classifier and emit `residency_state=... evidence=...` each protocol summary window.
7. Run validation + bounded live A/B captures (baseline, FAR override, keepalive ticks override).

## Validation plan
1. `cargo fmt --all`
2. `cargo check -p viewer_net -p viewer_app`
3. `cargo test -p viewer_net -p viewer_app`
4. bounded live runs:
   - baseline default
   - `VIEWER_APP_AGENT_UPDATE_FAR=192`
   - `VIEWER_APP_AGENT_UPDATE_KEEPALIVE_TICKS=6`

## Risks and open questions
- EventQueue decode instability may keep residency in `unknown/degraded` despite valid startup sends.
- `ObjectUpdate*` may remain absent; this slice is evidence-first, not guaranteed unblock.

## Deferred-too-early candidates captured
- None in this slice.

## Learnings pre-check
- Applied: L58, L59, L60, L61, L62, L63, L64.
- No conflict with existing architectural constraints.

## Completion criteria
- `startup_interest_gate` line emitted with PASS/FAIL and required message order/packet ids.
- `agent_update_keepalive` lines emitted with period/camera/far/control/reliable fields.
- `residency_state` line emitted in startup and post-steady-state windows.
- LLUDP gate remains explicit and unchanged acceptance criterion.
