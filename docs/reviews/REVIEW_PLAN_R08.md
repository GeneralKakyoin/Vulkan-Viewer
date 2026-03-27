# Review: PLAN_R08 Avatar Appearance and Attachment Render Foundation

## Verdict
Approved with required revisions.

## Architecture and Boundary Fit
- Boundary ownership is strong and matches repo law:
  - `viewer_core` owns scene-facing contracts and seam lifecycle.
  - `viewer_app` remains orchestration/mapping.
  - `viewer_render` remains GPU submission owner.
  - `viewer_ui` remains read-only presentation.
- Plan explicitly preserves the seam-owned lifecycle rule and dirty-only apply constraints, aligned with `L03` and `L04`.

## Correctness Concerns
- The lifecycle path is directionally correct, but one implementation-critical detail is still underspecified:
  - the exact bounded attachment cap and deterministic ordering rule are not fixed in the plan.
- Without explicit cap/order definitions, different implementations could diverge and create non-deterministic behavior under load.

## Modularity and Maintainability Concerns
- Reuse of existing A06 material contracts is the right approach and avoids ad-hoc render paths.
- Step sequence is clean and keeps policy and rendering responsibilities separated.
- Recommend naming the new seam lane(s) and role(s) explicitly in the plan to reduce implementation ambiguity.

## Validation Adequacy
- Validation ladder is appropriate (`fmt`, `check`, targeted tests, workspace tests when material).
- Runtime smoke via screenshot mode is a good deterministic baseline for visual verification.
- Add one explicit requirement for seam role removal verification in tests (attachment disappears when payload disappears).

## Risks and Open Questions
- Risk of draw churn is acknowledged and bounded.
- Open questions are valid, but one must be resolved in-plan before implementation:
  - exact R08 attachment cap and ordering rule.

## Learnings Delta Verdict
none — planning quality improved, but no new durable learning beyond existing entries was discovered.

## Required Revisions or Approval Status
- Required revision 1: Add explicit attachment cap value and deterministic ordering key (for example: stable by attachment id).
- Required revision 2: Name the exact planned seam lane(s) and scene role(s) for attachment payload lifecycle.
- Required revision 3: Add a targeted test requirement for seam removal lifecycle (present -> absent transition).
- Status after revisions: Approved for implementation.
