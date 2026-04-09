# Plan: Object Ingress Parallel Protocol Startup Investigation (2026-03-30)

## Scope
Build one bounded investigation slice that explains, with source-backed timing, what Firestorm does across LLUDP and simulator-host HTTP(S) during login, region entry, idle, and short teleport, then compare that against the current viewer path.

This slice is diagnostics and comparison support only. It does not authorize capability-behavior changes, persistent long-poll adoption, or new startup message promotions.

## Current known state
- The current viewer now has:
  - one-port first-simulator continuity
  - startup request/control subset parity (`AgentHeightWidth`, `AgentUpdate`, startup `AgentAnimation`, `SetAlwaysRun`, `MuteListRequest`, `MoneyBalanceRequest`, `AgentDataUpdateRequest`)
  - ACK-trailer parity
  - explicit `PacketAck` flush timing
- The current viewer still does **not** receive:
  - `RegionHandshake`
  - `ObjectUpdate*`
- The most recent bounded live run showed:
  - drained ACK queue
  - no `RegionHandshake`
  - no `ObjectUpdate*`
  - new first-steady-state packets `CameraConstraint` and `GenericMessage`
- Firestorm source and the new `Firestorms.pcapng` show meaningful non-UDP startup activity:
  - `LLViewerRegion::setSeedCapability()` installs `EventQueueGet` and starts `LLEventPoll`
  - `LLEventPoll` is a persistent long-poll consumer
  - `llappcorehttp.h` maps several capability classes to simulator-host HTTPS on `:12043`
  - the provided pcap shows repeated simulator-host TLS sessions on `:12043` plus heavier CDN/content traffic

## Files and components touched
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_OBJECT_INGRESS_NEXT_STEP_ROADMAP_2026-03-30.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_PARALLEL_PROTOCOL_STARTUP_INVESTIGATION_2026-03-30.md`
- `docs/reports/REPORT_OBJECT_INGRESS_PARALLEL_PROTOCOL_STARTUP_INVESTIGATION_2026-03-30.md`
- continuity docs: `docs/CURRENT_STATE.md`, `docs/HANDOFF.md`, `docs/LEARNINGS.md`

## Boundary check
- `viewer_net` owns transport/capability URL classification helpers and protocol summaries.
- `viewer_app` owns live-worker sequencing relay output and cross-protocol transcript emission.
- Firestorm remains protocol/reference evidence only, not an architecture template.
- No renderer, asset pipeline, UI, or grid-login architecture changes are allowed in this slice.

## Step sequence
1. Add a new approved plan/review pair for this investigation and mark it as superseding the narrower post-ACK-only classification direction.
2. Extend `viewer_net` protocol summaries so they can surface:
   - first-seen LLUDP milestones for `RegionHandshakeReply`, `CameraConstraint`, and `GenericMessage` in addition to the existing startup markers
   - URL-family classification helpers for simulator-host `:12043`, simulator-host `:12046`, asset CDN, bake-texture CDN, map CDN, and non-gating web/update traffic
3. Extend `viewer_app` startup/steady-state diagnostics so the bounded run emits a unified cross-protocol transcript covering:
   - seed-cap fetch start and result
   - capability inventory grouped by URL family
   - `EventQueueGet` schedule/start/result/ack progression/failure cadence
   - capability-driven HTTP(S) operations currently performed by the app on the live path
   - teleport-region continuity markers if they appear during the bounded window
4. Run the bounded connected viewer validation and preserve the new transcript lines.
5. Run a documented `tshark` extraction against `C:\\Users\\matti\\Desktop\\Firestorms.pcapng` to build a relative-timestamp comparison for:
   - simulator-host `:12043` TLS sessions
   - likely downstream CDN fetches
   - clearly non-gating web/update traffic
6. Write a report that compares our transcript to Firestorm source + pcap evidence and chooses exactly one next implementation branch:
   - persistent `EventQueueGet` adoption
   - missing simulator-host capability invocation
   - LLUDP startup ordering correction
   - teleport/region-crossing capability refresh correction

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- bounded connected run: `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`
- documented `tshark` extraction against `C:\\Users\\matti\\Desktop\\Firestorms.pcapng`

## Risks and open questions
- The viewer may still lack the gating behavior even if the transcript only proves the absence of simulator-host `:12043` traffic.
- The current app uses one-shot event-queue polling in a spawned task; that may reveal the gap without authorizing persistent long-poll implementation yet.
- CDN traffic volume is important context but may still be downstream of already having object/world references, so it must not be mistaken for the primary gate without stronger evidence.

## Deferred-too-early candidates captured
- Persistent `EventQueueGet` implementation remains deferred until the report proves that the current one-shot behavior is the most likely missing gate.
- Any new simulator-host capability client remains deferred until the report names the missing capability family.
- Any additional LLUDP startup message promotion remains deferred until the cross-protocol timeline shows that UDP ordering is still the most justified next branch.

## Learnings pre-check
- L32: once one-port continuity is proven, stop treating socket reuse as the primary blocker.
- L33: same-port simulator traffic does not imply `ObjectUpdate*` ingress.
- L38: when `unclassified=none` and `RegionHandshake` is absent, move away from receive-surfacing guesses.
- L39: explicit ACK flush can drain the queue without restoring object ingress.

## Exact completion criteria
- A bounded viewer run emits a readable cross-protocol transcript covering seed caps, capability families, `EventQueueGet`, and key LLUDP milestones.
- The Firestorm pcap extraction is documented in the execution report with relative timestamps and protocol-family grouping.
- The report states at least one concrete missing or mistimed behavior in the current viewer with repo/source/pcap evidence.
- The next implementation branch is narrowed to exactly one evidence-backed choice.
