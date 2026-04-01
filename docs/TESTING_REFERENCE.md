# TESTING_REFERENCE.md

Canonical, model-friendly reference for **how to validate changes in this repo**.

This file consolidates:
- the `cargo` commands we use as tests (static checks, unit tests, runtime smoke)
- deterministic runtime verification modes (notably `STRESS_TEST=...`)
- all repo-supported `VIEWER_*` env vars that act as test/verification knobs

If you add a new test mode or a new `VIEWER_*` env var, update this file in the same change.

---

## Choose the right validation (quick guide)

Always start with:
- `cargo fmt`
- `cargo check`

Then choose targeted verification based on what changed:

- **Docs-only change:** stop after `cargo fmt` + `cargo check`.
- **`viewer_core` types / scene / geometry:** `cargo test -p viewer_core` (plus `STRESS_TEST=2` runtime smoke when rendering-visible).
- **`viewer_render` GPU path:** `cargo test -p viewer_render` (if present) + runtime smoke via `cargo run -p viewer_app` with `STRESS_TEST=...`.
- **`viewer_asset` fixtures/cache/decoders:** `cargo test -p viewer_asset` (expects deterministic `test_assets/*` fixtures).
- **`viewer_net` transport/decode:** `cargo test -p viewer_net` + (optional) `cargo run -p viewer_net --example llsd_login_attempt`.
- **`viewer_ui` behavior/helpers:** `cargo test -p viewer_ui` + runtime smoke via `cargo run -p viewer_app` (offline mode recommended).
- **`viewer_app` orchestration/in-process worker:** `cargo test -p viewer_app` + runtime smoke (offline first, live second when applicable).

When behavior changed across crates, prefer a broader pass:
- `cargo test`

---

## Canonical commands (what “tests” look like here)

### Static checks

- `cargo fmt`
- `cargo check`

### Targeted unit tests (per crate)

- `cargo test -p viewer_app`
- `cargo test -p viewer_core`
- `cargo test -p viewer_render`
- `cargo test -p viewer_ui`
- `cargo test -p viewer_net`
- `cargo test -p viewer_grid`
- `cargo test -p viewer_asset`
- `cargo test -p viewer_platform`

### Runtime smoke / deterministic verification

Offline/bounded startup (recommended default for smoke):
- `VIEWER_APP_LIVE_STARTUP=off cargo run -p viewer_app`

Stress/verification modes:
- `VIEWER_APP_LIVE_STARTUP=off STRESS_TEST=2 cargo run -p viewer_app`
- `VIEWER_APP_LIVE_STARTUP=off STRESS_TEST=camera cargo run -p viewer_app`
- `VIEWER_APP_LIVE_STARTUP=off STRESS_TEST=screenshot cargo run -p viewer_app`
- `cargo run -p viewer_app --bin screenshot_diff -- <baseline_dir> <candidate_dir> [max_mean_abs_error]`

### Useful `cargo test` runner flags (general Rust)

These are not repo-specific, but are often useful for deterministic debugging:
- `cargo test -p viewer_app -- --nocapture`
- `cargo test -p viewer_app -- --test-threads=1`
- `cargo test -p viewer_app <name_substring>`
- `cargo test -p viewer_app <exact_name> -- --exact`

---

## Deterministic runtime verification: `STRESS_TEST`

`STRESS_TEST` is read by `viewer_app`.

Supported values:

| Value | Aliases | Meaning |
| --- | --- | --- |
| (unset) |  | Normal runtime |
| `1` |  | Legacy scene stress mode (moving proxy set) |
| `2` |  | Geometry torture scene (procedural + sculpt + mesh placeholders) |
| `camera` | `auto_camera`, `auto-camera`, `5` | Deterministic orbiting auto-camera |
| `screenshot` | `screenshots`, `capture`, `6` | Auto-camera + periodic PNG capture |

### Auto-camera tuning env vars (`STRESS_TEST=camera|screenshot`)

All defaults are the code defaults in `viewer_app`.

- `VIEWER_TEST_CAMERA_CENTER` (CSV `x,y,z`, default `0,0,0`)
- `VIEWER_TEST_CAMERA_RADIUS` (float, default `22`)
- `VIEWER_TEST_CAMERA_HEIGHT` (float, default `10`)
- `VIEWER_TEST_CAMERA_LOOK_HEIGHT` (float, default `2`)
- `VIEWER_TEST_CAMERA_SPEED` (float radians/sec, default `0.5`)
- `VIEWER_TEST_CAMERA_PHASE` (float radians, default `0`)
- `VIEWER_TEST_CAMERA_PATH_FILE` (path to JSON waypoint script; when set and valid, overrides orbit camera path)

### Screenshot tuning env vars (`STRESS_TEST=screenshot`)

- `VIEWER_TEST_SCREENSHOT_DIR` (path, default `artifacts/screenshots`)
- `VIEWER_TEST_SCREENSHOT_EVERY_N_FRAMES` (u64, default `30`, min `1`)
- `VIEWER_TEST_SCREENSHOT_MAX_FRAMES` (u32, default `12`, min `1`)

