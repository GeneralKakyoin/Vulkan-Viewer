# LEARNINGS.md

A living record of durable engineering wisdom accumulated across agent sessions.
Every entry here represents something that **would have changed a plan if known upfront** —
a failed approach, a non-obvious behavior, a hard-won constraint, or a pattern that took
more than one session to establish correctly.

**Decision tree for where learnings go:**
- **This file** — durable engineering wisdom, non-obvious constraints, repeated failure patterns
- **`ARCHITECTURE.md`** — things about the finished system's contracts and invariants (not just wisdom)
- **Task plan files** — milestone-specific notes that apply only to that task

**When to write here:**
- When a user correction or clarification reveals a gap that future plans should not repeat
- When an approach failed and the reason is non-obvious from the codebase
- When a behavior surprised you during implementation or live testing
- After completing any milestone: run through your assumptions and flag which ones were wrong

**When to read here:**
- Before planning any task — validate that your plan doesn't repeat known failures
- During post-implementation review — check your work against each entry

---

## Learnings Decision Tree (Quick Reference)

| Learning type | Goes in |
|---|---|
| Durable wisdom, repeated failure pattern | `LEARNINGS.md` |
| System contract or runtime invariant | `ARCHITECTURE.md` |
| Task-specific note, one-off tradeoff | Task plan file |
| Protocol behavior observed from Firestorm | `docs/RESEARCH/<topic>.md` |

---

## L01 — LLSD login wire format is not reliably accepted; XML-RPC is the safe fallback

**Category:** Protocol / `viewer_net`

**Learned when:** First live SL login attempts using LLSD codec failed despite correct field shapes.

**What happened:** The Second Life login endpoint claims to accept LLSD but in practice rejects it with no meaningful error under certain conditions. Firestorm sends XML-RPC by default. Switching to XML-RPC with a single-step fallback resolved this.

**Rule for future plans:** Any plan touching the login codec path must account for this. Do not assume LLSD is sufficient just because the endpoint advertises it. The LLSD → XML-RPC fallback in `viewer_net` must remain in place.

**Files affected:** `viewer_net` codec selection path.

---

## L02 — `$1$<md5>` credential normalization is required for SL login compatibility

**Category:** Protocol / `viewer_grid`

**Learned when:** Login requests with raw passwords were rejected even when all other fields were correct.

**What happened:** The SL login endpoint expects the password field in Firestorm's `$1$<md5>` format, not a plaintext password. This is not documented in any public API spec but is explicit in Firestorm's login code.

**Rule for future plans:** Any plan that touches `viewer_grid::SecondLifeAdapter` or the login request shape must preserve this normalization. Do not ever pass a raw password string.

**Files affected:** `viewer_grid` login adapter.

---

## L03 — Seam-owned roles must only be created via `apply_world_object_ingestion_seam`

**Category:** Architecture / `viewer_core`

**Learned when:** Early integration attempts created scene roles in the snapshot apply path. This caused roles to persist after seam state changed, because the seam apply path controls the remove lifecycle.

**What happened:** When roles are created outside the seam apply path, they have no lifecycle owner — nothing removes them when the condition that created them disappears. The seam apply path is the only code that has a complete view of "what should exist and what shouldn't."

**Rule for future plans:** Never create or remove seam-owned roles in `apply_live_visual_snapshot`, in startup code, or in UI callbacks. The only valid creation/removal path is `Scene::apply_world_object_ingestion_seam(...)`.

**Files affected:** `viewer_core::Scene`.

---

## L04 — Dirty-only apply is load-bearing, not just an optimization

**Category:** Architecture / `viewer_app`

**Learned when:** Removing the dirty-check on seam/snapshot apply caused subtle rendering artifacts and extra per-frame CPU cost in the adapter path.

**What happened:** The `WorldObjectIngestionAdapter` allocation runs every frame and is not free. Without the dirty check, scene state is reset and re-derived every frame from scratch, which (a) loses any accumulation that the scene was doing internally and (b) causes render flicker on roles that should be stable.

