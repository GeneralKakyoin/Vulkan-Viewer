# Plan: Object Ingress EventQueue Control Consumption (2026-03-30)

## Scope
Consume and surface the specific simulator-host `EventQueueGet` messages that are already arriving in the live run so the viewer ingests structured simulator/world data instead of only logging message-name counts.

This slice is bounded to:
- preserving nested EventQueue body data
- extracting actionable/summarizable fields for `EnableSimulator`, `EstablishAgentCommunication`, and `ParcelProperties`
- relaying the extracted details during the bounded live run

Out of scope:
- full multi-region circuit orchestration
- teleport/session handoff parity
- broader capability-family expansion beyond the active first-region EventQueue lane
- claiming LLUDP `ObjectUpdate*` is fixed unless the live run proves it

## Current known state
- Persistent `EventQueueGet` is now active when seed caps are fetched early.
- The bounded live run now receives real simulator-host EventQueue traffic:
  - `AgentGroupDataUpdate`
  - `AgentStateUpdate`
  - `EnableSimulator`
  - `ParcelProperties`
  - `ChatterBoxInvitation`
- The current app behavior only:
  - advances `ack`
  - logs message-name summaries
  - extracts chat-like events
- The current EventQueue parser keeps only flat scalar body fields, so nested LLSD body content can be dropped before the app sees it.
- Firestorm source shows:
  - `LLEventPoll` dispatches EventQueue messages by name (`reference/firestorm/indra/newview/lleventpoll.cpp`)
  - `process_enable_simulator(...)` treats `EnableSimulator` as actionable control traffic that enables a simulator circuit and sends `UseCircuitCode` (`reference/firestorm/indra/newview/llworld.cpp`)
  - `LLEstablishAgentCommunication::post(...)` installs per-region seed capabilities from EventQueue-delivered HTTP messages (`reference/firestorm/indra/newview/llworld.cpp`)
- LLUDP world-object ingress remains blocked:
  - `RegionHandshake = none`
  - `RegionHandshakeReply = none`
  - `ObjectUpdate* = none`

## Files and components touched
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- continuity artifacts under `docs/reports/`, `docs/CURRENT_STATE.md`, `docs/HANDOFF.md`, and `docs/LEARNINGS.md`

## Boundary check
- `viewer_net` owns EventQueue parsing and reusable extraction helpers.
- `viewer_app` owns worker scheduling, relay surfacing, and bounded runtime interpretation.
- This plan does not authorize broad region-session state machines or new crate-boundary crossings.

## Step sequence
1. Extend EventQueue parsing so nested LLSD/JSON body values can be preserved in a stable flattened form instead of dropping non-scalar submaps.
2. Add bounded extraction/summarization helpers in `viewer_net` for:
   - `EnableSimulator`
   - `EstablishAgentCommunication`
   - `ParcelProperties`
3. Update `viewer_app` to relay those extracted details when EventQueue responses arrive.
4. Keep the live output concise and evidence-focused:
   - event name/count summary
   - extracted target/seed/parcel details when available
5. Re-run one bounded connected validation and answer:
   - whether EventQueue already carries actionable simulator/world details on our path
   - whether any newly surfaced details justify a later follow-up branch such as multi-region circuit follow-up
   - whether LLUDP object ingress changes

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- bounded connected run: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`

## Risks and open questions
- `EnableSimulator` evidence may still point to neighbor-region preparation rather than the first missing object-ingress gate.
- Preserving nested EventQueue bodies may reveal useful control data without immediately changing LLUDP ingress.
- This slice must not silently drift into full region-follow or teleport orchestration.

## Deferred-too-early candidates captured
- Full multi-region circuit/session follow-up remains deferred until this slice proves the EventQueue-delivered simulator targets are stable and actionable.
- Automatic per-region seed-cap installation from `EstablishAgentCommunication` remains deferred until the extracted fields are confirmed in our live path.

## Learnings pre-check
- L23: `EnableSimulator` retarget alone is not a sufficient fix.
- L32: one-port continuity means socket reuse is no longer the primary blocker.
- L33: any simulator traffic is not the same as object ingress.
- L40: once simulator-host caps and persistent EventQueue are proven, prioritize consuming that lane over more standalone LLUDP guesses.

## Exact completion criteria
- EventQueue body parsing preserves nested actionable values rather than dropping them.
- The bounded live run surfaces concrete `EnableSimulator` / `EstablishAgentCommunication` / `ParcelProperties` details when present.
- The result clearly states whether we now ingest structured simulator/world data from EventQueue and whether LLUDP object ingress changed.
