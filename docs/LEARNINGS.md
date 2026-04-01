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

## L38 — When bounded forensics show `unclassified=none` while `RegionHandshake` stays absent, the next branch should move to ACK/control timing rather than receive surfacing

**Category:** Protocol / `viewer_net` + `viewer_app`

**Learned when:** Completing the March 30, 2026 ACK/receive forensics slice with bounded live startup transcript summaries.

**What happened:** The new startup and first-steady-state forensic relays showed that raw packet message numbers were already being surfaced and none remained in the unclassified bucket, yet `RegionHandshake` and `ObjectUpdate*` were still absent while pending ACK IDs continued to accumulate. That ruled out receive-path surfacing/classification as the primary next branch on the current path.

**Rule for future plans:** After bounded first-simulator forensics show `unclassified=none` and `RegionHandshake` is still absent, do not spend the next slice on receive classification/surfacing tweaks. Move the next plan to ACK/control timing or another tighter protocol control prerequisite instead.

**Files affected:** object-ingress forensics planning, `viewer_net` startup control/ACK follow-up work, `viewer_app` relay interpretation.

---

## L39 — Explicit ACK flush timing can drain the first-simulator ACK queue without restoring `RegionHandshake` or object ingress

**Category:** Protocol / `viewer_net` + `viewer_app`

**Learned when:** Running the March 30, 2026 bounded live validation after adding an explicit `PacketAck` flush on the active `SocialCircuit`.

**What happened:** The new ACK-flush slice succeeded mechanically: the queue drained to zero and an explicit outbound `PacketAck` was observed on-wire. Even so, the run still showed `region_handshake_updates=0` and `update_messages=0 total_objects=0`.

**Rule for future plans:** Do not assume that draining the pending ACK queue is sufficient to unlock object ingress. If explicit ACK flush timing succeeds mechanically but `RegionHandshake` and `ObjectUpdate*` remain absent, use the newly surfaced packet mix to choose the next clue-driven slice instead of stacking more ACK guesses immediately.

**Files affected:** `viewer_net` ACK/control follow-up work, `viewer_app` live-worker timing, post-ACK object-ingress planning.

---

## L40 — Once cross-protocol diagnostics prove simulator-host `:12043` caps are present and only a one-shot `EventQueueGet` start appears, prioritize persistent EventQueue behavior over more LLUDP startup guesses

**Category:** Protocol / `viewer_net` + `viewer_app`

**Learned when:** Completing the March 30, 2026 parallel-protocol startup investigation using Firestorm source, `Firestorms.pcapng`, and the new bounded live transcript.

**What happened:** The viewer successfully fetched first-region capabilities on simulator-host HTTPS `:12043` and surfaced a single `EventQueueGet:start ack=0 ...` in the bounded run, but still never surfaced `RegionHandshake`, `RegionHandshakeReply`, or `ObjectUpdate*`. Firestorm source showed that the corresponding path is implemented as a persistent `LLEventPoll`, not sparse one-shot probing.

**Rule for future plans:** After diagnostics show the simulator-host capability lane is present and the current viewer only starts a one-shot `EventQueueGet`, stop promoting more standalone LLUDP startup packets first. Move the next branch to persistent EventQueue behavior before widening into other capability families or returning to UDP ordering guesses.

**Files affected:** object-ingress planning, `viewer_net` capability/event-queue follow-up, `viewer_app` live startup diagnostics.

---

## L41 — Persistent EventQueue plus one-shot `EnableSimulator` port follow-up can open real simulator/world-data ingress without restoring LLUDP object updates

**Category:** Protocol / `viewer_net` + `viewer_app`

**Learned when:** Running the March 30, 2026 bounded live validations after preserving nested EventQueue bodies and sending one-shot `UseCircuitCode` follow-up to EventQueue-delivered simulator ports.

**What happened:** The viewer began ingesting structured simulator/world data from the simulator-host EventQueue lane, including nested `ParcelProperties` content and port-only `EnableSimulator` details (`13013`, `13000`, `13001`). The worker then sent one-shot `UseCircuitCode` follow-up to those ports on the current simulator host. Even so, the run still showed `RegionHandshake = none`, `RegionHandshakeReply = none`, and `ObjectUpdate* = none`.

