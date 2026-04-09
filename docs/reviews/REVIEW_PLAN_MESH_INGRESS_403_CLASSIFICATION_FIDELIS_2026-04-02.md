# Review: PLAN_MESH_INGRESS_403_CLASSIFICATION_FIDELIS_2026-04-02

## Verdict
Approved.

## Architecture and Boundary Fit
- Scope stays within `viewer_net` (protocol decode) and `viewer_app` (runtime classification relay).
- No ownership drift across `viewer_net`/`viewer_grid` boundary.

## Correctness Concerns
- `ObjectExtraParams` decode must honor packet structure (`ParamSize` + `ParamData` variable field) and avoid offset desync.
- 403 bucketing should preserve current retry classification semantics (`MissingCapability` for deterministic 4xx).

## Modularity and Maintainability
- Reuse existing mesh extra-param decoder (`decode_mesh_asset_id_from_extra_params`) to avoid duplicate protocol parsing.
- Keep 403 bucket helper small and pure for testability.

## Validation Adequacy
- Planned checks are sufficient: fmt, crate checks, targeted tests, bounded live run.

## Risks and Open Questions
- Live runs may still show 403 due simulator/capability policy even after decode improvements.
- `ObjectExtraParams` may be sparse on route/time window.

## Learnings Delta Verdict
none - This review does not introduce a durable lesson beyond existing L73/L74 guidance.

## Required Revisions or Approval Status
Approved as written.
