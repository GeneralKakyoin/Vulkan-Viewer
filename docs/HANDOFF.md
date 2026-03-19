# HANDOFF.md

## Last Completed Work

- Added configurable login wire-format selection
- Added minimal LLSD codec support
- Added a manual LLSD login attempt path outside `viewer_app`
- Proved that the live Second Life login endpoint is reachable with the current stack
- Proved that the response is structured and decoded correctly
- Began auth-field compatibility correction work
- Confirmed the current blocker is payload/auth compatibility, not transport failure
- Corrected LLSD `passwd` semantics to legacy `$1$<md5>` encoding with idempotent handling for already-prefixed values
- Added focused tests for LLSD `passwd` encoding behavior

---

## What This Means

The architecture is holding.

Specifically:
- renderer foundation is working
- network/grid separation is working
- real HTTP login transport is working
- redirect handling is working
- diagnostics are working
- codec evolution is working

The project is no longer blocked on broad architecture. It is blocked on protocol correctness.

---

## Immediate Next Task

Focus only on:

**real login compatibility alignment for the LLSD request payload**

This includes:
- auth field semantics
- identifier handling
- exact top-level request envelope
- live compatibility verification through the manual login path

Keep the current boundaries intact:
- `viewer_net` = transport/session/codec selection
- `viewer_grid` = shaping/interpretation/policy

---

## Constraints

Do not:
- start simulator work
- start capability/bootstrap work
- integrate login into `viewer_app`
- do broad refactors
- break the JSON path
- copy Firestorm code

---

## Files Most Likely Involved Next

- `crates/viewer_grid/src/lib.rs`
- `crates/viewer_net/src/lib.rs`
- `crates/viewer_net/examples/llsd_login_attempt.rs`
- `docs/RESEARCH/firestorm_login_flow.md`
- any new login compatibility research notes

---

## What To Check After The Next Change

- did the live endpoint response change meaningfully?
- is the failure still structural or now an auth-state outcome?
- does `LoginTrace` show a clearer progression?
- do automated tests still pass?
- were `CURRENT_STATE.md` and `HANDOFF.md` updated?
- does the endpoint move past the prior "missing password" behavior after legacy `passwd` formatting?
