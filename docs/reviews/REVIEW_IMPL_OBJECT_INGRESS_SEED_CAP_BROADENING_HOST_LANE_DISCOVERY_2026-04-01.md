# Review: Implementation Object Ingress Seed-Cap Broadening And Host-Lane Discovery (2026-04-01)

## Verdict
approved

## Architecture and boundary fit
- Seed-capability request shaping remains in `viewer_net`.
- Host-lane discovery and relay summarization remain in `viewer_app`.
- No crate-boundary or architecture drift detected.

## Correctness concerns
- Baseline-cap exclusion for host-lane summaries is explicit and bounded (`EventQueueGet`, `InterestList`, `RegionObjects`, `UntrustedSimulatorMessage`).
- Broad seed request list is deterministic and now requests capability names needed for alternate-lane discovery (including asset-cdn lanes such as `GetMesh`, `GetMesh2`, `GetTexture`, `ViewerAsset`).
- Follow-up region-seed logging reuses the same summary helper, avoiding divergent reporting paths.

## Modularity and maintainability concerns
- Host-lane summarization is isolated in a helper (`summarize_non_baseline_caps_by_host`), keeping startup flow readable.
- Seed request expansion is centralized in `DEFAULT_SEED_CAPABILITY_REQUEST`.
- Tests cover both expanded request body membership and baseline-cap exclusion behavior.

## Validation adequacy
- `cargo fmt --all`: pass
- `cargo check -p viewer_net -p viewer_app`: pass
- `cargo test -p viewer_net`: pass
- `cargo test -p viewer_app`: pass
- bounded live run: pass for evidence capture and new host-lane relay lines

## Risks and open questions
- This slice improves lane visibility but does not itself prove object ingress will use those alternate lanes.
- LLUDP object ingress remains unresolved and still requires a separate unblock branch.

## Learnings delta verdict
add

## Required revisions or approval status
Approved; continuity updates and next-branch framing should now pivot from lane discovery to targeted lane semantics/probing.
