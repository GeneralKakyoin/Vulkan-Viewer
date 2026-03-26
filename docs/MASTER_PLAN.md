# MASTER_PLAN.md

This file is the durable constitution for the viewer project.
It defines long-lived project intent, operating principles, architectural ownership, and non-negotiable boundaries.

It does **not** track current status, current priorities, recent progress, or milestone history.
Those belong in:

* `docs/CURRENT_STATE.md`
* `docs/HANDOFF.md`
* `docs/TASKS.md`
* `docs/plans/`
* `docs/reviews/`
* `docs/reports/`

---

## Project mission

Build a modern, maintainable Rust viewer that can connect to a real Second Life-compatible grid while preserving clean architecture, disciplined incremental progress, and long-term extensibility.

The project should evolve in bounded, reviewable steps.
Compatibility matters, but it must not come at the cost of architectural collapse.

---

## Project identity

This project is:

* a compatibility-oriented virtual world viewer
* a real client with live-grid ambitions
* a long-term maintainable Rust codebase
* a system that should remain understandable by future agents and contributors

This project is **not**:

* a throwaway prototype
* a game engine experiment
* a direct architectural clone of Firestorm or any legacy viewer
* a place for broad uncontrolled refactors in pursuit of short-term convenience

---

## Long-term scope

The long-term direction of the project is:

1. maintain a stable application and rendering runtime foundation
2. support real login, session, and simulator connectivity
3. maintain a bounded but real connected-world slice
4. expand from diagnostic and bounded world rendering toward full asset-backed world rendering
5. later extend toward broader viewer capability and parity-oriented features

This phase map is intentionally high level.
Detailed sequencing, milestone planning, and current focus belong elsewhere.

---

## Core principles

When tradeoffs appear, prefer them in this order:

1. **Compatibility first**
   The viewer should preserve real viewer behavior where practical and avoid breaking protocol or world expectations through guesswork.

2. **Stability second**
   A smaller stable capability is better than a broader unstable one.

3. **Maintainability third**
   New work should keep the codebase modular, understandable, and fit for later expansion.

4. **Performance fourth**
   Performance matters, but it should not justify premature architectural shortcuts that damage long-term structure.

5. **Feature breadth last**
   Do not chase surface area before the underlying architecture can support it cleanly.

---

## Architectural ownership

Each crate should preserve a clear responsibility boundary.
The exact interface details may evolve, but the ownership model should remain stable.

### `viewer_app`

Owns application orchestration, top-level runtime flow, and wiring between subsystems.
It should coordinate systems, not absorb renderer, protocol, or asset internals.

### `viewer_core`

Owns shared domain state, scene-facing world structures, common data types, and core viewer logic that should not depend on a specific transport or rendering backend.

### `viewer_render`

Owns GPU-facing rendering internals, pipelines, buffers, materials, draw preparation, and render execution.
It should not become the owner of high-level app policy.

### `viewer_net`

Owns low-level transport, session wiring, packet/capability mechanics, and protocol-adjacent plumbing.
It should not own high-level grid semantics or viewer policy.

### `viewer_grid`

Owns grid-meaningful interpretation, policy, and user-facing semantics derived from lower-level network behavior.
It should not become a transport crate.

### `viewer_asset`

Owns asset loading, decoding, caching, and asset-backed content preparation.
It should not become an orchestration or scene-policy crate.

### `viewer_platform`

Owns platform integration concerns and should remain isolated from higher-level viewer policy where possible.

---

## Non-negotiable boundaries

The following rules are repository law unless deliberately revised by explicit architectural decision:

* Do not mix grid semantics into `viewer_net`.
* Do not mix transport mechanics into `viewer_grid`.
* Do not turn `viewer_app` into a dumping ground for subsystem internals.
* Do not move renderer policy upward just because it is convenient for a local change.
* Do not collapse crate boundaries to chase a short-term fix.
* Do not change repo structure broadly unless the change is explicitly planned and justified.
* Do not rely on chat memory instead of repository documentation for continuity.
* Do not treat temporary implementation convenience as architectural permission.

---

## Firestorm usage policy

Firestorm is a **behavioral and protocol reference**, not an architecture template.

Use Firestorm to:

* understand viewer behavior
* confirm protocol expectations
* study legacy handling where behavior matters

Do not use Firestorm to:

* copy architecture wholesale
* import its layering assumptions into this repo
* justify breaking current crate ownership

When translating behavior from Firestorm, preserve this repo’s boundaries and architecture.

---

## Documentation philosophy

The repository must remain usable by a new agent or contributor without depending on hidden session memory.

That means:

* durable project law belongs in stable docs
* current truth belongs in `CURRENT_STATE.md`
* exact next step belongs in `HANDOFF.md`
* active priorities belong in `TASKS.md`
* durable lessons belong in `LEARNINGS.md`
the entire project roadmap is layed out in `docs/PLAN.md`
* task-specific planning, review, and execution evidence belong in saved artifacts under `docs/plans/`, `docs/reviews/`, and `docs/reports/`

Documentation should reduce drift, not duplicate it.

---

## Success definition

Progress is successful when it does all of the following at the same time:

* improves real viewer capability or confidence in that capability
* preserves architectural cleanliness
* keeps crate ownership understandable
* leaves enough written continuity for future work to resume safely
* avoids destructive shortcuts that create more rework later

A larger feature is **not** a success if it damages maintainability, boundaries, or repository continuity.

---

## Continuity rule for this file

`MASTER_PLAN.md` should stay durable.
If a section starts depending on words like **current**, **latest**, **next**, or **recently completed**, it probably belongs in another document.
