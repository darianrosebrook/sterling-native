> **NOTE: Quarantined v1 canonical contract. Non-authoritative for Sterling Native. See [promotion criteria](README.md).**

# Sterling Schema Registry v1

**Status**: Canonical reference — authoritative index of all versioned schemas in the Sterling system.
**Scope**: JSON Schemas, internal schema IDs, design schemas, API schemas.
**Layer**: Cross-cutting (governance, proofs, operators, reasoning, linguistics)

---

## §1 Purpose

Sterling uses two kinds of schemas:

1. **JSON Schema documents** (`.json` files with `"$schema"` meta-key) — machine-validatable structure contracts for serialized artifacts. Used for replay verification, certification, and inter-system exchange.
2. **Internal schema IDs** (`sterling.<namespace>.<name>.v<N>` strings) — logical identifiers registered in `core/contracts/schema_registry.py` that bind Python dataclasses to their canonical serialized form.

This document is the single index for both. It does not reproduce schema content — use the file paths to locate the authoritative source.

---

## §2 JSON Schema Documents

### §2.1 Proof and Evidence Schemas

Located in `core/proofs/schemas/`. All files follow the `<name>.v1.json` naming convention and target JSON Schema draft-07.

| Schema File | `$id` | Purpose |
|-------------|-------|---------|
| `artifact_ref.v1.json` | `sterling.artifact_ref.v1` | Content-addressed reference to a stored artifact |
| `bound_inputs.v1.json` | `sterling.bound_inputs.v1` | Input binding for a certification run (`BoundInputsV1`) |
| `case_record.v1.json` | `sterling.case_record.v1` | Determinism check record per test case |
| `certificate.v1.json` | `sterling.certificate.v1` | TD-12 certificate of correctness (top-level legitimacy artifact) |
| `certified_failure.v1.json` | `sterling.certified_failure.v1` | Structured failure outcome with recovery options |
| `evidence_manifest.v1.json` | `sterling.evidence_manifest.v1` | Manifest of all evidence artifacts for a run |
| `h2_bundle_verification.v1.json` | `sterling.h2_bundle_verification.v1` | H2 evidence bundle verification result |
| `h2_capability_summary.v1.json` | `sterling.h2_capability_summary.v1` | Per-capability summary within H2 evidence bundle |
| `h2_evidence_bundle_manifest.v1.json` | `sterling.h2_evidence_bundle_manifest.v1` | H2 evidence bundle manifest (multi-regime) |
| `h2_invariance_results.v1.json` | `sterling.h2_invariance_results.v1` | Cross-example invariance check results |
| `h2_safeguard_attestations.v1.json` | `sterling.h2_safeguard_attestations.v1` | Safeguard attestation records for H2 |
| `h3_evidence_bundle_manifest.v1.json` | `sterling.h3_evidence_bundle_manifest.v1` | H3 evidence bundle manifest (linguistic I/O) |
| `hypothesis_state.v1.json` | `sterling.hypothesis_state.v1` | Hypothesis lifecycle state snapshot |
| `identified_json.v1.json` | `sterling.identified_json.v1` | Generic content-addressed JSON value |
| `identified_json_array.v1.json` | `sterling.identified_json_array.v1` | Content-addressed JSON array |
| `intent_annotation.v1.json` | `sterling.intent_annotation.v1` | Intent classification annotation |
| `invariant_result.v1.json` | `sterling.invariant_result.v1` | Result of a single invariant check |
| `json_value.v1.json` | `sterling.json_value.v1` | Typed JSON value wrapper |
| `metrics_bundle.v1.json` | `sterling.metrics_bundle.v1` | Domain-agnostic metrics with optional extension block (`MetricsBundleV1`) |
| `ms_certificate.v1.json` | `sterling.ms_certificate.v1` | MS certificate — hash-locked attestation envelope |
| `ms_report.v1.json` | `sterling.ms_report.v1` | MS run report |
| `ms_run_manifest.v1.json` | `sterling.ms_run_manifest.v1` | MS run manifest with environment and inputs |
| `operator_inventory.v1.json` | `sterling.operator_inventory.v1` | Snapshot of registered operators at run time |
| `outcome.v1.json` | `sterling.outcome.v1` | Single task outcome record |
| `outcome_rows.v1.json` | `sterling.outcome_rows.v1` | Per-task outcome rows with deterministic `outcome_hash` (`OutcomeRowsV1`) |
| `pragmatic_context.v1.json` | `sterling.pragmatic_context.v1` | Pragmatic context snapshot |
| `prior_influence_ir.v1.json` | `sterling.prior_influence_ir.v1` | Prior influence artifact IR |
| `prior_ir.v1.json` | `sterling.prior_ir.v1` | Prior IR with artifact ref and quantized weights |
| `p22/p22_percept_observation.v0.json` | `sterling.p22.percept_observation.v0` | P22 non-authoritative perception observation envelope |
| `p22/p22_percept_state.v1.json` | `sterling.p22.percept_state.v1` | P22 authoritative percept state snapshot |
| `p22/p22_render_intent.v1.json` | `sterling.p22.render_intent.v1` | P22 realization intent envelope |
| `p22/p22_verification_report.v1.json` | `sterling.p22.verification_report.v1` | P22 realization verification report |
| `replay_trace.v1.json` | `sterling.replay_trace.v1` | Replay trace with verification hash (`ReplayTraceV1`) |
| `run_manifest.v1.json` | `sterling.run_manifest.v1` | Full run manifest (git, env, inputs, schemas, policies) |
| `semantic_delta_ir.v1.json` | `sterling.semantic_delta_ir.v1` | Semantic delta IR between two states |
| `semantic_edit.v1.json` | `sterling.semantic_edit.v1` | Single semantic edit operation |
| `semantic_ir.v1.json` | `sterling.semantic_ir.v1` | Semantic IR (event/entity graph) |
| `semiotic_mappings.v1.json` | `sterling.semiotic_mappings.v1` | Semiotic feature mappings |
| `solver_contract.v1.json` | `sterling.solver_contract.v1` | Per-domain solver contract with budgets and tie-breaks |
| `solver_contract_bundle.v1.json` | `sterling.solver_contract_bundle.v1` | Bundle of per-domain solver contracts (`SolverContractBundleV1`) |
| `state_snapshot.v1.json` | `sterling.state_snapshot.v1` | Snapshot of a StateNode at a point in search |
| `step_record.v1.json` | `sterling.step_record.v1` | Record of a single search step |
| `syntax_layer.v1.json` | `sterling.syntax_layer.v1` | UD-style dependency tree layer |
| `td12_report.v1.json` | `sterling.td12_report.v1` | TD-12 full report |
| `td7_1_necessity_verdict.v1.json` | `sterling.td7_1_necessity_verdict.v1` | TD-7 verdict: operator necessity |
| `td7_2_invariant_completeness_verdict.v1.json` | `sterling.td7_2_invariant_completeness_verdict.v1` | TD-7 verdict: invariant completeness |
| `td7_3_value_admissibility_verdict.v1.json` | `sterling.td7_3_value_admissibility_verdict.v1` | TD-7 verdict: value admissibility |
| `td7_4_failure_closure_verdict.v1.json` | `sterling.td7_4_failure_closure_verdict.v1` | TD-7 verdict: failure closure |
| `td7_correctness_evidence_overlay.v1.json` | `sterling.td7_correctness_evidence_overlay.v1` | TD-7 correctness evidence overlay |
| `td7_step_record_with_overlays.v1.json` | `sterling.td7_step_record_with_overlays.v1` | Step record with TD-7 overlays |
| `tool_transcript.v1.json` | `sterling.tool_transcript.v1` | Tool call transcript with canonical request/response (`ToolTranscriptV1`) |
| `utterance_snapshot.v1.json` | `sterling.utterance_snapshot.v1` | Snapshot of an UtteranceState |
| `verdict.v1.json` | `sterling.verdict.v1` | Generic gate verdict (PASS/FAIL/SKIPPED) |
| `world_snapshot.v1.json` | `sterling.world_snapshot.v1` | Snapshot of WorldState |

