# Review: Plan Object Ingress UntrustedSimulatorMessage POST Probe Fallback (2026-04-01)

## Verdict
approved

## Architecture and boundary fit
- Probe-method fix remains in `viewer_net` transport ownership.
- `viewer_app` orchestration stays unchanged.
- No crate-boundary violations.

## Correctness concerns
- Fallback should trigger only on GET `405 Method Not Allowed`, not all failures.
- Preserve explicit error reporting for non-success POST outcomes.

## Modularity and maintainability concerns
- Prefer helper reuse for capability HTTP fetch paths (GET/POST) to avoid code duplication.

## Validation adequacy
- fmt/check/tests + bounded live run with dedicated artifact are adequate.

## Risks and open questions
- Endpoint may require specific message/body schema beyond method alignment.
- Probe success may still not affect LLUDP gate.

## Learnings delta verdict
- none (plan stage)

## Required revisions or approval status
- Approved as written.
