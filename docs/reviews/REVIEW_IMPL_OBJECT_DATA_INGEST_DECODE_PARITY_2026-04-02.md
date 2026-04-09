# Review: Implementation Object Data Ingestion/Decoding Parity 2026-04-02

## Verdict
Approved.

## Architecture And Boundary Fit
- `viewer_net` owns packet decode/state retention changes.
- `viewer_app` remains a pure mapping/orchestration bridge.
- `viewer_core` owns seam payload and scene proxy transform policy.
- No `viewer_net`/`viewer_grid` ownership erosion observed.

## Correctness Concerns
- Decode paths now treat packet-family payloads with explicit per-block lengths and preserve fail-safe `Option` behavior on malformed data.
- `ObjectUpdateCached` remains ID-only by design, matching packet layout.

## Modularity And Maintainability Concerns
- Additive field changes (`position_centi`) were propagated through typed contracts cleanly.
- Parsing logic remains helper-scoped rather than inlined into observation dispatch.

## Validation Adequacy
- Required checks and targeted tests were executed and passed:
  - `cargo fmt --all`
  - `cargo check -p viewer_net -p viewer_core -p viewer_app`
  - `cargo test -p viewer_net`
  - `cargo test -p viewer_core`
  - `cargo test -p viewer_app`

## Risks And Open Questions
- Scene placement uses bounded projection of region-local coordinates for diagnostics, not full world-meter parity.
- Proxy rotation is still fallback/identity in this slice.

## Learnings Delta Verdict
- add
- Reason: new durable lesson recorded on compressed/terse positional payload usage and fallback policy.

## Required Revisions Or Approval Status
- Approved; no blocking revisions required.
