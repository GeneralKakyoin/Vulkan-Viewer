# Plan: ObjectUpdateCached Cache-Miss Recovery (2026-04-02)

## Scope
- Implement viewer-side LLUDP cache-miss follow-up for `ObjectUpdateCached` so we can request full object data and recover mesh UUID-bearing `ExtraParams`.
- Keep scope limited to transport/decode path in `viewer_net` plus bounded verification in `viewer_app` runtime logs.

## Current Known State
- `ObjectUpdateCached` IDs are decoded and inserted into object-feed state, but no follow-up request is sent.
- Firestorm source path (`llviewerobjectlist.cpp` + `llviewerregion.cpp`) issues `RequestMultipleObjects` for cache misses after cached updates.
- Live runs often show object ingress PASS but no mesh UUID discovery (`mesh_candidates=none`, no decoded object-feed mesh IDs).

## Files / Components Touched
- `crates/viewer_net/src/lib.rs`
- `docs/reviews/REVIEW_PLAN_OBJECTUPDATECACHED_CACHE_MISS_RECOVERY_2026-04-02.md`
- `docs/reports/REPORT_OBJECTUPDATECACHED_CACHE_MISS_RECOVERY_2026-04-02.md`
- Continuity docs (`docs/CURRENT_STATE.md`, `docs/HANDOFF.md`, `docs/LEARNINGS.md` if needed)

## Boundary Check
- Keep changes inside networking/protocol transport (`viewer_net`).
- No cross-crate API surface expansion unless strictly necessary.
- No architecture changes; behavior parity only.

## Step Sequence
1. Add pending cache-miss request queueing for decoded `ObjectUpdateCached` local IDs.
2. Implement LLUDP `RequestMultipleObjects` medium-packet encoder (`CacheMissType=0` full object) and flush sender on active social circuit.
3. Flush pending requests during `poll_social_events` loop (bounded chunks), preserving existing handshake/ack behavior.
4. Add targeted tests for encoder/flush behavior and queue draining.
5. Run formatting/check/tests and bounded live run to verify requests are emitted and observe whether full object updates increase mesh ID discovery.

## Validation Plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- bounded live run:
  - `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app` (time-bounded)
  - verify logs show cache-miss request send evidence and inspect object-feed/mesh fetch signals

## Risks / Open Questions
- Excess request volume if cached IDs churn; mitigate via dedupe queue and bounded flush chunks.
- Simulator may still not provide mesh UUIDs quickly in bounded windows.
- Request shape correctness depends on medium-packet encoding parity.

## Deferred-Too-Early Candidates
- CacheMissType CRC-mismatch heuristics (type `1`) based on local object cache state (requires full cache implementation).
- Persistent object cache parity with Firestorm.

## Learnings Pre-Check
- L74: RegionObjects mesh-candidate extraction is best-effort; mesh IDs should come from other lanes.
- L73/L74 context: force deterministic evidence and avoid assuming regional payloads include mesh IDs.
- Existing Firestorm parity captures confirm RequestMultipleObjects path is relevant for cached-object flows.

## Completion Criteria
- `viewer_net` sends `RequestMultipleObjects` after `ObjectUpdateCached` IDs are observed.
- Queue drains reliably on social circuit without regressions to existing startup sends/acks.
- Validation commands pass.
- Bounded live logs capture request-send evidence and updated object/mesh observability status.
