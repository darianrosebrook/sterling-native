> **NOTE: Quarantined v1 canonical contract. Non-authoritative for Sterling Native. See [promotion criteria](README.md).**

Schema Registry (Draft)
=======================

Purpose
-------
Rendered from the code-owned registry in core/contracts/schema_registry.py.

Entries
-------
| schema_id | schema_version | owner | canonical_doc | hash_critical |
| --- | --- | --- | --- | --- |
| sterling.text_intake_ir.v1 | 1.0.0 | core/text | docs/canonical/text_io_contract_v1.md | true |
| sterling.run_result.v1 | 1.2.0 | core/engine | docs/canonical/text_boundary_index.md | true |
| sterling.text_realization_ir.v1 | 0.x | core/realization | docs/specifications/text-semantic-ir/realization-ir-text-contract.md | true |
| sterling.trace_bundle.stub.v0 | 0.0.1 | core/text | docs/roadmaps/text_io_swm_boundary_harness.md | false |
| sterling.trace_bundle.v1 | 1.0.0 | core/contracts | docs/roadmaps/text_io_swm_boundary_harness.md | true |
| sterling.completeness_declaration.v1 | 1.0.0 | core/contracts | docs/roadmaps/text_io_swm_boundary_harness.md | true |
| sterling.goal_satisfaction_witness.v1 | 1.0.0 | core/contracts | docs/roadmaps/text_io_swm_boundary_harness.md | true |
| sterling.search_decision_witness.v1 | 1.0.0 | core/contracts | docs/roadmaps/text_io_swm_boundary_harness.md | true |
| sterling.state_graph.v1 | 1.0.0 | core/reasoning | docs/roadmaps/text_io_swm_boundary_harness.md | true |
| sterling.final_state.v1 | 1.0.0 | core/reasoning | docs/roadmaps/text_io_swm_boundary_harness.md | true |
| sterling.linguistic_ir.v0 | 0.0.1 | core/linguistics | docs/reference/canonical/linguistic_ir_contract_v0.md | true |
| sterling.linguistic_delta_patch.v0 | 0.0.1 | core/linguistics | docs/reference/canonical/linguistic_ir_contract_v0.md | true |
| sterling.operator_witness.v0 | 0.0.1 | core/linguistics | docs/reference/canonical/linguistic_ir_contract_v0.md | true |
| sterling.meaning_state_digest.v0 | 0.0.1 | core/linguistics | docs/canonical/semantic_working_memory_contract_v0.md | true |
| sterling.episode_trace.v0 | 0.0.1 | core/linguistics | docs/canonical/semantic_working_memory_contract_v0.md | true |
| sterling.myelin_sheath.v0 | 0.0.1 | core/linguistics | docs/canonical/semantic_working_memory_contract_v0.md | true |
| sterling.capability_descriptor.v1 | 1.0.0 | core/domains | docs/planning/capability_primitives_bundle/philosophy.md | true |
| sterling.capability_claim_registry.v1 | 1.0.0 | core/domains | docs/planning/capability_primitives_bundle/philosophy.md | true |
| sterling.primitive_spec.v1 | 1.0.0 | core/domains | docs/planning/capability_primitives_bundle/philosophy.md | true |
| sterling.conformance_suite.v1 | 1.0.0 | core/domains | docs/planning/capability_primitives_bundle/philosophy.md | true |
| sterling.domain_declaration.v1 | 1.0.0 | core/domains | docs/planning/capability_primitives_bundle/philosophy.md | true |
| sterling.domain_session.v1 | 1.0.0 | core/domains | docs/planning/capability_primitives_bundle/philosophy.md | false |
| sterling.p22.percept_observation.v0 | 0.1.0 | core/proofs/p22 | docs/reference/canonical/primitives/p22_perceptual_substrate_and_visual_realization.md | true |
| sterling.p22.percept_state.v1 | 1.0.0 | core/capsules/p22 | docs/reference/canonical/primitives/p22_perceptual_substrate_and_visual_realization.md | true |
| sterling.p22.render_intent.v1 | 1.0.0 | core/proofs/p22 | docs/reference/canonical/primitives/p22_perceptual_substrate_and_visual_realization.md | true |
| sterling.p22.verification_report.v1 | 1.0.0 | core/proofs/p22 | docs/reference/canonical/primitives/p22_perceptual_substrate_and_visual_realization.md | true |

Investment Plan (When Ready)
-----------------------------
1) Enforce generation in CI (fail if this file diverges).
2) Expand registry to include all boundary artifact schemas.
3) Require boundary docstrings to reference registry entries only.

Notes
-----
- This file is intended to be generated; do not edit manually once generation is enforced.
