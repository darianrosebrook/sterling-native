---
status: "Living audit — updated 2026-02-28"
authority: architecture
date: 2026-02-28
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

## Capability Import Backlog

v1 is not code to port — it is a catalog of proof obligations. Each group below identifies what v1 delivered, what the v2 substrate must host, and the first world or milestone that exercises it.

### Import Group A: Truth-regime world diversity

**Why**: Current worlds are deterministic lattice variants. They prove determinism and search evidence but do not force the hard semantics v1 cared about: tool rollback, partial observability, stochasticity. The clean sheet (`clean_sheet_architecture.md` §6) explicitly calls for a truth-regime suite.

**Import obligations from v1**:

| Truth regime | v1 source | What v2 must prove | First world |
|-------------|-----------|--------------------|----|
| Tool safety | `governance_certification_contract_v1.md` (tool transcript sections) | Transactional semantics (stage/commit/rollback) with auditable transcripts. Every external interaction has a transcript artifact bound into the bundle. | Transactional KV Store |
| Partial observability | `core_features.md` (belief, probes), Mastermind test-scenario | Belief discipline via probe operators and trace-visible belief changes. Belief set monotonicity under probes. No hidden observation channels. | Mastermind-like world |
| Stochastic certification | `conformance.md` (distributional evaluation) | Seed/witness binding so certification binds to evidence, not "the environment." Exact replay from recorded evidence; distributional evaluation over seed sets. | Slippery Grid world |

**Guardrail (G6)**: Enforce "tools are modeled, not executed" initially. Stage/commit/rollback should be trace-visible contracts with transcripts; real side effects come later behind an explicit non-cert mode.

### Import Group B: Operator registry + lifecycle

**Why**: Without a registry, Rust proves deterministic search but the semantic authority for "what operators exist and what they mean" remains Python-only and unverifiable at the boundary. If operator identity lives in code structure (function names, match arms) rather than in data, every new operator gets added "the easy way" and you recreate the v1 drift pattern.

**Import obligations from v1**:

- **Stable operator identity** — external to code structure, content-addressed.
- **Operator contract** — mechanically checkable at the boundary (preconditions, effects, args schema).
- **Legality checking** — fail-closed on unknown operators, precondition failure.
- **Operator-set digest** — bound into evidence so drift is detectable per-run.
- **Packaging/install surface** — so operator sets are auditable, transferable artifacts.

**v1 reference**: `operator_registry_contract_v1.md`, `operator_policy.md`, `core/operators/engine/registry.py`.

**Guardrail (G7)**: This is Phase 0, not Phase 2. See §Operator Registry MVP below.

### Import Group C: Governance / certification campaigns

**Why**: The clean sheet argues for simplifying governance (DEV vs CERTIFIED, two modes, one policy object). The retrospective's "governance is the system" point means deterministic replay + refusal contracts + promotion gates are cognition infrastructure, not scaffolding.

**Import obligations from v1**:

- **Typed verdicts/refusals** as first-class artifacts (not log messages).
- **Campaign notion** (even if simplified) binding: policy snapshot, operator set, evidence bundle(s), and acceptance criteria.
- **Fail-closed enforcement** at the governance level (not just type-system fail-closed).

**v1 reference**: `governance_certification_contract_v1.md`, `conformance.md`, `core/governance/gate_verdict.py`.

**Guardrail (G1)**: Write an ADR pinning certification authority. If Python is the control plane, every Python cert must be reducible to a set of Rust-verified artifacts + explicit policy/campaign metadata.

### Import Group D: Induction pipeline (collapsed)