### §2.2 Report Schemas

Located in `core/reports/schemas/`. JSON Schema draft-07.

| Schema File | Purpose |
|-------------|---------|
| `rollout_report_eval.schema.json` | Rollout evaluation report structure |
| `td_report.schema.json` | Technical decision report structure |

### §2.3 API Schemas

Located in `docs/reference/capability_primitives_bundle/schemas/`. JSON Schema draft 2020-12.

| Schema File | Purpose |
|-------------|---------|
| `SolveRequestV1.schema.json` | External API solve request (task + input + options) |
| `SolveResponseV1.schema.json` | External API solve response (result + explanation + metadata) |

### §2.4 Design Schemas

Located in `docs/design/`.

| Schema File | Purpose | Notes |
|-------------|---------|-------|
| `domain_pack_manifest.json` | JSON Schema for a Sterling Domain Pack Manifest v1 | Defines the self-describing artifact contract for packaging a domain: `dimensionality`, `adapters`, `operators`, `invariants`, `policy`, `certification`. See §4 for field semantics. |
| `domain_pack_manifest_example.json` | Example instance for the `toy_graph_nav` domain | Placeholder sha256s — non-normative reference only |

### §2.5 Domain Config Schemas

Located in `configs/`. These are domain config instances validated against the Domain Pack Manifest schema.

