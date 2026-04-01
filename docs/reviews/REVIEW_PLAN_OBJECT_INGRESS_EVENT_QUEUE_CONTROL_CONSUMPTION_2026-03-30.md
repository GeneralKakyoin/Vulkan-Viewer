# Review: Plan Object Ingress EventQueue Control Consumption (2026-03-30)

## Verdict
Approved.

## Architecture and boundary fit
- Keeps EventQueue parse/extract logic in `viewer_net`.
- Keeps runtime surfacing in `viewer_app`.
- Avoids widening into `viewer_grid`, renderer, or broad session orchestration.

## Correctness concerns
- Do not treat `EnableSimulator` arrival by itself as proof that object ingress is fixed.
- Preserve existing chat extraction behavior while extending EventQueue field preservation.
- Keep extracted summaries bounded and deterministic so diagnostics stay readable.

## Modularity and maintainability concerns
- Prefer generic nested-body flattening helpers over one-off string scraping in the app.
- Keep event-specific extraction helpers small and optional so later slices can reuse them without coupling all EventQueue behavior together.

## Validation adequacy
- `fmt`, targeted `check`, targeted tests, and one bounded connected run are adequate for this slice.

## Risks and open questions
- The live path may surface only neighbor-region control data, not the first-region object gate.
- `EstablishAgentCommunication` may still be absent in the bounded window even after parser improvement.

## Learnings delta verdict
- `none`
- The implementation result will decide whether a new durable learning is warranted.

## Approval status
Approved to implement as written.
