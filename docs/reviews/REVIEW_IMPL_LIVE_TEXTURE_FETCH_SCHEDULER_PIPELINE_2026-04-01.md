# Review: Implementation Live Texture Fetch Scheduler Pipeline (2026-04-01)

## Verdict
Approved.

## Architecture And Boundary Fit
- `viewer_app` now owns bounded request scheduling/orchestration for live texture fetch commands.
- `viewer_grid` request-shape ownership is preserved via `AssetCapabilityPolicy::texture_url_candidates(...)`.
- `viewer_net` transport ownership is preserved via `fetch_texture_asset_bytes(...)`.
- No crate-boundary violation observed.

## Correctness Concerns
- Addressed:
  - Removed ad-hoc per-command texture fetch spawn path in favor of queued/in-flight pipeline.
  - Added bounded retry policy limited to retryable reasons and capped attempts.
  - Updated texture ingest decode from PNG-only to shared decode supporting J2C and PNG.
- Residual risk:
  - Scheduler constants (`max inflight`, attempt/backoff caps) are heuristic and may need live tuning.

## Modularity And Maintainability Concerns
- Scheduler logic is explicit and local in `viewer_app` worker loop with typed queue/in-flight/completion structs.
- Retry/backoff decision is factored into helper functions with unit tests.

## Validation Adequacy
- Adequate for this slice:
  - `cargo fmt --all`
  - `cargo check -p viewer_app -p viewer_asset -p viewer_net -p viewer_grid`
  - `cargo test -p viewer_app -p viewer_asset`
  - bounded runtime screenshot smoke with manual image review

## Risks And Open Questions
- Live-run evidence for throughput/prioritization under heavy request load is still needed.
- Full Firestorm fetch subsystem parity (thread model / UDP fallback / range orchestration) remains out of scope.

## Learnings Delta Verdict
- `add`: new durable learning added (`L65`) for live texture decode path contract.

## Required Revisions Or Approval Status
- Approval status: approved as implemented.