Recommended 1-frame smoke (fast, artifact-based):
```powershell
$env:VIEWER_APP_LIVE_STARTUP='off'
$env:STRESS_TEST='screenshot'
$env:VIEWER_TEST_SCREENSHOT_DIR='artifacts/screenshots_smoke'
$env:VIEWER_TEST_SCREENSHOT_EVERY_N_FRAMES='1'
$env:VIEWER_TEST_SCREENSHOT_MAX_FRAMES='1'
cargo run -p viewer_app
```

### Required screenshot review step

When using `STRESS_TEST=screenshot` (or any screenshot capture path), validation is not complete until
the produced image files are manually reviewed for expected scene behavior (camera framing, visible
geometry, fallback colors, fog/sky intent, and obvious corruption/regression).

Command success alone is insufficient for visual verification.

Implementation/report handoff should include:
- exact screenshot directory used
- at least one screenshot filename reviewed
- a one-line visual verdict

---

## Offline snapshot vs in-process live worker

`viewer_app` loads a sanitized “live visual snapshot” from disk **unless** the in-process live worker is enabled and producing snapshots.

### `.env` loading (dotenvy)

- `viewer_app` loads `.env` once at startup (best-effort) before reading `VIEWER_*` variables.
- `viewer_relay` also loads `.env` once at startup.
- `viewer_net --example llsd_login_attempt` does **not** load `.env` (set env vars explicitly for that example).

Treat `.env` as secrets-bearing local configuration; never paste credentials into repo docs or reports.

- `VIEWER_LIVE_VISUAL_SNAPSHOT_PATH`
  - Used by: `viewer_app`, `viewer_net` `llsd_login_attempt` example
  - Meaning: path to read/write sanitized `LiveVisualSnapshot` JSON
  - Default: `live_visual_snapshot.json`

### `VIEWER_APP_LIVE_STARTUP`

Controls whether `viewer_app` tries to start the in-process live network worker.

- `VIEWER_APP_LIVE_STARTUP=off` (aliases: `disabled`, `false`, `0`)
  - disables the worker (offline/bounded mode)
- `VIEWER_APP_LIVE_STARTUP=on` (aliases: `enabled`, `true`, `1`)
  - forces the worker on; missing required login env becomes a startup failure
- `VIEWER_APP_LIVE_STARTUP=auto` (default)
  - enables the worker only when required login env vars are present

---

## `viewer_app` in-process live worker env vars

These variables are read by `viewer_app` when in-process live startup is enabled (`VIEWER_APP_LIVE_STARTUP=on|auto` and login envs are present).

### Required (to start worker)

- `VIEWER_LOGIN_ENDPOINT`
- `VIEWER_LOGIN_USERNAME`
- `VIEWER_LOGIN_PASSWORD`

### Login shaping / safety toggles

- `VIEWER_LOGIN_TIMEOUT_SECS` (u64, default `15`)
- `VIEWER_LOGIN_WIRE_FORMAT` (`llsd` | `json` | `xmlrpc`, default `llsd`)
- `VIEWER_LOGIN_START` (`home` | `last` | URI-like string, default `last`)
- `VIEWER_LOGIN_AGREE_TOS` (bool-like, default `false`)
- `VIEWER_LOGIN_READ_CRITICAL` (bool-like, default `true`)
- `VIEWER_LOGIN_MFA_TOKEN` (optional string)

### First-simulator probe controls (worker)

- `VIEWER_FIRST_SIM_RECEIVE_BIND` (default `0.0.0.0:0`)
- `VIEWER_FIRST_SIM_RECEIVE_TIMEOUT_SECS` (u64, default `5`)
- `VIEWER_FIRST_SIM_RECEIVE_MAX_PACKETS` (usize, default `8`)
- `VIEWER_FIRST_SIM_POST_MOVEMENT_TAIL_PACKETS` (usize, default `4`)
- `VIEWER_FIRST_SIM_POST_MOVEMENT_TIMEOUT_SECS` (optional u64; if unset, uses receive timeout)
- `VIEWER_FIRST_SIM_STOP_ON_REGION_CONTROL` (bool-like, default `false`)

### Worker scheduling / polling knobs

- `VIEWER_APP_IN_PROCESS_PROBE` (bool-like, default `true`)
- `VIEWER_APP_WORKER_TICK_MS` (u64, default `60`, min `10`)
- `VIEWER_APP_AUTO_TELEPORT_SLURL` (optional string)
  - one-shot reconnect-teleport target for terminal-driven capture runs
  - accepts the same supported SLURL forms as the in-app `Network Debug` control
- `VIEWER_APP_AUTO_TELEPORT_DELAY_TICKS` (u32, default `40`, max `10000`)
  - delay after connected steady state before the one-shot auto-teleport fires
- `VIEWER_APP_REGION_OBJECTS_REPROBE_DELAY_TICKS` (u32, default `80`, `1..10000`)
  - reconnect-only bounded delay before the one-shot post-reconnect `RegionObjects` re-probe
  - used to test whether first post-reconnect `typed_sample=none` is a timing artifact
