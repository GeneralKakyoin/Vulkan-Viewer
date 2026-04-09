# Plan: Live Texture Fetch Scheduler Pipeline (2026-04-01)

## Objective
Add a bounded live texture fetch scheduler/pipeline in `viewer_app` so live texture requests are queued, prioritized, deduplicated, and retried deterministically, while preserving existing crate boundaries and Firestorm-aligned URL shaping.

## Scope
- In scope:
  - Replace ad-hoc per-command `RequestTexture` spawn behavior with scheduler queue + bounded in-flight dispatch.
  - Deterministic priority ordering and dedupe by `AssetID`.
  - Bounded retry/backoff for retryable fetch failures.
  - Keep transport fetch on `viewer_net::fetch_texture_asset_bytes(...)` with `viewer_grid` URL-shape helpers.
  - Decode worker `TextureAsset` bytes via `viewer_asset::decode_texture_rgba8(...)` in app ingest path.
  - Add targeted tests for retry/backoff decision helpers.
- Out of scope:
  - Full Firestorm texture subsystem parity (cache thread model, deep reprioritization semantics, UDP texture lane parity).
  - Mesh/material/sound scheduler expansion.
  - New crate boundaries or asset ownership relocation.

## Current Known State
- Live texture requests are currently spawned ad hoc in `viewer_app` on every `RequestTexture` command.
- Current ingest path decodes `TextureAsset` bytes with PNG-only decode, which mismatches observed `ViewerAsset` J2C responses.
- Lane request-shaping matrix already exists and confirms `texture_id` over `ViewerAsset` can return `200 image/x-j2c`.

## Files And Components Touched
- `crates/viewer_app/src/main.rs`
- Docs:
  - `docs/reviews/REVIEW_PLAN_LIVE_TEXTURE_FETCH_SCHEDULER_PIPELINE_2026-04-01.md`
  - `docs/reviews/REVIEW_IMPL_LIVE_TEXTURE_FETCH_SCHEDULER_PIPELINE_2026-04-01.md`
  - `docs/reports/REPORT_LIVE_TEXTURE_FETCH_SCHEDULER_PIPELINE_2026-04-01.md`
  - `docs/CURRENT_STATE.md`
  - `docs/HANDOFF.md`
  - `docs/LEARNINGS.md` (delta decision)

## Boundary Check
- `viewer_grid`: capability/request-shape meaning remains here (unchanged).
- `viewer_net`: HTTP fetch mechanics remain here (unchanged).
- `viewer_app`: orchestrates request scheduling and worker lifecycle (new bounded queue logic belongs here).
- No cross-crate ownership inversion.

## Step Sequence
1. Add bounded scheduler data types/helpers in `viewer_app` for live texture fetch commands.
2. Replace `LiveFeedCommand::RequestTexture` ad-hoc spawn with enqueue-only behavior.
3. Add per-tick dispatch stage that launches up to `N` in-flight fetches from the queue by priority.
4. Add per-tick completion stage that ingests fetch task results, applies retry/backoff policy, and emits `TextureAsset`/`TextureAssetFailed` updates.
5. Switch app ingest decode for `LiveFeedUpdate::TextureAsset` to `viewer_asset::decode_texture_rgba8(...)`.
6. Add/adjust targeted unit tests for retry and backoff helper behavior.
7. Run validation commands and record outcomes.
8. Update continuity artifacts (report/state/handoff/learnings delta).

## Validation Plan
1. `cargo fmt --all`
2. `cargo check -p viewer_app -p viewer_asset -p viewer_net -p viewer_grid`
3. `cargo test -p viewer_app -p viewer_asset`
4. Optional bounded runtime smoke:
   - `VIEWER_APP_LIVE_STARTUP=on`
   - `cargo run -p viewer_app`

## Risks And Open Questions
- Scheduler values (in-flight cap, retry count/backoff) are heuristic and may require tuning with live evidence.
- Queueing in `viewer_app` worker improves bounded behavior but is not full parity with Firestorm's threaded fetch subsystem.

## Deferred-Too-Early Candidates Captured
- Full multi-class asset scheduler parity (mesh/material/sound with range/chunk pipeline) remains deferred (existing deferred list entry: full multi-class live asset streaming parity).

## Learnings Pre-Check
- Applies: `L06`, `L62`, `L64` (boundary discipline and capability-shaped request evidence).
- Applies: `L22`, `L54` (validation rigor for runtime evidence).
- No additional pre-existing learning blocks this bounded scheduler slice.

## Completion Criteria
- Live texture requests no longer dispatch ad hoc per command; they flow through bounded queue/in-flight pipeline.
- Retry/backoff behavior is deterministic and bounded for retryable transport failures.
- `TextureAsset` decode path accepts J2C/PNG via shared decode helper.
- Validation commands pass (or failures are explicitly documented).
- Continuity docs and review/report artifacts are updated.
