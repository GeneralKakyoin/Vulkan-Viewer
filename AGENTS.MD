# AGENTS.md

## Purpose
This file defines how AI agents and subagents should operate in this repository.

The goal is to make the rewrite durable across many chats, many models, and many handoffs without loss of quality. Agents must treat repository documentation as the source of truth and must preserve project continuity.

This project is a from-scratch Rust rewrite of a Second Life / OpenSim viewer. It is a networked virtual-world viewer, not a game engine.

---

## Autonomous Execution Policy

For implementation sessions, agents have explicit permission to execute milestone-driven work autonomously.

Agents should continue working toward the assigned bounded milestone without stopping after every narrow substep.

Agents may continue autonomously until one of these stop conditions is reached:
1. the milestone is achieved
2. a real blocker is reached that cannot be resolved with the current repository context
3. the next change would cross a forbidden boundary
4. the next action would require higher-risk approval outside the configured autonomy settings

### Expected behavior
- choose the smallest sequence of steps that advances the current milestone
- validate after meaningful changes
- use repository skills when they apply
- use low-cost subagents for bounded implementation or research work when runtime policy allows
- keep the main session focused on higher-level reasoning, arbitration, boundary checks, and final integration
- update continuity docs when project state changes materially
- checkpoint with a commit when a meaningful milestone or sub-milestone is reached

### Do not stop merely to report incremental progress
Agents should prefer completing the current bounded milestone over pausing after each tiny fix.

