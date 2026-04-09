# Review: Implementation Live Mesh Blocker Isolation (2026-04-03)

## Verdict
- Approved with one remaining environment blocker for live verification.

## Architecture / Boundary Fit
- `viewer_grid` now owns ordered mesh capability candidate shaping, including cap metadata.
- `viewer_net` owns decode counters and fetch-attempt capture.
- `viewer_app` only formats and emits relay evidence.

## Correctness Concerns
- Ordered mesh candidate fallback is now covered in unit tests, including falling through after a `403`.
- App-side diagnostics now expose decoded mesh-ID evidence directly, which should reduce ambiguity in future live captures.
- No correctness issue was found in the additive relay changes during touched-crate validation.

## Modularity / Maintainability
- `MeshCapabilityRequestCandidate` keeps cap metadata explicit and avoids ad-hoc string parsing in the app worker.
- `AssetFetchAttempt` provides a bounded transport evidence shape reusable for mesh diagnostics.

## Validation Adequacy
- `cargo fmt --all` PASS
- `cargo check -p viewer_net -p viewer_grid -p viewer_app` PASS
- `cargo test -p viewer_grid` PASS
- `cargo test -p viewer_net` PASS
- `cargo test -p viewer_app` PASS
- bounded offline runtime smoke with screenshot capture PASS
- bounded live capture NOT RUN in this shell because `VIEWER_LOGIN_ENDPOINT`, `VIEWER_LOGIN_USERNAME`, and `VIEWER_LOGIN_PASSWORD` were missing

## Risks / Open Questions
- Fresh live diagnosis is still pending because the shell lacked login env.
- Until that run is completed, the primary runtime blocker remains narrowed to:
  - missing decoded live mesh IDs, or
  - external denial across all mesh caps

## Learnings Delta Verdict
- none; this slice improved observability but did not establish a new durable invariant yet.

## Approval Status
- Implementation accepted.
