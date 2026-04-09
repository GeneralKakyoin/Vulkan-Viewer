# Plan: Wave 9 Viewer Grid Helper Extraction

## Objective
Apply the repository's cross-crate modularization pattern (Wave 9) to `viewer_grid`. Reduce `crates/viewer_grid/src/lib.rs` size by extracting cohesive, bounded helper clusters into dedicated crate-local utility modules.

## User Review Required
This plan reduces `crates/viewer_grid/src/lib.rs` to a lightweight wiring/re-export module, completing the crate-local extraction pattern for `viewer_grid`. Please approve whether `grid_login.rs` and `asset_capability_policy.rs` are the desired file names.

## Proposed Changes

### `viewer_grid`
The monolithic `lib.rs` in `viewer_grid` currently contains grid login structs, adapters, and asset capability policies. These will be partitioned.

#### [MODIFY] `crates/viewer_grid/src/lib.rs`
- Remove `GridLoginAdapter`, `SecondLifeAdapter` and associated login structures, moving them to `grid_login.rs`.
- Remove `AssetCapabilityPolicy` and associated probe shape structures, moving them to `asset_capability_policy.rs`.
- Add `pub mod grid_login;` and `pub mod asset_capability_policy;`.
- Re-export extracted structs (`pub use grid_login::*;` and `pub use asset_capability_policy::*;`).

#### [NEW] `crates/viewer_grid/src/grid_login.rs`
- Relocate grid login structs (`StartLocation`, `LoginIntent`, `GridLoginRequest`, `GridLoginResponse`, `SessionBootstrap`, etc.).
- Relocate `GridLoginAdapter` trait and `SecondLifeAdapter` struct.
- Relocate `SECOND_LIFE_CLIENT_CHANNEL` constants.
- Relocate `secondlife_request_*` tests here.

#### [NEW] `crates/viewer_grid/src/asset_capability_policy.rs`
- Relocate `AssetCapabilityPolicy`, `CapabilityProbeMethod`, `ViewerAssetQueryKey`, `CapabilityProbeRequestShape`, and `MeshCapabilityRequestCandidate`.
- Relocate helper functions `extend_unique` and `extend_mesh_request_candidates`.
- Relocate `asset_policy_*`, `viewer_asset_probe_*`, and `mesh_url_candidates_*` tests here.

## Out of Scope
- Altering login or capability resolution logic.
- Modifying behavior of legacy_login or continuity.

## Open Questions
None. The process strictly adheres to previous `viewer_*` helper extraction directives.

## Verification Plan
### Automated Tests
- `cargo fmt --all`
- `cargo check --workspace`
- `cargo test -p viewer_grid` (verifies the tests execute passingly from within their new modules).
