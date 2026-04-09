# Review: Plan Object Ingress Two-Lane Transport Probe (2026-04-01)

## Verdict
approved

## Architecture and boundary fit
- Transport probing remains in `viewer_net`.
- Lane target selection and diagnostic emission remain in `viewer_app`.
- No ownership violations detected.

## Correctness concerns
- Keep probe output bounded (status/content-type/body-bytes only).
- Ensure target selection is deterministic and excludes baseline capability names.
- Probe at most two lanes to avoid startup noise and latency bloat.

## Modularity and maintainability concerns
- Prefer small helper functions for lane target selection and probe summary formatting.
- Reuse existing capability URL classification helpers instead of introducing parallel classification logic.

## Validation adequacy
- `cargo fmt --all`
- `cargo check -p viewer_net -p viewer_app`
- `cargo test -p viewer_net`
- `cargo test -p viewer_app`
- bounded live run with dedicated JSONL artifact

## Risks and open questions
- Non-success HTTP responses may still be the expected lane behavior for unauthenticated/malformed one-shot requests.
- Transport evidence may not immediately map to object-ingress semantics.

## Learnings delta verdict
none (plan stage)

## Required revisions or approval status
Approved as written.
