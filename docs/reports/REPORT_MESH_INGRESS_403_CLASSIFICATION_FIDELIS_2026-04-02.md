# Report: Mesh Ingress and 403 Classification Hardening (Fidelis) 2026-04-02

## Summary of Implemented Work
- Added LLUDP `ObjectExtraParams` low-message (`99`) classification and decode path in `viewer_net`.
- Wired decoded mesh IDs from `ObjectExtraParams` entries into object-feed upsert/export.
- Added mesh 403 bucket tagging in `viewer_app` relay output (`bucket=...`) while preserving existing failure-reason classification.
- Added targeted tests for:
  - low-message `ObjectExtraParams` mesh decode
  - object-feed promotion from observed `ObjectExtraParams`
  - 403 bucket classification behavior in `viewer_app`.

## Files Changed
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_app/src/main.rs`
- `docs/reviews/REVIEW_PLAN_MESH_INGRESS_403_CLASSIFICATION_FIDELIS_2026-04-02.md`
- `docs/reviews/REVIEW_IMPL_MESH_INGRESS_403_CLASSIFICATION_FIDELIS_2026-04-02.md`
- `docs/reports/REPORT_MESH_INGRESS_403_CLASSIFICATION_FIDELIS_2026-04-02.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`

## Validation Run
- `cargo fmt --all` -> PASS
- `cargo check -p viewer_net -p viewer_app` -> PASS
- `cargo test -p viewer_net` -> PASS
- `cargo test -p viewer_app` -> PASS
- bounded live run (Fidelis, no fixture mesh):
  - log: `artifacts/logs/live_mesh_fidelis_object_extra_params_2026-04-02_200245.log`
  - network: `artifacts/logs/network_debug_mesh_fidelis_object_extra_params_2026-04-02_200245.jsonl`
  - result: LLUDP object gate reaches `PASS`; lane probes still show `ViewerAsset mesh_id` `403`.
- bounded live run (Fidelis, fixture mesh `947d4505-eb76-2ef5-c049-e7882881d689`):
  - log: `artifacts/logs/live_mesh_fidelis_fixture_2026-04-02_200524.log`
  - network: `artifacts/logs/network_debug_mesh_fidelis_fixture_2026-04-02_200524.jsonl`
  - result: `mesh_fetch: queued` observed; failure now includes `bucket=AccessDenied`.

## Result Status
- Implemented and validated in code/tests.
- Live diagnostics improved: mesh 403 root-cause bucket is now explicit.
- Full live mesh readiness remains blocked by server-side/access-path denial (`403 AccessDenied`) on tested mesh IDs.

## Risks / Follow-up Items
- Need valid in-region mesh IDs that are authorized on this session/cap path to reach `mesh_fetch: ready`.
- Continue capturing for explicit inbound `ObjectExtraParams` evidence in live logs; route/time may be sparse.

## Learnings Delta
none - no new durable repository learning identified; this was a bounded parity/diagnostics increment.

## Continuity Updates Performed
- Updated `docs/CURRENT_STATE.md` with this slice and Fidelis live evidence.
- Updated `docs/HANDOFF.md` with exact blocker and next step.
