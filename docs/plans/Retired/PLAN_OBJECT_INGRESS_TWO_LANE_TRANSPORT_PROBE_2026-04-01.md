# Plan: Object Ingress Two-Lane Transport Probe (2026-04-01)

## Objective
Investigate the two newly surfaced non-baseline capability lanes by adding bounded startup transport probes that report concrete HTTP behavior per lane without claiming semantic interpretation.

## Scope
- Add a generic one-shot capability transport probe in `viewer_net` that returns status/content-type/body-size for a capability URL.
- Add deterministic lane-target selection in `viewer_app` to probe two host-lane families from non-baseline seed capabilities.
- Emit explicit relay/protocol lines for probe start/result so lane behavior is visible in bounded logs.
- Add/extend unit tests for probe response handling and lane-target selection behavior.

## Current known state
- Seed-cap discovery now surfaces non-baseline capabilities grouped by host (`asset-cdn` and `simhost`).
- LLUDP object ingress remains blocked (`ObjectUpdate*` absent).
- Existing readiness probes cover baseline capabilities (`EventQueueGet`, `InterestList`, `RegionObjects`, `UntrustedSimulatorMessage`) but not newly surfaced non-baseline lanes.

## Files and components touched
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_OBJECT_INGRESS_TWO_LANE_TRANSPORT_PROBE_2026-04-01.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_TWO_LANE_TRANSPORT_PROBE_2026-04-01.md`
- implementation review/report + continuity docs after validation

## Boundary check
- Transport invocation helper remains in `viewer_net` (transport ownership).
- Lane selection and diagnostics emission remain in `viewer_app` (orchestration/observability ownership).
- No crate-boundary or architecture changes.

## Step sequence
1. Add a public `viewer_net` one-shot capability transport probe API returning bounded transport metadata.
2. In `viewer_app`, select up to two non-baseline lane targets (one per host-family where available) from seed capabilities.
3. Run probes once per startup session and log:
   - lane host family
   - capability name
   - classified URL family
   - HTTP status / content-type / body bytes
4. Add/adjust tests for:
   - transport probe status/body-size reporting
   - deterministic two-lane target selection and baseline-cap exclusion.
5. Run validation ladder including bounded live run to capture new relay evidence.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- bounded live run:
  - `VIEWER_APP_LIVE_STARTUP=on`
  - `VIEWER_NETWORK_DEBUG_LOG_PATH=artifacts/logs/network_debug_two_lane_transport_probe_2026-04-01.jsonl`
  - `cargo run -p viewer_app` (bounded capture)

## Risks and open questions
- Some lane probes may return expected non-success statuses (e.g., missing query/body requirements); this is still useful transport evidence.
- Probing must stay bounded and lightweight to avoid startup latency spikes.
- Lane transport success does not imply object-ingress semantic relevance.

## Deferred-too-early candidates captured
- Automatic semantic decode/parsing for non-baseline lane bodies is deferred.
- Repeated polling/probing loops for alternate lanes are deferred (one-shot startup only in this slice).
- No `DEFERRED_FEATURES.md` update required for this bounded observability slice.

## Learnings pre-check
- L58: continue capability/readiness investigation rather than new LLUDP startup guessing.
- L59/L60/L61: treat HTTP transport outcomes as first-class evidence and preserve ordering discipline.
- L62: lane discovery is groundwork; next step is explicit lane probing before conclusions.

## Completion criteria
- Two lane probes execute in bounded startup flow when candidates are available.
- Logs clearly show probe target and transport outcome per lane.
- Validation commands pass and continuity artifacts are updated with exact findings.
