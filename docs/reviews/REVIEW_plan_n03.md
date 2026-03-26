# Review: PLAN_N03 Bounded World/Object Decode Expansion for Richer Render Feed

## Verdict
Approved.

## Architecture and boundary fit
- Fits the repository milestone ordering (`R01` -> `A02` -> `N03`) and focuses on feed richness rather than rendering/asset expansion.
- Preserves the `viewer_net` (transport/decode) vs `viewer_grid` (meaning/policy) boundary by constraining work to typed, bounded extraction.
- Preserves seam ownership invariants: new world-object roles are created/removed only through `viewer_core::Scene::apply_world_object_ingestion_seam(...)`.

## Correctness concerns
- Object update payloads are variable and some variants are zerocoded; decode must remain bounded, defensive, and never panic on malformed bodies.
- Lifecycle handling must avoid accidental mass-deletes when the feed is truncated/capped; the plan should ensure truncation is signaled and removal rules are conservative.

## Modularity and maintainability concerns
- Keep object-feed decode isolated behind small helper functions in `viewer_net` rather than growing `observe_first_simulator_inbound_payload(...)` into a monolith.
- Keep `viewer_app` changes limited to snapshot bridging and preserve dirty-only apply semantics.

## Validation adequacy
- Validation ladder is appropriate: `cargo fmt`, `cargo check`, targeted crate tests (`viewer_net`, `viewer_core`, optionally `viewer_app`), then broader `cargo test` when cross-crate behavior changes.
- Runtime verification via `cargo run -p viewer_app` is necessary since the milestone changes live feed behavior.

## Risks and open questions
- Exact caps (max tracked objects, max exported per snapshot, max kill list) should be explicit constants with tests enforcing boundedness.
- If `ObjectUpdate` (zerocoded) parsing is introduced, ensure the zerocode decode is bounded and tested for expansion limits.

## Required revisions / approval status
No required revisions. Approved for implementation.