**Rule for future plans:** Never remove the dirty-only guards on seam apply and snapshot apply in `viewer_app`. Any plan that changes the scene update path must explicitly preserve the dirty semantics or document why they're being changed.

**Files affected:** `viewer_app` frame loop, `viewer_core::Scene`.

---

## L05 — `Unknown` traffic in post-AMC diagnostics is intentional signal, not noise

**Category:** Protocol / `viewer_net`

**Learned when:** An early attempt to "clean up" the post-AMC diagnostics by mapping all observed packet IDs to named kinds removed the Unknown bucket. This broke the diagnostic value of the tool.

**What happened:** The `Unknown` bucket is how we detect new traffic patterns. If it's empty, it means everything is typed, which requires live observation evidence. Silently mapping a packet ID to a kind without evidence just hides potential misclassification.

**Rule for future plans:** Do not expand typed LLUDP classification without a live observation record in `docs/RESEARCH/post_amc_bootstrap_boundary_map.md`. Unknown traffic appearing is a signal to investigate, not to suppress.

**Files affected:** `viewer_net` packet classification.

---

## L06 — `viewer_net` ↔ `viewer_grid` boundary collapses under schedule pressure

**Category:** Architecture

**Learned when:** Multiple sessions have shown that when under pressure to get a feature working quickly, logic that interprets what a capability *means* or what a response *implies* ends up in `viewer_net` rather than `viewer_grid`.

**What happened:** The boundary between "send/receive bytes" and "interpret meaning" is conceptually clear but under implementation pressure the temptation is to handle interpretation where the bytes already are. This is the most critical architectural line in the project.

**Rule for future plans:** Any plan that touches both `viewer_net` and `viewer_grid` must explicitly name which files belong to which side of the boundary. If a plan does not do this, reject it and ask for that clarity before implementing.

**Files affected:** Any plan touching login, capability fetch, or bootstrap decode.

---

## L07 — `LiveVisualSnapshot` must never carry sensitive session data

**Category:** Architecture / Security / `viewer_net`

**Learned when:** Early snapshot designs included session IDs and seed capability URLs for "developer convenience." These would have been serialized to `live_visual_snapshot.json` on disk in the fallback path.

**What happened:** Session IDs and capability URLs are not display fields — they are secrets. The snapshot is a communication channel between the worker and the app, but it also has a file serialization fallback which would expose those values to disk.

**Rule for future plans:** The `LiveVisualSnapshot` type must never gain fields for: session ID, secret, seed capability URL, circuit code, or any credential. Display-relevant derived data (e.g. "connected: yes/no", "region name") is fine. Raw secrets are not.

**Files affected:** `viewer_core::LiveVisualSnapshot`, `viewer_net` worker emit path.

---

## L08 — Depth texture must be recreated on resize, not on every frame

**Category:** Rendering / `viewer_render`

**Learned when:** An early renderer attempt recreated the depth texture each frame "to be safe." This caused a GPU memory leak and significant frame-time overhead.

**What happened:** `wgpu` depth textures are GPU resources. Creating one per frame allocates a new resource without releasing the last one until the GPU finishes using it. The correct behavior is: create once at startup, recreate only when the window surface dimensions change.

**Rule for future plans:** Any rendering change that touches the depth texture must only recreate it in the resize handler, never in the frame render function.

**Files affected:** `viewer_render` renderer, depth texture lifecycle.

---

## L09 — Render pipelines are expensive and must be created at startup

**Category:** Rendering / `viewer_render`

**Learned when:** A convenience approach during early MeshKind expansion created pipelines lazily on first use. This caused frame hitches when new mesh kinds appeared in the scene for the first time.

**What happened:** `wgpu` pipeline creation involves shader compilation and can take 10–100ms. Any plan that creates pipelines after startup will produce visible hitches.

**Rule for future plans:** All render pipelines must be created in the renderer initialization path. When adding a new `MeshKind`, add its pipeline at startup alongside the existing ones — never defer it to first use.