**Why**: The "scientific method loop" claim (rubric #8) requires a propose→evaluate→promote cycle. The clean sheet (`clean_sheet_architecture.md` §4) gives the compression target: 5 modules, evaluators are the extension point, promotion packaging is uniform.

**Import obligations from v1**:

- **Propose→evaluate→promote loop** producing promotable operators (or operator policies) with regression gates.
- **Standard evaluation packet format** so future worlds slot in without bespoke pipelines.
- Start with inducing a policy/scorer table; graduate to inducing operator definitions once the registry exists.

**v1 reference**: `operator_induction_contract_v1.md`, `core/induction/`.

### Import Group E: Memory MVP

**Why**: Without memory artifacts, learning and transfer remain narratively true but mechanically absent. v1 had SWM, decay, landmarks, episode chaining. v2 does not need to port it wholesale — but it needs an MVP memory artifact suite that is content-addressed and replay-linked.

**Import obligations from v1**:

- **Episode identity + durable summaries** (landmarks or equivalent).
- **Governed memory updates** (operators or explicit post-pass artifacts), not ad hoc mutable objects.
- **Content-addressed, bundle-linked** memory artifacts.

**v1 reference**: `semantic_working_memory_contract_v0.md`, `core_features.md` §landmarks.

### Import Group F: Text boundary

**Why**: Realization depends on higher-level linguistic structures the Rust substrate doesn't model. But parity still requires a minimal contract enforcing "surface is non-authoritative; IR is authoritative" (ADR 0003 already establishes this principle for neural components).

**Import obligations from v1**:

- A minimal text boundary demo: parse/render components as advisory, never authority.
- A verifiable realization artifact surface (even if the realizer remains Python).

**v1 reference**: `text_io_contract_v1.md`, `text_hard_ir_contract_v1.md`, `linguistic_ir_contract_v0.md`.

---

## Guardrails

Eight footgun risks identified during cross-codebase audit. These are authority and format problems that create long-lived drift if not addressed early.

### G1. Two v2 codebases without a hard authority boundary

**Risk**: "Rust owns the corridor; Python owns everything above" is workable, but becomes a corner if you don't force a single certification authority decision. Otherwise two quasi-authorities emerge: Rust verifies bundles, Python issues governance claims that may or may not be mechanically reducible to those bundles.

**Guardrail**: Write an ADR pinning this as an invariant. If the decision is "Python is certification control plane, Rust is evidence generator + verifier," then the interface contract comes first: every Python cert must be reducible to a set of Rust-verified artifacts + explicit policy/campaign metadata. Moving more governance into Rust is a deliberate phase change, not emergent creep.

**Status**: Resolved — ADR 0006.

### G2. Competing evidence packaging (ArtifactBundleV1 vs H2/TD-12)

**Risk**: The biggest long-term coherence risk. Rust bundles are clean, deterministic, fail-closed. Python's H2/TD-12 is a richer governance attestation system. If both evolve independently, you pay a permanent "translation tax" and every future claim must answer "which bundle is canonical?"

**Guardrail options (choose one and document it)**:
- **A) Nesting**: TD-12/H2 becomes a wrapper that *imports* an ArtifactBundleV1 digest basis as a required substrate artifact. Governance sits on top of a Rust bundle, never parallel.
- **B) Parallel, disjoint scopes**: Rust bundles certify execution/search integrity only; Python certifies cross-run/campaign claims. Requires a strict "no overlapping claims" rule.

**Status**: Resolved — ADR 0007.

### G3. Cross-codebase compatibility without equivalence harness

**Risk**: Docs assert shared wire formats and compatibility. Without mechanical enforcement, drift appears in tiny places (domain prefix registries, canonical JSON edge cases, schema descriptor differences), and docs quietly become aspirational again.

**Guardrail**: Treat cross-codebase equivalence as a first-class capability with lock tests. See §Cross-Codebase Equivalence Harness below.

### G4. Hash domain prefix registry fragmentation

**Risk**: Domain constants defined across crates (`kernel/src/proof/hash.rs`, `harness/src/bundle.rs`, `search/src/node.rs`). New domains get added opportunistically → eventual collision or accidental semantic changes.

**Guardrail**: Create a single canonical "hash domain registry" doc plus a lock test that enumerates all `DOMAIN_*` constants and fails on duplicates or unregistered additions.

### G5. Search evidence schema ossification before truth-regime worlds land

**Risk**: SearchGraphV1 / SearchTapeV1 are canonical surfaces. Once tool-use / partial observability / stochastic worlds land, you may need additional binding fields or event types that don't fit without version churn.

**Guardrail**: Decide the extension mechanism now:
- **Strict version bumps** with explicit migration rules (fine, but commit to it), OR
- **Extension blocks** with domain-separated sub-records that remain canonical but don't force schema breaks.

