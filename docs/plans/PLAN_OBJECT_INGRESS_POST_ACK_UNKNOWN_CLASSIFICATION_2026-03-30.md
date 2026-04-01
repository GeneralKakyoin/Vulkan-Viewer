# Plan: Object Ingress Post-ACK Unknown Classification (2026-03-30)

## Objective
Use the new post-ACK-flush live evidence to classify the newly surfaced first-simulator packet kinds and determine whether they represent a remaining control prerequisite or simply broader traffic that still does not explain missing `RegionHandshake` / `ObjectUpdate*`.

## Scope
- Classify the newly surfaced post-ACK packet numbers from the bounded live run:
  - high `22` (`0x00000016`)
  - low `261` (`0xffff0105`)
- Add bounded typed classification and relay surfacing for those packets in `viewer_net` / `viewer_app`.
- Compare the updated transcript against Firestorm/reference behavior before promoting any new behavior change.

Out of scope:
- more ACK timing changes in the same slice
- broad new startup message promotions
- terrain or asset work
- speculative protocol behavior changes without classification evidence

## Current known state
- The explicit ACK flush timing slice succeeded in draining the pending ACK queue.
- That slice did **not** restore:
  - `RegionHandshake`
  - `ObjectUpdate*`
- The bounded live run now shows two previously unclassified packet numbers after the first steady-state window:
  - `0x00000016`
  - `0xffff0105`
- Firestorm message-template lookup identifies those packet numbers as:
  - `CameraConstraint` (High 22)
  - `GenericMessage` (Low 261)

## Files and components touched
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- continuity artifacts under `docs/reports/`, `docs/CURRENT_STATE.md`, `docs/HANDOFF.md`, and `docs/LEARNINGS.md`

## Boundary check
- Packet-number classification belongs in `viewer_net`.
- Relay surfacing belongs in `viewer_app`.
- No renderer, asset, UI, or `viewer_grid` ownership changes are authorized.

## Step sequence
1. Add typed classification for:
   - `CameraConstraint`
   - `GenericMessage`
2. Surface them in the bounded forensic relay output.
3. Re-run the bounded connected validation.
4. Compare whether:
   - they were already present but previously hidden,
   - or they appear only after explicit ACK flush timing changes.
5. Use that result to choose the next branch:
   - deeper control-prerequisite parity if one of these packets is part of the gate,
   - or renewed Firestorm comparison if they are only broader traffic.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- bounded connected run: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`

## Risks and open questions
- These packets may be correlated but not causal.
- `GenericMessage` is a container-like message family, so classification may still require payload-level observation before meaning is clear.
- This slice should not drift back into speculative control-message guessing.

## Deferred-too-Early Candidates Captured
- Any behavior change based on `CameraConstraint` or `GenericMessage` remains deferred until classification and live comparison are complete.

## Learnings pre-check
- L27: LLUDP reliability replies cannot be approximated casually.
- L38: once bounded forensics show `unclassified=none` while `RegionHandshake` stays absent, move to ACK/control timing rather than receive surfacing.
- L39: explicit ACK flush timing can drain the queue without restoring object ingress.

## Completion criteria
- The newly surfaced packet numbers are no longer unnamed in the live forensics path.
- The repo has a tighter, evidence-backed next branch instead of a vague “something else is missing” state.
