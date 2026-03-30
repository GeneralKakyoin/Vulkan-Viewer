# Plan: Object Ingress Next-Step Roadmap (2026-03-30)

## Objective
Turn the current object-ingress blocker into a decision-complete roadmap that makes the immediate next step explicit and constrains the allowed follow-on directions after that step completes.

The immediate goal is not to guess another startup packet. The immediate goal is to use bounded cross-protocol evidence to choose one justified behavior-change slice that can restore first-simulator `ObjectUpdate*` ingress.

## Scope
- Define the immediate next implementation slice for object-ingress recovery.
- Define the possible follow-on directions that are allowed after the observability slice completes.
- Define the branch-selection criteria so the next fix is chosen from evidence rather than guesswork.
- Keep the roadmap bounded to first-simulator startup control, simulator-host capability behavior, ACK behavior, receive-path surfacing, and startup-state progression.

Out of scope:
- Broad Firestorm startup mirroring in one sweep
- Terrain decode/render work from `LayerData`
- Asset HTTP or texture rendering work
- UI or renderer behavior changes
- Cross-crate architecture changes

## Current known state
- The active retained first-simulator path is now proven to stay on one local UDP port.
- The viewer already sends the currently confirmed Firestorm startup control/request subset:
  - `AgentThrottle`
  - `AgentHeightWidth`
  - `AgentUpdate`
  - observed startup `AgentAnimation`
  - `SetAlwaysRun`
  - `MuteListRequest`
  - `MoneyBalanceRequest`
  - `AgentDataUpdateRequest`
- ACK-trailer parity is already implemented for outbound first-simulator datagrams.
- Live runs still end with:
  - `update_messages=0`
  - `total_objects=0`
  - `region_handshake_updates=0`
- The completed parallel-protocol investigation now proves:
  - the viewer fetches first-region capabilities on simulator-host HTTPS `:12043`
  - the fetched capability inventory includes `EventQueueGet`, `AgentProfile`, `GetDisplayNames`, `SimulatorFeatures`, and `MapLayer` on that simulator-host lane
  - the bounded run surfaces `EventQueueGet:start ack=0 ...` but no completion before shutdown
  - `RegionHandshake`, `RegionHandshakeReply`, and `ObjectUpdate*` remain absent
- Firestorm source and pcap evidence show simulator-host `:12043` traffic continuing in parallel with LLUDP and implement `EventQueueGet` as a persistent `LLEventPoll`.

## Plan audit and active schedule

### Completed or otherwise closed plans
- `PLAN_clippy_fixes.md`
  - completed via `docs/reports/REPORT_clippy_workspace_cleanup.md`
- `PLAN_STARTUP_RECEIVE_ACTIVATION_2026-03-29.md`
  - completed as part of the startup/socket discipline arc
- `PLAN_STARTUP_PROTOCOL_PARITY_2026-03-29.md`
  - completed via `docs/reports/REPORT_STARTUP_PROTOCOL_AND_SOCIAL_SOCKET_DISCIPLINE_2026-03-29.md`
- `PLAN_SINGLE_SOCIAL_SOCKET_DISCIPLINE_2026-03-29.md`
  - completed via `docs/reports/REPORT_STARTUP_PROTOCOL_AND_SOCIAL_SOCKET_DISCIPLINE_2026-03-29.md`
- `PLAN_U14.md`
  - completed; continuity docs already treat `U14` as finished

### Closed but not active follow-up plans
- `PLAN_LLUDP_RELIABILITY_REPLIES_2026-03-29.md`
  - attempted, regressed startup, reverted, and should not return to the active schedule without stronger evidence
- `PLAN_OBJECT_INGRESS_CONTROL_BLOCK_PARITY_2026-03-30.md`
  - staged promotions were exhausted (`AgentHeightWidth`, `SetAlwaysRun`, startup `AgentAnimation`) and the remaining work moved to forensics

### Active unfinished plan on the schedule
1. `PLAN_OBJECT_INGRESS_PERSISTENT_EVENT_QUEUE_2026-03-30.md`
   - selected after the parallel-protocol investigation showed simulator-host caps are present and the current viewer still only reaches a one-shot `EventQueueGet:start ...` without a surfaced completion
2. Follow-on branches remain constrained by this roadmap after that slice:
   - specific simulator-host capability invocation if persistent `EventQueueGet` is necessary but not sufficient
   - LLUDP startup ordering correction only if capability polling no longer looks like the primary gap
   - teleport/region-crossing capability refresh work only after the initial region-entry path is healthy

## Files and components touched
- `docs/plans/PLAN_OBJECT_INGRESS_NEXT_STEP_ROADMAP_2026-03-30.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`