| File | Domain | Purpose |
|------|--------|---------|
| `cap2a_wordnet_corridor_v1.json` | WordNet | Corridor regime config |
| `cap2b_wordnet_hub_v1.json` | WordNet | Hub regime config |

### §2.6 CAWS Infrastructure Schemas

Located in `.caws/schemas/`. Internal to CAWS tooling, not Sterling artifacts.

| Schema File | Purpose |
|-------------|---------|
| `scope.schema.json` | CAWS feature spec scope definition |
| `waivers.schema.json` | Quality gate waiver records |
| `working-spec.schema.json` | CAWS working spec (project-level) |
| `worktrees.schema.json` | CAWS worktree registry |

---

## §3 Internal Schema ID Registry

Sterling's Python codebase uses `sterling.<namespace>.<name>.v<N>` strings to bind dataclasses to their serialized form. The canonical registry is at `core/contracts/schema_registry.py`.

### §3.1 Core Artifact Types

| Schema ID | Python Type | Location |
|-----------|-------------|----------|
| `sterling.run_result.v1` | `RunResultV1` | `core/engine/run_result.py` |
| `sterling.trace_bundle.v1` | `TraceBundleV1` | `core/contracts/trace_bundle.py` |
| `sterling.trace_bundle.stub.v0` | Stub trace bundle | `core/contracts/trace_bundle.py` |
| `sterling.state_graph.v1` | `StateGraph` | `core/reasoning/state_graph.py` |
| `sterling.final_state.v1` | Final state witness | `core/contracts/` |
| `sterling.completeness_declaration.v1` | `CompletenessDeclaration` | `core/contracts/` |
| `sterling.goal_satisfaction_witness.v1` | Goal satisfaction record | `core/contracts/` |
| `sterling.search_decision_witness.v1` | Search decision record | `core/contracts/` |

### §3.2 Text and Linguistics

| Schema ID | Python Type | Location |
|-----------|-------------|----------|
| `sterling.text_intake_ir.v1` | `TextIntakeIRv1` | `core/text/intake_ir.py` |
| `sterling.text_realization_ir.v1` | Realization IR | `core/text/realizer.py` |
| `sterling.linguistic_ir.v0` | `LinguisticIR` | `core/linguistics/ir_v0/container.py` |
| `sterling.linguistic_delta_patch.v0` | `LinguisticDeltaPatchV0` | `core/linguistics/ir_v0/patch_v0.py` |
| `sterling.operator_witness.v0` | `OperatorWitnessV0` | `core/linguistics/ir_v0/witness_v0.py` |
| `sterling.meaning_state_digest.v0` | Meaning state digest | `core/linguistics/ir_v0/` |
| `sterling.episode_trace.v0` | Episode trace | `core/linguistics/ir_v0/episode_trace.py` |
| `sterling.myelin_sheath.v0` | `MyelinSheath` | `core/linguistics/ir_v0/myelin_sheath.py` |

### §3.3 Domain and Capability

| Schema ID | Python Type | Location |
|-----------|-------------|----------|
| `sterling.capability_descriptor.v1` | `CapabilityDescriptorV1` | `core/domains/capability_descriptor.py` |
| `sterling.capability_claim_registry.v1` | Claim registry | `core/domains/capability_claim_registry.py` |
| `sterling.primitive_spec.v1` | `PrimitiveSpecV1` | `core/domains/primitive_spec.py` |
| `sterling.conformance_suite.v1` | `ConformanceSuiteV1` | `core/domains/conformance_suite.py` |
| `sterling.domain_declaration.v1` | `DomainDeclarationV1` | `core/domains/domain_handshake.py` |
| `sterling.domain_session.v1` | Domain session | `core/domains/` |
| `sterling.p22.percept_observation.v0` | `PerceptObservationIRv0` | `core/proofs/p22/evidence_types_v0.py` |
| `sterling.p22.percept_state.v1` | `PerceptStateV1` | `core/capsules/p22/percept_state_v1.py` |
| `sterling.p22.render_intent.v1` | `RenderIntentV1` | `core/proofs/p22/verification_types_v1.py` |
| `sterling.p22.verification_report.v1` | `VerificationReportV1` | `core/proofs/p22/verification_types_v1.py` |