**Files affected:** `viewer_render` initialization.

---

## L10 — LLUDP packet IDs must come from the message template, not from memory

**Category:** Protocol / `viewer_net`

**Learned when:** An incorrect packet ID (guessed from memory rather than the template) silently misclassified traffic for several sessions before being caught.

**What happened:** Two packets had similar names and IDs that were easy to confuse. Using the wrong ID caused an entire traffic category to be misclassified as Unknown, hiding real handshake packets.

**Rule for future plans:** Every LLUDP message number must be sourced from `reference/firestorm/scripts/messages/message_template.msg`. Never hard-code a message ID from memory. When in doubt, use the `firestorm_research` skill to look it up.

**Files affected:** `viewer_net` packet classification, LLUDP decode.

---

## L11 — Coarse location decode must handle missing ID blocks gracefully

**Category:** Protocol / `viewer_net`

**Learned when:** Live coarse location updates from SL sometimes omit the agent ID block when the coarse count is low. Early code assumed the ID block was always present, causing a decode panic.

**What happened:** The Firestorm-extended coarse payload includes agent IDs, but the original/reduced coarse payload may not. The decode path must treat the ID block as optional and fall back to a deterministic placeholder ID when absent.

**Rule for future plans:** Any plan touching coarse location decode must account for both the reduced payload (no IDs) and the extended payload (with IDs). The self-avatar placeholder must persist using a deterministic fallback when the ID block is absent.

**Files affected:** `viewer_net` coarse location decode, `viewer_app` avatar merge.

---


---

## L12 — Octree must be synchronized via `sync_spatial()` before rendering

**Category:** Architecture / Rendering / `viewer_core`

**Learned when:** Implementing the hierarchical scene graph in Milestone M1.

**What happened:** If `Octree::insert` or `Octree::remove` are called in isolation without a structured synchronization pass, the Octree quickly becomes inconsistent with the scene. Specifically, with a scene graph, moving a parent doesn't immediately update the AABBs of children. The `sync_spatial()` method was introduced to propagate world matrices and update Octree entries in a single, stable pass.

**Rule for future plans:** Always call `scene.sync_spatial()` at the start of the frame (in `viewer_app::redraw`) before passing the scene to the renderer. Do not attempt to update the Octree ad-hoc during individual transform changes unless a very specific use case requires it.

**Files affected:** `viewer_core::Scene`, `viewer_app` frame loop.

---

## L13 — Quaternions must be initialized to Identity [0,0,0,1], never Zero [0,0,0,0]

**Category:** Architecture / Math / `viewer_core`

**Learned when:** Adding the `rotation` field to the `Transform` struct.

**What happened:** Many parts of the codebase were using `Transform` initializers without the new `rotation` field. A common mistake is to zero-initialize it. A zero quaternion is mathematically invalid for rotations and causes `quat_to_mat4` to produce a zero matrix, effectively making objects invisible or infinitely small.

**Rule for future plans:** Always initialize `Transform.rotation` to the identity quaternion `[0.0, 0.0, 0.0, 1.0]`. If a helper function is added for `Transform::default()`, ensure it uses the identity.

**Files affected:** `viewer_core::Transform` and its many initializers.

---

## L14 — Explicit type annotations are required for `gltf` reader iteration

**Category:** Rust / `viewer_asset`

**Learned when:** Resolving persistent build errors in `mesh_loader.rs` during Milestone 2.

**What happened:** The `gltf` crate's reader returned types that were difficult for Rust's type inference to resolve automatically, especially when destructuring or mapping. This led to "type must be known at this point" errors that were not resolved by standard trait imports.

**Rule for future plans:** Any plan involving `gltf` or similar deeply-nested iterator readers must include explicit type annotations for variables receiving the output (e.g., `let (document, buffers, images): (gltf::Document, Vec<gltf::buffer::Data>, Vec<gltf::image::Data>) = ...`). This avoids cascading inference failures.