**Rule for future plans:** Once persistent EventQueue is live and one-shot `EnableSimulator` port follow-up is in place, do not spend another slice repeating the same port follow alone. Move the next branch to per-region seed-cap / `EstablishAgentCommunication` evidence or another specific simulator-host prerequisite instead.

**Files affected:** `viewer_net` EventQueue parsing and explicit-target `UseCircuitCode` helpers, `viewer_app` EventQueue follow-up policy, post-EventQueue object-ingress planning.

---

## L42 — Expanding the primary seed-cap request can surface `InterestList`, `RegionObjects`, and `UntrustedSimulatorMessage` without surfacing `EstablishAgentCommunication` or restoring LLUDP object ingress

**Category:** Protocol / `viewer_net` + `viewer_app`

**Learned when:** Running the March 30, 2026 bounded live validation after expanding the primary simulator seed-cap request and adding bounded explicit seed-cap follow-up support.

**What happened:** The primary simulator on `:12043` began returning additional Firestorm-evidenced capabilities including `InterestList`, `RegionObjects`, and `UntrustedSimulatorMessage`. Even so, the same bounded run still did not surface `EstablishAgentCommunication`, and LLUDP object ingress remained at `RegionHandshake = none`, `RegionHandshakeReply = none`, and `ObjectUpdate* = none`.

**Rule for future plans:** Once the primary simulator returns these broader region capabilities, do not keep spending slices merely widening the seed-cap request. Move the next branch to probing the now-proven object-related cap (`RegionObjects`) or another specific read-side simulator-host action.

**Files affected:** primary seed-cap request policy, app-side seed-cap follow-up policy, post-seed-cap object-ingress planning.

---

## L43 — The primary simulator `RegionObjects` capability can provide the first object-related simulator data even while LLUDP `ObjectUpdate*` remains absent

**Category:** Protocol / `viewer_net` + `viewer_app`

**Learned when:** Running the March 30, 2026 bounded live `RegionObjects` capability probe after surfacing that cap on the primary simulator-host `:12043` lane.

**What happened:** The read-only `RegionObjects` probe returned a UUID-keyed top-level map, and the first surfaced UUID keys classified as nested `map` values. At the same time, LLUDP object ingress still remained at `RegionHandshake = none`, `RegionHandshakeReply = none`, and `ObjectUpdate* = none`.

**Rule for future plans:** Once `RegionObjects` is proven to return structured object-related payloads, treat that as a valid opening for “any object data” work and move the next branch to bounded field extraction from those maps. Do not require LLUDP `ObjectUpdate*` parity before acknowledging that the simulator is already sending usable object-related data on another supported path.

**Files affected:** `viewer_net` capability inspection helpers, `viewer_app` one-shot capability probe policy, object-ingress follow-up planning.

---

## L44 — Bounded `RegionObjects` child-map extraction can expose stable inner object fields before any LLUDP object-update parity exists

**Category:** Protocol / `viewer_net` + `viewer_app`

**Learned when:** Running the March 30, 2026 bounded live validation after extending the `RegionObjects` probe to inspect the first UUID-keyed child maps.

**What happened:** The viewer surfaced repeatable inner keys `A`, `B`, `C`, `D`, `can_be_volume`, and `description`, along with repeated scalar values `A=100`, `B=100`, `C=100`, and `D=100`, from the first `RegionObjects` child maps. LLUDP `RegionHandshake` and `ObjectUpdate*` still remained absent.

**Rule for future plans:** Once bounded `RegionObjects` child extraction is live, prefer the next slice to semantic/source-backed interpretation of those fields rather than further transport expansion. The opening is already real; the new problem is understanding it.

**Files affected:** `viewer_net` `RegionObjects` inspection helpers, `viewer_app` `RegionObjects` relay summarization, post-opening object-ingress planning.

---

## L45 — The first proven `RegionObjects` child-map family matches Firestorm pathfinding linkset/object schema, and bool-like `0/1` values must be normalized before deriving semantics

**Category:** Protocol / `viewer_net` + `viewer_app`

