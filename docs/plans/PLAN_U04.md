# Plan: U04 Core Usability and Workflow Shell Alignment

## Summary
Make the existing debug/social/profile/diagnostic UI feel like one coherent “daily driver” shell by consolidating status presentation, tightening chat/IM/profile interaction feedback, and standardizing cache freshness indicators—without changing crate boundaries or adding new protocol semantics.

## Objective
- Provide a clear, explicit, and consistent set of user-facing session states (`disabled`, `starting`, `connected`, `reconnecting`, `failed`) and present them prominently.
- Reduce “where do I look?” UI friction by regrouping existing windows/panels into clearer workflow groupings (Session, Social, Diagnostics).
- Make chat/IM/profile workflows self-explanatory via consistent status chips, disabled controls when unavailable, and visible error reasons.
- Standardize profile cache indicators as `fresh` / `stale` / `unknown` (per-tab), based on the existing TTL policy in `viewer_app`.

## Why now
- Roadmap sequencing puts `U04` after `R01` + `A02` + `N03`: once the render/asset/feed triad exists, operator workflow becomes the bottleneck for validating richer world behavior safely.
- The repo already has substantial UI surfaces (chat/IM/profile, runtime relay, debug/perf), but status signaling is fragmented (string-only startup status, multiple “failed” concepts, raw timestamps).

## In scope
- **Session status UX**
  - Define a shared, UI-friendly session status type in `viewer_core` (or equivalent shared contract) representing: `disabled`, `starting`, `connected`, `reconnecting`, `failed` (with optional concise reason).
  - Update `viewer_app` to map its existing in-process live worker states (including reconnect loop) into the shared session status value.
  - Update `viewer_ui` to render session status as a consistent “chip” (label + color) and make it visible in the primary workflow surface.
- **Workflow panel grouping (UI information architecture)**
  - Consolidate current “Debug” + “Performance” + “Live Visual Snapshot” + “Runtime Relay” presentation into a clearer grouping:
    - Session (connection/world presence basics)
    - Social (chat + IM + nearby + profile entry points)
    - Diagnostics (relay/events, snapshot, perf)
  - Keep this bounded to rearranging existing content and light UI affordances (collapsing headers/tabs), not a broad visual redesign.
- **Chat + IM flow tightening**
  - Disable send actions when not connected (or when in reconnecting/failed state) and show the reason inline.
  - Standardize status messaging for:
    - connection state (`ChatConnectionState`)
    - send state (`ChatSendStatus`)
    - “draft unsent” indicators
  - Keep existing message rendering and thread ordering; only adjust workflow feedback and guardrails.
- **Profile workflow and cache freshness**
  - Introduce a shared cache freshness concept (`fresh` / `stale` / `unknown`) derived from:
    - per-tab `ProfileLoadState` (`status`, `last_updated_unix_ms`, `error`)
    - existing `VIEWER_APP_PROFILE_CACHE_TTL_SECS` policy used by `viewer_app`
  - Render per-tab freshness chips and replace raw “updated: <unix_ms>” presentation with:
    - freshness label
    - concise “age” readout (still deterministic; avoid locale/timezone formatting)
    - visible error reason on failed loads
  - Keep data ownership unchanged: `AvatarProfileState` remains the source of truth; UI computes indicators.
- **Diagnostics usability**
  - Add minimal filtering affordances for runtime relay viewing (level filter and/or category text filter).
  - Keep the relay as an observation surface only (no new protocol behavior).

## Out of scope
- In-app credential entry / full login UI workflow (beyond status display and offline guidance).
- New transport/protocol semantics or additional network features.
- Broad visual redesign, theming overhaul, docking system adoption, or persistent UI layout save/restore.
- Large new feature surfaces (inventory, map, build tools, etc.).

## Current known state
- `viewer_ui` renders multiple separate windows: “Debug”, “Performance”, “Chat + IM” (optional), “Avatar Profile” (when open), and “Runtime Relay”. (`crates/viewer_ui/src/lib.rs`)
- `viewer_app` maintains an in-process live startup state as `LiveStartupStatus` but only passes a formatted string into the UI (`live_startup_status: &str`). (`crates/viewer_app/src/main.rs`)
- Chat connection state is a typed enum in `viewer_core` (`ChatConnectionState`) and is already rendered as a colored chip in the UI. (`crates/viewer_core/src/lib.rs`, `crates/viewer_ui/src/lib.rs`)
- Profile load state exists per tab (`ProfileLoadState` with `last_updated_unix_ms`), and fetch TTL policy exists in `viewer_app` (`should_fetch_profile_tab(..., ttl_secs)`). (`crates/viewer_core/src/lib.rs`, `crates/viewer_app/src/main.rs`)
- Profile UI currently shows raw `last_updated_unix_ms` and status color, but no freshness concept and no consistent “stale” indicator.

## Files and components touched
- `crates/viewer_core/src/lib.rs`
  - Add shared UX-facing session status type (and helpers) and cache freshness helpers/types (as needed).
