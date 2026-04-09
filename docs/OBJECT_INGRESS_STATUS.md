# Object Ingress Status

Last updated: 2026-04-01

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
  - `landimpact`
- The viewer now surfaces live `position` values and `position_shape` for the first bounded `RegionObjects` pathfinding-linkset records.
- `RegionObjectsInspection` now emits a bounded typed object sample list using trusted cross-region fields.
- The app relay/debug path now emits a compact `typed_sample=...` summary for the current region.
- The `typed_sample=...` relay has now been live-validated in a bounded connected run.
- Reconnect-only delayed `RegionObjects` re-probe logic is implemented in `viewer_app`, with explicit relay markers for `reprobe_armed`, `reprobe_ok`, and `reprobe_err`.
- `RegionObjects` transcript lines now include `host_family=...` tags for route identity on primary/reprobe success/error paths.
- A runtime-gated LLUDP startup parity-bundle mode now exists in `viewer_app`:
  - `VIEWER_APP_LLUDP_STARTUP_PARITY_BUNDLE=on|off` (default `off`)
- First-simulator forensics now emit an explicit LLUDP startup gate verdict line with bounded local-id preview.
- Capability-readiness matrix relays are now live for `EventQueueGet`, `InterestList`, `UntrustedSimulatorMessage`, and `RegionObjects`.
- One-shot capability-readiness probes now run for available `InterestList` and `UntrustedSimulatorMessage` URLs.
- Capability-readiness probes can now be gated on first EventQueue success:
  - `VIEWER_APP_CAPABILITY_PROBES_REQUIRE_EVENT_QUEUE_OK` (default `true`)
- `UntrustedSimulatorMessage` probe now supports GET->POST fallback and reports transport-success readiness when the response body is non-LLSD.
- EventQueue has bounded cap-not-found reconnect handling (`VIEWER_APP_EVENT_QUEUE_CAP_NOT_FOUND_BEFORE_RECONNECT`, default `3`).
- Seed-capability discovery is now Firestorm-aligned and broad (rather than minimal).
- Startup/follow-up relays now include host-grouped non-baseline capability names (`seed non-baseline capability names by host: ...`).
- Startup now runs bounded two-lane non-baseline transport probes (max two host families).

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
- A broader-SLURL reconnect capture (`Morris`) produced a new split result:
  - pre-teleport still showed valid typed samples (`Object`, `bamboo`)
  - post-reconnect first bounded startup probe returned `keys=<root>` and `typed_sample=none`
  - this points to a likely post-reconnect timing window rather than a proven schema regression
- Reconnect re-probe is now live-validated, and for the tested target path it still returned `typed_sample=none` after delay.
- This weakens a pure timing explanation for post-reconnect emptiness and points more toward route/region-dependent behavior on this lane.
- A follow-up target-comparison run (`secondlife://Ahern/50/60/70`) showed the opposite outcome in the same bounded session:
  - pre-reconnect simhost returned `typed_sample=none`
  - post-reconnect simhost returned rich `typed_sample=...` on both primary probe and delayed re-probe
- Current interpretation: this lane is operational but variable by reconnect target/region/content, and must be evaluated with paired captures rather than single-run conclusions.
- Paired A/B captures with identical knobs now reproduced an inversion:
  - Morris run: rich pre-reconnect, empty post-reconnect
  - Ahern run: empty pre-reconnect, rich post-reconnect
- In these runs, outcome aligned with simulator-host family:
  - `simhost-04e63a...` => rich typed samples
  - `simhost-0a962c...` => empty `typed_sample`
- The new LLUDP startup parity-bundle gate was run live and stayed `FAIL`:
  - startup and first steady-state lines both reported:
    - `object_update=none`
    - `update_messages=0`
    - `total_objects=0`
    - `local_ids=none`