**Files affected:** `viewer_asset::mesh_loader`.

---

## L15 — `LLVolume` generation must preserve SL-compatible face bitmasks

**Category:** Architecture / Rendering / `viewer_core`

**Learned when:** Implementing the procedural volume generator in `llvolume.rs`.

**What happened:** The Second Life protocol identifies faces using specific bitmasks (e.g., `FACE_PATH_BEGIN`, `FACE_INNER_SIDE`). If the generator produces geometry without these IDs attached to submeshes, future material application (M3) and texture-matrix transforms (M3) will be unable to target specifically-textured faces (like the "inner" side of a hollowed tube).

**Rule for future plans:** All procedural geometry generators must emit `SubMesh` units that map to these canonical SL face IDs. Never merge distinct SL faces into a single mesh without preserving the ID-based look-up path.

**Files affected:** `viewer_core::geometry::llvolume`.

---

## L16 — `white_view` fallback for empty texture IDs ensures valid bind groups

**Category:** Rendering / `viewer_render`

**Learned when:** Implementing Milestone A06 (Texture & Material Integration).

**What happened:** When a `MaterialDescriptor` has an empty `AssetID` for a texture slot, the renderer must still provide a valid `wgpu::TextureView` to the bind group. Using the loading (yellow) or missing (magenta) views for "legally empty" slots (like an unassigned normal map) would produce visual noise. An opaque white 1x1 texture is used instead to ensure the shader's multiplication (`base_color * mesh_color`) remains identity when no texture is intended.

**Rule for future plans:** Always use `white_view` as the fallback for empty or unassigned texture slots in `create_material_bind_group`. Reserve `loading_view` and `missing_view` for cases where an ID is present but the asset is not yet available.

**Files affected:** `viewer_render::RenderBackend`.

---

## L17 — Attachments that are derived from app-local avatar samples fit better as seam payloads than snapshot fields

**Category:** Architecture / `viewer_core` / `viewer_app`

**Learned when:** Implementing R08 attachment proxies on top of the existing avatar sample path.

**What happened:** The attachment data was derived from the app's current avatar sample list, not from the live worker snapshot itself. Extending `WorldObjectIngestionItem` would have forced every snapshot lane constructor to grow new fields even though the payload was orthogonal to the existing snapshot decode data.

**Rule for future plans:** When a bounded visual payload is derived from app-side state rather than the live snapshot, prefer a seam-side payload collection on `WorldObjectIngestionSeam` and keep the snapshot item constructors unchanged unless the worker truly owns the new data.

**Files affected:** `viewer_core::WorldObjectIngestionSeam`, `viewer_app` avatar-to-seam mapping.

---

## L18 — Egui window visibility state must be updated outside the window closure

**Category:** Rust / `viewer_ui`

**Learned when:** Implementing U09 visibility toggles for Diagnostics and Social windows.

**What happened:** Attempting to update `self.show_diagnostics = open_state` inside the `egui::Window::show` closure caused a borrow checker error because the closure borrows `self` (or the builder borrows it) and you cannot mutably borrow `self` while the closure is active.

**Rule for future plans:** Use a local mutable boolean for the `open` parameter, and assign its value back to the `self` field *after* the window's `show` call has completed.

**Files affected:** `viewer_ui/src/lib.rs`.

---

## L19 — Priority-based cache eviction requires deterministic tie-breaking for stability

**Category:** Architecture / Asset / `viewer_asset`

**Learned when:** Refactoring `FixtureTextureCache` from LRU to a priority-metadata queue in Milestone A10.

**What happened:** When multiple assets have the same priority (e.g., all `Normal` or all `Active`), a non-deterministic eviction choice can cause "asset thrashing" where the same set of textures are repeatedly loaded and evicted every frame. By strictly ordering by `(priority, last_touched_tick, asset_id)`, we ensure that if someone must be evicted, it's always the same candidate until state changes, providing frame-to-frame stability.

**Rule for future plans:** Any cache eviction policy must include a deterministic fallback (like `AssetID` or a creation-sequencer) to break ties between items of equal priority or age.