- `crates/viewer_app/src/main.rs`
  - Map in-process worker states into the shared session status type; pass it into `viewer_ui` (instead of a plain string).
  - Expose profile cache TTL to the UI (as a parameter or via shared helper calls) so freshness is computed consistently.
- `crates/viewer_ui/src/lib.rs`
  - Refactor UI layout into clearer Session/Social/Diagnostics grouping.
  - Render session status chip and standardized status messaging.
  - Add cache freshness chips and concise age/error presentation in profile UI.
  - Add minimal runtime relay filters.
- Tests (targeted, no golden UI tests)
  - `crates/viewer_ui/src/lib.rs` unit tests for helper functions (status chip labels/colors, freshness computation).
  - `crates/viewer_core/src/lib.rs` unit tests if new shared helpers/types are added there.

## Boundary check
- `viewer_ui` owns presentation and emits `UiActions` only; it does not own session logic or state transitions.
- `viewer_app` owns orchestration and maps internal worker state into shared, sanitized status types for the UI.
- `viewer_core` owns shared domain types and deterministic helper logic; it must not depend on app/UI crates.
- No changes that introduce `viewer_ui` dependency on `viewer_app` types.
- No renderer (`viewer_render`) ownership changes and no transport (`viewer_net`/`viewer_grid`) semantic expansion.

## Step sequence
1. Session status contract
   - Add `viewer_core` type representing UX session status + optional reason.
   - Add a single mapping function in `viewer_app` from `LiveStartupStatus` (and reconnect loop state) to the shared UX status.
   - Update `viewer_ui::UiSystem::render` signature to accept the typed status and render a chip consistently.
2. UI regrouping (bounded)
   - Consolidate status + presence basics into a “Session” surface (window or panel).
   - Consolidate snapshot/perf/relay into a “Diagnostics” surface (with collapsing sections).
   - Keep “Social” as the primary interaction surface; retain the ability to hide/show it if currently supported.
3. Chat/IM workflow guardrails
   - Gate send buttons and Enter-to-send by connection/session status.
   - Ensure failure reasons are visible but non-spammy (single compact error row).
4. Profile freshness standardization
   - Add a shared freshness computation helper (pure function) and UI chip rendering.
   - Replace raw unix-ms display with `(fresh|stale|unknown)` + concise “age” and error.
5. Diagnostics filters
   - Add minimal relay filters (level toggles and/or category substring filter) with deterministic defaults.
6. Closeout polish (bounded)
   - Ensure layout remains usable at small window sizes and does not overlap critically by default.
   - Ensure no new allocations/expensive formatting occur in hot loops beyond what UI already does.

## Validation plan
- Always run:
  - `cargo fmt`
  - `cargo check`
- Targeted tests (expected for this milestone):
  - `cargo test -p viewer_core` (if shared helpers/types are added)
  - `cargo test -p viewer_ui`
  - `cargo test -p viewer_app` (if UI wiring changes touch app logic/tests)
- Broader:
  - `cargo test` if cross-crate behavior changes are non-trivial.
- Runtime smoke:
  - `VIEWER_APP_LIVE_STARTUP=off`, `cargo run -p viewer_app` and verify:
    - session status shows `disabled` with clear guidance
    - chat/IM/profile surfaces remain usable in offline mode (with disabled send actions)
  - Optional live smoke (when available) to observe `starting` → `connected` and forced reconnect → `reconnecting`.

## Risks and open questions
Risks:
- UI re-grouping can accidentally hide critical diagnostics; keep sections discoverable via clear headers and defaults.
- Session status mapping risks misrepresenting “reconnecting” as “failed” (or vice versa); mapping must be explicit and tested.
- Profile freshness must remain deterministic and must match fetch TTL behavior; compute freshness via the same TTL used in app fetch decisions.

Open questions:
- Whether “reconnecting” should be shown as a distinct variant in the internal `LiveStartupStatus` state machine, or derived purely as a UX mapping from existing failure classes (plan assumption: derive UX mapping first; only add new internal variants if needed for clarity).
- Default window/panel positions that avoid overlap on common screen sizes; keep changes minimal and test by running once.

## Deferred-too-early candidates captured
- In-app login/credential UI (defer to later `U09+` once shell is stable and security/UX decisions are explicit).
- Persistent egui layout save/restore or docking system adoption (defer to later `U09+`; adds complexity and long-term UX surface area).
- Export/copy diagnostics bundles (log/snapshot/relay) from UI (defer to later `U09+`; needs format and ownership decisions).

## Completion criteria
- Session status is represented as an explicit shared UX type and is displayed consistently as one of: `disabled`, `starting`, `connected`, `reconnecting`, `failed`.
- UI surfaces are grouped into Session/Social/Diagnostics in a way that reduces workflow friction while preserving existing information.
- Chat and IM sends are correctly gated with clear status/error feedback.
- Profile UI shows per-tab freshness (`fresh`/`stale`/`unknown`) consistent with TTL behavior and presents errors clearly.
- Required validation commands are run and pass (or any failures are explicitly documented during execution).
