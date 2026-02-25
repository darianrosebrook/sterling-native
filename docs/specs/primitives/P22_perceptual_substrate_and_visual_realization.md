---
authority: architecture
status: imported
source: tmp/capability_primitives_bundle
version: v0.4
---
# P22: Perceptual Substrate + Visual Realization

## What it solves

P22 adds a governed, replayable perception and realization substrate. Perception and generation models can propose evidence or candidates, but only governed operators can mutate authoritative state or commit generated artifacts.

## Formal signature

- `sense(media, policy, detector) -> PerceptObservationIRv0`
- `commit(observation, budgets) -> PerceptStateV1, PerceptDeltaIRv0`
- `derive_render_intent(percept_state) -> RenderIntentV1`
- `verify(render_intent, measurement_observation) -> VerificationReportV1`
- `commit_rendered_asset(report, candidate_asset_hash) -> MediaAssetRefV1 | fail_closed`

## Key gates

- Golden digest lock for capsule spec + canonical payloads.
- Contract separation (proofs/capsule logic do not import domains/worlds).
- Determinism harness over recorded evidence.
- Transfer validation across structurally distinct perception fixtures.
- Verify-then-commit fail-closed behavior.

## Minimal wedge (scaffold)

- P22-A image intake with deterministic canonicalization and budget checks.
- P22-C render intent verification over recorded measurement evidence.

## Required artifacts

- `sterling.p22.percept_observation.v0`
- `sterling.p22.percept_state.v1`
- `sterling.p22.render_intent.v1`
- `sterling.p22.verification_report.v1`