**Learned when:** Completing the March 30, 2026 bounded semantic-mapping slice using Firestorm pathfinding references and a live `RegionObjects` capture.

**What happened:** The previously opaque `A/B/C/D`, `can_be_volume`, and `description` keys lined up directly with Firestorm’s `LLPathfindingLinkset` / `LLPathfindingObject` fields. The live payload also encoded booleans as `0/1`, so a naive `true/false` parser mis-derived `linkset_use` until those values were normalized.

**Rule for future plans:** Once `RegionObjects` child maps show this field family, treat them as pathfinding linkset/object payloads unless stronger evidence contradicts it. Normalize bool-like `0/1` fields before deriving semantics such as `linkset_use`, and cite `llpathfindinglinkset.cpp` / `llpathfindingobject.cpp` rather than guessing from field letters.

**Files affected:** `viewer_net` `RegionObjects` semantic helpers, `viewer_app` relay interpretation, post-semantic object-ingress planning.

---

## L46 — Typed `RegionObjects` pathfinding summaries can surface real names and owners, but the live lane may still contain multiple field-shape variants

**Category:** Protocol / `viewer_net` + `viewer_app`

**Learned when:** Completing the March 31, 2026 bounded pathfinding field-promotion slice on top of the existing `RegionObjects` semantic mapping.

**What happened:** The new typed lane successfully surfaced named fields such as `profile`, `linkset_use`, `walkability`, `name`, and `owner`, but the first live records were not fully uniform. Some entries had a normal `(No Description)` value while others surfaced comma-like numeric payloads in `description`, and no separate `position` field appeared in the bounded first-object sample.

**Rule for future plans:** Once typed `RegionObjects` summaries are live, do not assume every UUID-keyed child map is the same canonical pathfinding-object shape. Add bounded variant discrimination or shape hints before treating `description` or missing `position` as settled semantics.

**Files affected:** `viewer_net` typed `RegionObjects` extraction helpers, `viewer_app` typed relay summarization, post-promotion object-ingress planning.

---

## L47 — Live `RegionObjects` linkset records can carry valid LLSD-array positions even when early bounded summaries make them look absent

**Category:** Protocol / `viewer_net` + `viewer_app`

**Learned when:** Completing the March 31, 2026 position-shape clarification slice on top of the existing `RegionObjects` pathfinding lane.

**What happened:** The earlier bounded summaries made the tuple-description records look like `...no_position` variants, but tighter inspection showed those same live records actually carry `position` as LLSD arrays of length 3. The problem was not field absence; it was that the extraction/relay path was not surfacing the position evidence clearly enough.

**Rule for future plans:** When a live `RegionObjects` summary appears to be missing a canonical field, first distinguish “field absent” from “field present but not surfaced/decoded.” Add explicit presence/shape evidence before concluding the schema changed or the field is missing on-wire.

**Files affected:** `viewer_net` `RegionObjects` position inspection helpers, `viewer_app` bounded relay summarization, post-variant object-ingress planning.

---

## L48 — Bounded tuple slot analysis can prove repeatable structure without yet proving semantics

**Category:** Protocol / `viewer_net` + `viewer_app`

**Learned when:** Completing the March 31, 2026 tuple slot-analysis slice on top of the existing typed `RegionObjects` lane.

**What happened:** Once the relay surfaced bounded tuple slot analysis, the live `RegionObjects` tuple records showed a clear repeated six-slot structure with only one varying slot in the current sample. Even so, the current evidence still came from just two same-name records, which is not enough to assign semantic labels safely.

**Rule for future plans:** When a repeated tuple/string content pattern appears, first prove which slots are constant versus varying and preserve the raw values. Do not assign semantic names to tuple slots until the sample widens beyond a narrow same-object slice or a Firestorm reference directly supports the mapping.

**Files affected:** `viewer_net` tuple analysis helpers, `viewer_app` `RegionObjects` relay summarization, post-analysis object-ingress planning.

---

## L49 — Once code-side tuple widening is in place, a still-narrow tuple family usually means the current live content is narrow, not that the extractor is too small

**Category:** Protocol / `viewer_net` + `viewer_app`

**Learned when:** Completing the March 31, 2026 tuple sample-widening slice after increasing the bounded `RegionObjects` child-map window and distinct-name reporting.

