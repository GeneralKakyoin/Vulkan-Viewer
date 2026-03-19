# CURRENT_STATE.md

## Current Phase
Phase C - Post-login bootstrap

Real login compatibility is now proven against the live Second Life endpoint. The active focus has moved to safe post-login bootstrap sequencing, beginning with seed capability handling.

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
- XML-RPC credential shaping normalized to legacy `first`/`last` semantics
- configurable wire format
- manual login attempt example
- successful real live login achieved
- minimal seed capability fetch transport path implemented in `viewer_net`

### Research / Continuity
- Firestorm login flow documented
- continuity-stack docs established
- scope model established
- workflow discipline defined in AGENTS

---

## Current Blocker

Login payload compatibility is no longer the primary blocker. The immediate blocker is implementing post-login bootstrap safely while preserving crate boundaries and avoiding premature simulator/world integration.

---

## Most Likely Immediate Work

- seed capability bootstrap kickoff
- expand capability interpretation typing in `viewer_grid` while keeping transport in `viewer_net`
- fetch and inspect early bootstrap capability responses from live login state
- keep capability/bootstrap logic separate from simulator transport
- expand diagnostics for post-login bootstrap flow

---

## Current Next Step

The smallest correct next step is:

**begin seed capability bootstrap using already-proven live login results**

---

## Do Not Do Yet

- simulator connection
- event queue
- world streaming
- login UI integration
- broad viewer feature work