- `VIEWER_APP_EVENT_QUEUE_POLL_TIMEOUT_MS` (u64, default `45000`, min `100`)
- `VIEWER_APP_EVENT_QUEUE_POLL_EVERY_TICKS` (u32, default `10`, min `1`)
- `VIEWER_APP_EVENT_QUEUE_FAILURES_BEFORE_RECONNECT` (u32, default `0`)
- `VIEWER_APP_SOCIAL_POLL_TIMEOUT_MS` (u64, default `35`, min `5`)
- `VIEWER_APP_SOCIAL_POLL_MAX_PACKETS` (usize, default `4`)
- `VIEWER_APP_NEARBY_POLL_TIMEOUT_MS` (u64, default `40`, min `5`)
- `VIEWER_APP_NEARBY_POLL_MAX_PACKETS` (usize, default `2`)
- `VIEWER_APP_NEARBY_SEND_RECEIVE_TIMEOUT_MS` (u64, default `40`, min `5`)
- `VIEWER_APP_NEARBY_SEND_RECEIVE_PACKETS` (usize, default `0`)
- `VIEWER_APP_PROFILE_CACHE_TTL_SECS` (u64, default `120`)
- `VIEWER_NETWORK_DEBUG_LOG_PATH` (path, default `logs/network_debug.jsonl`)
  - append-only JSONL sink for network-debug relay categories used by the in-app `Network Debug` window

Example bounded reconnect-teleport capture run:
```powershell
$env:VIEWER_APP_LIVE_STARTUP='on'
$env:VIEWER_APP_AUTO_TELEPORT_SLURL='secondlife://Ahern/50/60/70'
$env:VIEWER_APP_AUTO_TELEPORT_DELAY_TICKS='40'
$env:VIEWER_NETWORK_DEBUG_LOG_PATH='artifacts/logs/network_debug_region_change.jsonl'
cargo run -p viewer_app
```

---

## Fixture-backed acquisition smoke knobs

These are used to validate asset/render integration without relying on live texture fetch.

- `VIEWER_FIXTURE_TEXTURES`
  - unset/empty/`0`: disabled
  - `1` / `true` / `yes` / `on`: enable default fixture IDs (`water_diffuse`, `stone_diffuse`, `stone_normal`)
  - otherwise: comma-separated asset IDs (example: `stone_diffuse,stone_normal`)

Fixture location:
- `test_assets/*` (repo directory; used by `viewer_asset::FixtureTextureCache`)

Budget tuning:
- `VIEWER_ASSET_CACHE_BUDGET_MB` (default `512`, clamped `16..4096`)
- `VIEWER_RENDER_VRAM_BUDGET_MB` (default `512`, clamped `16..4096`)
- `VIEWER_ASSET_SOURCE_MODE` (`fixture` | `auto` | `live`, default `auto`)
- `VIEWER_ASSET_LIVE_TIMEOUT_MS` (default `10000`, clamped `250..120000`)

Renderer debug forcing (narrow, verification-only):
- `VIEWER_RENDER_FORCE_AVATAR_PROXY_FALLBACK` (bool-like; forces avatar proxy fallback path)

---

## Social cache knobs (debug/workflow verification)

- `VIEWER_SOCIAL_CACHE_PATH` (default `logs/social_cache.db`)
- `VIEWER_SOCIAL_CACHE_MAX_MESSAGES_PER_THREAD` (default `500`, min `50`)

---

## Manual diagnostic entry points (used as “tests”)

### `viewer_net` manual login attempt example

Runs:
- `cargo run -p viewer_net --example llsd_login_attempt`

Reference:
- `crates/viewer_net/examples/README.md` (canonical for this example)

Key defaults differ from `viewer_app` worker defaults:
- `VIEWER_FIRST_SIM_RECEIVE_MAX_PACKETS` default `3`
- `VIEWER_FIRST_SIM_POST_MOVEMENT_TAIL_PACKETS` default `0`

### `viewer_app` relay binary (`viewer_relay`)

Runs:
- `cargo run -p viewer_app --bin viewer_relay`

Required:
- `VIEWER_LOGIN_ENDPOINT`
- `VIEWER_LOGIN_USERNAME`
- `VIEWER_LOGIN_PASSWORD`

Optional:
- `VIEWER_LOGIN_WIRE_FORMAT` (default `llsd`)
- `VIEWER_LOGIN_START` (default `last`)
- `VIEWER_FIRST_SIM_RECEIVE_BIND` (default `0.0.0.0:0`)
- `VIEWER_RELAY_LOG_PATH` (default `logs/viewer_relay.jsonl`)
- `VIEWER_RELAY_RECONNECT` (bool-like, default `true`)

---

## Historical / retired / doc-only flags (not implemented today)

These show up in old plans/research docs but are **not** currently implemented in code:

- `STRESS_TEST=3` (retired plan M3: “Material Showcase”)
- `STRESS_TEST=4` (retired plan M4: performance comparison / object scale stress)
- `VIEWER_FETCH_RANDOM_OBJECT` (retired plan M4)
- `VIEWER_TARGET_UUID` (mentioned in `docs/RESEARCH/targeted_asset_fetching.md`)

If one of these becomes real again, update this file and `docs/CURRENT_STATE.md` as part of that implementation.
