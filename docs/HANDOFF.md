# HANDOFF.md

## Last Completed Work

- Added minimal seed capability fetch support in `viewer_net` after logged-in session establishment
- Seed capability response parsing currently extracts top-level capability name -> URL pairs for initial inspection
- Added focused test coverage for login -> seed capability fetch path using LLSD capability map fixture
- Updated manual login example to optionally fetch and list capability names (`VIEWER_FETCH_SEED_CAPS=true`)
- Added one-shot EventQueueGet inspection path (no polling loop) with minimal top-level diagnostics and event-name extraction
- Corrected EventQueueGet one-shot timeout behavior for long-held responses
- Live behavior shifted from transport send failure to upstream HTTP proxy/server error response (no event payload yet)
- Achieved successful real Second Life login through the live endpoint
- Confirmed `GridLoginResult::Success` path with real bootstrap/session population (sensitive values intentionally not recorded here)
- Completed login compatibility milestone transition from payload-envelope debugging to post-login startup work
- Preserved crate boundaries while reaching live login:
  - `viewer_net` remained transport/codec/session lifecycle
  - `viewer_grid` remained request shaping and response interpretation

---

## What Changed In Project Understanding

- Login compatibility is now credible and proven in live conditions
- The highest-risk unknown is no longer login acceptance; it is post-login bootstrap sequencing and capability handling
- Seed capability startup is now the critical path before any simulator/world integration work

---

## Immediate Next Task

Focus only on:

**seed capability bootstrap using successful login output already available in-session**

This includes:
- expand capability typing/interpretation boundary in `viewer_grid`
- exercise seed capability fetch against live successful login state
- continue one-shot EventQueueGet transport stabilization until first parseable event envelope is observed
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