**Files affected:** `viewer_asset::texture_fixture`.
---
 
 ---
 
## L21 — Fragment fog should use explicit camera-to-world distance, not `clip_position.w`
 
 **Category:** Rendering / `viewer_render`
 
 **Learned when:** Implementing Milestone R12 (Environment and Atmospheric Baseline).
 
**What happened:** The initial R12 implementation used `clip_position.w` in the fragment path as a fog-depth proxy. In practice, this produced weak/incorrect fog behavior against baseline start/end defaults and made tuning less predictable. Switching to `distance(in.world_pos, camera.camera_position.xyz)` restored deterministic, physically-intuitive fog control while remaining bounded and inexpensive.
 
**Rule for future plans:** For bounded distance fog in this codebase, prefer explicit camera-to-world distance in shader math. Treat `clip_position.w` as an optimization candidate only if validated with render tests and default-environment visual checks.
 
 **Files affected:** `viewer_render` SCENE_SHADER WGSL.

---

## L22 — Screenshot smoke is only valid after manual image review

**Category:** Validation / Process

**Learned when:** A13 completion review and screenshot-smoke verification.

**What happened:** A `cargo run` screenshot smoke can succeed while still producing visually incorrect output (wrong camera framing, fallback colors, or obvious scene corruption). Treating command success as complete visual validation risks shipping regressions.

**Rule for future plans:** Any validation step that captures screenshots must include manual review of at least one produced image and must report the reviewed file path plus a one-line visual verdict in handoff/report artifacts.

**Files affected:** `docs/TESTING_REFERENCE.md`, milestone reports/handoffs that claim visual verification.

## L23 — Firestorm parity for `EnableSimulator` retarget alone is insufficient; object-feed ingress can still remain zero

**Category:** Protocol / `viewer_net` + `viewer_app`

**Learned when:** Running repeated Firestorm-vs-viewer protocol comparisons on March 29, 2026.

**What happened:** Firestorm logs showed high `ObjectUpdate*` traffic while our viewer remained at `object_updates=0`. Adjusting `EnableSimulator` retarget behavior did not, by itself, unblock object-feed ingress. This means the blocker is deeper than simple event-queue retarget gating and requires packet-path parity validation (classification, socket/circuit behavior, or post-login receive pipeline) with concrete capture evidence.

**Rule for future plans:** Do not treat `EnableSimulator` retarget changes as a sufficient fix for missing world updates. Any proposed fix must be validated by a capture showing non-zero `object_updates` in the viewer log, not just successful login/retarget diagnostics.

**Files affected:** `viewer_app` runtime handoff/retarget policy, `viewer_net` first-sim receive and classification path, `tools/firestorm/compare_proto_logs.ps1` workflow.

---

## L24 — The first social circuit must preserve the probe socket if startup object traffic begins immediately after `AgentMovementComplete`

**Category:** Protocol / `viewer_net`

**Learned when:** Investigating why the viewer received ongoing simulator traffic but still reported `object_updates=0` during March 29, 2026 live comparisons.

**What happened:** The initial handshake probe used one UDP socket, then `open_social_circuit()` bound a different socket and re-sent `UseCircuitCode` / `CompleteAgentMovement`. Firestorm captures showed the simulator emits dense `ObjectUpdate*` traffic immediately after `AgentMovementComplete`, so dropping the first socket could discard the startup object burst before the long-lived receive path began.

**Rule for future plans:** Any plan that separates first-simulator handshake probing from long-lived UDP receive must preserve socket continuity across that boundary or explicitly justify why startup object traffic cannot be lost. Do not assume a second handshake on a new socket is harmless during initial world entry.

**Files affected:** `viewer_net` first-simulator probe and social-circuit socket lifecycle.

---

## L25 — Socket continuity can restore a completed startup handshake without restoring object-feed ingress

**Category:** Protocol / `viewer_net` + `viewer_app`

