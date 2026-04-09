# REVIEW_PLAN_OBJECT_UUID_FOCUS_INGEST_RENDER_2026-04-02

## Verdict
Approved with one required guard: invalid UUID input must disable focus mode deterministically (no partial matching).

## Architecture and Boundary Fit
- `viewer_net` retains protocol facts (object FullID UUID) and exposes additive decode metadata.
- `viewer_core` receives additive snapshot contract field only.
- `viewer_app` owns runtime focus policy and environment parsing.
- Crate boundaries remain preserved.

## Correctness Concerns
- Focus filter must compare normalized lowercase UUID strings.
- Export ordering must remain deterministic after adding object UUID metadata.

## Modularity and Maintainability Concerns
- Keep filter logic in one helper function in `viewer_app` to avoid duplicate path drift.

## Validation Adequacy
- Required: fmt/check + targeted tests + bounded live capture with exact focus UUID.

## Risks and Open Questions
- Compressed/cached updates may not include full UUID; focus view may lag until full ObjectUpdate appears.

## Learnings Delta Verdict
- none (plan-stage): no new durable learning yet.

## Required Revisions or Approval Status
- Approval status: Approved.
- Required revision: include invalid UUID guard behavior in implementation/tests.
