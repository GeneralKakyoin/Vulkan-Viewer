# [PLAN] Resolve Workspace Clippy Warnings

This plan outlines the systematic resolution of clippy warnings identified across the `Vulkan-Viewer` workspace to improve code quality, maintainability, and idiomaticity.

## Scope
- Address all warnings reported by `cargo clippy --workspace --all-targets`.
- Focus on low-risk, high-value fixes (collapsible if, derivable impls, needless range loops, etc.).
- Refactor the `UiSystem::render` function to resolve the `too_many_arguments` warning by introduction of a parameter struct.

## Current Known State
- Workspace is stable (R08/U09/A10 baseline).
- Standard clippy check reports approximately 30+ warnings across `viewer_app`, `viewer_asset`, `viewer_core`, `viewer_grid`, `viewer_net`, `viewer_render`, and `viewer_ui`.
- `too_many_arguments` (26) in `viewer_ui` is the most significant architectural warning.

## Files and Components Touched
- `crates/viewer_app/src/main.rs`
- `crates/viewer_asset/src/texture_fixture.rs`
- `crates/viewer_core/src/geometry/llvolume.rs`
- `crates/viewer_core/src/lib.rs`
- `crates/viewer_core/src/material/animation.rs`
- `crates/viewer_core/src/spatial.rs`
- `crates/viewer_grid/src/legacy_login.rs`
- `crates/viewer_grid/src/lib.rs`
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_render/src/lib.rs`
- `crates/viewer_render/src/texture_provider.rs`
- `crates/viewer_ui/src/lib.rs`

## Boundary Check
- All changes are local to crates and do not change cross-crate interfaces EXCEPT for `viewer_ui::UiSystem::render` which is called by `viewer_app`.
- The `viewer_ui` change will require a coordinated update in `viewer_app`.
- `viewer_net` and `viewer_grid` boundaries remain intact.

## Step Sequence

### 1. Preparation
- [x] Inspect exact warning locations again before editing.

### 2. Implementation - Independent Crates
- [x] **viewer_core**: Fix `derivable_impls`, `collapsible_if`, `needless_range_loop`, `needless_return`, and digit grouping.
- [x] **viewer_asset**: Implement `Default` for `FixtureTextureCache`, collapse `if`.
- [x] **viewer_grid**: Box large enum variant in `GridLoginResult`, collapse `if`.
- [x] **viewer_net**: Derive `Default`, use `unwrap_or_default`, fix case-insensitive comparison, remove redundant struct update.
- [x] **viewer_render**: Replace single-binding match with `let`, fix `field_reassign_with_default`, add `Default` for `DefaultTextureProvider`.

### 3. Implementation - UI and App (Coordinated)
- [x] **viewer_ui**: Define `RenderInput` struct to group the 25+ arguments.
- [x] **viewer_ui**: Update `UiSystem::render` signature to use `RenderInput`.
- [x] **viewer_ui**: Fix `manual_div_ceil` and `collapsible_if`.
- [x] **viewer_app**: Update call site in `main.rs` to use `RenderInput`.
- [x] **viewer_app**: Fix `collapsible_if`, `is_multiple_of`, and `field_reassign_with_default` in `main.rs`.

### 4. Final Polish
- [x] Run `cargo fmt` across the workspace.

## Validation Plan

### Automated Tests
- [x] Run `cargo clippy --workspace --all-targets`: Must return no warnings.
- [x] Run `cargo test --workspace`: All existing unit tests (e.g., in `viewer_asset`, `viewer_grid`, `viewer_net`) must pass.

### Manual Verification
- [x] Run `cargo run -p viewer_app`: Smoke attempt launched `viewer_app.exe`; bounded command timeout ended run window before interactive verification.

## Risks and Open Questions
- **Risk**: Refactoring `UiSystem::render` might introduce subtle bugs if arguments are misaligned during the transition to a struct.
- **Mitigation**: Careful field-by-field mapping and immediate verification with `cargo check`.

## Learnings Pre-check
- Follow `AGENTS.md` and `docs/LEARNINGS.md` regarding `unwrap()` (avoid in critical paths) and crate boundaries.
- Ensure `viewer_render` internals do not leak.

## Exact Completion Criteria
- `cargo clippy` is clean for the entire workspace.
- `cargo test` passes for the entire workspace.
- The viewer app starts and displays the UI correctly.
- Continuity documents (`CURRENT_STATE.md`, `HANDOFF.md`, etc.) are updated.