**Learned when:** Running the March 29, 2026 connected verification after implementing retained-socket handoff from the first-simulator probe into `open_social_circuit()`.

**What happened:** The live run reached `handshake_complete=true` with retained-socket traffic observations, but `object_feed` counters remained `update_messages=0 total_objects=0` and `region_handshake_updates=0`. That means socket continuity fixed a real protocol defect, but object-feed ingress still depends on an additional parity gap after startup handshake continuity is restored.

**Rule for future plans:** Do not assume that preserving the first-simulator socket is the final fix for missing world objects. After socket continuity is in place, the next debugging step must capture the current post-`AgentMovementComplete` packet mix and compare it against Firestorm before changing more transport behavior.

**Files affected:** `viewer_net` startup receive path, `viewer_app` live verification/relay diagnostics.

---

## L26 — Nearby chat polling in the live worker must reuse the active social circuit

**Category:** Protocol / `viewer_net` / `viewer_app`

**Learned when:** Live startup debugging after the retained-socket fix still showed zero object ingress.

**What happened:** The worker loop was calling `poll_nearby_chat_udp(...)`, which opened a fresh handshaked UDP socket on every tick. That silently reintroduced first-simulator socket churn inside steady-state operation.

**Rule for future plans:** Any continuous live-worker traffic that talks to the first simulator must reuse the active `SocialCircuit` unless there is explicit protocol evidence that a separate socket is required. Do not call fresh-socket handshake helpers inside steady-state polling loops.

---

## L27 — LLUDP reliability replies cannot be approximated casually

**Category:** Protocol / `viewer_net`

**Learned when:** A bounded attempt to add naive `PacketAck`/`CompletePingCheck` handling regressed live startup from `handshake_complete=true` to `handshake_complete=false`.

**What happened:** The live regression showed that LLUDP reliability behavior is more delicate than copying a few flags and message IDs. Without a protocol-accurate design, even well-intentioned ack/ping reply code can break the first-simulator handshake.

**Rule for future plans:** Do not implement LLUDP reliability/ack behavior from message-template snippets alone. Use stronger upstream/protocol evidence and verify against live handshake behavior before keeping any such change.

---

## L28 — Texture capability URLs must be treated as ordered candidates, not one exact string

**Category:** Asset / `viewer_grid` + `viewer_net` + `viewer_app`

**Learned when:** Investigating why the bounded live texture bridge still had no reliable proof of a fetched scene texture on March 29, 2026.

**What happened:** The scene-texture path built a single exact capability URL and fetched it once, while Firestorm’s texture path appends `/?texture_id=...` and our profile-image path already relied on bounded URL fallbacks. That let scene textures and profile images drift apart and left `GetTexture` sensitive to a slash/no-slash mismatch.

**Rule for future plans:** Any plan touching `GetTexture` or `ViewerAsset` must keep ordered candidate URL generation in `viewer_grid` and route both scene textures and profile images through one shared `viewer_net` fetch helper. Do not let those paths maintain separate URL-shaping logic.

**Files affected:** `viewer_grid::AssetCapabilityPolicy`, `viewer_net` texture HTTP helpers, `viewer_app` live texture request path.

---

## L29 — Recurring `AgentUpdate` cadence alone does not unblock object ingress once startup parity is already present

**Category:** Protocol / `viewer_net` + `viewer_app`

**Learned when:** Running the March 29, 2026 connected verification after adding bounded recurring `AgentUpdate` keepalives on the active retained social circuit.

**What happened:** The viewer already had retained-socket continuity, startup drain, single-social-socket discipline, and startup `RegionHandshakeReply` / `AgentThrottle` / reliable `AgentUpdate`. Adding recurring non-reliable `AgentUpdate` keepalives did not change the live packet mix: the capture still showed the same seven startup kinds and `object_feed` remained at `update_messages=0 total_objects=0`.

**Rule for future plans:** After startup parity already includes a reliable `AgentUpdate`, do not spend another slice on `AgentUpdate` cadence alone unless new packet-mix evidence justifies it. Shift the next fix toward protocol-accurate first-simulator control/reliability behavior or another specifically evidenced missing control path.

