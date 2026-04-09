# Review: Implementation Endpoint-Aware LLUDP Handshake Re-prime + Reply Targeting (2026-04-03)

## Verdict
- Approved.

## Architecture / Boundary Fit
- Changes remain within `viewer_net` transport/runtime handshake behavior and `viewer_app` strict-mode loop orchestration.
- No crate-boundary violations or architecture reshaping.

## Correctness Findings
- `RegionHandshakeReply` now targets a prioritized endpoint chain (`last_region_handshake_sender` -> `last_bootstrap_sender` -> circuit target), reducing wrong-endpoint reply risk.
- Inbound sender endpoint tracking is now wired in all active first-simulator receive loops where sender address is available.
- Strict-mode reprime now sends a bounded handshake bundle (`UseCircuitCode` + `CompleteAgentMovement`) before startup-interest reprime.
- New targeted tests cover:
  - reply target preference behavior
  - reprime bundle endpoint fan-out + message types

## Modularity / Maintainability
- Endpoint selection and sender tracking are isolated into helper methods in `Connection`.
- `viewer_app` loop changes remain small and call one transport-level API.

## Validation Adequacy
- `cargo fmt --all` passed.
- `cargo check -p viewer_net -p viewer_app` passed.
- Targeted `viewer_net` tests for reply path and reprime bundle passed.
- Full `viewer_app` tests passed.

## Risks / Open Questions
- Route/server variance may still prevent inbound `RegionHandshake` despite improved targeting and bounded reprime.
- This slice improves handshake progression odds but does not by itself guarantee route acceptance.

## Learnings Delta Verdict
- none.
Reason: no new cross-route durable invariant was proven yet; this slice adds mitigations and tests but no fresh universal protocol rule.

## Required Revisions
- None.
