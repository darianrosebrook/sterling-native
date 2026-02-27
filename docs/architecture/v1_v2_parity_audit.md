---
status: "Living audit — updated 2026-02-27"
authority: architecture
date: 2026-02-27
purpose: "Capability-level migration map from Sterling v1 to v2. Defines proof obligations for supersession. Drives both docs/reference/v1 cleanup and v2 roadmap."
---
# V1→V2 Parity Audit (Capability Migration Map)

## Purpose

Sterling Native (v2) has crossed the threshold from substrate prototype to a coherent deterministic execution + search + evidence system. This audit enumerates the capabilities v1 delivered (or claimed), maps each to its v2 implementation status, and produces an explicit migration plan for what v2 must implement (or intentionally deprecate) to fully supersede v1 documentation under `docs/reference/v1/`.

This is not a doc-promotion list (that's [`v1_contract_promotion_queue.md`](v1_contract_promotion_queue.md)). It is a capability-level parity map with proof obligations.

## Definitions

Parity means one of three things per capability:

1. **Semantic parity**: v2 reproduces the v1 capability's externally observable behavior under equivalent inputs.
2. **Contract parity**: v2 provides an equal-or-stronger formal contract (schema + verification + tests) even if internals differ.
3. **Intentional divergence**: v2 replaces or deletes the v1 capability; the rationale and replacement contract are recorded.

A capability is **superseded** only when:
- It has a v2 contract surface (in `docs/canonical/`),
- It has mechanical enforcement (verification + lock tests),
- And the v1 doc is annotated with a stable mapping.

---

## Executive Summary

### What v2 already proves (hard substrate wins)

- Deterministic compile→apply execution with replay verification.
- Deterministic best-first search with auditable transcript.
- Hot-loop search tape with chain-hash integrity and Cert tape→graph equivalence.
- Content-addressed bundles with fail-closed verification (Base vs Cert profiles).
- Scorer advisory-only contract enforced by API shape.

### What v2 does not yet provide (reasoning-system wins)

- Truth-regime diversity beyond deterministic lattice/search worlds.
- Partial observability with belief discipline and probe operators.
- Stochastic world certification and distributional evaluation.
- Learning/induction pipeline producing promotable operators.
- Memory substrate (SWM, landmarks, decay, value-learning beyond simple scorers).
- Text boundary/realization contracts.

### What comes next (not "port more code" — "port more proof obligations")

v1 is not a monolith to port. It is a catalog of *proof obligations* that the v2 substrate must eventually host — either natively in Rust, or as a Python control plane that consumes Rust evidence. The next work is not "port more v1 code to Rust"; it is "build truth-regime worlds and an operator registry so the substrate proves the next class of claims."

### Strategic conclusion

v2 has built the verification-grade engine block. v1 supersession now depends on breadth capabilities and their governance: worlds, memory, induction, and text. Eight guardrails (§Guardrails below) prevent the next phase from creating long-lived drift.

---

## Capability Parity Matrix

### A. Execution Substrate (Carrier + State + Operators)

#### A1. Compilation boundary (payload→ByteState)

| Field | Value |
|-------|-------|
| v1 reference | `docs/reference/v1/canonical/state_model_contract_v1.md`, `hashing_contracts_v1.md` |
| v2 status | **Implemented** |
| v2 code | `kernel/src/carrier/compile.rs`, `kernel/src/carrier/bytestate.rs`, `kernel/src/carrier/registry.rs` |
| v2 canonical doc | `docs/canonical/bytestate_compilation_boundary.md` |
| CAWS spec | SPINE-001 M1 |
| Lock tests | `tests/lock/tests/s1_m1_golden_fixtures.rs` (8 tests), `tests/lock/tests/s1_m1_determinism.rs` (10 tests), `tests/lock/tests/s1_m1_crossproc.rs` (4 env variants) |
| Parity target | **Contract parity (stronger)** — v2 has golden fixtures from v1 oracle, typed failures, and cross-OS determinism. v1 had no equivalent mechanical verification. |
| Proof obligations | Golden fixtures locked. Replay verification demonstrates bit-identical state. Schema/registry changes must force epoch/version bump or fail closed. |
| Gaps | No explicit "epoch bump / schema evolution" lock test that forces a version boundary. |
| Next tasks | Add epoch-evolution lock test: change schema, verify old bundles fail or require migration. |

#### A2. Canonicalization + content addressing

| Field | Value |
|-------|-------|
| v1 reference | `docs/reference/v1/canonical/hashing_contracts_v1.md` |
| v2 status | **Implemented** |
| v2 code | `kernel/src/proof/hash.rs` (SHA-256 + domain prefixes), `kernel/src/proof/canon.rs` (single canonical JSON) |
| v2 canonical doc | `docs/canonical/bytestate_compilation_boundary.md` §hashing |
| CAWS spec | SPINE-001 M1 |
| Lock tests | `s1_m1_determinism.rs::one_canonical_json_impl` (greps for alternative impls), `s1_m1_determinism.rs::ordering_invariance`, `s1_m1_golden_fixtures.rs::hash_v1_vectors` |
| Parity target | **Contract parity** — single canonicalizer invariant enforced by test. |
| Proof obligations | Single canonicalizer. Domain separation registry enumerated. |
| Gaps | Domain prefix constants are defined in two crates (`kernel/src/proof/hash.rs` and `harness/src/bundle.rs`). No single "domain registry" canonical doc. |
| Next tasks | Document hash domain registry as a canonical page listing all `DOMAIN_*` constants. |

#### A3. Operator taxonomy + dispatch

| Field | Value |
|-------|-------|
| v1 reference | `docs/reference/v1/canonical/operator_registry_contract_v1.md`, `operator_policy.md` |
| v2 status | **Partial** |
| v2 code | `kernel/src/operators/signature.rs` (OperatorSignature, OperatorCategory S/M/P/K/C), `kernel/src/operators/apply.rs` (apply() with SET_SLOT) |
| v2 canonical doc | `docs/canonical/glossary.md` §Operator Layer |
| CAWS spec | SPINE-001 M2 (operator dispatch), SC-001 M1 (search operators) |
| Lock tests | `s1_m2_determinism.rs` (apply round-trip), `sc1_search_determinism.rs` (search operator legality) |
| Parity target | **Contract parity** for operator contract shape; **Not started** for operator breadth. |
| Gaps | v1 had 28 operators across 5 categories. v2 has SET_SLOT plus search-level expansion. No operator registry artifact, no capability-gating policies, no induced operators, no operator lifecycle. SET_SLOT is hardcoded in `apply()`, not invoked via stable op_id from a registry. |
| Next tasks | **Operator Registry MVP (Phase 0)** — see §Operator Registry MVP below. Move SET_SLOT into a content-addressed `OperatorRegistryV1` artifact. This is prerequisite to all subsequent phases. |

#### A4. Deterministic replay (carrier level)

| Field | Value |
|-------|-------|
| v1 reference | `docs/reference/v1/canonical/proof_evidence_system_v1.md` |
| v2 status | **Implemented** |
| v2 code | `kernel/src/proof/replay.rs` (replay_verify), `kernel/src/carrier/trace_writer.rs` + `trace_reader.rs` (.bst1 format) |
| v2 canonical doc | `docs/canonical/philosophy.md` §4 (evidence layers) |
| CAWS spec | SPINE-001 M2, M4 |
| Lock tests | `s1_m2_determinism.rs`, `s1_m2_crossproc.rs`, `s1_m2_divergence.rs` (O(1) divergence localization) |
| Parity target | **Contract parity (stronger)** — v1 had no binary trace format or O(1) divergence localization. |
| Proof obligations | Bit-identical ByteTrace across runs. Divergence localized to exact frame. |
| Gaps | None for carrier level. |

---

### B. Search + Scoring

#### B1. Frontier search engine

| Field | Value |
|-------|-------|
| v1 reference | `docs/reference/v1/canonical/reasoning_framework.md`, `search_complexity.md` |
| v2 status | **Implemented** |
| v2 code | `search/src/search.rs` (search loop), `search/src/frontier.rs` (BestFirstFrontier), `search/src/node.rs` (SearchNodeV1), `search/src/graph.rs` (SearchGraphV1) |
| v2 canonical doc | `docs/canonical/glossary.md` §Search Layer, `docs/canonical/search_evidence_contract.md` |
| CAWS spec | SC-001 M1 |
| Lock tests | `sc1_search_determinism.rs` (12 tests: determinism N=10, graph completeness, metadata binding, expansion ordering, dead-end tagging), `sc1_crossproc.rs` |
| Parity target | **Contract parity** — v2 search is deterministic with auditable transcript; v1 search had 3 implementations with drift risk. |
| Proof obligations | Deterministic expansion ordering locked. Graph transcript completeness (no silent pruning). |
| Gaps | Single search strategy (best-first). No multi-strategy scaffolding. No backtracking search. |
| Next tasks | Multi-heuristic scoring scaffolding (advisory-only). Consider whether backtracking is a v2 goal. |

#### B2. Scoring surfaces

| Field | Value |
|-------|-------|
| v1 reference | `docs/reference/v1/canonical/value_function_components_v1.md` |
| v2 status | **Partial** |
| v2 code | `search/src/scorer.rs` (ValueScorer trait, UniformScorer, TableScorer) |
| CAWS spec | SC-001 M1 (UniformScorer), M3.2 (TableScorer) |
| Lock tests | `sc1_search_determinism.rs` (scorer advisory-only), `sc1_m3_2_table_scorer.rs` (12 tests: reorder proof, digest binding, tamper detection, round-trip) |
| Parity target | **Contract parity** for "scoring is advisory-only" (proven). **Not started** for learned value decomposition. |
| Gaps | No learned value heads. No composable value components (structural + learned hybrid). No "why this plan" explanation surfaces. |
| Next tasks | Decide: port v1 value-component system or redesign around v2 ValueScorer trait. |

#### B3. Search health metrics

| Field | Value |
|-------|-------|
| v1 reference | (no v1 equivalent) |
| v2 status | **Implemented** |
| v2 code | `search/src/graph.rs` (compute_health_metrics, SearchHealthMetricsV1) |
| CAWS spec | SC-001 M3.3 |
| Lock tests | `sc1_m3_3_health_metrics.rs` (8 tests: golden snapshot, determinism, histogram invariants, non-binding proof) |
| Parity target | **Intentional divergence** — new v2 capability. |

---

### C. Evidence + Proof

#### C1. Proof-carrying artifacts + verification

| Field | Value |
|-------|-------|
| v1 reference | `docs/reference/v1/canonical/proof_evidence_system_v1.md`, `conformance.md` |
| v2 status | **Implemented** |
| v2 code | `harness/src/bundle.rs` (ArtifactBundleV1, verify_bundle, VerificationProfile, BundleVerifyError), `harness/src/bundle_dir.rs` (write/read/verify persistence) |
| v2 canonical doc | `docs/canonical/search_evidence_contract.md` |
| CAWS spec | SPINE-001 M3 (bundle model), SC-001 M3.0 (search bundle persistence), SC-001 M4 (tape verification, profiles) |
| Lock tests | `s1_m3_harness.rs`, `s1_m4_bundle_dir.rs`, `sc1_m3_persistence.rs`, `sc1_m4_tape_bundle.rs` (20 tests: tamper detection, fail-closed, Base/Cert profiles, round-trip) |
| Parity target | **Contract parity (stronger)** — v1 had no content-addressed bundles, no fail-closed verification pipeline, no verification profiles. |
| Proof obligations | Base vs Cert profile semantics stable and documented. Bundle persistence fail-closed at read boundary. |
| Gaps | None for current scope. |

#### C2. Search tape (hot-loop evidence)

| Field | Value |
|-------|-------|
| v1 reference | (no v1 equivalent — this is a v2 improvement) |
| v2 status | **Implemented** |
| v2 code | `search/src/tape_writer.rs`, `search/src/tape_reader.rs`, `search/src/tape_render.rs`, `search/src/tape.rs` |
| v2 canonical doc | `docs/canonical/search_evidence_contract.md` §Tape verification |
| CAWS spec | SC-001 M4, TAPE-001 (performance), TAPE-002 (tape as bundle artifact) |
| Lock tests | `sc1_m4_tape_bundle.rs` (20 tests), `sc1_tape_equiv.rs` |
| Parity target | **Intentional divergence** — new capability with no v1 analogue. |
| Proof obligations | Chain hash integrity. Cert tape→graph equivalence. Header binding to authoritative artifacts. |

#### C3. Policy snapshots

| Field | Value |
|-------|-------|
| v1 reference | `docs/reference/v1/canonical/governance_certification_contract_v1.md` (partial) |
| v2 status | **Implemented** |
| v2 code | `harness/src/policy.rs` (PolicySnapshotV1, PolicyConfig, build_policy, enforce_*) |
| CAWS spec | SPINE-001 M5 |
| Lock tests | `s1_m5_policy.rs` (policy enforcement, snapshot binding, budget limits) |
| Parity target | **Contract parity** for "policy is an artifact." v1 governance campaign model is not ported (intentional — v2 uses simpler Base/Cert). |

---

### D. World Adapter Protocol + Transfer

#### D1. World harness contract

| Field | Value |
|-------|-------|
| v1 reference | `docs/reference/v1/canonical/world_adapter_protocol_v1.md` |
| v2 status | **Implemented (structural)** |
| v2 code | `harness/src/contract.rs` (WorldHarnessV1), `search/src/contract.rs` (SearchWorldV1) |
| v2 worlds | `harness/src/worlds/rome_mini.rs`, `rome_mini_search.rs`, `slot_lattice_search.rs` + `slot_lattice_regimes.rs` (6 regimes) |
| CAWS spec | SPINE-001 (RomeMini), SC-001 M1 (RomeMiniSearch), SC-001 M3.1 (SlotLatticeSearch) |
| Lock tests | `sc1_m3_1_slot_lattice.rs` (12 tests: 6 regimes with threshold assertions) |
| Parity target | **Contract parity** for harness contract shape. **Not started** for truth-regime diversity. |
| Gaps | All 3 worlds are deterministic lattice/search worlds. No tool-use, partial observability, or stochastic worlds. |
| Proof obligations | At least one world per truth regime passes the same harness without bespoke patches. |
| Next tasks | Build truth-regime worlds (see supersession plan below). |

#### D2. Transfer packs / certification packets

| Field | Value |
|-------|-------|
| v1 reference | `docs/reference/v1/capability_campaign_plan.md`, `PROVING-GROUNDS.md` |
| v2 status | **Partial** |
| v2 code | Bundle verification pipeline serves as the structural base for transfer evidence. |
| Gaps | No formal `TransferPackV1` schema. No template-driven generator. No cross-world claim equivalence tests. |
| Next tasks | Define `TransferPackV1` schema tying claims → artifacts → verification profile. |

---

### E. Memory (SWM, Landmarks, Decay)

#### E1. Semantic Working Memory

| Field | Value |
|-------|-------|
| v1 reference | `docs/reference/v1/canonical/semantic_working_memory_contract_v0.md`, `core_features.md` |
| v2 status | **Not started** |
| Parity target | **TBD** — v2 likely needs a redesigned substrate. v1 SWM was v0/doc-only; the v1 contract promotion queue marks it as Archive. |
| Proof obligations | Memory artifacts must be content-addressed and replay-linked. Memory updates must be governed (operators), not ad hoc. |

#### E2. Landmarks + compression

| Field | Value |
|-------|-------|
| v1 reference | `docs/reference/v1/canonical/core_features.md` §landmarks |
| v2 status | **Not started** |
| Parity target | **TBD** — landmark discovery depends on episode history, which depends on worlds + memory. |

#### E3. Decay / activation dynamics

| Field | Value |
|-------|-------|
| v1 reference | `docs/reference/v1/canonical/core_features.md` §decay, path algebra |
| v2 status | **Not started** |
| Parity target | **TBD** |

---

### F. Learning / Induction

#### F1. Operator induction pipeline

| Field | Value |
|-------|-------|
| v1 reference | `docs/reference/v1/canonical/operator_induction_contract_v1.md` |
| v2 status | **Not started** |
| Parity target | **Intentional redesign** — v1's 120-file pipeline will be collapsed. v2 targets 5 modules with evaluators as extension point (per `clean_sheet_architecture.md` §4). |
| Proof obligations | Propose→evaluate→promote loop produces an operator with lock tests and regression gates. No silent regressions on previously certified claims. |

---

### G. Tool Safety (Transactional Semantics)

#### G1. Tool world + transactional operators

| Field | Value |
|-------|-------|
| v1 reference | `docs/reference/v1/canonical/governance_certification_contract_v1.md` (tool transcript sections) |
| v2 status | **Not started** |
| Parity target | **Contract parity** — stage/commit/rollback semantics with proof trail. |
| Proof obligations | Stage/commit/rollback verified by replay evidence. 100% tool actions have transcripts. No side effects without commit. |
| Next tasks | Build Transactional KV Store truth-regime world (see supersession plan). |

---

### H. Text / Realization / Boundary Contracts

#### H1. Text IO boundary, IR partitions

| Field | Value |
|-------|-------|
| v1 reference | `docs/reference/v1/canonical/text_io_contract_v1.md`, `text_hard_ir_contract_v1.md`, `linguistic_ir_contract_v0.md`, `text_boundary_index.md` |
| v2 status | **Not started** |
| Parity target | **TBD** — depends on whether v2 pursues the v1 four-partition IR or a new design. |
| Proof obligations | Minimal: a text boundary demo that enforces "surface is non-authoritative, IR is authoritative." |
| Open decision | Whether v2 implements v1 IR partitioning or designs a new realization pipeline. |

#### H2. Discourse / speech act contracts

| Field | Value |
|-------|-------|
| v1 reference | `docs/reference/v1/canonical/discourse_intent_contract_v1.md` |
| v2 status | **Not started** |
| Parity target | **TBD** — v1 contract promotion queue marks this as Rewrite. |

---

## Supersession Plan

### Phase 1: Declare what is already superseded (immediate)

For each v1 doc whose capability is **Implemented**, add a header note to the v1 file:

```
> **Superseded by v2**: {v2 canonical doc} + {code locations}
> v2 evidence: {CAWS spec} + {lock test files}
```

**Target v1 docs for Phase 1 annotation:**

| v1 doc | Superseded by |
|--------|--------------|
| `hashing_contracts_v1.md` | `kernel/src/proof/hash.rs` + `canon.rs`; SPINE-001 M1 |
| `proof_evidence_system_v1.md` | `docs/canonical/search_evidence_contract.md` + `harness/src/bundle.rs`; SPINE-001 M3, SC-001 M4 |
| `state_model_contract_v1.md` | `docs/canonical/bytestate_compilation_boundary.md` + `kernel/src/carrier/`; SPINE-001 M1 |
| `reasoning_framework.md` | `search/src/search.rs` + `frontier.rs` + `graph.rs`; SC-001 M1 |
| `sterling_architecture_layers.md` | `docs/canonical/philosophy.md` §1 (four-layer authority stack) |
| `world_adapter_protocol_v1.md` | `harness/src/contract.rs` + `search/src/contract.rs`; SC-001 M1 |
| `claim_schema_system_v1.md` | `.caws/specs/` YAML structure; CAWS workflow |
| `core_constraints_v1.md` | `docs/canonical/core_constraints.md` |
| `north_star.md` | Still valid as thesis; v2 search engine is the realization |

### Phase 2: Close parity gaps for the endgame narrative

Priority order (each unlocks a success-rubric claim):

| Priority | Capability | Unlocks | Estimated scope |
|----------|-----------|---------|-----------------|
| 1 | Tool truth-regime world | Rubric #4 (tool safety) | New world + 10-15 lock tests |
| 2 | Partial observability world | Rubric #6 (belief discipline) | New world + probe operators + 10-15 lock tests |
| 3 | Stochastic world | Rubric #7 (seed/witness certification) | New world + seed binding + 10-15 lock tests |
| 4 | Induction MVP | Rubric #8 (learning) | New module + propose/evaluate/promote pipeline |
| 5 | Memory substrate MVP | Strengthens #5 and #8 | Landmark-like compression as governed operator output |
| 6 | Text boundary MVP | Endgame narrative completeness | Minimal text IO demo enforcing surface non-authority |

### Phase 3: Formal deprecation

Once a capability is superseded:
1. Mark v1 doc as archived reference (header annotation).
2. Promote or rewrite into v2 canonical doc.
3. Ensure at least one CAWS spec anchors the evidence.

---

## Open Decisions

These must be resolved to complete parity. Each should become a decision record (ADR) when resolved.

1. **Value function architecture**: Port v1 composable component system (structural + learned heads with hybrid combiner) vs redesign around v2 `ValueScorer` trait?
2. **Memory substrate**: Is SWM a first-class artifact suite (content-addressed, bundle-linked) or an operator-defined side channel?
3. **Text boundary**: Does v2 implement v1's four-partition IR (Surface/Syntax/Semantics/Hard) or design a new realization pipeline?
4. **Governance depth**: Does Base/Cert expand into a richer certification campaign model, or remain minimal?

---

## Relationship to other documents

- **[`v1_contract_promotion_queue.md`](v1_contract_promotion_queue.md)**: Tracks doc-level promotion status. This audit tracks capability-level parity.
- **[`v2_success_rubric.md`](v2_success_rubric.md)**: The scorecard this audit's supersession plan is designed to advance.
- **[`clean_sheet_architecture.md`](clean_sheet_architecture.md)**: The target architecture. This audit measures progress against that target.
- **[`docs/canonical/search_evidence_contract.md`](../canonical/search_evidence_contract.md)**: The v2 canonical doc that supersedes v1's proof/evidence system for the search layer.
