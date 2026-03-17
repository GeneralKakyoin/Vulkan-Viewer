# AGENTS.md

## Project
This repository is a from-scratch Rust rewrite of a Second Life / OpenSim viewer.

Primary priorities, in order:
1. Compatibility
2. Stability
3. Maintainability
4. Performance
5. Feature breadth

This is a networked virtual-world viewer, not a game engine.

## Stack defaults
Prefer these unless an ADR says otherwise:
- Rust
- wgpu
- winit
- egui
- bevy_ecs (standalone only, not full Bevy)
- tokio
- serde
- tracing

Firestorm code is for reference only.

Agents must:
- NOT copy Firestorm code directly
- NOT replicate its architecture
- extract behavior, not implementation
- design clean Rust equivalents

Do not introduce:
- full Bevy engine
- Vulkan-only architecture
- large framework dependencies without a documented reason



## Core architecture rules
Keep boundaries strict:
- renderer must not depend on grid-specific logic
- grid logic must not depend on renderer internals
- UI must not contain protocol or asset logic
- networking must remain testable in isolation
- asset/cache logic must be separable from rendering

Preferred crate layout:
- viewer_app
- viewer_core
- viewer_render
- viewer_ui
- viewer_net
- viewer_grid
- viewer_asset
- viewer_platform

## Documentation rules
Treat docs as product assets.

Core docs:
- docs/PROJECT_BRIEF.md
- docs/ARCHITECTURE.md
- docs/ROADMAP.md

Decisions:
- docs/DECISIONS/000x-name.md

Research notes:
- docs/RESEARCH/

Do not mix rough notes into source-of-truth docs.

When architecture, boundaries, or workflow assumptions change, update docs in the same task.

## Build and validation expectations
Before finishing a coding task:
- ensure the code builds
- run the narrowest relevant checks first
- avoid expensive full-repo validation unless the task needs it

Typical commands:
- cargo fmt
- cargo check
- cargo test

Prefer minimal validation for small, localized changes.

## Change policy
Prefer:
- small, reversible changes
- narrow diffs
- explicit interfaces
- incremental progress

Avoid:
- speculative rewrites
- broad refactors unrelated to the task
- hidden coupling
- engine-style global state

## Subagent policy
Use subagents only when delegation is explicitly allowed by the task or user instruction.

For this repository, if the prompt says or implies any of the following, delegation is allowed and encouraged:
- "use subagents"
- "parallelize"
- "delegate"
- "split this up"
- "use agents when useful"
- "orchestrate this"

When delegation is allowed, prefer subagents for:
- codebase exploration
- repetitive edits
- narrow file-local implementations
- test triage
- log/error inspection
- documentation drafting
- research summaries
- bounded comparisons

Be cautious with parallel write-heavy tasks that touch the same files.

## Delegation strategy
Main agent responsibilities:
- understand the task
- decide whether delegation is worth the cost
- keep the main thread clean
- assign bounded tasks
- resolve conflicts
- integrate outputs
- make final architectural decisions

Subagent responsibilities:
- execute a narrow task
- return concise results
- avoid unnecessary exploration
- avoid broad refactors
- avoid touching unrelated files

Subagents should return distilled findings, not long raw logs.

## Model selection policy
Optimize for low usage by default.

Use the cheapest model that is likely to succeed.

Recommended model routing:
- easy, repetitive, well-bounded tasks:
  - prefer gpt-5-mini
  - or gpt-5.1-codex-mini for coding-specific delegated tasks
- medium implementation tasks with some repo context:
  - prefer gpt-5.1-codex
- architecture, orchestration, conflict resolution, tricky debugging, and final integration:
  - prefer gpt-5.3-class / strongest available coding model in the current Codex environment
  - if the environment exposes gpt-5.4 as the primary top-tier Codex model, use that instead

Practical rule:
- minions do the bulk work
- strongest model handles planning, arbitration, and merge decisions

Do not spend a top-tier model on rote boilerplate, mechanical edits, or simple summaries.

## Delegation heuristics
Delegate when at least one of these is true:
- the task can be split into independent parts
- the work is noisy and would pollute the main thread
- multiple files or subsystems need inspection
- there is a repetitive mechanical transformation
- architecture review is easier after parallel reconnaissance

Do not delegate when:
- the task is tiny
- the task is highly sequential
- the task centers on one small file
- coordination overhead would exceed the savings

## Preferred subagent roles
Use lightweight, temporary specialist roles such as:
- Scout: explore relevant files and return only the important paths and findings
- Implementer: make a bounded code change in a clearly defined area
- Tester: run targeted validation and summarize failures
- Researcher: compare alternatives and produce a concise recommendation
- Docs: draft or update focused documentation
- Refactorer: perform mechanical renames or local cleanup only
- Resolver: analyze conflicting outputs and present merge options

Create these organically as needed. Do not create agents just to mirror an org chart.

## Subagent task template
When spawning a subagent, give it:
- one clear objective
- explicit scope boundaries
- files or directories to focus on
- success criteria
- model preference based on task difficulty
- instruction to keep output concise

Example expectations:
- "Inspect viewer_render startup only; identify the minimum files to change."
- "Update docs/ROADMAP.md only; do not edit code."
- "Run targeted validation for viewer_net and summarize failures in 5 bullets max."

## Conflict resolution
If multiple subagents disagree:
1. preserve correctness and compatibility first
2. prefer the simpler design
3. prefer the option with cleaner module boundaries
4. document the decision if it is architectural

Escalate to a stronger model for arbitration when needed.

## Repo-specific engineering guidance
This project is expected to evolve through a thin vertical slice:
1. app shell
2. renderer bootstrap
3. UI overlay
4. core scheduling
5. networking/login
6. first world rendering
7. assets/cache
8. compatibility expansion

Do not front-load advanced features before the slice works.

## First milestone
A valid first milestone is:
- app launches
- window opens
- wgpu initializes
- screen clears
- egui overlay renders
- app exits cleanly

No networking or asset loading is required before this milestone.

## Final behavior rules
When in doubt:
- simplify
- isolate
- document
- choose the lower-cost path unless the task genuinely needs stronger reasoning

Keep the main thread focused.
Use minions for bounded work.
Use stronger models for decisions.
