# Review: Plan Object Ingress Capability Probe EventQueue Gating (2026-04-01)

## Verdict
approved

## Architecture and boundary fit
- Plan keeps capability ordering logic in `viewer_app` orchestration.
- No transport/protocol ownership drift into `viewer_net`.
- No crate-boundary changes.

## Correctness concerns
- Must ensure deferred probes execute at most once each after gate open.
- Must log explicit deferred state so non-execution in bounded runs is diagnosable.

## Modularity and maintainability concerns
- Keep probe execution path centralized (avoid duplicate per-cap probe branches across startup and loop code).
- Prefer small helper(s) for readability over large inline blocks.

## Validation adequacy
- Validation ladder is sufficient: fmt/check/tests + bounded live run with dedicated artifact.

## Risks and open questions
- If EventQueue never reaches `ok`, probes remain deferred and evidence may be inconclusive.
- Ordering gate may not change `404/405`, but that still yields decision-grade evidence.

## Learnings delta verdict
- none (plan-stage review; no new durable learning yet)

## Required revisions or approval status
- Approved as written.