Don't pretend the current schema won't face extension pressure once worlds diversify.

**Status**: Resolved — ADR 0008. Additive fields within a version; breaking changes require bumps.

### G6. Tool worlds executing real side effects

**Risk**: A transactional tool world can be modeled as pure state transitions, but only if "tool I/O" is an evidence artifact (transcript) bound into the verification story. If you implement tool worlds that actually perform side effects at runtime ("just for the demo"), you'll fight your own determinism model.

**Guardrail**: Enforce "tools are modeled, not executed" at first. Stage/commit/rollback should be trace-visible contracts with transcripts. Real side effects come later behind an explicit non-cert mode.

### G7. Delaying operator registry too long

**Risk**: You can build truth-regime worlds with a minimal operator surface, but if stable operator IDs + signature legality + packaging are postponed, semantics get encoded into "world-specific candidate enumeration." Later induction/promotion will have nothing stable to hook into.

**Guardrail**: Operator registry MVP is Phase 0. See §Operator Registry MVP. Don't import the whole v1 operator universe; instead define a very small registry that makes governance and learning possible later: stable IDs, signature masks, legality checks, minimal packaging/export surface.

### G8. Parallel docs without enforcement

**Risk**: Two parallel documents drift: parity audit in sterling-native and supersession map in sterling.

**Guardrail**: This parity audit is the primary source of truth for cross-codebase capability mapping. Sterling's `v1_v2_supersession_map.md` links to it and only summarizes.

---

## Operator Registry MVP

This is Phase 0 — prerequisite to truth-regime worlds and all subsequent phases. The goal is not to import the whole v1 operator universe, but to make operator identity and contract mechanically checkable so governance and learning can hook in later.

### Design

**1. OperatorRegistryV1 as normative artifact**

A canonical, content-addressed JSON artifact included in every bundle and bound into verification.

```
{
  "entries": [
    {
      "arg_byte_count": 12,
      "category": "M",
      "contract_epoch": "v1",
      "cost_model": "unit",
      "effect_kind": "writes_one_slot_from_args",
      "effect_mask": { "active": [], "dimensions": [0, 0], "values": [] },
      "name": "SET_SLOT",
      "op_id": [1, 1, 1, 0],
      "precondition_mask": { "active": [], "dimensions": [0, 0], "values": [] },
      "status_effect_mask": { "dimensions": [0, 0], "values": [] }
    }
  ],
  "schema_version": "operator_registry.v1"
}
```

Note: `operator_set_digest` is NOT embedded in `operator_registry.json`.
It is computed externally as `canonical_hash(DOMAIN_BUNDLE_ARTIFACT, <bytes above>)`
and bound into `verification_report.json`, `search_graph.json` metadata, and
tape header. This avoids the self-referential digest problem.

Key: the registry is authoritative data. Rust code is an implementation of entries in that data. This decouples extensibility from crate/module churn.

**2. apply() requires registry snapshot (fail-closed)**

```
apply(state, op_id, op_args, registry: &OperatorRegistryV1) -> Result<(new_state, step_record), ApplyError>
```

Fail-closed on: unknown `op_id`, args decode mismatch vs `args_schema`, precondition failure, reserved fields. No implicit "current registry" and no "default operator set." The registry snapshot is part of the evidence chain.

**3. Bind operator-set identity into evidence**

Same pattern as policy/scorer binding:
- `verification_report.json`: include `operator_registry_digest`
- `search_graph.json` metadata: include `operator_registry_digest`
- `SearchTapeV1` header: include `operator_registry_digest`

This prevents the most subtle drift: "same tape/graph shape, different operator meanings."

**4. Dispatch is implementation, not contract**

In Rust: `BTreeMap<OpId, &'static dyn OperatorImpl>` for builtins. Later: `InstalledOperatorBundle` for induced/imported operators. The dispatch map satisfies the registry; it is not the registry. Verification can assert "every registry entry used in this run had an implementation present."

**5. Lock tests**

