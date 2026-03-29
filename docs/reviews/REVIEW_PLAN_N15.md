# Review: PLAN_N15 Continuity Probe Command Wiring and Recovery Signaling

## Verdict
Approved with implementation guardrails.

## Architecture and boundary fit
- `viewer_ui` emits intent only.
- `viewer_app` owns cooldown/single-flight orchestration and worker command dispatch.
- `viewer_net` owns live probe transport execution.
- `viewer_grid` remains classification/policy owner via existing probe-outcome mapping helpers.

## Correctness concerns
- Retry flow must reject when probe execution is unavailable in current startup mode.
- Retry flow must avoid overlapping probe requests (single-flight) in addition to cooldown gating.
- Probe completion must update continuity probe diagnostics deterministically in snapshot-facing state.

## Modularity and maintainability concerns
- Keep retry gating logic centralized in app recovery dispatch so UI remains a thin emitter.
- Avoid introducing renderer/UI coupling into probe transport result mapping.

## Validation adequacy
- Planned validation ladder is adequate (`fmt`, `check`, targeted tests, workspace tests, runtime screenshot smoke).
- Connected-live probe verification remains environment-dependent and should be explicitly marked if unavailable.

## Risks and open questions
- Connected proof for live probe path depends on valid credentials/environment.
- Probe classification quality still depends on transport error classification granularity from `viewer_net`.

## Learnings delta verdict
none - No durable learning identified at plan review stage; constraints are already covered by existing entries (`L05`, `L06`, `L07`, `L10`, `L22`).

## Required revisions or approval status
Approved for bounded implementation exactly as scoped.