- Latest EventQueue-gated readiness run showed ordering-sensitive outcomes:
  - `InterestList` probe => `ok` (`keys=mode,stats`) after first `EventQueueGet:ok`
  - `UntrustedSimulatorMessage` probe => `ok` via GET->POST fallback (`probe_transport_status=200`, `probe_decode=unparsed_body`)
- Latest seed-cap broadening run showed explicit multi-host lane discovery:
  - `asset-cdn:count=4 names=GetMesh,GetMesh2,GetTexture,ViewerAsset`
  - `simhost-...:count=102 ...`
- Interpretation update: alternate capability lanes are now proven present; this has not yet translated into LLUDP object ingress recovery.
- Latest two-lane probe outcomes on this path:
  - `asset-cdn` / `ViewerAsset` => `403 application/xml` (`body_bytes=275`)
  - `simhost-...` / `SimulatorFeatures` => `503 text/plain` (`body_bytes=13`)
- Interpretation update: both lanes respond at transport level; non-2xx currently reflects lane behavior/policy, not lane absence.
- EventQueue continues to degrade into `cap not found` failures later in the run; bounded reconnect now triggers when the cap-not-found threshold is reached.

## Current Best Evidence
- Firestorm pathfinding references match the currently surfaced `RegionObjects` field family:
  - `reference/firestorm/indra/newview/llpathfindingobject.cpp`
  - `reference/firestorm/indra/newview/llpathfindinglinkset.cpp`
- Latest bounded teleport capture:
  - `artifacts/logs/network_debug_region_objects_tuple_teleport_capture_2026-03-31.jsonl`
- Latest bounded typed-feed live validation:
  - `artifacts/logs/network_debug_region_objects_typed_feed_live_validation_rerun_2026-03-31.jsonl`
- Latest broader-SLURL capture:
  - `artifacts/logs/network_debug_region_objects_typed_feed_broader_slurl_capture_2026-04-01.jsonl`
- Latest target-comparison capture (`Ahern`):
  - `artifacts/logs/network_debug_region_objects_post_reconnect_reprobe_target_ahern_2026-04-01.jsonl`
- Latest typed-field promotion capture (`landimpact`):
  - `artifacts/logs/network_debug_region_objects_typed_landimpact_2026-04-01.jsonl`
- Latest paired A/B reconnect captures:
  - `artifacts/logs/network_debug_region_objects_reconnect_ab_morris_2026-04-01.jsonl`
  - `artifacts/logs/network_debug_region_objects_reconnect_ab_ahern_2026-04-01.jsonl`
- Latest capability-readiness run:
  - `artifacts/logs/network_debug_capability_readiness_2026-04-01.jsonl`
- Latest EventQueue-gated capability-readiness run:
  - `artifacts/logs/network_debug_capability_probe_event_queue_gating_2026-04-01.jsonl`
- Latest untrusted POST-fallback capability-readiness run:
  - `artifacts/logs/network_debug_untrusted_probe_post_fallback_2026-04-01.jsonl`
- Latest seed-cap broadening host-lane discovery run:
  - `artifacts/logs/network_debug_seed_cap_broadening_host_lane_discovery_2026-04-01.jsonl`
- Latest two-lane transport probe run:
  - `artifacts/logs/network_debug_two_lane_transport_probe_2026-04-01.jsonl`

## Next Active Branch
1. Add lane-specific invocation-shape probes for the two measured lanes (`ViewerAsset`, `SimulatorFeatures`) so transport outcomes can be interpreted beyond generic GET.
2. Correlate shaped-lane outcomes with existing LLUDP gate failure baseline:
   - `artifacts/logs/network_debug_lludp_startup_parity_bundle_2026-04-01.jsonl`
   - `artifacts/logs/network_debug_two_lane_transport_probe_2026-04-01.jsonl`
3. Keep strict pass/fail acceptance unchanged for any future LLUDP-ingress claim.

## Immediate Questions
- What request shaping does `ViewerAsset` require on this path beyond unauthenticated generic GET?
- Is the `SimulatorFeatures` `503` transient service behavior or a stable lane policy on this simhost route?
