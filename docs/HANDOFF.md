# HANDOFF: Object Ingress PCAP Forensics And Next Plan

## What Changed
- Preserved the two most important external packet captures in-repo:
  - `artifacts/pcaps/firestorm_object_ingress_reference_2026-03-30_firee.pcapng`
  - `artifacts/pcaps/app_object_ingress_reference_2026-03-30_App.pcapng`
- Added a preservation manifest with stable sha256 hashes in `artifacts/pcaps/README.md`.
- Added a research summary at `docs/RESEARCH/OBJECT_INGRESS_PCAP_FORENSICS_2026-03-30.md`.
- Added the next bounded plan and review:
  - `docs/plans/PLAN_OBJECT_INGRESS_RUNTIME_SOCKET_FORENSICS_2026-03-30.md`
  - `docs/reviews/REVIEW_PLAN_OBJECT_INGRESS_RUNTIME_SOCKET_FORENSICS_2026-03-30.md`

## Validation Run
- `Get-FileHash C:\Users\matti\Desktop\firee.pcapng -Algorithm SHA256`: PASSED
- `Get-FileHash C:\Users\matti\Desktop\App.pcapng -Algorithm SHA256`: PASSED
- local Python pcap parsing of preserved Firestorm and app simulator conversations: PASSED
- No runtime code or cargo validation was run in this documentation/planning slice.

## Exact Current State
- Firestorm reference capture:
  - single coherent simulator flow on `16.144.39.130:13001`
  - large `ObjectUpdateCached` burst appears after a richer pre-burst send sequence
- App reference capture:
  - same simulator endpoint reached
  - no object burst on the handshake/control flow
  - additional local UDP port observed receiving simulator traffic during the capture window
- Current best interpretation:
  - runtime local-port/socket continuity is not yet proven in practice
  - extra Firestorm pre-burst requests are important evidence, but not yet safe to treat as the next implementation target

## Exact Next Step
- Implement `docs/plans/PLAN_OBJECT_INGRESS_RUNTIME_SOCKET_FORENSICS_2026-03-30.md`.
- Add bounded runtime local-port/socket diagnostics to the first-simulator path and verify whether the blocked viewer still uses more than one local UDP port during startup and early steady-state traffic.

## Blockers / Risks
- `App.pcapng` capture start timing leaves some ambiguity about when the earlier local port was created.
- Blindly mirroring Firestorm’s extra pre-burst messages remains intentionally deferred until the runtime socket lifecycle is verified.