**What happened:** The widened extraction path worked mechanically, but the bounded live run still produced only two tuple samples from the same object name (`DSS Candlier Frame`). That showed the limiting factor had moved from the code-side summary cap to the actual live region/window content.

**Rule for future plans:** After widening the extractor and confirming the live tuple sample is still narrow, stop adding more local logging in the same place. Move the next branch to a broader capture window or different-region evidence run before assigning semantics.

**Files affected:** `viewer_net` `RegionObjects` child-map limits and tuple analysis, `viewer_app` relay summaries, post-widening object-ingress planning.

---

## L50 — If a longer same-region tuple capture still stays narrow, the next evidence branch should change region instead of only increasing time

**Category:** Protocol / `viewer_net` + `viewer_app`

**Learned when:** Completing the March 31, 2026 longer same-region tuple-capture slice after the extractor had already been widened.

**What happened:** A 110-second bounded live run still produced only the same two tuple samples from `DSS Candlier Frame`, with the same slot pattern and the same varying slot `4`. That showed the limiting factor was no longer local logging depth or capture duration in the current region.

**Rule for future plans:** After widening the extractor and extending the same-region capture window, do not spend another slice increasing local runtime in place. Move the next evidence step to a region-change / teleport capture, and if that still does not broaden the tuple family, treat the tuple as likely object-local content rather than a generally decodable structure.

**Files affected:** object-ingress capture strategy, tuple-analysis planning, continuity docs.

---

## L51 — Reconnect-targeted SLURL support should normalize to Firestorm-style `uri:Region&x&y&z`, not pass raw SLURLs through as login start strings

**Category:** Protocol / `viewer_app` + UI/startup orchestration

**Learned when:** Implementing the March 31, 2026 bounded reconnect-based SLURL teleport slice.

**What happened:** Firestorm references (`llslurl.cpp`, `lllogininstance.cpp`) show that location-style login targets are converted into `uri:Region&x&y&z` strings before login shaping. Passing raw `secondlife://...` or maps URLs through directly would not match the proven start-location format already accepted by the login/bootstrap path.

**Rule for future plans:** When adding SLURL-driven start-location or reconnect controls, normalize supported SLURL forms into Firestorm-style login start strings first. Keep that logic in `viewer_app` orchestration/UI parsing, and defer true in-session teleport parity to a separate transport/handoff milestone.

**Files affected:** `viewer_app` start-location parsing, worker command/reconnect flow, network-debug teleport tooling.

---

## L52 — Once region change shows stable cross-region typed `RegionObjects` fields and the tuple disappears, promote the stable fields instead of continuing tuple-first decoding

**Category:** Protocol / `viewer_net` + `viewer_app`

**Learned when:** Completing the March 31, 2026 reconnect-based teleport capture and the follow-on typed-feed promotion.

**What happened:** The teleport capture changed both simulator host and object population. The new region still surfaced stable typed `RegionObjects` fields such as `name`, `owner`, `position`, `profile`, `linkset_use`, and `walkability`, but the earlier tuple-like description variant no longer appeared. That showed the tuple was likely local object content rather than the next universal protocol schema to decode.

**Rule for future plans:** After a region-change capture broadens the `RegionObjects` sample and the previously suspicious tuple content disappears, stop treating tuple interpretation as the primary next branch. Promote the stable cross-region fields into a bounded typed feed first, and only return to tuple decoding if new evidence shows it matters again.

**Files affected:** `viewer_net` `RegionObjects` typed extraction, `viewer_app` relay summarization, object-ingress planning.

---

## L53 — Treat first post-reconnect `RegionObjects` emptiness as a timing question before treating it as a schema/content conclusion

**Category:** Protocol / `viewer_app` + `viewer_net`

**Learned when:** Completing the April 1, 2026 broader-SLURL reconnect capture (`secondlife://Morris/128/128/25`) after typed-feed live validation.

**What happened:** In a single bounded run, pre-teleport `RegionObjects` surfaced healthy `typed_sample=...` records, but the first post-reconnect startup probe on a different simhost returned `keys=<root>` and `typed_sample=none`. The transport/session remained alive and event queue progressed, so immediate emptiness could not be safely interpreted as total lane failure.

