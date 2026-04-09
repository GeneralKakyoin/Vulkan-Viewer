# Plan: Object Ingress Seed-Cap Broadening And Host-Lane Discovery (2026-04-01)

## Objective
Broaden seed-capability discovery to a Firestorm-aligned capability set and add explicit per-host capability-name logging so we can validate whether object-relevant data is routed through non-current HTTP capability lanes.

## Scope
- Expand `viewer_net` seed capability request list from the current minimal set to a Firestorm-aligned broader set.
- Add bounded startup/follow-up logging in `viewer_app` that summarizes non-baseline capability names by host-family.
- Update/extend tests impacted by the expanded request list and new helper logic.

## Current known state
- LLUDP object ingress remains blocked on current `simhost-0eec...` captures (`ObjectUpdate*` absent).
- Current seed request is narrow (`EventQueueGet`, `UntrustedSimulatorMessage`, `InterestList`, `RegionObjects`, plus existing profile/texture entries already requested by code).
- Firestorm requests a substantially broader capability list during seed setup (`llviewerregion.cpp` capability append list).
- Current logs do not provide a host-family grouped view of discovered non-baseline capability names.

## Files and components touched
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/plans/PLAN_OBJECT_INGRESS_SEED_CAP_BROADENING_HOST_LANE_DISCOVERY_2026-04-01.md`
- `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_SEED_CAP_BROADENING_HOST_LANE_DISCOVERY_2026-04-01.md`
- implementation review/report/continuity docs after validation

## Boundary check
- Seed-capability request shaping and fetch transport remain in `viewer_net` (transport/probe ownership).
- Host-lane diagnostics emission remains in `viewer_app` orchestration/logging.
- No crate-boundary or architecture changes.

## Step sequence
1. Expand `DEFAULT_SEED_CAPABILITY_REQUEST` to a Firestorm-aligned list (bounded to known capability names from `llviewerregion.cpp`).
2. Add helper(s) in `viewer_app` to summarize discovered non-baseline capabilities by host-family (excluding baseline object-ingress probes).
3. Emit new per-host capability-name logs at startup seed fetch success and region-seed follow-up success.
4. Update/extend unit tests for seed request body expectations and new capability-host summary helper.
5. Run validation ladder (`fmt`, `check`, targeted tests).

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`

## Risks and open questions
- Broader seed requests may increase startup payload size/latency slightly.
- Some capabilities may be absent on specific simulator paths; logging should handle sparse sets cleanly.
- This slice improves discovery/observability; it may still confirm that no alternate object lane is available on a given host.

## Deferred-too-early candidates captured
- Automatic invocation/probing of newly discovered capability names is deferred.
- Capability-specific semantics for non-baseline lanes remain deferred pending evidence.
- No `DEFERRED_FEATURES.md` update required for this bounded observability slice.

## Learnings pre-check
- L58: avoid repeated LLUDP startup guessing once gate fails; pivot to capability-readiness investigation.
- L59: treat capability outcomes as first-class readiness evidence.
- L60: preserve EventQueue gating context when interpreting capability outcomes.
- L61: transport-ready interpretation can differ from semantic decode-ready.

## Completion criteria
- Seed request list is materially broadened and validated in tests.
- Logs include explicit per-host summaries of discovered non-baseline capability names.
- Validation commands pass and continuity artifacts are updated with exact outcomes.