### §3.4 Induction and Hypothesis Pipeline

| Schema ID | Python Type | Location |
|-----------|-------------|----------|
| `sterling.hypothesis.v1` | `HypothesisIR` | `core/induction/hypothesis.py` |
| `sterling.episode_commitment.v0` | `EpisodeCommitmentV0` | `core/induction/episode_commitment.py` |
| `sterling.episode_batch_root.v1` | Episode batch root | `core/induction/` |
| `sterling.fixture_manifest.v1` | `FixtureManifestV1` | `core/induction/fixture_manifest.py` |
| `sterling.fixture.v1` | `FixtureIRV1` | `core/proofs/fixture_ir.py` |
| `sterling.sandbox_report.v1` | `SandboxReport` | `core/induction/sandbox_report.py` |
| `sterling.validation_report.v1` | Validation report | `core/induction/` |
| `sterling.e2e_certificate.v1` | E2E certificate | `core/induction/` |
| `sterling.promotion_decision.v1` | `PromotionDecisionRecord` | `core/induction/promotion_decision.py` |

### §3.5 Governance and Gates

| Schema ID | Python Type | Location |
|-----------|-------------|----------|
| `sterling.gate_result.v1` | `GateResult` (v1) | `core/governance/gate_verdict.py` |
| `sterling.gate_result.v2` | `GateResult` (v2) | `core/governance/gate_verdict.py` |
| `sterling.governance_failure_witness.v2` | `GovernanceFailureWitness` | `core/governance/failure_witness.py` |

### §3.6 Prior and Memory

| Schema ID | Python Type | Location |
|-----------|-------------|----------|
| `sterling.prior.v1` | `PriorIR` | `core/induction/prior_ir.py` |
| `sterling.prior_influence.v1` | `PriorInfluenceIR` | `core/induction/prior_ir.py` |
| `sterling.prior_artifact_envelope.v0` | `PriorArtifactEnvelopeV0` | `core/induction/prior_artifact_envelope.py` |
| `sterling.decision_packet.v1` | `DecisionPacket` | `core/memory/packet.py` |
| `sterling.projection_packet.v1` | `ProjectionPacketV1` | `core/memory/projection.py` |

### §3.7 K6 Fence and Benchmark

| Schema ID | Purpose | Location |
|-----------|---------|----------|
| `sterling.k6.fence_config.v1` | K6 fence configuration | `core/benchmarks/` |
| `sterling.k6.run_manifest.v1` | K6 run manifest | `core/benchmarks/` |
| `sterling.k6.results_summary.v1` | K6 results summary | `core/benchmarks/` |
| `sterling.k6.proof_bundle_manifest.v1` | K6 proof bundle manifest | `core/benchmarks/` |
| `sterling.k6.applied_operator_trace.v1` | Applied operator trace | `core/benchmarks/` |
| `sterling.k6.replay_verification_result.v1` | Replay verification result | `core/benchmarks/` |
| `sterling.k6.evidence_index.v1` | Evidence index | `core/benchmarks/` |
| `sterling.k6.bundle_evidence_index.v1` | Bundle-level evidence index | `core/benchmarks/` |
| `sterling.k6.per_config_evidence_index.v1` | Per-config evidence index | `core/benchmarks/` |
| `sterling.k6.policy.v1` | K6 evaluation policy | `core/benchmarks/` |

### §3.8 K1 Test Substrate

| Schema ID | Purpose | Location |
|-----------|---------|----------|
| `sterling.k1.test_substrate.v1` | K1 test substrate envelope | `core/induction/k1_test_substrate.py` |
| `sterling.k1.pn.payload.v1` | PN K1 substrate payload | `core/induction/` |
| `sterling.k1.wordnet.payload.v1` | WordNet K1 substrate payload | `core/induction/` |
| `sterling.k1.dialogue.payload.v1` | Dialogue K1 substrate payload | `core/induction/` |
| `sterling.k1.pn.operator_pattern.v1` | PN operator pattern prediction spec | `core/induction/` |
| `sterling.k1.wordnet.operator_pattern.v1` | WordNet operator pattern prediction spec | `core/induction/` |
| `sterling.k1.heldout_manifest.v1` | K1 held-out manifest | `core/induction/` |

---

