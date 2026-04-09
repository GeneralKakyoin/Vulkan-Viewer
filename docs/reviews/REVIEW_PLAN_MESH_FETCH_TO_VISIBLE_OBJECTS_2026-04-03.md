# Review: Plan Mesh Fetch To Visible Objects (2026-04-03)

## Verdict
Approved with one scoped deferral.

## Architecture and boundary fit
- The plan keeps mesh byte format ownership in `viewer_asset` and runtime orchestration in `viewer_app`.
- `viewer_core::GeometrySource::Mesh` remains unchanged, which fits the current crate boundaries.
- The plan correctly avoids widening into renderer or scene architecture changes beyond existing upload seams.

## Correctness concerns
- The plan must use protocol-backed SL mesh decode, not a glTF guess.
- Offline verification should prove visible mesh geometry, but it does not by itself prove live object material parity.

## Modularity and maintainability concerns
- Typed mesh lifecycle state is a good replacement for ad hoc raw byte maps.
- A deterministic synthetic SL mesh fixture is preferable to hard-coding one-off runtime bytes in `viewer_app`.

## Validation adequacy
- The validation ladder is sufficient for this slice.
- Runtime acceptance must include screenshot review, not just process success.

## Risks and open questions
- Compact lifecycle JSON verification may still need extra hardening.
- Full live object texture/material plumbing is correctly identified as too early for this slice.

## Learnings delta verdict
- `add`
- Reason: a deterministic synthetic SL mesh verification path is a durable repo lesson for future render/debug slices.

## Required revisions or approval status
- Approved as written.
- Record full live object material/texture parity in `docs/plans/DEFERRED_FEATURES.md` rather than stretching this slice.
