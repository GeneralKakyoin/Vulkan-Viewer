---
trigger: always_on
---

Antigravity IDE Rules — Vulkan-Viewer Supplement
These rules supplement AGENTS.md and are specific to the Vulkan-Viewer Rust workspace. They are written for the Antigravity IDE custom rules field and focus on precision guards the existing rules do not cover at the level agents need to operate reliably.

Rule S1 — Read the right doc first for the task type
Before starting any task, identify the doc category and read accordingly:

Task type	Required reading (in order)
Continuing from previous session	
HANDOFF.md
 → 
CURRENT_STATE.md
 → 
TASKS.md
Rendering change	
ARCHITECTURE.md
 → 
RENDERING_ROADMAP.md
 → relevant RENDERING_PHASE_*.md
Protocol / login / bootstrap change	
INTERFACES.md
 → 
RESEARCH/firestorm_login_flow.md
 or relevant RESEARCH file
Simulator packet / handshake change	
RESEARCH/firestorm_first_simulator_handshake.md
 → 
RESEARCH/post_amc_bootstrap_boundary_map.md
New Firestorm behavior question	Use firestorm_research skill; do not read Firestorm source directly
UI change	
INTERFACES.md
 (viewer_ui boundary) → 
CURRENT_STATE.md
 (what is already built)
Seam / ingestion change	
CURRENT_STATE.md
 (seam lane list) → 
INTERFACES.md
 (seam ownership rule)
Do not substitute memory for doc reading. Docs are the source of truth.

Rule S2 — Crate boundary quick-check (use before writing a single line)
Answer these three questions before writing code:

Which crate owns this? Name it explicitly. If you cannot name it, re-read 
ARCHITECTURE.md
.
What must not change? Name at least one crate that must not be touched by this change.
Is this within the current roadmap? Check 
TASKS.md
 Active Tasks section.
If any answer requires reasoning beyond "it is clearly stated in ARCHITECTURE.md or INTERFACES.md", stop and document why before writing code.

The viewer_net ↔ viewer_grid boundary is the most critical. Repeat:

viewer_net sends/receives and manages session state.
viewer_grid shapes requests and interprets response meaning.
These must never merge.
Rule S3 — Seam ownership is non-negotiable
The ingestion seam is the boundary between live protocol state and rendered scene state.

Rules that must never be broken:

Only Scene::apply_world_object_ingestion_seam(...) creates or removes seam-owned scene roles.
Scene::apply_live_visual_snapshot(...) must not create or remove seam-owned roles.
Seam lanes are owned by viewer_core. Adapting data into lanes belongs in viewer_core (via WorldObjectIngestionAdapter).
viewer_app feeds the adapter each frame and applies the resulting seam. It does not shape lane payloads directly.
If a new decoder field needs to appear in scene space, the path is always: viewer_net decode → LiveVisualSnapshot field → WorldObjectIngestionAdapter → new seam lane → Scene::apply_world_object_ingestion_seam(...).

Rule S5 — wgpu and Rust-specific coding standards
wgpu specifics:

All GPU resources (Buffer, Texture, BindGroup, Pipeline) are owned by viewer_render. Never pass them to other crates.
Pipeline creation is expensive. Pipelines are initialized once and reused. Do not create pipelines per-frame.
Depth texture must be recreated on window resize, not on every frame. Handle resize events explicitly.
Prefer push constants or uniform buffers for per-object data; avoid CPU-side iteration inside render passes.
When adding a new MeshKind, add a corresponding draw path in viewer_render. The mesh-kind mapping table must stay complete.
Rust specifics:

Use #[derive(Debug, Clone, PartialEq)] on all domain model types by default.
Derive Eq and Hash where stable identity comparison is needed (e.g., entity IDs, enum keys).
Prefer Option<T> returns over panics for missing-data cases in decode paths.
Never use unwrap() in decode or handshake paths. Use ? or explicit classification.
Avoid clone() on large state structs inside hot loops. Use dirty-flag patterns (already established in viewer_app).
Prefer u8, u16, u32 explicitly for wire protocol field types. Do not use usize for protocol field widths.
Rule S6 — Test requirements for the current phase
Every meaningful added type, function, lane, role, or adapter requires a focused test.

For this project, "focused test" means:

Decode path: test the decode function in isolation with a known byte sequence.
Seam lane: test presence when prerequisites are met AND absence when prerequisites are missing.
Scene role: test that apply_world_object_ingestion_seam(...) creates the role when the lane is present, and removes it when the seam is empty.
Boundary: test both sides of conditional behavior — never just the happy path.
App dirty-check: if seam or snapshot apply behavior changes, test the dirty-only semantics explicitly.
Do not write tests that duplicate coverage already present.
Do write tests that would catch a future regression of the specific behavior you just added.

