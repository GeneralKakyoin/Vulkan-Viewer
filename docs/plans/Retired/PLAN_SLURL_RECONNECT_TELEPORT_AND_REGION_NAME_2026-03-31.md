# Plan: SLURL Reconnect Teleport And Current Region Name (2026-03-31)

## Objective
Add a bounded reconnect-based teleport action driven by SLURLs and make the current region name available in the live/debug surface.

## Scope
- surface the current region name through the live snapshot/debug path
- accept SLURL-like teleport input from the in-app `Network Debug` window
- normalize supported SLURL forms into Firestorm-style login start strings
- queue a reconnect to the requested start location through the existing worker command lane
- log the teleport request and reconnect transition in the existing network-debug JSONL / relay path

Out of scope:
- true in-session teleport parity
- map/search UI
- teleport history
- broad region-handoff/session-orchestration parity

## Current known state
- `viewer_app` already owns startup config parsing, the in-process worker lifecycle, and the live worker command lane.
- `viewer_ui` already has a `Network Debug` window and action return path via `UiActions`.
- `viewer_grid::StartLocationIntent` already supports arbitrary URI strings.
- Firestorm reference shows login start locations for location targets should be shaped as `uri:Region&x&y&z`, not sent as raw `secondlife://...` strings:
  - `reference/firestorm/indra/newview/llslurl.cpp`
  - `reference/firestorm/indra/newview/lllogininstance.cpp`
- The app already tracks a `world_sim_name` fallback locally, but the current region name is not part of `LiveVisualSnapshot` and is not exposed as a first-class live/debug field.

## Files and components touched
- `crates/viewer_core/src/lib.rs`
  - add a safe derived current-region-name field to `LiveVisualSnapshot`
- `crates/viewer_app/src/main.rs`
  - add SLURL normalization helpers
  - add reconnect-teleport command handling to the worker command lane
  - update snapshot/debug aggregation and relay logging
- `crates/viewer_ui/src/lib.rs`
  - add `Network Debug` SLURL input + action button
  - surface current region name in the window
- `docs/plans/DEFERRED_FEATURES.md`
  - capture true in-session teleport parity as deferred

## Boundary check
- `viewer_app` remains the orchestration owner of UI actions, worker commands, reconnect policy, and start-location normalization.
- `viewer_core` only gains a derived display-safe field; no secrets or transport handles are added.
- `viewer_ui` remains display/input only and returns actions through `UiActions`.
- `viewer_net` is not expanded for this slice because reconnect-based teleport reuses the existing login/bootstrap path.

## Step sequence
1. Add `current_region_name` to `LiveVisualSnapshot`, populate it from bootstrap start location and later override it when decoded simulator name becomes available.
2. Add bounded SLURL normalization in `viewer_app` for:
   - `secondlife://Region/x/y/z`
   - `secondlife:///app/teleport/Region/x/y/z`
   - `https://maps.secondlife.com/secondlife/Region/x/y/z`
3. Extend `UiSystem` / `UiActions` so `Network Debug` can submit an SLURL teleport request.
4. Extend the live worker command lane with a reconnect-to-start-location action.
5. When the command is received, log the request, swap the active start location, and force a reconnect so the next login/bootstrap targets the requested location.
6. Surface the current region name and pending teleport input state in the `Network Debug` section and relay/log output.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_core -p viewer_ui -p viewer_app`
- `cargo test -p viewer_core`
- `cargo test -p viewer_ui`
- `cargo test -p viewer_app`
- bounded runtime smoke:
  - `VIEWER_APP_LIVE_STARTUP=off cargo run -p viewer_app`
- live reconnect validation when credentials are present:
  - `VIEWER_APP_LIVE_STARTUP=on cargo run -p viewer_app`
  - manually enter an SLURL in `Network Debug`
  - confirm relay/log shows teleport request normalization and reconnect

## Risks and open questions
- Reconnect-based teleport is intentionally not the same as in-session teleport; users may expect seamless movement later.
- Region-name truth may remain bootstrap-derived until a decoded simulator name appears, because `RegionHandshake` is still absent on the blocked path.
- SLURL parsing must stay bounded to supported forms and reject ambiguous inputs clearly.

## Deferred-too-early candidates captured
- true in-session teleport/session-handoff parity beyond reconnect-based retargeting

## Learnings pre-check
- L06: keep start-location interpretation and reconnect policy clearly in `viewer_app`; do not collapse transport/meaning boundaries under schedule pressure.
- L07: only add a safe derived region-name field to the snapshot; no session secrets or capability URLs.
- L41: avoid widening this into broader `EnableSimulator` / handoff parity work.
- L49: keep the current object-ingress investigation moving in bounded steps rather than broad transport churn.
- L50: this reconnect-based teleport is meant to support the next evidence branch, not replace it with a large session-parity initiative.

## Completion criteria
- The app exposes the current region name in the live/debug surface.
- The `Network Debug` window accepts a supported SLURL and returns a teleport action.
- A submitted SLURL is normalized to the expected login start-string shape and logged.
- The worker reconnects using the new start location without introducing new crate-boundary violations.
