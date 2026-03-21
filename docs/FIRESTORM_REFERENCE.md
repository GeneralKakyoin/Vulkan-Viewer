# FIRESTORM_REFERENCE.md

## Purpose

This document defines how Firestorm should be used in this repository.

Firestorm is included as a **behavior reference**, not as an implementation template.

Its value is in helping us understand:
- protocol expectations
- login/session behavior
- bootstrap and capability sequencing
- compatibility quirks
- edge-case viewer behavior

It must not dictate:
- project architecture
- code structure
- dependency choices
- rendering design
- Rust implementation style

---

## Core Rule

**Extract behavior, not implementation.**

For every Firestorm investigation, the workflow is:

1. identify relevant Firestorm files
2. understand the behavior or sequence
3. document the findings in `docs/RESEARCH/*`
4. translate the findings into clean Rust abstractions inside the correct crate boundary

Do not copy Firestorm code.

---

## What Firestorm Is Good For

Use Firestorm as reference for:

### Login and Session
- login request/response expectations
- redirect/indeterminate login behavior
- MFA/TOS/update-required flows
- session bootstrap fields

### Bootstrap and Capabilities
- seed capability startup
- EventQueueGet bootstrapping
- early post-login ordering
- first-region initialization

### Protocol and Compatibility
- request field expectations
- payload shaping
- state transitions
- edge-case handling
- Second Life / OpenSim differences

### Viewer Behavior
- what conventional viewers do
- where compatibility expectations come from
- how legacy systems sequence startup

---

## What Firestorm Must NOT Be Used For

Do not use Firestorm as reference for:

### Architecture
- crate structure
- module layout
- subsystem ownership
- dependency direction

### Code Reuse
- direct code copying
- near-copy translation
- transplanting class structures into Rust

### Rendering Design
- renderer architecture
- Vulkan/wgpu design
- scene representation choices
- UI architecture

### Project Management
- roadmap structure
- documentation strategy
- agent workflow
- continuity process

---

## Current Firestorm-Relevant Areas

At the current phase of the project, Firestorm is most relevant for:

1. real login compatibility
2. auth payload and request-envelope expectations
3. response interpretation
4. post-login bootstrap sequencing
5. seed capability and event-queue startup
6. simulator handshake research later

It is not currently the right reference for renderer work, UI architecture, or project structure.

---

## Firestorm Research Workflow

When a task needs Firestorm research, use this process:

### Step 1 — Frame the Question
Define exactly what must be learned.

Examples:
- What fields does login_to_simulator expect?
- How does Firestorm classify redirect login results?
- What is the ordering after successful login before the world comes online?

### Step 2 — Locate Relevant Files
Identify the smallest relevant set of files in the Firestorm reference tree.

### Step 3 — Extract Behavior
Capture:
- sequence of steps
- important fields
- important state changes
- special cases / branches
- assumptions or uncertainties

### Step 4 — Document Findings
Write or update a note in `docs/RESEARCH/`.

Recommended format:
- question
- relevant Firestorm files
- behavior summary
- important fields / state transitions
- implications for this rewrite
- what remains uncertain

### Step 5 — Translate Into This Project
Map findings into the existing crate boundaries:
- `viewer_net` = transport/session mechanics
- `viewer_grid` = shaping/interpretation/policy
- `viewer_app` = orchestration
- later capability/bootstrap code = post-login startup

---

## Required Firestorm Research Output

Every meaningful Firestorm research pass should produce:

- relevant file list
- concise behavior summary
- explicit statement of what belongs in which crate
- known uncertainties
- a note in `docs/RESEARCH/*`

If the work changes project understanding materially, also update:
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `docs/MASTER_PLAN.md` if milestone/state changed

---

## Translation Rules

When translating Firestorm behavior:

### Keep
- behavioral intent
- sequencing logic
- compatibility requirements
- protocol meaning

### Replace
- class-heavy structures
- legacy coupling
- architecture assumptions
- implementation style
- rendering/UI patterns

### Improve
- separation of concerns
- testability
- typing
- observability
- maintainability

---

## Current Boundary Reminder

At the current project stage:

### `viewer_net`
Owns:
- transport
- redirects
- session state
- wire format selection
- diagnostics

### `viewer_grid`
Owns:
- login request shaping
- response interpretation
- login result classification
- bootstrap typing

This split must be preserved even when Firestorm suggests legacy coupling.

---

## Known Current Research Topics

Already researched or currently active:
- Firestorm login flow
- request/response field expectations
- redirect handling
- bootstrap/session field meaning
- auth payload compatibility

Likely upcoming topics:
- seed capability usage
- capability bootstrap ordering
- event queue startup
- first simulator handshake

---

## What To Do When Firestorm Conflicts With The Current Design

If a Firestorm behavior seems to imply architecture changes:

1. do not silently change architecture
2. document the conflict
3. explain the behavior in `docs/RESEARCH/*`
4. propose the smallest compatible Rust design
5. only update architecture docs if a real decision is made

---

## Success Condition For Firestorm Use

Firestorm is being used correctly if:
- the rewrite becomes more compatible
- docs become more precise
- crate boundaries remain clean
- no code is copied
- future agents can understand the extracted behavior from repo docs alone