**Files affected:** `viewer_net` first-simulator send helpers, `viewer_app` live-worker scheduling, connected RCA artifacts.

---

## L30 — ACK-trailer parity alone does not restore object ingress once the retained-socket baseline is already in place

**Category:** Protocol / `viewer_net`

**Learned when:** Running the March 29, 2026 connected verification after implementing bounded first-simulator ACK-trailer parity on top of retained-socket continuity, startup interest sends, and recurring `AgentUpdate` cadence.

**What happened:** The transport layer began collecting reliable inbound packet IDs, appending Firestorm-style ACK trailers to outbound first-simulator datagrams, and decoding inbound bodies without trailer corruption. Validation passed, but the bounded live run still showed `update_messages=0`, `total_objects=0`, `region_handshake_updates=0`, and the same seven startup packet kinds while `handshake_complete=true`.

**Rule for future plans:** After the retained-socket and startup-interest baseline is already present, do not assume missing ACK trailers are the final blocker for object ingress. If ACK-trailer parity is in place and live ingress is still zero, move the next slice to a different evidenced post-`AgentMovementComplete` control/parity gap rather than revisiting the same reliability surface.

**Files affected:** `viewer_net` first-simulator transport path, connected RCA artifacts, follow-up object-ingress planning.

---

## L31 — Packet captures can falsify code-level assumptions about first-simulator socket continuity

**Category:** Protocol / `viewer_net`

**Learned when:** Preserving and decoding the March 30, 2026 Firestorm and app pcap artifacts for the blocked object-ingress path.

**What happened:** Code inspection and unit coverage suggested retained-socket continuity was already repaired, but the preserved app capture still showed simulator traffic on an additional local UDP port during the same capture window while the handshake/control flow on another local port never received the matching object burst. The Firestorm capture, by contrast, showed a single coherent local-port flow immediately before `ObjectUpdateCached` ingress.

**Rule for future plans:** When first-simulator continuity is in doubt, do not rely only on code structure, unit tests, or bounded app logs. Preserve packet captures, compare local-port usage directly, and let runtime wire evidence decide whether socket continuity is truly fixed before implementing more message parity.

**Files affected:** first-simulator transport debugging, pcap preservation artifacts, follow-up object-ingress planning.

---

## L32 — Once live diagnostics prove one-port continuity, socket reuse should stop being treated as the primary blocker

**Category:** Protocol / `viewer_net` + `viewer_app`

**Learned when:** Running the March 30, 2026 connected validation after implementing bounded first-simulator socket diagnostics and worker relay summaries.

**What happened:** The live run showed one local UDP port (`65241`) across probe, social-circuit open, startup prime, and the first steady-state polling window, all with `split=false`. The viewer still received ongoing same-port simulator traffic, but `object_feed` remained at `update_messages=0 total_objects=0`.

**Rule for future plans:** When live diagnostics show one shared first-simulator local port end to end, do not keep spending slices on socket continuity alone. Move the next plan to message/control parity or inbound classification on that same port unless newer wire evidence contradicts the diagnostics.

**Files affected:** `viewer_net` runtime socket diagnostics, `viewer_app` relay summaries, object-ingress follow-up planning.

---

## L33 — Received same-port simulator traffic can still be the wrong class of data for object ingress

**Category:** Protocol / `viewer_net`

**Learned when:** Inspecting the March 30, 2026 `improved.pcapng` capture after live socket diagnostics had already proven one-port continuity.

**What happened:** The viewer app was clearly receiving real simulator traffic on the correct first-simulator port, but the dominant inbound packets were `LayerData`, `CoarseLocationUpdate`, `AttachedSound`, `ViewerEffect`, `TestMessage`, and related control/social traffic rather than `ObjectUpdate*`. That means "we are receiving data from the simulator" is not enough to claim object ingress is working.

