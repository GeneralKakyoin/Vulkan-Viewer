# TASKS.md

## Current Focus
Only work on the smallest steps that improve real login compatibility without breaking architecture boundaries.

---

## Active Tasks

### T1 — Verify auth payload semantics
Why it matters:
The live endpoint is still rejecting the current auth payload shape.

Dependencies:
- working LLSD codec
- manual login attempt path
- live trace output

Done when:
- request field semantics are better aligned
- live endpoint response changes meaningfully
- tests still pass

---

### T2 — Verify exact LLSD request envelope
Why it matters:
The payload may still be structurally valid LLSD but wrong for the endpoint’s expected request shape.

Dependencies:
- T1
- Firestorm/SL behavior research

Done when:
- request envelope is explicitly documented
- compatibility corrections are implemented
- manual live attempt shows a changed result or a more precise blocker

---

### T3 — Improve compatibility fixtures
Why it matters:
The live request path is ahead of fixture coverage.

Dependencies:
- T1/T2 findings

Done when:
- LLSD fixtures cover the currently understood auth/request cases
- tests validate the current encoding assumptions

---

### T4 — Document live-login compatibility findings
Why it matters:
Protocol alignment work is brittle unless continuity docs stay current.

Dependencies:
- every meaningful auth/payload change

Done when:
- `docs/RESEARCH/*` updated
- `docs/CURRENT_STATE.md` updated
- `docs/HANDOFF.md` updated

---

## Next Tasks After Login Acceptance

### T5 — Validate successful real login
Done when:
- `GridLoginResult::Success` is achieved
- session/bootstrap fields are real and populated

### T6 — Seed capability bootstrap
Done when:
- seed capability can be queried safely
- bootstrap data is logged and interpreted

### T7 — Event queue startup
Done when:
- minimal event queue polling works
- early live events can be received and classified

### T8 — Simulator handshake
Done when:
- first region connection is established
- first meaningful world-state data arrives

---

## Explicitly Deferred

Do not work on these yet unless the plan changes:
- viewer login UI
- simulator rendering integration
- asset-backed world rendering
- inventory workflows
- chat/map/teleport shells
- media/voice
- OpenSim divergence support
