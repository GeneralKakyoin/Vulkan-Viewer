# Review: Plan Face-Accurate Texture + Material Parity (2026-04-03)

## Verdict
Approved.

## Architecture and boundary fit
- Plan keeps decode responsibilities in `viewer_net`, propagation in `viewer_app`, and scene material application in `viewer_core`.
- No boundary violations or renderer-contract widening are required for this slice.

## Correctness concerns
- `TextureEntry` exception parsing must preserve default-first semantics and fail-soft behavior on malformed/truncated payloads.
- Per-face override precedence must be deterministic and should not regress legacy fallback for objects without face payload.
- Texture scheduling expansion must remain deduped and bounded.

## Modularity and maintainability concerns
- Additive payload contracts are appropriate; avoid replacing existing single-texture fields until parity is fully stable.
- Keep diagnostics bounded and artifact outputs compact.

## Validation adequacy
- `fmt`, targeted `check`, and per-crate tests (`viewer_net`, `viewer_app`, `viewer_core`, `viewer_render`) are adequate.
- Bounded live run with dedicated debug log is required and sufficient for this slice.

## Risks and open questions
- Material-extension references are additive here; full resolution via RenderMaterials remains a follow-up.
- Startup windows may briefly under-report face materials before steady-state updates arrive.

## Learnings delta verdict
- `add`
- Reason: this slice can establish a durable parser-trap learning around `TextureEntry` default-field double-consumption hazards.

## Required revisions or approval status
- Approved as written.