**Rule for future plans:** When debugging missing world objects, distinguish receipt of any simulator traffic from receipt of the specific `ObjectUpdate*` family. Treat `LayerData` and other same-port traffic as important observability signal, but do not count it as object-feed success or use it to prematurely shift the investigation toward HTTP asset fetches.

**Files affected:** first-simulator traffic classification, object-ingress follow-up planning, pcap-driven diagnostics.

---

## L34 — Startup `MuteListRequest`, `MoneyBalanceRequest`, and `AgentDataUpdateRequest` parity does not by itself restore object ingress

**Category:** Protocol / `viewer_net` + `viewer_app`

**Learned when:** Running the March 30, 2026 connected validation after adding the Firestorm-evidenced startup request subset on the already-confirmed one-port first-simulator path.

**What happened:** The live run showed the viewer sending `MuteListRequest` (`262`), `MoneyBalanceRequest` (`313`), and `AgentDataUpdateRequest` (`386`) on the retained simulator port, but `object_feed` still remained at `update_messages=0 total_objects=0 region_handshake_updates=0`.

**Rule for future plans:** Once the one-port path is proven and this startup request subset is on-wire, do not spend another slice re-tuning the same requests. Move the next plan to a later control/reply gap such as `SetAlwaysRun`, `AgentAnimation`, or a deeper receive-path comparison.

**Files affected:** `viewer_net` startup request helpers, `viewer_app` startup prime path, object-ingress follow-up planning.

---

## L35 — `AgentHeightWidth` alone does not restore object ingress on the current one-port path

**Category:** Protocol / `viewer_net`

**Learned when:** Running the March 30, 2026 connected validation after promoting `AgentHeightWidth` as the first staged control-block packet from the newer Firestorm `Test.pcapng` evidence.

**What happened:** The viewer stayed on one retained first-simulator port and successfully inserted `AgentHeightWidth` into the startup control sequence, but `object_feed` still remained at `update_messages=0 total_objects=0 region_handshake_updates=0`.

**Rule for future plans:** Once `AgentHeightWidth` is on-wire and live ingress is still zero, do not spend another slice re-tuning the same packet or its static dimensions alone. Promote the next smallest later control-block candidate or a tighter ACK-timing slice instead.

**Files affected:** `viewer_net` startup control helpers, object-ingress staged control-block planning.

---

## L36 — `SetAlwaysRun` alone does not restore object ingress on the current one-port path

**Category:** Protocol / `viewer_net`

**Learned when:** Running the March 30, 2026 connected validation after promoting `SetAlwaysRun` as the next staged control-block packet following `AgentHeightWidth`.

**What happened:** The viewer stayed on one retained first-simulator port and successfully inserted `SetAlwaysRun` into the startup control sequence, but `object_feed` still remained at `update_messages=0 total_objects=0 region_handshake_updates=0`.

**Rule for future plans:** Once `SetAlwaysRun` is on-wire and live ingress is still zero, do not spend another slice re-tuning that packet alone. Move the next staged promotion to `AgentAnimation` or to a tighter ACK-timing investigation.

**Files affected:** `viewer_net` startup control helpers, object-ingress staged control-block planning.

---

## L37 — The observed startup `AgentAnimation` packet alone does not restore object ingress on the current one-port path

**Category:** Protocol / `viewer_net`

**Learned when:** Running the March 30, 2026 connected validation after promoting the observed Firestorm startup `AgentAnimation` packet following `AgentHeightWidth` and `SetAlwaysRun`.

**What happened:** The viewer stayed on one retained first-simulator port and inserted the observed startup `AgentAnimation` packet into the startup control sequence, but `object_feed` still remained at `update_messages=0 total_objects=0 region_handshake_updates=0`.

**Rule for future plans:** Once the observed startup `AgentAnimation` packet is on-wire and live ingress is still zero, stop promoting more standalone startup control messages. Move the next plan to ACK/control-reply behavior instead.

**Files affected:** `viewer_net` startup control helpers, object-ingress staged control-block planning.

---
