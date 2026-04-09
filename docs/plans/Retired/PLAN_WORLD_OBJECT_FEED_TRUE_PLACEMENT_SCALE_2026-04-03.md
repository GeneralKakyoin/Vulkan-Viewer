# Plan: World Object Feed True Placement + Scale (2026-04-03)

## Scope
- Replace debug-cluster placement for world-object-feed proxies with decoded in-region positioning.
- Correct axis mapping for SL/object-feed `X,Y,Z` input into renderer/world `X,Z,Y` convention.
- Preserve existing fallback behavior for objects with missing decoded position data.

## Current known state
- `viewer_net` now exports decoded object positions and scales from LLUDP object updates.
- `viewer_core::world_object_feed_proxy_transform(...)` still compresses object positions into a tiny bounded cluster.
- Current scale mapping is axis-direct (`x,y,z`), which does not match the existing world-space `Y-up` convention used elsewhere.

## Files and components touched
- `crates/viewer_core/src/lib.rs` (transform path + tests)
- continuity docs/review/report artifacts for this slice

## Boundary check
- `viewer_core` remains owner of scene-space transform semantics for ingestion proxies.
- No `viewer_net`, `viewer_grid`, renderer contract, or crate-boundary changes.
- No protocol decode changes in this slice.

## Step sequence
1. Update `world_object_feed_proxy_transform(...)` to:
   - map decoded position in meters without cluster compression
   - map axes as `world_x <- decoded_x`, `world_y <- decoded_z`, `world_z <- decoded_y`
   - preserve ring fallback for missing decoded positions
2. Update scale mapping to the same axis convention (`x,z,y`) with bounded safety clamps.
3. Add regression tests that lock:
   - decoded position maps to deterministic in-region coordinates
   - decoded scale preserves expected axis mapping
4. Run targeted validation for `viewer_core`.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_core`
- `cargo test -p viewer_core`

## Risks and open questions
- True in-region placement can move objects farther from the debug-anchor cluster, which may change first-look framing in existing screenshot smoke paths.
- Rotation/orientation parity is still out of scope; this slice is position + scale only.

## Deferred-too-early candidates
- Add decoded object rotation application (quaternion/path-specific) after position/scale is stable.
- Add camera-fit helper for live verification when true placement spreads objects beyond old debug cluster.

## Learnings pre-check
- `L76`: decoded positional payloads should feed ingest placement when present.
- `L13`: keep transform rotation identity quaternion valid.

## Exact completion criteria
- World-object-feed proxies use decoded in-region coordinates when available.
- Scale axes are mapped consistently with `Y-up` world convention.
- `viewer_core` targeted checks and tests pass.
