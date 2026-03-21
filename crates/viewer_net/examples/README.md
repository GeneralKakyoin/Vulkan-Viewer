# Manual Login Attempt

This example performs a controlled login attempt through `viewer_net` using:
- `LoginWireFormat` selected via environment
- `Connection::login_with_trace(...)`
- `SecondLifeAdapter`

It is manual-only and requires environment variables. It does not store credentials.

## Required env vars
- `VIEWER_LOGIN_ENDPOINT`
- `VIEWER_LOGIN_USERNAME`
- `VIEWER_LOGIN_PASSWORD`

## Optional env vars
- `VIEWER_LOGIN_START` (`last` | `home` | URI-like string, default: `last`)
- `VIEWER_LOGIN_AGREE_TOS` (`true/false`, default: `false`)
- `VIEWER_LOGIN_READ_CRITICAL` (`true/false`, default: `true`)
- `VIEWER_LOGIN_MFA_TOKEN`
- `VIEWER_LOGIN_TIMEOUT_SECS` (default: `15`)
- `VIEWER_LOGIN_WIRE_FORMAT` (`llsd` | `json` | `xmlrpc`, default: `llsd`)
- `VIEWER_FETCH_SEED_CAPS` (`true/false`, default: `false`)
- `VIEWER_INSPECT_EVENT_QUEUE_ONCE` (`true/false`, default: `false`)
- `VIEWER_INSPECT_SIMULATOR_FEATURES_ONCE` (`true/false`, default: `false`)
- `VIEWER_INSPECT_MAP_LAYER_ONCE` (`true/false`, default: `false`)
- `VIEWER_INSPECT_FIRST_SIM_HANDSHAKE_ONCE` (`true/false`, default: `false`)
  - explicit first-simulator probe enable flag
  - probe is also auto-enabled if any `VIEWER_FIRST_SIM_*` control variable is set
- `VIEWER_FIRST_SIM_RECEIVE_BIND` (UDP bind target for one-shot receive, default: `0.0.0.0:0`)
- `VIEWER_FIRST_SIM_RECEIVE_TIMEOUT_SECS` (default: `5`)
- `VIEWER_FIRST_SIM_RECEIVE_MAX_PACKETS` (bounded receive packet count, default: `3`)
- `VIEWER_FIRST_SIM_POST_MOVEMENT_TAIL_PACKETS` (how many packets to keep after first `AgentMovementComplete`, default: `0`)
- `VIEWER_FIRST_SIM_POST_MOVEMENT_TIMEOUT_SECS` (optional post-AMC receive timeout override; if unset, uses `VIEWER_FIRST_SIM_RECEIVE_TIMEOUT_SECS`)
- `VIEWER_FIRST_SIM_STOP_ON_REGION_CONTROL` (`true/false`, default: `false`; stop early once first `CrossedRegion`/`ConfirmEnableSimulator` is observed)
- `VIEWER_LIVE_VISUAL_SNAPSHOT_PATH` (optional output path for sanitized live visual snapshot JSON; default: `live_visual_snapshot.json`)

## First-Simulator Probe Enable Rules
- Probe runs when either:
  - `VIEWER_INSPECT_FIRST_SIM_HANDSHAKE_ONCE=true`, or
  - any `VIEWER_FIRST_SIM_*` probe-control env var is set
- If neither condition is met, the example prints an explicit message that probe is disabled and how to enable it.

## Run
```powershell
$env:VIEWER_LOGIN_ENDPOINT="https://your-grid-login-endpoint"
$env:VIEWER_LOGIN_USERNAME="your.username"
$env:VIEWER_LOGIN_PASSWORD="your-password"
$env:VIEWER_LOGIN_WIRE_FORMAT="xmlrpc"
$env:VIEWER_FETCH_SEED_CAPS="true"
$env:VIEWER_INSPECT_EVENT_QUEUE_ONCE="true"
$env:VIEWER_INSPECT_SIMULATOR_FEATURES_ONCE="true"
$env:VIEWER_INSPECT_MAP_LAYER_ONCE="true"
$env:VIEWER_INSPECT_FIRST_SIM_HANDSHAKE_ONCE="true"
$env:VIEWER_FIRST_SIM_RECEIVE_BIND="0.0.0.0:0"
$env:VIEWER_FIRST_SIM_RECEIVE_TIMEOUT_SECS="5"
$env:VIEWER_FIRST_SIM_RECEIVE_MAX_PACKETS="3"
$env:VIEWER_FIRST_SIM_POST_MOVEMENT_TAIL_PACKETS="2"
$env:VIEWER_FIRST_SIM_POST_MOVEMENT_TIMEOUT_SECS="15"
$env:VIEWER_FIRST_SIM_STOP_ON_REGION_CONTROL="true"
$env:VIEWER_LIVE_VISUAL_SNAPSHOT_PATH="live_visual_snapshot.json"
cargo run -p viewer_net --example llsd_login_attempt
```

## Live Visual Snapshot Bridge
- This example writes a sanitized live visual snapshot JSON after login/probe.
- Default output path: `live_visual_snapshot.json` in the current working directory.
- `viewer_app` can load this snapshot (same default path, or via `VIEWER_LIVE_VISUAL_SNAPSHOT_PATH`) to show live-derived status in the on-screen debug overlay.
- Sensitive values such as session IDs and seed URLs are intentionally not included.

## Output
- High-level login outcome (`success`, `failed`, `requires_tos`, etc.)
- Sanitized `LoginTrace`
- Interpreted typed result or transport/protocol error details