Later implementation from this roadmap is expected to touch:
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`

## Boundary check
- `viewer_net` owns first-simulator transport observability, ACK/control tracking, and receive classification.
- `viewer_app` owns concise relay lines that surface those bounded summaries.
- `viewer_grid` is not the owner of this slice because the unresolved issue is transport/control/receive visibility, not grid-meaning interpretation.
- No renderer, asset, or UI ownership changes are authorized by this roadmap.

## Step sequence
1. Execute the immediate next step:
   - implement `PLAN_OBJECT_INGRESS_PERSISTENT_EVENT_QUEUE_2026-03-30.md`
2. Keep one active persistent EventQueue consumer on the fetched simulator-host URL.
3. Preserve bounded diagnostics for:
   - poll start
   - poll response/timeout/failure
   - ack progression
   - event names/counts
4. Run one bounded connected validation and preserve the log.
5. Compare the result against the completed parallel-protocol investigation report and Firestorm references.
6. Choose exactly one of the follow-on branches below.
7. Write or update the next implementation plan before changing behavior again.

## Follow-on branch map

### Branch A: ACK / control-reply timing correction
Choose this branch only if the new transcript shows one or more of the following:
- inbound reliable/control traffic arrives, but our ACK queue does not drain in the same window
- outbound ACK-trailer usage is absent or delayed relative to the working sequence
- explicit `PacketAck` activity appears out of order around the startup control block
- `RegionHandshake` or object updates appear to be gated behind an ACK/control exchange that never settles

Expected implementation shape:
- small `viewer_net` transport/control change only
- no broad reliability scheduler
- no speculative ping/check parity unless the transcript specifically justifies it

Not allowed in this branch:
- broad reliability mirroring
- adding multiple unrelated startup packets at the same time

### Branch B: Receive-path surfacing / classification correction
Choose this branch only if the new transcript shows one or more of the following:
- raw inbound message numbers are present in the startup window but absent from typed summaries
- packets are reaching the socket but not the current startup reporting path
- `RegionHandshake` or object-update-family packets are present but not counted correctly
- important packets remain in the unclassified bucket with stable message numbers

Expected implementation shape:
- small `viewer_net` classification/export fix
- possibly a bounded `viewer_app` summary-path correction if the packets are already decoded but not surfaced

Not allowed in this branch:
- using guessed packet IDs from memory
- collapsing `viewer_net` and `viewer_grid` responsibilities

### Branch C: Startup ordering correction
Choose this branch only if the new transcript shows one or more of the following:
- we send the right packet family but in a materially different relative order from Firestorm
- ACK/control activity is present, but our local ordering diverges before the first working-object window
- a specific reorder is the smallest justified change after comparison

Expected implementation shape:
- reorder only the smallest justified control block segment
- preserve staged causality so the effect of the reorder remains measurable

Not allowed in this branch:
- broad “match Firestorm exactly” mirroring
- combining reordering with unrelated transport behavior changes

### Branch D: Simulator really is not sending object data on our path
Choose this branch only if the new transcript proves all of the following:
- receive-path surfacing is working,
- classification is working,
- ACK/control exchange is plausibly healthy,
- and the relevant object-update family is genuinely absent on-wire in the bounded window

Expected implementation shape:
- stop implementation guessing
- write a new research-backed plan focused on the smallest still-missing protocol/control prerequisite

Not allowed in this branch:
- claiming object ingress is fixed
- skipping the revised plan step

## Validation plan
- For this roadmap artifact:
  - no code validation required beyond documentation consistency
- For the immediate next implementation slice authorized by this roadmap:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_app`
  - bounded connected run: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`

## Risks and open questions
- The main risk is skipping the observability slice and jumping straight to one favored theory.
- The transcript may reveal a smaller branch than expected; the roadmap must stay evidence-led.
- The correct next fix may be a narrow ordering or surfacing change rather than transport behavior.
- If the transcript proves a truly absent `ObjectUpdate*` window, the next plan must return to Firestorm evidence before any protocol change.

## Deferred-too-Early Candidates Captured
- No new deferred candidates were added here because the active deferred set already captures the main “too early” moves for this area:
  - broad ACK/control behavior changes before transcript observability
  - broad Firestorm control-block mirroring
  - terrain decode/render parity
- This roadmap keeps those existing deferrals in force rather than duplicating them.

## Learnings pre-check
- L27: LLUDP reliability replies cannot be approximated casually.
- L30: ACK-trailer parity alone does not restore object ingress.
- L32: one-port continuity means socket reuse should stop being treated as the primary blocker.
- L33: receiving simulator traffic is not the same as receiving object ingress.
- L34: startup request parity alone is not sufficient.
- L35: `AgentHeightWidth` alone is not sufficient.
- L36: `SetAlwaysRun` alone is not sufficient.
- L37: startup `AgentAnimation` alone is not sufficient.

## Completion criteria
- The repo has one roadmap artifact that states the immediate next step clearly.
- The roadmap names the only acceptable post-observability branches.
- Branch selection is driven by observable evidence, not by another standalone packet guess.
- The roadmap preserves current crate boundaries and does not authorize broad parity work prematurely.
