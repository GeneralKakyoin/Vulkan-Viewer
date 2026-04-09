# Review: Impl Object Ingress Lane Request Shaping (2026-04-01)

## Verdict
APPROVED

## Architecture and boundary fit
- Implementation keeps capability request-shape modeling in `viewer_grid`.
- Probe transport/method/header/body execution remains in `viewer_net`.
- Startup matrix gating and protocol relays remain in `viewer_app`.
- No boundary violations found.

## Correctness concerns
- Simhost shaped probes are correctly deferred until `EventQueueGet:ok`.
- `ViewerAsset` probes are now shape-specific by query key and include URL variant evidence.
- Existing untrusted readiness behavior remains intact.

## Modularity and maintainability concerns
- New probe executor is reusable and avoids duplicative cap-specific transport code.
- Probe metadata (`decode`, `response_class`, `body_preview_hash`) is bounded and useful.

## Validation adequacy
- `cargo fmt --all`: PASSED
- `cargo check -p viewer_grid -p viewer_net -p viewer_app`: PASSED
- `cargo test -p viewer_grid -p viewer_net -p viewer_app`: PASSED
- bounded live run (`cargo run -p viewer_app`, timeout-bounded): evidence captured in
  `artifacts/logs/network_debug_lane_request_shaping_2026-04-01.jsonl`

## Risks and open questions
- `UntrustedSimulatorMessage` shaped lane still reports GET `405` while readiness path recovers via fallback semantics.
- LLUDP object ingress remains blocked and unchanged by this diagnostic slice.

## Learnings delta verdict
- `add` (L64)

## Required revisions or approval status
- None.
