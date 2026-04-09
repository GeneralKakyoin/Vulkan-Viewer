# Review: Implementation Object Ingress Two-Lane Transport Probe (2026-04-01)

## Verdict
approved

## Architecture and boundary fit
- Transport probe primitive is implemented in `viewer_net` (`probe_capability_transport_once`).
- Lane target selection and diagnostic orchestration are implemented in `viewer_app`.
- No crate-boundary drift observed.

## Correctness concerns
- Lane probes are bounded one-shot GET requests and report transport metadata (`status`, `content_type`, `body_bytes`) without semantic overreach.
- Baseline object-ingress capabilities are excluded from lane target selection.
- Selection is deterministic and capped to two host families.

## Modularity and maintainability concerns
- Lane-target selection is isolated in `select_non_baseline_lane_probe_targets`.
- Probe execution and logging are isolated in `run_non_baseline_lane_transport_probes_once`.
- `viewer_net` probe helper reuses existing capability-fetch transport path.

## Validation adequacy
- `cargo fmt --all`: pass
- `cargo check -p viewer_net -p viewer_app`: pass
- `cargo test -p viewer_net`: pass
- `cargo test -p viewer_app`: pass
- bounded live run: pass with new lane probe evidence captured

## Risks and open questions
- Non-2xx outcomes can reflect auth/policy requirements rather than lane absence.
- Lane transport visibility improved, but object-ingress semantic relevance still requires targeted follow-up probes.

## Learnings delta verdict
add

## Required revisions or approval status
Approved; proceed to continuity updates and follow-up planning from concrete lane probe outcomes.