**Rule for future plans:** When reconnecting into a new region/simhost, do not assume the first bounded `RegionObjects` startup probe is representative. Add one bounded delayed re-probe marker before concluding that post-reconnect typed object data is absent.

**Files affected:** object-ingress capture strategy, reconnect probe timing plans, continuity decision flow.

---

## L54 — Direct-binary live validation must use a freshly built executable; stale binaries create false-signal protocol conclusions

**Category:** Validation / runtime verification workflow

**Learned when:** Implementing the April 1, 2026 post-reconnect re-probe timing branch and attempting bounded live verification under Windows file-lock contention.

**What happened:** `cargo check`/tests passed, but local `cargo build` of `viewer_app` intermittently failed with Windows file-lock errors in `target*`. Running a previously built executable produced plausible runtime logs but could not be trusted as evidence for newly added probe markers.

**Rule for future plans:** For any runtime/protocol validation that depends on newly added relay markers, require an explicit successful fresh binary build before interpreting live logs. If build locking blocks that, report the branch as implementation-complete but live-unvalidated.

**Files affected:** runtime validation process, execution reports, continuity status wording.

---

## L55 — A successful delayed post-reconnect `RegionObjects` re-probe can still return `typed_sample=none`, so timing alone is not always the root cause

**Category:** Protocol / `viewer_app` reconnect diagnostics

**Learned when:** Completing the April 1, 2026 authoritative `cargo run -p viewer_app` live validation of reconnect re-probe timing (`VIEWER_APP_REGION_OBJECTS_REPROBE_DELAY_TICKS=80`).

**What happened:** The run showed `RegionObjects:reprobe_armed` and a later `post-reconnect re-probe ... typed_sample=none` on the same simhost (`simhost-0a962ce03cdb50c3e...`). This proves the re-probe path executed, but delayed timing did not recover typed object samples for that route.

**Rule for future plans:** After implementing bounded delayed re-probe, do not keep increasing delay by default. First compare across different reconnect targets; if emptiness persists across targets, classify it as route/region-dependent behavior and adjust branch priorities.

**Files affected:** reconnect evidence strategy, `RegionObjects` branch selection, continuity next-step framing.

---

## L56 — Reconnect `RegionObjects` outcomes must be judged across at least two targets; one target can stay empty while another returns rich typed samples with the same timing settings

**Category:** Protocol / `viewer_app` reconnect diagnostics

**Learned when:** Completing the April 1, 2026 Ahern target-comparison run after L55 (`VIEWER_APP_REGION_OBJECTS_REPROBE_DELAY_TICKS=80`).

**What happened:** With the same bounded startup/reprobe settings, one simhost path (`simhost-0a962ce03cdb50c3e...`) returned `typed_sample=none`, while reconnecting to `secondlife://Ahern/50/60/70` on `simhost-04e63a701b66ed282...` returned rich typed samples on both primary probe and delayed re-probe.

**Rule for future plans:** Do not classify reconnect `RegionObjects` behavior from a single target run. Require paired target comparison before deciding whether to deepen typed-field promotion or pivot branches.

**Files affected:** reconnect evidence strategy, branch-selection criteria, continuity decision framing.

---

## L57 — Under identical reconnect timing knobs, `RegionObjects` outcomes can invert across target/simhost paths, so single-run direction calls are unsafe

**Category:** Protocol / reconnect diagnostics and planning

**Learned when:** Completing paired A/B captures on April 1, 2026 (`Morris` vs `Ahern`) with identical startup/reprobe settings.

**What happened:** The Morris-target run produced rich typed samples before reconnect and `typed_sample=none` after reconnect, while the Ahern-target run produced the opposite sequence in the same configuration window. The split aligned with simulator-host path (`simhost-04e63a...` rich, `simhost-0a962c...` empty) in these captures.

**Rule for future plans:** Do not pick the next branch from a single reconnect capture. Require paired A/B evidence with matched knobs and compare pre/post reconnect outcomes before deciding between typed-feed expansion and other branches.

**Files affected:** object-ingress decision workflow, reconnect evidence plans, continuity branch selection.

---
