# Review: Plan Firestorm Mesh Parity Hardening (2026-04-02)

## Verdict
- Approved.

## Architecture / Boundary Fit
- Keeps protocol decode/fetch in `viewer_net`, URL policy in `viewer_grid`, and command orchestration in `viewer_app`.
- Does not use Firestorm as architecture template; uses it as behavior/protocol reference.

## Correctness Concerns
- Compressed parser must use bounded/defensive skipping for optional fields and fail-safe to `None` on malformed variants.
- Mesh URL helper must avoid broad fallback paths that diverge from Firestorm query shape for mesh requests.

## Modularity / Maintainability
- Dedicated mesh URL helper in `viewer_grid` improves intent clarity and avoids ad-hoc URL generation in `viewer_app`.
- Reusing existing `decode_mesh_asset_id_from_extra_params` avoids duplicate parsers.

## Validation Adequacy
- Includes fmt/check/tests on touched crates plus bounded live run; adequate for this slice.

## Risks / Open Questions
- SL policy 403 may persist even with correct request shaping.
- Full Firestorm-equivalent LOD byte-range loading remains out-of-scope.

## Learnings Delta Verdict
- none (pre-check only); evaluate durable addition after live run.

## Required Revisions / Approval Status
- No revisions required.
