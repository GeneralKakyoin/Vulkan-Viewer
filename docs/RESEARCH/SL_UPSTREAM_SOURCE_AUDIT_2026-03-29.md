# Source Audit: Second Life Upstreams (2026-03-29)

## Scope
Checked:
- https://github.com/secondlife
- https://bitbucket.org/lindenlab/

Goal context: `N15` continuity probe wiring and LLUDP protocol correctness.

## What is useful
1. **Authoritative LLUDP message template**
   - Source: `secondlife/master-message-template`
   - Files mirrored locally:
     - `reference/secondlife/master-message-template/message_template.msg`
     - `reference/secondlife/master-message-template/message_template.msg.sha1`
     - `reference/secondlife/master-message-template/README.md`
   - Why useful: canonical message IDs and frequency classes used by `viewer_net` packet classification and probe-path safety.

2. **Canonical viewer upstream location**
   - Bitbucket `lindenlab/viewer` `README.md` explicitly says repository moved to:
     - https://github.com/secondlife/viewer
   - Local capture:
     - `docs/RESEARCH/lindenlab_viewer_moved_readme.md`
   - Why useful: avoids pulling stale Bitbucket branches as primary protocol reference.

## Verification notes
- `master-message-template` current SHA1 (from upstream and local mirror):
  - `31d38b956b2926f00f4c40658e4244e88aa1c5c9`
- `secondlife/viewer` `scripts/messages/message_template.msg` SHA1:
  - `aaecaf01b6954c156662f572dc3ecaf26de0ca67`
- Existing Firestorm local template SHA1:
  - `f0e9d30b03fe7823a7050cf1a86653e4de4bc256`
- Result: Firestorm local template snapshot is **not hash-identical** to current master template. Viewer-template snapshot is also different from master-template.

## N15 relevance
- For N15, we should continue to use local Firestorm code for behavior references, but source **message IDs** from the mirrored authoritative `master-message-template` when touching LLUDP classifications or probe-related packet IDs.
- Spot-check confirms key IDs currently used in `viewer_net` still align (e.g., `PacketAck`, `UseCircuitCode`, `CompleteAgentMovement`, `HealthMessage`, `RegionHandshake`, `CrossedRegion`, `ConfirmEnableSimulator`, `AgentDataUpdate`, `ImprovedInstantMessage`, `RetrieveInstantMessages`).

## Source links
- https://github.com/secondlife
- https://github.com/secondlife/viewer
- https://github.com/secondlife/master-message-template
- https://raw.githubusercontent.com/secondlife/viewer/master/scripts/messages/message_template.msg
- https://raw.githubusercontent.com/secondlife/master-message-template/master/message_template.msg
- https://raw.githubusercontent.com/secondlife/master-message-template/master/message_template.msg.sha1
- https://bitbucket.org/lindenlab/
- https://api.bitbucket.org/2.0/repositories/lindenlab/viewer/src/moved/README.md
