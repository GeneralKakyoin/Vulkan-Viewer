# Review: OBJECT_INGRESS_POST_AMC_REQUEST_PARITY_2026-03-30

## Verdict
Accepted.

## Architecture and Boundary Fit
- The implementation keeps LLUDP request construction and classification in `viewer_net`.
- Startup send orchestration remains in `viewer_app`.
- No crate-boundary drift into rendering, asset transport, or grid semantics was introduced.

## Correctness Concerns
- The three new startup requests use the expected Firestorm/message-template IDs:
  - `MuteListRequest` `262`
  - `MoneyBalanceRequest` `313`
  - `AgentDataUpdateRequest` `386`
- The helper bodies stay bounded and match the expected block shapes for the current startup slice.
- `LayerData` is classification-only, which is correct for the approved scope.

## Modularity and Maintainability Concerns
- The implementation uses dedicated helper functions rather than embedding packet assembly in the app worker.
- Startup request parity is grouped as one explicit method, which keeps the prime path readable and bounded.

## Validation Adequacy
- `cargo fmt --all`, targeted `cargo check`, targeted crate tests, and a bounded connected run all completed successfully.
- The connected run provided the decisive behavioral answer for this slice: the extra startup requests were sent, but object ingress remained zero.

## Risks and Open Questions
- The next blocker is still unresolved.
- The remaining gap may be:
  - `SetAlwaysRun`
  - `AgentAnimation`
  - another control/reply behavior
  - or an inbound observation gap between pcap-level traffic and live receive summaries

## Learnings Delta Verdict
- add: the live result produced a durable lesson that this narrowed startup request subset is not sufficient on the current one-port path.

## Required Revisions or Approval Status
- Approved as implemented.