### Continuity reminder
After meaningful state changes, update:
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`

## Cost-Efficient Model Policy

This repository should optimize for low usage and long continuity.

### Default model strategy
- Prefer the cheapest model that is likely to succeed.
- For subagents, default to `gpt-5.4-mini` for implementation, repetitive edits, test updates, scaffolding, documentation, and file-local debugging.
- Escalate to `gpt-5.3` only when the task genuinely requires stronger reasoning.

### Escalation triggers
Use a stronger model only when at least one of these is true:
- architectural ambiguity or boundary conflict
- protocol ambiguity that cannot be resolved from current docs/research
- competing subagent outputs need arbitration
- root-cause debugging spans multiple subsystems and cheaper passes failed
- the task changes project direction, interfaces, or milestone sequencing

### Subagent usage rules
- Prefer one small subagent over several.
- Do not spawn parallel subagents unless the work is clearly independent.
- Do not use stronger models for boilerplate, rote summaries, formatting, or mechanical refactors.
- If a cheap subagent can gather facts and a stronger model only needs to decide, do exactly that.

### Practical model routing
- `gpt-5.4-mini`:
  - repetitive code edits
  - doc updates
  - test fixes
  - scaffolding
  - simple bug fixes
  - narrow protocol comparison work
- `gpt-5.3-codex`:
  - architecture decisions
  - interface changes
  - protocol ambiguity resolution
  - conflict resolution between subagents
  - milestone planning when the next step is unclear

### Failure policy
If a `gpt-5.4-mini` subagent stalls or produces uncertain output:
1. summarize what it found
2. escalate only the unresolved part
3. keep the stronger-model task narrow

## Core Project Principles

1. Compatibility first
2. Stability second
3. Maintainability third
4. Performance fourth
5. Feature breadth last

Additional rules:
- Firestorm is a behavior reference, not an architecture template
- Never copy Firestorm code
- Keep crate boundaries strict
- Prefer small, reversible changes
- Make assumptions explicit
- Do not silently change architecture

---

## Scope Guardrails

### Final Goal
The long-term goal is feature parity with conventional Second Life / OpenSim viewers while replacing legacy architecture with a modern Rust-based design.

### Current Delivery Phase
The current delivery phase is still login compatibility and post-foundation protocol alignment.

Agents must not jump ahead into simulator/world integration until login/bootstrap is credible and documented.

---

## Repository Truth Rule

Agents must treat repository documentation as the primary source of truth, not prior chat memory.

Before making changes, agents must read in this order:

1. `AGENTS.md`
2. `docs/MASTER_PLAN.md`
3. `docs/CURRENT_STATE.md`
4. `docs/HANDOFF.md`
5. `docs/ARCHITECTURE.md`
6. `docs/INTERFACES.md`
7. `docs/FIRESTORM_REFERENCE.md`
6. relevant `docs/RESEARCH/*`
7. then inspect code

Before changing anything, the agent must identify:
- current phase
- immediate objective
- boundaries not to break
- smallest correct next milestone

---

## Workflow Discipline (MANDATORY)

For every meaningful change, agents must follow this workflow:

1. Make the code change
2. Validate the change
   - `cargo check`
   - `cargo test`
   - manual run if applicable
3. Commit the change
4. Update `docs/CURRENT_STATE.md`
5. Update `docs/HANDOFF.md`
6. Update `docs/MASTER_PLAN.md` only if a milestone or project state has materially changed

This workflow exists to preserve continuity across agents and across time.

### Continuity Rule
Agents must assume:
- future agents may have zero useful chat context
- repository docs are the only reliable continuity mechanism

Therefore:
- always update `CURRENT_STATE.md` and `HANDOFF.md` after meaningful progress
- keep changes small, explicit, and well-documented
- never rely on memory of previous interactions

Failure to follow this workflow is a continuity failure.

---

## Current Technology Direction

Preferred stack unless an ADR explicitly changes it:

- Rust
- wgpu
- winit
- egui
- bevy_ecs (standalone only if needed, not full Bevy)
- tokio
- serde
- tracing

Login transport stack currently:
- reqwest
- JSON codec
- LLSD codec (minimal support)
- typed grid adapter boundary

Do not introduce:
- full Bevy engine
- Vulkan-only architecture
- large framework dependencies without a documented reason

---

## Crate Boundaries

### `viewer_app`
Owns:
- application startup
- event loop wiring
- window orchestration
- camera/input wiring
- scene wiring

Must not own:
- protocol semantics
- rendering internals

### `viewer_core`
Owns:
- camera
- transforms
- scene model
- renderable instance descriptions
- shared domain types

### `viewer_render`
Owns:
- GPU initialization
- render pipelines
- mesh buffers
- depth buffer
- camera uniform handling
- object rendering
- debug spatial rendering

### `viewer_ui`
Owns:
- egui integration
- debug UI
- display-only diagnostics

Must not own:
- protocol logic
- business logic

### `viewer_net`
Owns:
- connection lifecycle
- session state
- HTTP transport
- redirect following
- codec selection
- login diagnostics and trace collection

Must not own:
- grid-specific meaning

### `viewer_grid`
Owns:
- request shaping
- response interpretation
- login result classification
- session/bootstrap typing
- grid-specific policy

Must not own:
- transport

### `viewer_asset`
Reserved for:
- asset fetching
- caching
- texture/mesh asset pipeline

### `viewer_platform`
Reserved for:
- OS/platform integration concerns

---

## Firestorm Policy

Firestorm source is reference-only.

Use Firestorm for:
- protocol behavior
- login/session behavior
- capability/bootstrap sequence
- compatibility research
- edge-case understanding

Do not use Firestorm for:
- project structure
- code reuse
- architecture
- dependency choices
- rendering design

When using Firestorm:
1. identify behavior
2. document findings in `docs/RESEARCH`
3. translate that behavior into clean Rust abstractions

---

## Agent Roles

### Architect Agent
Responsible for:
- module boundaries
- system interactions
- data flow proposals
- risks and tradeoffs
- keeping implementation aligned with the brief and plan

Must not:
- silently redesign the project
- dump large implementation changes without being asked

### Scaffolding Agent
Responsible for:
- folder structures
- starter files
- crate/module skeletons
- interface placeholders
- minimal templates

Must:
- scaffold only what is needed
- avoid overbuilding

### Implementation Agent
Responsible for:
- focused production code for a defined task
- small, testable units
- preserving architecture
- readable and maintainable code

Must:
- work from a clearly defined task
- avoid unrelated changes
- note assumptions and unfinished areas

### Refactor Agent
Responsible for:
- reducing duplication
- improving readability
- preparing systems for future work
- improving separation of concerns

Must:
- preserve behavior unless explicitly told otherwise
- keep changes scoped

### Testing Agent
Responsible for:
- unit tests
- integration test suggestions
- smoke tests
- validating assumptions

Must:
- focus on meaningful coverage
- avoid low-value tests

### Documentation Agent
Responsible for:
- updating markdown docs
- summarizing decisions
- documenting boundaries
- maintaining continuity files

Must:
- reflect actual project state
- avoid vague claims

### Research Agent
Responsible for:
- library comparisons
- protocol research
- compatibility investigations
- implementation reference gathering

Must:
- separate facts from recommendations
- highlight uncertainty
- avoid making final decisions unless asked

---

## Optional Specialized Agents

These can be used as needed:

- Rendering Agent
- Networking Agent
- Asset Pipeline Agent
- UI Agent
- Performance Agent

Use them only when it helps. Do not create complexity for its own sake.

---

## Skills Usage Policy

Agents must prefer repository skills for repeatable workflows instead of re-prompting from scratch.

### Skill discovery
Check repository skills before starting non-trivial work.

## Skills Policy

Before starting medium or large work:
1. check whether a repository skill applies
2. use the skill explicitly when the match is clear
3. only proceed without a skill if no existing skill fits

Primary repo skills live in:
- `.agents/skills/viewer/`

If the `viewer/` skill directory is not present, use:
- `.agents/skills/`

Expected core skills:
- plan-next-step
- implement-feature
- protocol-alignment
- debug-trace-analysis
- firestorm-research
- doc-update
- validate-and-commit

### When to use skills
Use a skill when:
- the workflow is repeatable
- the task matches a skill description
- the skill reduces prompt length or context load
- the skill encodes project-specific rules or boundaries

### Continuity rule
If a repeated workflow appears more than once, create or update a skill for it.

## Agent Operating Rules

### Rule 1: Stay in Scope
Only perform the task assigned. If a task pressures architecture changes, flag that explicitly.

### Rule 2: Make Assumptions Visible
If something is unclear, state assumptions. Do not bury them in code.

### Rule 3: Prefer Small Reviewable Outputs
Smaller, checkable outputs are better than large opaque dumps.

### Rule 4: Respect Project Documents
All work must align with:
- `docs/PROJECT_BRIEF.md`
- `docs/ROADMAP.md`
- `docs/MASTER_PLAN.md`
- `docs/CURRENT_STATE.md`
- `docs/HANDOFF.md`
- `AGENTS.md`

### Rule 5: Do Not Overengineer Early
Prefer the simplest structure that supports the current phase.

### Rule 6: Separate Proposal from Implementation
State clearly whether you are:
- proposing
- scaffolding
- implementing
- refactoring
- documenting
- validating

---

## Standard Task Format

When assigning work to an agent, use this structure where possible:

### Task
Short description of the work.

### Goal
What success looks like.

### Inputs
Relevant files, modules, constraints, or docs.

### Output Required
What the agent should produce.

### Boundaries
What must not change.

### Notes
Assumptions, roadmap phase, compatibility concerns, or follow-up context.

---

## Handoff Rules

When one agent hands off to another, include:
- what was completed
- what assumptions were made
- what remains unfinished
- what files changed
- what risks or unknowns remain

---

## Review Expectations

Review agent output for:
- project alignment
- scope control
- readability
- maintainability
- unnecessary complexity
- hidden architecture changes

Human review remains required for major technical direction.

---

## Current Project Rule

The current next step is:

**continue login compatibility alignment until the real LLSD login endpoint recognizes credentials correctly**

Do not move to simulator/bootstrap yet unless login compatibility is working or rigorously characterized and documented.
