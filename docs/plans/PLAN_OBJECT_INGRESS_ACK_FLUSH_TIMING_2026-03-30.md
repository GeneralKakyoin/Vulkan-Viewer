# Plan: Object Ingress ACK Flush Timing (2026-03-30)

## Objective
Promote the smallest justified post-forensics behavior change by correcting ACK/control timing on the active first-simulator circuit, without reintroducing the earlier broad reliability-reply experiment.

## Scope
- Add one bounded outbound ACK-flush path for the active first-simulator `SocialCircuit`.
- Trigger that flush only when pending reliable packet IDs exist and the startup/steady receive window would otherwise leave them queued.
- Keep the change inside `viewer_net` transport helpers and the existing `viewer_app` live-worker orchestration points.
- Preserve the current one-port socket path, startup control ordering, and receive surfacing instrumentation.

Out of scope:
- `StartPingCheck` / `CompletePingCheck` parity
- broad reliability scheduler or resend window logic
- new startup control-message promotions
- receive classification changes

## Current known state
- The completed forensics slice now shows:
  - `unclassified=none` on the bounded startup and first steady-state windows
  - `region_handshake=none`
  - `object_update=none`
  - explicit inbound `PacketAck` observations are present
  - pending ACK queue grew from `4` to `8` by the first steady-state summary
  - outbound appended ACK-trailer usage occurred only once on the current blocked path
- That evidence rules out receive surfacing/classification as the primary next branch and points to ACK/control timing instead.
- Prior naive reliability replies regressed startup and were reverted, so the next change must stay smaller and more controlled than that earlier experiment.

## Files and components touched
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- continuity artifacts under `docs/reports/`, `docs/CURRENT_STATE.md`, `docs/HANDOFF.md`, and `docs/LEARNINGS.md`

## Boundary check
- ACK queueing, flush behavior, and LLUDP control send mechanics belong in `viewer_net`.
- Trigger timing from bounded startup/worker phases belongs in `viewer_app`.
- No `viewer_grid`, renderer, asset, or UI ownership changes are authorized.

## Step sequence
1. Add a bounded `viewer_net` helper that sends an explicit `PacketAck` datagram on an existing `SocialCircuit` for the currently queued ACK IDs.
2. Keep the helper narrow:
   - no resend logic
   - no ping reply logic
   - no duplicate-suppression machinery
3. Call that helper only at one or two bounded points where the forensics show queued ACK IDs can otherwise remain stranded:
   - after startup social-prime receive work
   - optionally after the first steady-state receive window if IDs remain queued
4. Preserve the existing transcript/forensics relay lines so the next connected run proves whether the queue drains and whether `RegionHandshake` / `ObjectUpdate*` behavior changes.
5. Stop after that one timing adjustment and reassess before any broader reliability work.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- bounded connected run: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`

Success criteria:
- pending ACK queue no longer grows unbounded across the bounded startup/first-steady-state window on the same path
- forensics relay clearly shows whether `RegionHandshake` or `ObjectUpdate*` behavior changed

## Risks and open questions
- Explicit ACK flush timing may still be correlated rather than causal.
- Sending `PacketAck` at the wrong point could still perturb startup behavior, so the helper must be scheduled conservatively and validated immediately.
- If the queue drains but `RegionHandshake` remains absent, the next branch may need to target a later control reply rather than receive surfacing.

## Deferred-too-Early Candidates Captured
- Broader reliability reply work (`StartPingCheck` / `CompletePingCheck`, resend windows, duplicate suppression) remains deferred until this smaller ACK-flush slice is validated.

## Learnings pre-check
- L27: LLUDP reliability replies cannot be approximated casually.
- L30: ACK-trailer parity alone does not restore object ingress.
- L32: socket continuity is no longer the primary blocker.
- L33: receiving simulator traffic is not the same as receiving object ingress.
- L38: once bounded forensics show `unclassified=none` while `RegionHandshake` stays absent, the next fix should move to ACK/control timing rather than receive surfacing.

## Completion criteria
- The next behavior-change slice is explicit and limited to ACK flush timing.
- The repo keeps the change smaller than the reverted broad reliability experiment.
- Connected validation leaves a tighter answer about whether ACK/control timing is the remaining gate for object ingress.
