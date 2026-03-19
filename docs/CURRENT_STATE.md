# CURRENT_STATE.md

## Current Phase
Phase B — Credible login

The foundation and architecture-proof work are complete enough to support real login compatibility work. The active focus is making the live LLSD login request acceptable to the real endpoint without breaking crate boundaries.

---

## Working

### Runtime / Rendering
- renderer/ui/input/camera stack
- sandbox scene
- multiple objects
- ground plane
- cube
- world-axis marker
- depth buffer
- corrected controls

### Architecture
- clear crate boundaries
- `viewer_app` remains orchestration-only
- `viewer_render` owns rendering
- `viewer_ui` owns egui/debug UI
- `viewer_net` owns transport/session
- `viewer_grid` owns grid-specific semantics

### Login Stack
- HTTP login transport
- redirect handling
- typed login request/result/bootstrap models
- login diagnostics/trace
- JSON codec
- minimal LLSD codec
- experimental XML-RPC `login_to_simulator` codec path in `viewer_net`
- LLSD `passwd` normalization to legacy `$1$<md5>` form
- configurable wire format
- manual LLSD login attempt example

### Research / Continuity
- Firestorm login flow documented
- continuity-stack docs established
- scope model established
- workflow discipline defined in AGENTS

---

## Current Blocker

The real LLSD login request reaches the live Second Life login endpoint, but the endpoint still does not accept the credential/payload shape as valid enough for successful login progression.

This is now a protocol-compatibility problem, not a transport-architecture problem.

---

## Most Likely Immediate Work

- continue LLSD auth/payload alignment
- verify XML-RPC field-level compatibility (not just envelope) using controlled live attempts
- compare XML-RPC key/auth outcomes between account-style and legacy first/last identifiers
- verify account identifier handling
- improve live-compatibility fixtures
- rerun controlled real login attempts with sanitized trace inspection

---

## Current Next Step

The smallest correct next step is:

**continue aligning auth payload semantics and exact LLSD request structure until the real endpoint recognizes credentials correctly**

---

## Do Not Do Yet

- simulator connection
- event queue
- world streaming
- capability bootstrap
- login UI integration
- broad viewer feature work
