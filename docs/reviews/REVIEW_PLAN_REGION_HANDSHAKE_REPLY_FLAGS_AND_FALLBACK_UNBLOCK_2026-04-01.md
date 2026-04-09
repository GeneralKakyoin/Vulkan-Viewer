# Review: Plan RegionHandshakeReply Viewer-Flags Correction and Fallback Unblock (2026-04-01)

## Verdict
Approved.

## Architecture and boundary fit
- Correct ownership: protocol mechanics in `viewer_net`, orchestration trigger in `viewer_app`.
- No crate-boundary drift.

## Correctness concerns
- Plan correctly identifies that `RegionHandshakeReply` flags are viewer capability flags, not simulator region flags.
- Fallback path is bounded (single-send, stage-gated), minimizing regression risk.

## Modularity and maintainability concerns
- The fix is narrow and testable.
- Existing handshake path remains intact; fallback is additive and explicit.

## Validation adequacy
- Adequate:
  - fmt/check/tests on touched crates
  - bounded runtime capture with explicit artifact path.

## Risks and open questions
- Fallback reply may be ignored if server enforces prior inbound handshake semantics.
- Additional viewer flag bits might still be required.

## Learnings delta verdict
- none (pending implementation/run evidence)

## Required revisions or approval status
- No revisions required.
