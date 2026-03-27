# Plan: R08 Avatar Appearance and Attachment Render Foundation

## Objective
Establish a bounded, deterministic avatar appearance path that renders avatar body proxies plus a small attachment proxy set through existing `viewer_core` -> `viewer_render` material/render contracts, while preserving seam-owned lifecycle behavior.

## Scope
- Add typed avatar appearance and attachment proxy contracts in `viewer_core` for scene-facing use.
- Map bounded appearance/attachment data from existing social/continuity inputs in `viewer_app`.
- Render deterministic attachment proxies in `viewer_render` using existing A06 material/fallback pipeline.
- Keep lifecycle create/update/remove for seam-owned avatar/attachment roles strictly in seam apply paths.
- Add bounded diagnostics exposure for avatar/attachment continuity status in UI (read-only).
- Fix deterministic limits for R08:
  - max attachment proxies per avatar: `MAX_R08_ATTACHMENTS_PER_AVATAR = 8`
  - ordering key before truncation: stable ascending tuple `(attachment_id, local_id)` (string/number lexical as available)

## Current known state
- `N07` provides bounded continuity state (handoff phase, active/previous region, neighbor summaries) and continuity seam diagnostics.
- Avatar placeholders already render in bounded form via existing `AvatarRenderMode` and `MeshKind::AvatarProxy` usage.
- A06 material contracts are active and deterministic, including `white_view` fallback behavior for empty texture slots (L16).
- Social/profile workflows exist from U04, but attachment representation and richer avatar appearance are not yet wired as first-class bounded contracts.

## Files and components touched
- `crates/viewer_core/src/lib.rs`
  - avatar/attachment domain contract additions
  - seam-side attachment payload collection updates (`WorldObjectIngestionSeam::avatar_attachments`)
  - scene apply behavior for seam-owned avatar/attachment proxy roles (`InstanceRole::WorldAvatarAttachmentProxy`)
- `crates/viewer_app/src/main.rs`
  - mapping from worker/social snapshot into bounded avatar appearance + attachment seam payloads
  - dirty-only apply path preservation
- `crates/viewer_render/src/lib.rs`
  - deterministic rendering path updates for bounded attachment proxies
  - reuse existing material bindings/fallbacks; no ad-hoc material path
- `crates/viewer_ui/src/lib.rs` (bounded, optional for milestone verification)
  - read-only diagnostics lines for avatar/attachment continuity state
- tests in `viewer_core`, `viewer_app`, and `viewer_render` for mapping/lifecycle/determinism

## Boundary check
- `viewer_core` owns avatar/attachment scene-domain contracts and seam lifecycle semantics.
- `viewer_app` remains orchestration and mapping only; it must not create/remove seam-owned roles directly.
- `viewer_render` owns draw submission internals and material binding decisions.
- `viewer_ui` remains display-only and derives status from core/app-provided state.
- `viewer_net`/`viewer_grid` remain transport/meaning boundaries; R08 does not introduce transport logic into render/UI.

## Step sequence
1. Define bounded avatar/attachment contract types in `viewer_core`.
2. Add seam-side attachment payload collection wiring (`WorldObjectIngestionSeam::avatar_attachments`) with the R08 cap/order rule.
3. Extend scene seam apply to create/update/remove `InstanceRole::WorldAvatarAttachmentProxy` only in seam-owned paths.
4. Map bounded avatar/attachment appearance inputs in `viewer_app` without violating dirty-only update guards.
5. Update renderer dispatch for attachment proxies using existing mesh/material binding rules.
6. Add focused diagnostics exposure in UI only if needed for milestone verification.
7. Add targeted tests for:
   - seam lane mapping determinism
   - seam-owned create/remove lifecycle for attachment roles (explicit present -> absent payload transition test)
   - renderer deterministic handling of attachment proxy materials/fallbacks
8. Run validation ladder and close with report/review/continuity updates.

## Validation plan
- Always:
  - `cargo fmt --all -- --check`
  - `cargo check --workspace`
- Targeted:
  - `cargo test -p viewer_core`
  - `cargo test -p viewer_render`
  - `cargo test -p viewer_app`
  - `cargo test -p viewer_ui` (if touched)
- Broader:
  - `cargo test --workspace` when cross-crate behavior changes materially
- Runtime smoke:
  - `VIEWER_APP_LIVE_STARTUP=off STRESS_TEST=screenshot cargo run -p viewer_app`
  - bounded visual check with avatar/attachment proxies present and stable across repeated runs

## Risks and open questions
- Risk: attachment proxy expansion can increase draw-call churn; keep bounded set small and deterministic for R08.
- Risk: ambiguous ownership between social/avatar state and seam payloads; preserve seam lifecycle authority in `viewer_core`.
- Open question: whether attachment diagnostics need dedicated UI rows in R08 or can rely on scene-visible verification.

## Deferred-too-early candidates captured
- Full skeletal animation graph / retargeting / AO layers (defer beyond R08).
- Full baked texture and wearable/system-layer parity (defer beyond R08).
- Attachment category parity and full slot semantics (defer to later avatar/parity milestone after R08 baseline proves stable).

## Learnings pre-check
- `L03`: Seam-owned roles must only be created/removed in seam apply path.
- `L04`: Dirty-only scene/snapshot apply guards are load-bearing and must not be removed.
- `L08`: No per-frame pipeline/depth lifecycle regressions while adding attachment render paths.
- `L09`: Any new render pipelines required by R08 must be created at startup.
- `L12`: Preserve `sync_spatial()` frame synchronization assumptions for avatar/attachment proxy transforms.
- `L13`: Ensure any new transform initializers use identity quaternion.
- `L16`: Preserve `white_view` semantics for empty texture IDs in attachment material binding.

## Completion criteria
- Bounded avatar body and attachment proxies render deterministically via existing material contract paths.
- Attachment proxy lifecycle (create/update/remove) is driven by seam-owned apply behavior only.
- Dirty-only apply semantics remain unchanged in app orchestration.
- Required validation commands pass, or blockers are explicitly documented in review/report artifacts.
- Continuity docs (`CURRENT_STATE.md`, `HANDOFF.md`) reflect R08 status and next step.
