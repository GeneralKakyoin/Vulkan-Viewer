# HANDOFF.md

## Last Completed Work

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
- define minimal capability client boundary
- perform first seed capability request
- capture and classify early bootstrap capability payloads
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
- `docs/RESEARCH/firestorm_login_flow.md`
- new capability/bootstrap research notes in `docs/RESEARCH/*`

---

## What To Check After The Next Change

- does seed capability fetch return a valid, parseable response?
- are capability responses logged in a sanitized way?
- are capability/bootstrap interpretations staying in `viewer_grid`?
- do automated tests still pass?
- were `CURRENT_STATE.md`, `HANDOFF.md`, `MASTER_PLAN.md`, and `TASKS.md` updated when project state changed?
