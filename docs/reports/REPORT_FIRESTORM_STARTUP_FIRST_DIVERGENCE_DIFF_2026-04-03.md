# Report: Firestorm vs Viewer Startup LLUDP First-Divergence Diff (2026-04-03)

## Summary
Ran a bounded startup-sequence parity diff between:
- Firestorm capture: `artifacts/pcaps/FirestormsFidelis.pcapng`
- Fresh viewer non-strict run: `artifacts/logs/live_startup_parity_non_strict_2026-04-03_172944.log`

Generated normalized ordered comparison artifact:
- `artifacts/logs/startup_first_divergence_diff_2026-04-03_172944.json`

## Files Changed
- `docs/plans/PLAN_FIRESTORM_STARTUP_FIRST_DIVERGENCE_DIFF_2026-04-03.md`
- `docs/reviews/REVIEW_PLAN_FIRESTORM_STARTUP_FIRST_DIVERGENCE_DIFF_2026-04-03.md`
- `docs/reviews/REVIEW_IMPL_FIRESTORM_STARTUP_FIRST_DIVERGENCE_DIFF_2026-04-03.md`
- `docs/reports/REPORT_FIRESTORM_STARTUP_FIRST_DIVERGENCE_DIFF_2026-04-03.md`

## Comparison Result
### Firestorm first outbound startup sequence (first 10)
1. `UseCircuitCode`
2. `CompleteAgentMovement`
3. `ViewerEffect`
4. `PacketAck`
5. `RegionHandshakeReply`
6. `PacketAck`
7. `Low(0x0001)`
8. `AgentHeightWidth`
9. `AgentUpdate`
10. `AgentAnimation`

### Viewer first outbound startup sequence (first 10)
1. `UseCircuitCode`
2. `CompleteAgentMovement`
3. `RegionHandshakeReply`
4. `AgentThrottle`
5. `AgentHeightWidth`
6. `AgentUpdate`
7. `AgentAnimation`
8. `SetAlwaysRun`
9. `MuteListRequest`
10. `MoneyBalanceRequest`

### First Divergence
- Index: **3**
- Firestorm: `ViewerEffect (0x0000ff11)`
- Viewer: `RegionHandshakeReply (0xffff0095)`

## Interpretation
- We currently emit `RegionHandshakeReply` earlier than Firestorm’s observed startup ordering in this route capture.
- Firestorm shows an outbound `ViewerEffect` + `PacketAck` before its `RegionHandshakeReply` in the same startup window.
- This is a concrete, early, protocol-order divergence and a better next patch target than continuing to chase inbound `RegionHandshake` alone.

## Recommended Next Patch
- Introduce bounded ordering parity for fallback `RegionHandshakeReply` on no-handshake routes:
  - do not send fallback reply immediately after CAM
  - delay fallback reply until after first outbound ACK/control checkpoint (or first startup receive checkpoint), matching Firestorm’s observed relative ordering.
- Keep strict mode unchanged; apply this only to fallback path.

## Validation Run
- Firestorm extraction and decode script: PASS
- Fresh non-strict viewer startup capture: PASS
- Deterministic diff artifact generation: PASS

## Risks / Follow-up
- Cross-session comparison can include timing drift; patch should be bounded and reversible.
- After ordering patch, rerun same diff plus live object-gate check to confirm impact.

## Learnings Delta
- `none`.
Reason: evidence identifies next highest-confidence patch target but is not yet proven as a durable invariant across routes.

## Continuity Updates Performed
- Plan + plan review + implementation review + this report added for this investigation slice.
