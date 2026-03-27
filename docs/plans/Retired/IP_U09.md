# Implementation Plan - U09: Workflow Depth and Product Usability Expansion

This plan implements keyboard-driven workflow shortcuts, enforces a 10nd-second profile refresh cooldown, and improves diagnostics grouping for better operator usability.

## Proposed Changes

### [viewer_core]
- **AvatarProfileState**: Add `last_refresh_unix_ms: Option<u64>` to track cooldowns.
- **AvatarProfileTab**: No changes needed, already supports granular refreshes.

#### [MODIFY] [lib.rs](file:///c:/Users/matti/Desktop/Antigravity viewer/Vulkan-Viewer/crates/viewer_core/src/lib.rs)
- Add `last_refresh_unix_ms` to `AvatarProfileState`.

---

### [viewer_ui]
- **UiSystem**:
    - Add `show_diagnostics: bool` and `show_social: bool` visibility flags.
    - Default `show_diagnostics` to `true`, `show_social` to `false` (or based on current behavior).
- **Diagnostics Rendering**:
    - Group "Live Visual" and "Continuity" into a `CollapsingHeader` titled "Presence & Continuity".
    - Group "Performance" and "Scene Metrics" into "Performance & Metrics".
- **Shortcuts Hooking**:
    - The UI already returns `UiActions`. I will add shortcut toggle requests to `UiActions` or handle them in `viewer_app`.
    - *Decision*: Handle raw key state in `viewer_app` for F1-F3 to toggle visibility flags in `UiSystem`.
- **Profile Refresh**:
    - Disable the "Refresh" button in the Avatar Profile window if the 10s cooldown is active.
    - Display a countdown timer next to the button when on cooldown.

#### [MODIFY] [lib.rs](file:///c:/Users/matti/Desktop/Antigravity%20viewer/Vulkan-Viewer/crates/viewer_ui/src/lib.rs)
- Update `UiSystem` struct.
- Update `render` method to use visibility flags and implement grouping.
- Implement cooldown logic in profile window.

---

### [viewer_app]
- **Input Handling**:
    - Update `AppState::handle_input_event` (or `InputState`) to detect `F1`, `F2`, and `F3`.
    - `F1`: Toggle `ui.show_diagnostics`.
    - `F2`: Ensure Diagnostics is open and focus/expand "Presence & Continuity".
    - `F3`: Toggle `ui.show_social`.
- **Profile Refresh Routing**:
    - In `redraw`, when a refresh is triggered, update the `last_refresh_unix_ms` timestamp in the active `AvatarProfileState`.

#### [MODIFY] [main.rs](file:///c:/Users/matti/Desktop/Antigravity viewer/Vulkan-Viewer/crates/viewer_app/src/main.rs)
- Update shortcut handling logic.
- Update profile refresh processing logic.

## Verification Plan

### Automated Tests
- Run `cargo test` to ensure no regressions in core logic.
- Add a unit test in `viewer_core` for `AvatarProfileState` cooldown calculation if applicable.

### Manual Verification
1. **Shortcuts**:
    - Press `F1`: Confirm Diagnostics window toggles.
    - Press `F3`: Confirm Social window toggles.
    - Press `F2`: Confirm Diagnostics window opens (if closed) and is focused.
2. **Profile Refresh**:
    - Open an avatar profile.
    - Click "Refresh".
    - Confirm the button is disabled for 10 seconds.
    - Confirm the "cooldown" label appears.
3. **Diagnostics Grouping**:
    - Open Diagnostics.
    - Confirm "Presence & Continuity" group exists and contains relevant fields.
    - Confirm "Performance & Metrics" group exists.
4. **Failure States**:
    - Intentionally fail a profile load (if possible via env) and confirm the failure message is clear in the UI.

## Risks & Open Questions
- **F2 Focus**: Egui focus management can be tricky for windows. I will use `egui::Window::focus()` or similar.
- **Clock Drift**: Using `now_unix_ms()` is sufficient for a 10s cooldown.
