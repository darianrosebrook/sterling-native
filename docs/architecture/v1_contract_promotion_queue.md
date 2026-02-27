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

## v2 Implementation Status Key

| Status | Meaning |
|--------|---------|
| **Implemented** | Core concept is implemented in v2 code with tests. v1 doc is largely superseded. |
| **Partial** | Some aspects implemented; others remain v1-only or need v2 formalization. |
| **Not started** | v2 has no implementation of this concept yet. |

---

## Queue

| File | Summary | Disposition | v2 Status | v2 Location | Required Edits |
|------|---------|-------------|-----------|-------------|----------------|
| `north_star.md` | Sterling as path-finding over semantic state space | Promote | **Implemented** | `search/` crate: best-first graph search over compiled state space; `harness/` crate: evidence bundles with proof-carrying artifacts | Update metaphor to reflect v2 search engine; ground in ByteState nodes + operator transitions + tape evidence |
| `core_constraints_v1.md` | 11 architectural invariants (INV-CORE-01..11) | Promote | **Implemented** | `docs/canonical/core_constraints.md` | Already promoted; reconcile if divergent |
| `core_features.md` | Meta-architectural feature overview | Promote | **Partial** | Path algebra, decay, SWM not yet in v2; search + audit trail implemented | Update operator taxonomy to S/M/P/K/C, strip v1 source paths |
| `conformance.md` | Theory conformance (TC-1..TC-11) | Promote | **Partial** | Determinism, replay, and audit conformance proven by lock tests; governance mode conformance needs v2 formalization | Update governance mode references to DEV/CERTIFIED |
| `evaluation_gates_v1.md` | Research targets and measured gates | Promote | **Partial** | EVAL-01/02/03 need re-measurement against v2 surfaces | Rename gates if needed; strip v1 roadmap language |
| `hashing_contracts_v1.md` | Canonical serialization and content addressing | Promote | **Implemented** | `kernel/src/proof/hash.rs` (SHA-256 + domain prefixes), `kernel/src/proof/canon.rs` (single canonical JSON impl) | Superseded — v2 implementation is canonical. Promote with minimal edits. |
| `knowledge_graph_contract_v1.md` | KG registration and domain isolation | Promote | **Not started** | No KG in v2 yet; `RegistryV1` (kernel) handles Code32↔ConceptID mapping | Update to compilation boundary framing |
| `operator_registry_contract_v1.md` | Operator signatures, registry, applicability | Promote | **Partial** | `kernel/src/operators/` (OperatorSignature, apply()), `kernel/src/carrier/registry.rs` (RegistryV1 bijective mapping); v2 has minimal operator set vs v1's 28 | Adopt S/M/P/K/C taxonomy; strip v1 source paths |
| `proof_evidence_system_v1.md` | Proof and evidence system contract | Promote | **Implemented** | `harness/src/bundle.rs` (ArtifactBundleV1, verify_bundle, VerificationProfile), `search/src/tape*.rs` (SearchTapeV1), `kernel/src/proof/` (replay_verify, canonical hash) | Largely superseded. Write `docs/canonical/search_evidence_contract.md` as the v2 replacement. |
| `reasoning_framework.md` | Path-finding architecture and reasoning loop | Promote | **Implemented** | `search/src/search.rs` (search loop), `search/src/frontier.rs` (BestFirstFrontier), `search/src/graph.rs` (SearchGraphV1) | Update references to canonical docs; strip v1 source paths |
| `schema_registry.md` | Versioned JSON schema registry | Promote | **Partial** | `schemas/` directory has JSON schemas; no runtime schema registry in v2 | Update schema list for v2; pattern is sound |
| `schemas.md` | Authoritative schema index | Promote | **Partial** | `schemas/` directory needs inventory update | Update schema inventory for v2 |
| `state_model_contract_v1.md` | State model and search contract | Promote | **Implemented** | `kernel/src/carrier/bytestate.rs` (ByteStateV1), `kernel/src/carrier/compile.rs` (compile()), `search/src/node.rs` (SearchNodeV1) | Align with ByteState; update governance references |
| `sterling_architecture_layers.md` | Four-layer architecture definition | Promote | **Implemented** | `kernel/` (carrier+state+operators+proof), `search/` (search), `harness/` (worlds+orchestration); 3-crate split with one-way deps | Map layers to v2 crate structure; verify alignment |
| `text_boundary_index.md` | Text-boundary contracts and identity model | Promote | **Not started** | No text processing in v2 yet | Update schema references for v2 |
| `text_io_contract_v1.md` | Text I/O trust boundaries (surface vs authority) | Promote | **Not started** | No IR parser or surface realizer in v2 | Update IR references; trust model is timeless |
| `value_function_components_v1.md` | Composable value function architecture | Promote | **Partial** | `search/src/scorer.rs` (ValueScorer trait, UniformScorer, TableScorer); no learned value heads yet | Update component head names; pattern carries forward |
| `world_adapter_protocol_v1.md` | Domain-agnostic world adapter protocol | Promote | **Implemented** | `harness/src/contract.rs` (WorldHarnessV1), `search/src/contract.rs` (SearchWorldV1); 3 test worlds | Align with compilation boundary; update source paths |
| `claim_schema_system_v1.md` | Structured claim representation | Promote | **Implemented** | `.caws/specs/` (YAML specs with acceptance criteria, invariants, falsifiers, test pointers) | Update to v2 claim/falsifier vocabulary |
| `discourse_intent_contract_v1.md` | Discourse structure, intent, speech acts | Rewrite | **Not started** | No discourse processing in v2 | Heavily v1; needs fresh v2 design |
| `governance_certification_contract_v1.md` | Policy enforcement and certification | Rewrite | **Partial** | `harness/src/bundle.rs` (VerificationProfile Base/Cert), `harness/src/policy.rs` (PolicySnapshotV1); v2 governance is simpler than v1's multi-mode system | Deeply tied to v1 pipeline; v2 governance model differs |
| `operator_induction_contract_v1.md` | Operator sketch lifecycle, 3-tier store | Rewrite | **Not started** | No induction pipeline in v2 | v1-specific pipeline; v2 induction model TBD |
| `operator_policy.md` | Policy-first operator selection | Rewrite | **Partial** | `search/src/policy.rs` (SearchPolicyV1 controls search behavior); no operator-level policy gating yet | Tied to v1 examples; concept carries forward |
| `linguistic_ir_contract_v0.md` | Four-partition typed IR with trust boundaries | Rewrite | **Not started** | No IR in v2 | v0 partition design is v1-specific |
| `text_hard_ir_contract_v1.md` | Hard IR sidecar for pragmatics | Rewrite | **Not started** | No IR in v2 | Specific partition design is v1; v2 may restructure |
| `semantic_realization_convergence.md` | Eight-subsystem convergence for text generation | Rewrite | **Not started** | No text generation in v2 | References v1-specific subsystems |
| `light_vs_full.md` | Sterling Light vs Full variants | Archive | N/A | Superseded by four-layer architecture | N/A |
| `module_interdependencies_v1.md` | v1 import graph and bootstrap order | Archive | N/A | Superseded by `kernel ← search ← harness` crate structure | N/A |
| `semantic_working_memory_contract_v0.md` | Semantic working memory (v0, doc-only) | Archive | N/A | No SWM in v2 | N/A |

---

## Summary

| Disposition | Count | v2 Implemented | v2 Partial | v2 Not started |
|-------------|-------|----------------|------------|----------------|
| Promote | 19 | 10 | 6 | 3 |
| Rewrite | 7 | 0 | 2 | 5 |
| Archive | 3 | — | — | — |

**Key observation:** 10 of the 19 promotable contracts are already largely implemented in v2 code. The remaining gap is primarily in linguistic/discourse processing (not started) and governance formalization (partial). The rewrite-class contracts are mostly in domains v2 hasn't entered yet (induction, discourse, text generation).
