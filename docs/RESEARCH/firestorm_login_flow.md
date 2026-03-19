# Firestorm Login Flow (Reference Research)

## Scope
- Source inspected: `reference/firestorm` (behavioral reference only).
- Goal: extract sequence/responsibilities, not architecture or code.

## Key Files And Functions
- `reference/firestorm/indra/newview/llstartup.cpp`
  - Startup state machine around login:
  - `STATE_LOGIN_AUTH_INIT`
  - `STATE_LOGIN_PROCESS_RESPONSE`
  - `STATE_WORLD_INIT`
  - `process_login_success_response(...)`
  - Sends/coordinates `UseCircuitCode` and `CompleteAgentMovement` during initial world connect.
- `reference/firestorm/indra/newview/lllogininstance.cpp` / `.h`
  - `LLLoginInstance::constructAuthParams(...)`
  - `connect(...)`, `reconnect()`, `disconnect()`
  - `handleLoginEvent(...)`, `handleLoginFailure(...)`, `handleLoginSuccess(...)`
  - TOS / critical / update / MFA handling paths.
- `reference/firestorm/indra/viewer_components/login/lllogin.cpp` / `.h`
  - `LLLogin::Impl::loginCoro(...)`
  - Login transport orchestration, redirects, auth progress events, fail/success dispatch.
- `reference/firestorm/indra/newview/llviewerregion.cpp`
  - `LLViewerRegion::setSeedCapability(...)`
  - Seed capability request/retry and capability map population.
  - Event queue startup when `EventQueueGet` cap arrives.
- `reference/firestorm/indra/newview/llworld.cpp`
  - `LLEstablishAgentCommunication` HTTP node (region seed-cap updates during crossings).
  - Initial simulator circuit enable path.
- `reference/firestorm/indra/newview/llviewermessage.cpp`
  - `send_complete_agent_movement(...)`
  - Related movement/session handshake messaging.

## Login Sequence (Observed)
1. UI/startup gathers credentials + start location + selected grid (`llstartup` + login panels).
2. `LLLoginInstance` builds request payload:
   - method: `login_to_simulator`
   - params (identity/session/device/start flags)
   - options (requested optional datasets)
   - HTTP timeout/retry metadata.
3. `LLLogin` coroutine performs auth request, including redirect handling (`indeterminate` responses with `next_url` / `next_method`).
4. Auth result dispatch:
   - success -> `"online"/"connect"` event with response data.
   - failure -> `"offline"/"fail.login"` event with reason/message data.
5. `llstartup` processes auth response:
   - failure paths: user messaging, TOS, update, MFA, retry/abort decisions.
   - success path: `process_login_success_response(...)` unpacks session + first-region bootstrap data.
6. Startup transitions into world init:
   - initializes agent/session state
   - enables initial simulator circuit (`UseCircuitCode`)
   - sends `CompleteAgentMovement`
   - requests seed capabilities for first region.
7. Region capability bootstrap:
   - `seed_capability` used to request caps set
   - on `EventQueueGet`, event polling starts
   - additional caps initialize downstream services.

## Important Request Data (Login Request)
- Credentials/user identity fields.
- Start target (`home` / `last` / explicit URI-like start string).
- Session continuity hints (`last_exec_*`, previous session id).
- Device/client metadata (`channel`, `version`, platform, host id, machine hash).
- Feature/options list (inventory roots/skeleton, buddy list, event/classified categories, max groups, map/voice/config flags, etc.).
- MFA token/hash-related params (when present).

### Auth payload note: `passwd` semantics
- Firestorm references consistently use `params.passwd` for legacy XML-RPC style login payloads.
- Firestorm tests include legacy `passwd` values in `$1$...` form (see `reference/firestorm/indra/newview/tests/lllogininstance_test.cpp`).
- Firestorm login handler paths compute MD5 password digests for credential handling (`reference/firestorm/indra/newview/llloginhandler.cpp`).
- Compatibility implication for this rewrite: LLSD login payload should treat `passwd` as legacy-hash formatted data (`$1$<md5>`), not raw plaintext.

### XML-RPC envelope experiment (2026-03-19)
- Added a transport-level experimental XML-RPC codec path in `viewer_net` using:
  - `methodName = login_to_simulator`
  - one struct parameter carrying shaped login fields and `options` array
  - legacy `passwd` formatting (`$1$<md5>`)
- Controlled live comparison against `https://login.agni.lindenlab.com/cgi-bin/login.cgi` with dummy credentials:
  - LLSD path returned `reason=viewer-data` with `Missing password`
  - XML-RPC path returned `reason=key` with invalid-credentials style message
- Interpretation: endpoint behavior strongly suggests XML-RPC envelope recognition, and the current blocker likely moved from envelope-level mismatch toward field-level auth compatibility.

## Important Response Data (Login/Startup)
- Session identity:
  - `agent_id`
  - `session_id`
  - `secure_session_id`
  - `circuit_code`
- Initial region bootstrap:
  - `sim_ip`
  - `sim_port`
  - `region_x`
  - `region_y`
  - `seed_capability`
  - optional region size fields.
- Initial avatar/world context:
  - `start_location`
  - `look_at`
  - `home`
  - MOTD/message text.
- Optional datasets requested via `options`:
  - `inventory-root`, skeleton/lib roots, buddy-list, categories, login flags, group limits, etc.

## Responsibility Split (What To Keep Separate In Rust)

### Generic connection/session concerns (`viewer_net`)
- Transport-level login request/response exchange.
- Connection state machine (disconnected/connecting/authenticated/session active).
- Retry/redirect policy, timeout policy, error classification.
- Session token/id container and lifecycle transitions.
- Generic event stream hooks for progress and failure/success.

### Grid-specific behavior (`viewer_grid`)
- Request shape variants and option sets by grid family (SL/OpenSim differences).
- Mapping server-specific reason/message IDs into domain error enums.
- Capability namespace differences and required/optional caps policy.
- Grid-specific interpretation of optional response blocks.

### Startup/viewer orchestration (not in `viewer_net`)
- UI dialogs and user interaction (TOS/update/MFA prompts).
- Progress screen transitions and startup state machine.
- Agent/camera/world bootstrap ordering.
- Region object creation, simulator messaging bootstrap (`UseCircuitCode`, `CompleteAgentMovement`), and capability fan-out to subsystems.

## Recommended Minimal Rust Mapping
- `viewer_net`
  - `Connection` (already scaffolded) should evolve into:
  - `connect() -> AuthEnvelope`
  - `login(request) -> LoginResponseEnvelope`
  - `disconnect()`
  - no UI/startup branching.
- `viewer_grid`
  - `GridAdapter` trait:
    - build login request payload/options
    - parse/validate response into typed `GridLoginBootstrap`
    - classify login failures.
- `viewer_app` (or startup layer)
  - orchestrates user-facing decision points and state transitions.
  - hands final bootstrap data to scene/world/network runtime systems.

## Notes
- Firestorm combines transport, startup, and UI reaction through a large state machine and event pumps.
- For this Rust rewrite, keep the same external behavior ordering, but split boundaries earlier to avoid coupling `viewer_net` to startup/UI concerns.
