# HANDOFF.md

## Last Completed Work

- Updated repository agent-policy infrastructure (no product-code behavior change):
  - strengthened `AGENTS.MD` autonomous execution guidance for milestone-driven progress
  - clarified low-cost subagent usage expectations and main-session responsibilities
  - wired skill-discovery policy to check `.agents/skills/viewer/` first with `.agents/skills/` fallback
- Added repo Codex runtime config alignment in `.codex/config.toml`:
  - stronger main session model retained
  - `workspace-write` sandbox with network enabled
  - `on-request` approval policy
  - conservative subagent fan-out
- Added bounded low-cost subagent profiles:
  - `.codex/agents/worker.toml`
  - `.codex/agents/explorer.toml`
- Added minimal seed capability fetch support in `viewer_net` after logged-in session establishment
- Seed capability response parsing currently extracts top-level capability name -> URL pairs for initial inspection
- Added focused test coverage for login -> seed capability fetch path using LLSD capability map fixture
- Updated manual login example to optionally fetch and list capability names (`VIEWER_FETCH_SEED_CAPS=true`)
- Added one-shot EventQueueGet inspection path (no polling loop) with minimal top-level diagnostics and event-name extraction
- Corrected EventQueueGet one-shot timeout behavior for long-held responses
- Live behavior shifted from transport send failure to upstream HTTP proxy/server error response (no event payload yet)
- Added bounded one-shot EventQueueGet retry policy (3 attempts, retryable-only 5xx/transport cases)
- Added per-attempt diagnostics for EventQueueGet one-shot:
  - status (if present)
  - elapsed time
  - retryable classification
  - response headers when available
- Latest live run shows mixed retryable failures across attempts (HTTP 500 proxy-style and transport send) and still no parseable event names
- Added one-shot SimulatorFeatures fetch/inspection path to continue bootstrap observation with minimal top-level diagnostics
- Manual example now inspects SimulatorFeatures before EventQueueGet when both are enabled
- Added one-shot MapLayer fetch/inspection path with minimal top-level diagnostics
- Live MapLayer one-shot attempt currently returns HTTP 405 (method not allowed) with current GET request shape
- Added protocol-aligned LLSD `Accept` headers for one-shot capability requests (`SimulatorFeatures`, `MapLayer`, `EventQueueGet`) in `viewer_net`
- Added explicit `MapLayerLikelyLegacyUdp` error classification when one-shot MapLayer returns HTTP 405
- Added focused test coverage:
  - SimulatorFeatures one-shot sends LLSD `Accept` header
  - MapLayer one-shot sends LLSD `Accept` header
  - MapLayer HTTP 405 maps to explicit likely-legacy-UDP classification
- Live re-check confirms:
  - MapLayer: still 405, now explicitly classified as likely non-HTTP/legacy map path
  - SimulatorFeatures: still HTTP 503 Service Unavailable
  - EventQueueGet: still bounded retry failures (HTTP 500 proxy-style + occasional transport send failure)
- Achieved successful real Second Life login through the live endpoint
- Confirmed `GridLoginResult::Success` path with real bootstrap/session population (sensitive values intentionally not recorded here)
- Completed login compatibility milestone transition from payload-envelope debugging to post-login startup work
- Preserved crate boundaries while reaching live login:
  - `viewer_net` remained transport/codec/session lifecycle
  - `viewer_grid` remained request shaping and response interpretation

---

## What Changed In Project Understanding

- Autonomous execution can be more milestone-complete and less stop/start without weakening boundary controls.
- Repo skills and low-cost subagents are now first-class workflow defaults for repeatable bounded tasks.
- Login compatibility is now credible and proven in live conditions
- The highest-risk unknown is no longer login acceptance; it is post-login bootstrap sequencing and capability handling
- Seed capability startup is now the critical path before any simulator/world integration work
- MapLayer should no longer be treated as the best HTTP one-shot bootstrap candidate; immediate HTTP bootstrap evidence should prioritize SimulatorFeatures and EventQueueGet.

---

## Immediate Next Task

Focus only on:

**seed capability bootstrap using successful login output already available in-session**

This includes:
- expand capability typing/interpretation boundary in `viewer_grid`
- exercise seed capability fetch against live successful login state
- continue one-shot EventQueueGet transport stabilization until first parseable event envelope is observed
- use one-shot SimulatorFeatures inspection as parallel bootstrap evidence while EventQueueGet remains unstable
- keep MapLayer classified as likely legacy/unsupported for HTTP one-shot inspection unless new evidence suggests otherwise
- capture and classify early bootstrap capability payloads without sensitive value leakage
- keep bootstrap diagnostics explicit and sanitized

Keep the current boundaries intact:
- `viewer_net` = transport/session/codec and capability HTTP mechanics
- `viewer_grid` = grid-specific meaning and typed interpretation

---

## Constraints

Do not:
- start simulator connection work
- start world streaming/integration
- integrate login/bootstrap into `viewer_app`
- do broad refactors
- break JSON or LLSD compatibility paths
- copy Firestorm code

---

## Files Most Likely Involved Next

- `crates/viewer_net/src/lib.rs`
- `crates/viewer_grid/src/lib.rs`
- `crates/viewer_net/examples/llsd_login_attempt.rs`
- `crates/viewer_net/examples/README.md`
- `docs/RESEARCH/firestorm_login_flow.md`
- new capability/bootstrap research notes in `docs/RESEARCH/*`

---

## What To Check After The Next Change

- does seed capability fetch return a valid, parseable response?
- are capability responses logged in a sanitized way?
- are capability/bootstrap interpretations staying in `viewer_grid`?
- do automated tests still pass?
- were `CURRENT_STATE.md`, `HANDOFF.md`, `MASTER_PLAN.md`, and `TASKS.md` updated when project state changed?
