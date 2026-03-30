# Review: Implementation - Live Texture Capability URL Parity 2026-03-29

## Verdict
approved

## Architecture and boundary fit
- `viewer_grid` now owns ordered texture capability URL generation instead of leaving URL-shape drift split across call sites.
- `viewer_net` owns the shared HTTP retry/fetch helper and keeps transport execution separate from capability meaning.
- `viewer_app` only switched its worker request path to the shared helper and did not absorb new asset policy.

## Correctness concerns
- No blocking correctness issues found in the implemented slice.
- Connected live proof is still pending, so this is a bounded parity repair rather than a full user-visible confirmation.

## Modularity and maintainability concerns
- The change reduces duplication by removing the previous split between scene-texture and profile-image URL/fetch behavior.
- The ordered candidate list is bounded and test-covered, so it does not introduce uncontrolled fallback sprawl.

## Validation adequacy
- Adequate for this slice:
  - `cargo fmt --all`
  - `cargo check -p viewer_grid -p viewer_net -p viewer_app`
  - `cargo test -p viewer_grid -p viewer_net -p viewer_app`

## Risks and open questions
- The provided `fire.pcapng` only gave DNS/environment clues, not actual texture transfer payloads.
- Live texture proof may still be blocked upstream if the viewer still fails to ingest object/texture IDs from the simulator.

## Learnings delta verdict
add - The texture capability path drifted because scene textures and profile images maintained separate URL-shaping logic; that is a durable planning constraint.

## Required revisions or approval status
- No revisions required.