Rule S7 — Validation gate checklist
Do not report a task complete until all three pass:

cargo check          # must produce zero errors
cargo test           # must produce zero failures
If a UI or rendering change requires it:

cargo run -p viewer_app   # will time out in CLI because window is interactive — this is expected
If cargo check fails, fix it before running tests.
If cargo test fails on an unrelated test, investigate before reporting complete — do not ignore pre-existing failures.

Rule S8 — Diagnostic-first rendering changes
Rendering changes in the current phase must remain diagnostic-first.

This means:

New scene roles should emit visually readable markers, colors, or scale variations that can be understood from the debug camera position — not invisible or overlapping.
Transform/color choices should reflect the encoded data (e.g., health-basis-point influence on color).
New markers must not overlap the existing cluster center at the same transform with no distinction.
If adding more than 3 new scene roles in a single task, re-read the spatial hierarchy section of 
CURRENT_STATE.md
 to ensure the cluster readability is preserved.
Rule S9 — Protocol constants must be sourced from the message template
LLUDP message numbers and packet IDs must be derived from:

reference/firestorm/scripts/messages/message_template.msg
Do not invent or guess message IDs. If a message number is uncertain, consult the template file or use the firestorm_research skill. Incorrect message IDs will silently misclassify traffic.

Current typed message number reference (partial):

Message	Form	Number
UseCircuitCode	low	3
RegionHandshake	low	148
AgentMovementComplete	low	250
CompleteAgentMovement	low	249
HealthMessage	low	138
SimulatorViewerTimeMessage	low	150
AgentDataUpdate	low	387
CoarseLocationUpdate	medium	6
ViewerEffect	medium	17
CrossedRegion	medium	7
ConfirmEnableSimulator	medium	8
AttachedSound	medium	13
OnlineNotification	low	322
PacketAck	fixed-low	0xFFFB
Rule S10 — Continuity doc update triggers (when to update which doc)
What changed	Update these docs
Task completed or milestone reached	
CURRENT_STATE.md
, 
HANDOFF.md
Task priority changed	
TASKS.md
Crate boundary or interface changed	
INTERFACES.md
New rendering phase started/completed	
RENDERING_ROADMAP.md
, relevant RENDERING_PHASE_*.md
New Firestorm protocol understanding	docs/RESEARCH/<topic>.md
Project milestone shifted	
MASTER_PLAN.md
, 
Roadmap.md
If a change is purely additive and all existing docs still describe the state accurately, no update is needed. When in doubt, update 
HANDOFF.md
 — it is the primary relay doc.

Rule S11 — Output format (required at task completion)
Always report at end:

What changed: files and types modified or added
Crate boundaries respected: which crates changed and why each is correct
Validation performed: cargo check status, cargo test status, test count
Assumptions made: anything inferred that is not explicitly stated in docs
Continuity docs updated: yes/no and which files
Rule S12 — Avatar and social state boundary
Avatar and social state lives in viewer_core::SocialState.

viewer_ui reads from it for display only.
viewer_app writes to it from live connection data (coarse decode → merge → scene application).
viewer_net provides raw coarse decode; it does not own avatar identity or render mode.
viewer_core owns: NearbyPersonEntry, project_nearby_people(...), AvatarRenderMode, DirectImThread lifecycle.
viewer_ui must not reach into viewer_net state directly for avatar data.
Rule S13 — Startup and live-state worker boundary
The in-process live-state worker is owned by viewer_app.

The worker uses viewer_net::Connection internally.
The worker emits only sanitized LiveVisualSnapshot values over in-process channels.
Sensitive session data (session ID, seed capability URL, agent secret) must never be forwarded outside viewer_net or forward-logged to LiveVisualSnapshot.
viewer_app selects startup mode, manages worker status, and maps snapshot to seam/scene.
viewer_core and viewer_ui consume only the sanitized snapshot types — they must not receive live Connection handles.
Rule S14 — Do not remove Unknown classifications from traffic typing
When classifying inbound LLUDP packets, the Unknown classification is intentional and load-bearing.

Do not map an unverified packet ID to a named kind unless it has been observed repeatedly in live bounded probe runs and is explicitly recorded in 
RESEARCH/post_amc_bootstrap_boundary_map.md
.
Expanding typed classification without a live observation record silently hides potential issues.
Unknown traffic appearing in diagnostics is diagnostic signal — do not suppress it.
