# Review: PLAN_N07 Region Continuity and World Scaling

## Verdict
**Approved**

## Architecture and Boundary Fit
- **Compliance**: The plan is a model of boundary discipline. It correctly identifies `viewer_net` as the owner of transport/decode logic and `viewer_core` as the owner of the shared continuity contract.
- **Data Flow**: The use of `LiveVisualSnapshot` and `WorldObjectIngestionSeam` lanes for continuity state aligns perfectly with the established "Snapshot -> Seam -> Scene" pipeline.
- **Ownership**: The plan strictly observes the rule that only `Scene::apply_world_object_ingestion_seam` can create or remove seam-owned roles (in this case, for region anchors and neighbor markers).

## Correctness Concerns
- **Phase Transition**: The proposed deterministic handoff phase model (`none -> crossed -> confirm`) is correct for the observed LLUDP traffic patterns (`CrossedRegion`, `ConfirmEnableSimulator`).
- **Bounded State**: The commitment to bounded neighbor summaries and deterministic caps for continuity observations prevents memory/state explosion as the viewer moves through the grid.
- **Backward Compatibility**: Using `#[serde(default)]` for new snapshot fields ensures that saved sessions or fallback JSON files remain loadable.

## Modularity and Maintainability Concerns
- **Seam Lanes**: Adding dedicated `WorldObjectIngestionLane` entries for continuity prevents overloading existing traffic or presence lanes, keeping the "diagnostics-as-signal" principle intact.
- **UI Decoupling**: The decision to keep `viewer_ui` read-only and derived from the snapshot/core state preserves the orchestration boundary.

## Validation Adequacy
- **Ladder Alignment**: The validation plan follows the required ladder from `AGENTS.md`, including targeted crate tests, workspace checks, and runtime smoke tests.
- **Error Handling**: The focus on out-of-order signal handling and non-panicking decode logic is crucial for protocol robustness.

## Risks and Open Questions
- **Sequencing Discrepancy**: The plan correctly identifies the conflict between `HANDOFF.md` (labeling A07 next) and `PLAN.md` (labeling N07 next). This must be reconciled before implementation begins.
- **Neighbor Fidelity**: The plan wisely leaves the exact fidelity of neighbor-region hints as an implementation-time detail based on observed data, rather than speculating in the plan.

## Required Revisions or Approval Status
- **Status**: Approved.
- **Recommendation**: Before starting implementation, update `docs/HANDOFF.md` to reflect `N07` as the current priority, and ensure the "Lighting & PBR" milestone is correctly placed in the `PLAN.md` roadmap (e.g., as `R08` or similar) to ensure the ID system remains consistent.
