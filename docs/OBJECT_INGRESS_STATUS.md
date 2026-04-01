# Object Ingress Status

Last updated: 2026-03-31

## Working
- Login succeeds and the app reaches `handshake_complete=true`.
- A bounded reconnect-based SLURL teleport control now exists in `Network Debug`.
- First-simulator one-port continuity is working in bounded live runs.
- ACK queue draining works and explicit `PacketAck` sending is available.
- Simulator-host capability fetch on `:12043` works.
- Persistent `EventQueueGet` is alive and returns control/world data.
- `EnableSimulator` events are received and bounded `UseCircuitCode` follow-up is sent to announced ports.
- The primary simulator `RegionObjects` capability returns structured UUID-keyed payloads.
- `RegionObjects` child maps are classified as Firestorm-style pathfinding linkset/object payloads.
- The viewer now surfaces typed `RegionObjects` fields such as:
  - `profile`
  - `linkset_use`
  - `walkability`
  - `name`
  - `description`
  - `owner`
- The viewer now surfaces live `position` values and `position_shape` for the first bounded `RegionObjects` pathfinding-linkset records.
- `RegionObjectsInspection` now emits a bounded typed object sample list using trusted cross-region fields.
- The app relay/debug path now emits a compact `typed_sample=...` summary for the current region.
- The `typed_sample=...` relay has now been live-validated in a bounded connected run.

## Not Working
- LLUDP `RegionHandshake` has still not appeared in bounded live runs.
- LLUDP `RegionHandshakeReply` has still not appeared in bounded live runs.
- LLUDP `ObjectUpdate*` has still not appeared in bounded live runs.
- `object_feed` still reports:
  - `update_messages=0`
  - `total_objects=0`
- The app still does not ingest world objects through the LLUDP object-update path.

## Mixed / Partially Understood
- `RegionObjects` is working as an object-related simulator data lane, but it is not the same thing as LLUDP object ingress.
- The first typed `RegionObjects` records are not fully uniform:
  - some records have a placeholder text description like `(No Description)`
  - some records surface a six-value comma-separated numeric tuple in `description`
  - all currently surfaced first-object samples now include `position`, but the tuple-like `description` content is still not semantically explained
- The current bounded tuple analysis shows:
  - slots `0/1/2/3/5` constant
  - slot `4` varying between `2` and `3`
  - both currently surfaced tuple samples belong to `DSS Candlier Frame`
- Teleport capture showed the tuple family did not survive region change:
  - the next region surfaced different object names and cleaner free-text/empty descriptions
  - this strongly suggests the tuple was local object content rather than a universal schema
- The latest live rerun validated the new summary format, but it did not broaden into a clearly different visible object family.

## Current Best Evidence
- Firestorm pathfinding references match the currently surfaced `RegionObjects` field family:
  - `reference/firestorm/indra/newview/llpathfindingobject.cpp`
  - `reference/firestorm/indra/newview/llpathfindinglinkset.cpp`
- Latest bounded teleport capture:
  - `artifacts/logs/network_debug_region_objects_tuple_teleport_capture_2026-03-31.jsonl`
- Latest bounded typed-feed live validation:
  - `artifacts/logs/network_debug_region_objects_typed_feed_live_validation_rerun_2026-03-31.jsonl`

## Next Active Branch
- If broader sampling is needed, run another bounded reconnect pass with a different SLURL target.
- Otherwise, deepen the now-live-validated `RegionObjects` typed-feed lane.
- Keep LLUDP object-ingress investigation as a separate branch.

## Immediate Questions
- Which next region target gives a broader object family for typed-feed sampling?
- Is the current bounded field set enough for the next object-data consumer, or do we need one more promoted field such as `landimpact`?
- Do we now want to deepen the `RegionObjects` lane, or pivot back to LLUDP with the tuple issue deprioritized?
