---
authority: architecture
purpose: "Controlled intake mechanism for the 29 quarantined v1 canonical contracts. Each contract is categorized by disposition and required edits before it can enter docs/canonical/."
---
# v1 Contract Promotion Queue

**Rule: Nothing in `docs/reference/v1/canonical/` is normative for Sterling Native.** A contract becomes canonical only after it passes promotion criteria, is rewritten in v2 vocabulary, and is moved into `docs/canonical/`. Until then, it is historical reference material. Citing a quarantined contract as authority is an error.

**Promotion criteria** (from `docs/reference/v1/canonical/README.md`):
1. Aligns with the v2 compilation boundary spine
2. Uses DEV/CERTIFIED governance taxonomy
3. No parallel implementations (INV-CORE-12)
4. ByteTrace is canonical trace artifact (ADR 0002)
5. Has version metadata and change policy header

---

## Disposition Key

| Disposition | Meaning |
|-------------|---------|
| **Promote** | Concept is v2-relevant. Contract can be adapted to v2 vocabulary with targeted edits. |
| **Rewrite** | Concept is v2-relevant but document is too v1-specific for editing in place. Needs a fresh v2 doc. |
| **Archive** | v1-only concept that does not carry forward. Stays in `docs/reference/v1/canonical/` permanently. |

---

## Queue

| File | Summary | Disposition | Required Edits |
|------|---------|-------------|----------------|
| `north_star.md` | Sterling as path-finding over semantic state space | Promote | Minimal — update any v1 vocabulary references |
| `core_constraints_v1.md` | 11 architectural invariants (INV-CORE-01..11) | Promote | Already summarized in `docs/canonical/core_constraints.md`; reconcile if divergent |
| `core_features.md` | Meta-architectural feature overview | Promote | Update operator taxonomy to S/M/P/K/C, strip v1 source paths |
| `conformance.md` | Theory conformance (TC-1..TC-11) | Promote | Update governance mode references to DEV/CERTIFIED |
| `evaluation_gates_v1.md` | Research targets and measured gates | Promote | Rename gates if needed; strip v1 roadmap language |
| `hashing_contracts_v1.md` | Canonical serialization and content addressing | Promote | Align with ByteState hashing; minor terminology updates |
| `knowledge_graph_contract_v1.md` | KG registration and domain isolation | Promote | Update to compilation boundary framing |
| `operator_registry_contract_v1.md` | Operator signatures, registry, applicability | Promote | Adopt S/M/P/K/C taxonomy (ADR 0004); strip v1 source paths |
| `proof_evidence_system_v1.md` | Proof and evidence system contract | Promote | Update to ByteTrace authority (ADR 0002) |
| `reasoning_framework.md` | Path-finding architecture and reasoning loop | Promote | Update references to canonical docs; strip v1 source paths |
| `schema_registry.md` | Versioned JSON schema registry | Promote | Update schema list for v2; pattern is sound |
| `schemas.md` | Authoritative schema index | Promote | Update schema inventory for v2 |
| `state_model_contract_v1.md` | State model and search contract | Promote | Align with ByteState; update governance references |
| `sterling_architecture_layers.md` | Four-layer architecture definition | Promote | Map layers to v2 (Carrier/State/Operator/Search); verify alignment |
| `text_boundary_index.md` | Text-boundary contracts and identity model | Promote | Update schema references for v2 |
| `text_io_contract_v1.md` | Text I/O trust boundaries (surface vs authority) | Promote | Update IR references; trust model is timeless |
| `value_function_components_v1.md` | Composable value function architecture | Promote | Update component head names if changed; pattern carries forward |
| `world_adapter_protocol_v1.md` | Domain-agnostic world adapter protocol | Promote | Align with compilation boundary; update source paths |
| `claim_schema_system_v1.md` | Structured claim representation | Promote | Update to v2 claim/falsifier vocabulary |
| `discourse_intent_contract_v1.md` | Discourse structure, intent, speech acts | Rewrite | Heavily v1 (discourse phases, satisfaction FSM, source paths); needs fresh v2 design |
| `governance_certification_contract_v1.md` | Policy enforcement and certification | Rewrite | Deeply tied to v1 operator/induction pipeline; v2 governance model differs |
| `operator_induction_contract_v1.md` | Operator sketch lifecycle, 3-tier store | Rewrite | v1-specific pipeline (3-tier store, certification stages); v2 induction model TBD |
| `operator_policy.md` | Policy-first operator selection | Rewrite | Tied to v1 domain examples and implementation; concept carries forward |
| `linguistic_ir_contract_v0.md` | Four-partition typed IR with trust boundaries | Rewrite | v0 partition design (Surface/Syntax/Semantics/Hard) is v1-specific |
| `text_hard_ir_contract_v1.md` | Hard IR sidecar for pragmatics | Rewrite | Specific partition design is v1; v2 may restructure |
| `semantic_realization_convergence.md` | Eight-subsystem convergence for text generation | Rewrite | References v1-specific subsystems; concept relevant but document is not |
| `light_vs_full.md` | Sterling Light vs Full variants | Archive | Explicitly superseded by four-layer architecture |
| `module_interdependencies_v1.md` | v1 import graph and bootstrap order | Archive | Specific to v1 codebase structure; invalidated by v2 reorganization |
| `semantic_working_memory_contract_v0.md` | Semantic working memory (v0, doc-only) | Archive | Experimental v0 concept; likely superseded |

---

## Summary

| Disposition | Count | Action |
|-------------|-------|--------|
| Promote | 19 | Adapt to v2 vocabulary, update references, move to `docs/canonical/` |
| Rewrite | 7 | Write fresh v2 doc inspired by v1 contract, keep v1 as reference |
| Archive | 3 | Stays in `docs/reference/v1/canonical/` permanently |