- **No apply without registry**: grep/deny calling lower-level apply paths that bypass registry checking.
- **Registry digest binding**: tamper registry bytes → bundle verification fails.
- **Stable ordering**: canonical JSON ordering invariance for the registry.
- **Unknown op fail-closed**: applying a non-existent `op_id` is a typed error, never a no-op or panic.
- **Operator contract mismatch**: if implementation returns effects outside declared masks, fail.

### MVP sequence

1. Create `OperatorRegistryV1` schema + canonical doc (in `docs/canonical/`).
2. Add `operator_registry.json` as normative bundle artifact + bind digest in report/graph/tape header.
3. Refactor `SET_SLOT` to be a registered operator invoked via `op_id`.
4. Add lock tests (above).

### Naming disambiguation

Two "registries" coexist as distinct artifacts:

| Name | Type | Field in metadata | What it is |
|------|------|-------------------|------------|
| `RegistryV1` | `kernel/src/carrier/registry.rs` | `registry_digest` | Code32↔ConceptID bijective mapping (compilation boundary) |
| `OperatorRegistryV1` | New | `operator_set_digest` | Operator catalog: op_ids, signatures, legality contracts |

These are different artifacts with different digests. `registry_digest` is the concept/codebook digest. `operator_set_digest` is the operator catalog digest. Do not overload the field name. Both are normative bundle artifacts; both are bound into report/graph/tape.

### No-bypass invariant

There is no callable path that applies an operator without providing a registry snapshot. This is the operator analogue of ADR 0001 (compilation boundary): `apply()` requires the registry, and the registry is part of the evidence chain. No implicit "current operator set," no defaults, no "temporary shortcuts."

### What this avoids

If "operator MVP" is not explicitly a registry (data + identity + verification) and is instead "a few ops wired into `apply()`," you recreate the v1 drift pattern: call sites quietly become the schema, ad hoc conventions accrete, and later "real registry work" becomes a breaking migration.

---

## Truth Regime Matrix

All worlds must run under the same harness contract (`WorldHarnessV1` + `SearchWorldV1`). This matrix tracks which rubric claims each world is designed to falsify.

| World | Type | Rubric #1 (replay) | #2 (trace) | #3 (drift) | #4 (tool) | #5 (transfer) | #6 (belief) | #7 (stochastic) | #8 (learning) | #9 (ML demotion) |
|-------|------|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| **RomeMini** | Carrier fixture | X | — | X | — | — | — | — | — | — |
| **RomeMiniSearch** | Search fixture | X | X | X | — | X | — | — | — | X |
| **SlotLatticeSearch** (6 regimes) | Stress test | X | X | X | — | X | — | — | — | X |
| *Transactional KV Store* | Tool safety | X | X | X | **X** | X | — | — | — | X |
| *Mastermind-like* | Partial obs. | X | X | X | — | X | **X** | — | — | X |
| *Slippery Grid* | Stochastic | X | X | X | — | X | — | **X** | — | X |

*Italic* = planned, not yet built. **Bold X** = the primary claim this world is designed to falsify. Plain X = claim also exercised as a side effect.

---

## Cross-Codebase Equivalence Harness

Sterling (Python) and sterling-native (Rust) share wire formats (.bst1, canonical JSON, Code32 layout). Claiming compatibility without mechanical enforcement is a footgun (G3). This harness makes it testable.

**Minimal fixture set where both codebases must produce byte-identical artifacts**:

| Artifact | Python produces | Rust produces | Comparison |
|----------|----------------|---------------|------------|
| `compile()` output bytes | `core/carrier/compiler.py` | `kernel/src/carrier/compile.rs` | Byte-identical ByteStateV1 |
| ByteTrace bytes | `core/carrier/bytetrace.py` | `kernel/src/carrier/trace_writer.rs` | Byte-identical .bst1 |
| Payload hash + step chain digest | `core/proofs/` | `kernel/src/proof/trace_hash.rs` | Identical SHA-256 digests |
| SearchGraph canonical JSON | (Python StateGraph, if applicable) | `search/src/graph.rs` | Byte-identical canonical JSON |
| Tape header bindings | N/A (Python has no tape) | `search/src/tape.rs` | One-sided; verify Rust consistency |
| Bundle digest basis | N/A (Python uses H2 bundles) | `harness/src/bundle.rs` | One-sided; verify Rust consistency |

