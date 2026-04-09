# Review: Object Ingress Capability Readiness Invocation Investigation (2026-04-01)

## Verdict
Approved.

## Architecture and boundary fit
- `viewer_net` owns reusable capability transport probing (`InterestList` / `UntrustedSimulatorMessage` one-shot fetch helpers).
- `viewer_app` owns readiness-state tracking, relay summaries, and reconnect policy wiring.
- No boundary violations or cross-crate ownership drift detected.

## Correctness concerns
- Readiness matrix output is explicit and per-capability (`available`, `invoked`, `ok`, `err`, `last`).
- New EventQueue cap-not-found reconnect threshold is bounded and configurable.
- LLUDP gate remains strict and unchanged.

## Modularity and maintainability concerns
- Refactored simulator capability GET boilerplate in `viewer_net` into reusable helper paths.
- Kept probe logic bounded to startup/investigation context; no broad capability-client expansion.

## Validation adequacy
- `cargo fmt --all`: passed.
- `cargo check -p viewer_net -p viewer_app`: passed.
- `cargo test -p viewer_net`: passed.
- `cargo test -p viewer_app`: passed.
- bounded live run (`cargo run -p viewer_app`, timeout-bounded) captured readiness + gate evidence.

## Risks and open questions
- `InterestList` and `UntrustedSimulatorMessage` probe failures may be expected for this path, but they now provide concrete readiness signals.
- LLUDP object ingress still absent; follow-up must target capability-readiness interpretation/ordering, not LLUDP startup packet guessing.

## Learnings delta verdict
add — captured as `L59` in `docs/LEARNINGS.md`.