## §4 Domain Pack Manifest v1 Field Reference

The schema at `docs/design/domain_pack_manifest.json` (JSON Schema draft 2020-12) defines the self-describing artifact contract for a Sterling domain pack. It is the governance-level equivalent of a `DomainDeclarationV1` combined with a `CapabilityDescriptorV1`.

| Section | Required | Semantics |
|---------|----------|-----------|
| `manifest_version` | Yes | Must be `"1.0.0"` |
| `domain.id` | Yes | Stable lowercase identifier (`^[a-z][a-z0-9_\-]{2,64}$`) |
| `domain.version` | Yes | Semver of this pack |
| `kernel_compat` | Yes | `min_kernel_version` / `max_kernel_version` semver range |
| `dimensionality.observability` | Yes | `fully_observed` or `partially_observed`; optional `uncertainty_rep` |
| `dimensionality.dynamics` | Yes | `deterministic` or `stochastic` |
| `dimensionality.time` | Yes | `none`, `discrete_steps`, or `continuous_time` |
| `dimensionality.state_axes` | Yes | Array of: `symbolic_graph`, `spatial_geometry`, `temporal_sequence`, `resources_inventory`, `permissions_security`, `social_agents`, `epistemic_uncertainty` |
| `dimensionality.action_surface` | Yes | `reversibility` + `latency_classes` (`instant`/`fast`/`slow`/`external_io`) |
| `dimensionality.cost_model` | Yes | `primary_drivers` + optional `branching_hint` |
| `schemas` | Yes | Content-addressed refs (uri + sha256 + version) for `task_intent_ext`, `state_ext`, `action_ext`, `outcome_ext` |
| `adapters` | Yes | `intent_adapter`, `observation_adapter`, `action_adapter` (local_module / rpc_http / rpc_grpc) |
| `operators.catalog` | Yes | Per-operator: `id`, `version`, `impl_sha256`, `io_schema`, `preconditions`, `effects`, `cost` |
| `operators.namespacing` | Yes | `namespace_prefix` (e.g., `"toy"` → operator IDs like `"TOY.MOVE_EDGE"`) |
| `invariants.registry` | Yes | Per-invariant: `id`, `severity` (info/warn/error/fatal), `lanes`, `check_adapter` |
| `policy.lanes` | Yes | Active lanes: `exploration`, `certification`, `promotion`, `production` |
| `policy.operator_admissibility` | Yes | `approved`, `experimental`, `forbidden`, `deprecated` selector sets |
| `policy.learned_operator_policy` | Yes | `mode` (disabled/exploration_only/allowed_if_promoted), promotion requirements |
| `certification.default_budgets` | Yes | `max_steps`, `max_branching`, `max_time_ms` |
| `certification.replay` | Yes | `mode` (strict/statistical/simulator_strict), `min_runs`, `confidence` |
| `certification.evidence_tiers` | Yes | `normative` fields + optional `debug` fields |
| `certification.result_schema_ref` | Yes | Content-addressed ref to result schema |

---

## §5 Schema Naming Conventions

| Convention | Example | Notes |
|-----------|---------|-------|
| JSON Schema files | `certificate.v1.json` | `<artifact_name>.v<N>.json` in `core/proofs/schemas/` |
| Internal schema IDs | `sterling.certificate.v1` | `sterling.<namespace>.<name>.v<N>` |
| Design/API schemas | `domain_pack_manifest.json` | Named by artifact type, versioned by file path |
| Python dataclass binding | `schema_id = "sterling.run_result.v1"` | Set as class constant, registered in `core/contracts/schema_registry.py` |

**Version increment rules:**
- Increment `vN` when field semantics change or required fields are added
- Backward-incompatible changes require a new version; old version remains valid until explicitly deprecated
- The `core/contracts/schema_registry.py` registry is the source of truth for what versions are currently active

---

## §6 Related Documents

- [Hashing Contracts](hashing_contracts_v1.md) — How all schema artifacts are content-addressed
- [Governance Certification Contract](governance_certification_contract_v1.md) — How certificates and verdicts reference these schemas
- [Proof Evidence System](proof_evidence_system_v1.md) — Evidence bundle schemas (H2/H3) in detail
- [Operator Registry Contract](operator_registry_contract_v1.md) — Operator inventory schema
- [World Adapter Protocol](world_adapter_protocol_v1.md) — Domain pack manifest relationship to WorldAdapter
- [Module Interdependencies](module_interdependencies_v1.md) — Where each schema-owning module lives