**Implementation**: A small set of golden fixtures (payloads + schemas + registries) checked into both repos. CI in each repo verifies its output against the golden fixtures. The golden fixtures are the shared truth — not either codebase's output.

---

## Supersession Plan

### Phase 1: Declare what is already superseded (DONE)

9 v1 docs annotated with supersession headers (commit `e745fc6`):

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

### Phase 2: Build order for closing parity gaps

Each phase unlocks success-rubric claims. Phases are ordered so earlier phases create the infrastructure later phases need.

| Phase | Capability | Unlocks | Key deliverables | Falsifiers |
|-------|-----------|---------|------------------|------------|
| **0** | **Operator Registry MVP** | Stable operator identity for all subsequent phases | `OperatorRegistryV1` schema + artifact, SET_SLOT migration, digest binding in report/graph/tape, lock tests | Unknown op_id not fail-closed; operator contract mismatch undetected; registry digest not bound |
| **1a** | Transactional Tool World | Rubric #4 (tool safety) | KV-store world: STAGE/COMMIT/ROLLBACK/READ/VERIFY operators; tool transcript artifact; 10-15 lock tests | Tool action without transcript; side effect without commit; rollback unverifiable |
| **1b** | Partial Observability World | Rubric #6 (belief discipline) | Mastermind-like world: probe operators; belief-size monotonicity in trace; 10-15 lock tests | Belief inflation after probe; hidden observation channel; probe results not bound to evidence |
| **1c** | Stochastic World | Rubric #7 (seed/witness certification) | Slippery Grid world: seed/witness binding; exact replay from evidence; distributional eval over seed sets; 10-15 lock tests | Cannot replay recorded trajectory; cert depends on rerunning environment; no statistical protocol |
| **2** | Induction MVP | Rubric #8 (learning) | Propose→evaluate→promote cycle; standard evaluation packet; start with scorer/policy table induction | Promoted operator breaks previously certified claims; evaluators modified per-case |
| **3** | Memory MVP | Strengthens #5, #8 | Landmark candidates from traces → content-addressed artifacts; governed memory updates | Memory artifact not content-addressed; memory update not governed by operator |
| **4** | Text boundary MVP | Endgame narrative | Minimal parse/render demo; verifiable realization artifact surface (realizer stays in Python) | Surface treated as authority; IR bypass |
| **∥** | Cross-codebase equivalence harness | Validates "shared wire formats" claim | Golden fixtures in both repos; CI-verified byte-identical outputs | Any wire format drift between Python and Rust |

Phase ∥ (equivalence harness) can run in parallel with any phase.

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
5. **Certification authority location** — **Resolved: ADR 0006.** Python is certification control plane; Rust is evidence generator + verifier. Every Python cert must reference a Rust-verified artifact set by digest. Migration to Rust governance requires a new ADR.
6. **Evidence packaging relationship** — **Resolved: ADR 0007.** Nesting: TD-12/H2 must import ArtifactBundleV1 digest basis as required substrate artifact. Governance sits on top of Rust bundles, never parallel.
7. **Search schema extension mechanism** — **Resolved: ADR 0008.** Additive fields within a schema version; breaking changes require version bumps. No extension block indirection. Canonical JSON sorting makes additive fields safe. `operator_set_digest` is the first concrete test of this mechanism.

---

## Relationship to other documents

- **[`v1_contract_promotion_queue.md`](v1_contract_promotion_queue.md)**: Tracks doc-level promotion status. This audit tracks capability-level parity.
- **[`v2_success_rubric.md`](v2_success_rubric.md)**: The scorecard this audit's build order is designed to advance.
- **[`clean_sheet_architecture.md`](clean_sheet_architecture.md)**: The target architecture. This audit measures progress against that target.
- **[`docs/canonical/search_evidence_contract.md`](../canonical/search_evidence_contract.md)**: The v2 canonical doc that supersedes v1's proof/evidence system for the search layer.
- **Sterling `docs/architecture/v1_v2_supersession_map.md`**: The Python-side view of this same mapping. This parity audit is the primary source of truth (G8); the supersession map summarizes and links here.
