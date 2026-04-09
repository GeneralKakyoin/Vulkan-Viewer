# Review: Implementation — Mesh Ingress and 403 Classification Hardening (Fidelis) 2026-04-02

## Verdict
Approved.

## Architecture and Boundary Fit
- `viewer_net` change is decode/classification wiring only (`ObjectExtraParams` low-message ingest).
- `viewer_app` change is orchestration/diagnostics only (403 bucket detail text).
- No crate-boundary drift.

## Correctness Concerns
- Decoder now parses `AgentData` + `ObjectData[]` for low message 99 and only promotes mesh IDs from sculpt/mesh extra-param entries.
- 403 bucket tagging is additive to existing retry/failure classification (`MissingCapability` unchanged for non-retryable 4xx).

## Modularity and Maintainability
- Mesh extra-param entry decode is centralized in `decode_mesh_id_from_extra_param_entry(...)`.
- Runtime bucket helper is pure and covered by unit tests.

## Validation Adequacy
- `cargo fmt --all` PASS
- `cargo check -p viewer_net -p viewer_app` PASS
- `cargo test -p viewer_net` PASS
- `cargo test -p viewer_app` PASS
- bounded live Fidelis runs captured with and without fixture mesh IDs.

## Risks and Open Questions
- Live `ViewerAsset mesh_id` still returns `403 AccessDenied` for tested IDs on this route.
- No in-window live evidence yet that simulator is sending inbound `ObjectExtraParams` to the viewer on this region.

## Learnings Delta Verdict
none - This slice improved observability and decode coverage but did not establish a new durable repo-wide rule.

## Required Revisions or Approval Status
Approved as implemented.
