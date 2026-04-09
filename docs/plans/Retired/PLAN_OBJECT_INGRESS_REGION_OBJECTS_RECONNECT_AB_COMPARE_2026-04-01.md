# Plan: Object Ingress RegionObjects Reconnect A/B Compare (Morris vs Ahern) (2026-04-01)

## Scope
- Run paired bounded live captures using identical runtime knobs and different reconnect targets:
  - `secondlife://Morris/128/128/25`
  - `secondlife://Ahern/50/60/70`
- Compare `RegionObjects` primary probe and delayed re-probe outcomes.
- Produce a decision-ready branch recommendation.

Out of scope:
- LLUDP transport changes
- capability-surface expansion
- UI changes

## Current known state
- `typed_sample` now includes `landimpact` and is live-validated on Ahern baseline.
- Reconnect outcomes are known to be mixed across prior runs.
- LLUDP object ingress remains absent.

## Files and components touched
- `docs/reports/`
- continuity docs:
  - `docs/CURRENT_STATE.md`
  - `docs/HANDOFF.md`
  - `docs/OBJECT_INGRESS_STATUS.md`
  - `docs/LEARNINGS.md` (only if a new durable lesson emerges)

## Boundary check
- No code changes required unless a capture issue forces minimal instrumentation correction.
- No crate-boundary changes.

## Step sequence
1. Run bounded Morris capture with fixed knobs and dedicated artifacts.
2. Run bounded Ahern capture with same knobs and dedicated artifacts.
3. Extract `typed_sample` and re-probe evidence from both runs.
4. Produce side-by-side comparison and select the next active branch.
5. Update continuity docs and handoff.

## Validation plan
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- two bounded live runs:
  - Morris target
  - Ahern target

## Risks and open questions
- Region content can drift over time; A/B captures should be run close together.
- Login landing region may vary between runs.

## Deferred-too-early candidates captured
- none

## Learnings pre-check
- L56: require paired target comparison before branch decisions.

## Exact completion criteria
- Both captures complete with artifacts.
- Report states whether reconnect emptiness is target/content dependent, host-family dependent, or unresolved.
- One explicit next branch is selected and documented.
