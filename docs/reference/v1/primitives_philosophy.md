# Sterling Capability Ingestion Philosophy

Status: v1.1
Date: 2026-02-01
Derived from: `philosophy-convo.md` (raw discussion), formalized into enforceable architecture
Relates to: `00_IO_CONTRACT.md`, `templates/global_invariants.md`, all primitive specs P01–P21

---

## Core rule

Sterling owns semantics at the level of **contracts and invariants**, not at the level of domain objects or domain algorithms. Domains own sensors, object models, raw feeds, and often the implementation tricks that make a primitive performant. Sterling owns the definition of "what it means" for that capability to exist, how it is tested, how it is claimed, and how it composes with other capabilities.

Every architectural decision in this document follows from that single rule.

---

## 1. Three boundary separations

These separations must remain explicit. Blurring any of them produces domain coupling that is invisible until a second domain attempts to implement the same primitive.

### 1.1 Data plane vs. control plane

The **data plane** is the shape of information that crosses the boundary: envelopes, deltas, snapshots, KG fragments, operator signatures. It must remain boring and stable.

The **control plane** is how Sterling and a domain negotiate what they can do: capability claims, schema versions, feature flags, budgets, epochs/streams. It must be explicit and auditable.

**Sterling anchors:**

| Plane | Existing type | Location |
|-------|--------------|----------|
| Data | `ObservationIR`, `DeltaObservationIR` | `core/worlds/base.py:142–329` |
| Data | `SemanticDeltaIRv0` | `core/linguistics/ir_v0/container.py` |
| Data | `TraceBundleV1` | `core/contracts/trace_bundle.py` |
| Control | `WorldCapabilities` | `core/worlds/base.py:49–135` |
| Control | `RunIntent` | `core/governance/run_intent.py` |
| Control | `EvaluationContractV1` | `core/domains/domain_spec_v1.py:409–468` |

**Failure mode if blurred:** A schema field rename silently changes behavior. A domain starts sending extra fields and Sterling starts depending on them. The only fix is to keep the planes structurally separate in the type system.

### 1.2 Structural contract vs. semantic contract

**Structural contract:** types, required fields, enums, versioning, monotonic sequencing, determinism requirements at the serialization layer.

**Semantic contract:** invariants, ordering constraints, monotonicity rules, boundedness, fail-closed rules, and how uncertainty behaves.

Most teams stop at structural. That is how systems become domain-coupled: the only thing shared is a JSON shape, but not the meaning. Sterling's rigs build semantic contracts (conformance suites), which is the correct approach.

**Sterling anchors:**

| Contract layer | Existing type | Location |
|----------------|--------------|----------|
| Structural | `CanonicalSchemaEntry` (148+ schemas) | `core/contracts/schema_registry.py` |
| Structural | `SolveRequestV1.schema.json` | `docs/planning/capability_primitives_bundle/schemas/` |
| Semantic | `StateSchemaCommitV1` (invariant names, field dependencies) | `core/domains/domain_spec_v1.py:223–287` |
| Semantic | `ProbeCommitV1` (semantic probes as domain truth tests) | `core/domains/domain_spec_v1.py:337–406` |
| Semantic | `InvariantDependency` (explicit field→invariant links) | `core/domains/domain_spec_v1.py:143–185` |
| Conformance | `test_world_adapter_protocol_conformance.py` | `tests/unit/` |

### 1.3 Domain ontology vs. Sterling core ontology

Domains will always have their own class labels, taxonomy depth, and feature vocabularies. Sterling must not internalize those as core concepts. Sterling requires that any domain-specific vocabulary be namespaced, versioned, and either:

1. Treated as **opaque tokens**, or
2. Explicitly aligned via a **mapping artifact** that has its own governance.

This is where most "domain-agnostic" systems quietly die: they encode domain classes as first-class concepts in the engine rather than as external vocabularies with alignment layers.

**Sterling anchors:**

| Mechanism | Existing type | Location |
|-----------|--------------|----------|
| Namespace separation | `FieldID` (namespace + field_name + schema_version) | `core/domains/domain_spec_v1.py:86–140` |
| Opaque evidence payloads | `ObservationIR.payload: Dict[str, Any]` | `core/worlds/base.py:142–184` |
| Domain-typed descriptors | `DomainDescriptor` (domain_type enum, structural_signature) | `core/proofs/stage_l_transfer_claim.py:23–47` |
| KG namespace isolation | `KGRef` (logical_id, schema_id, content_hash) | `kg/registry.py` |

**Enforcement rule:** If you find yourself writing domain-specific constants (e.g., `HOSTILE_KINDS`, "Minecraft distance", "WordNet depth") inside a primitive spec or capsule, you are already sliding. Domain constants belong in the adapter. Sterling references them only through declared, injectable components.

---

## 2. What rigs are (and are not)

**A rig is not a dependency. A rig is a certification surface.**

A rig exists to do three jobs:

1. **Produce representative evidence streams** (including adversarial edge cases) in a repeatable format.
2. **Provide adapters** that connect a domain implementation to a Sterling-owned contract (the capsule).
3. **Provide proof artifacts**: passing conformance suites, determinism harnesses, drift detectors, and resource-bound guarantees.

**The rig does not define the primitive. The capsule does.**

This prevents the "leave learnings behind" problem: the learning is not in the Minecraft code or the WordNet adapter. It is in the capsule contract + tests + fixtures + invariants + proof hashes, which are owned by Sterling. The domain merely supplies an implementation and a proving surface.

**Sterling anchors:**

| Rig role | Existing pattern | Location |
|----------|-----------------|----------|
| Evidence production | `WorldAdapter.emit_observations()` | `core/worlds/base.py:452–522` |
| Adapter connection | `WorldAdapter` protocol (structural subtyping) | `core/worlds/base.py:452–522` |
| Proof artifacts | `data/certified_operators/index.json` (cert_ref, policy_hash, verification_state) | `data/certified_operators/` |
| Conformance suites | `ScenarioConfigV1`, `SuiteConfigV1` | `core/induction/scenario_suite.py` |
| Fixture governance | `GoldenFixtureDescriptor` (content-addressed) | `core/induction/golden_fixture_descriptor.py` |

---

## 3. The capability absorption pipeline

Absorption is a pipeline of **artifacts**, not a pipeline of code reuse.

When Sterling encounters a new capability proven by a rig, the goal is not to copy the rig's code into Sterling. The goal is to extract the **invariant**, encode it as a **contract + test + proof**, and register it so that any future domain can claim the same capability by passing the same gates.

### Step 0: Identify the primitive boundary

Select a capability Sterling should be able to claim. Write down what Sterling needs from the other side, and what Sterling guarantees if it receives it.

**Existing primitive catalog:** `docs/planning/capability_primitives_bundle/primitives/P01–P21`

Each primitive spec defines:
- Formal signature (state, operators, objective)
- What must be proven
- Minimal substrate requirements
- Certification gates (signature, performance, transfer)

### Step 1: Define the capsule (Sterling-owned)

A capsule contains:

| Component | Purpose | Sterling anchor |
|-----------|---------|----------------|
| **Contract types** | Domain-agnostic, stable naming | `CanonicalSchemaEntry` in `core/contracts/schema_registry.py` |
| **Primitive specs** | Data-driven obligation documents | `PrimitiveSpecV1` in `core/domains/primitive_spec.py`; P01–P05 factories; `data/primitive_specs/index.json` |
| **Primitive registry** | Singleton lookup by (primitive_id, contract_version) | `PrimitiveRegistry` in `core/domains/primitive_registry.py` |
| **Conformance suite descriptors** | Content-addressed suite identity with code binding | `ConformanceSuiteV1` in `core/domains/conformance_suite.py`; `suite_impl_ref` prevents silent suite swap |
| **Version identifiers** | Compatibility rules | `schema_version` fields throughout; `DomainSpecV1.domain_id` |
| **Invariants** | Semantic properties | `InvariantDependency` in `core/domains/domain_spec_v1.py` |
| **Conformance suite(s)** | Determinism harnesses | `ScenarioConfigV1` in `core/induction/scenario_suite.py` |
| **Optional extensions** | Each with its own sub-claim | `ClaimCapability` enum in `core/induction/scenario_suite.py` |

**No domain imports. No domain constants. No domain taxonomies.** If a capsule contains a reference to a domain-specific concept, that concept must be injected through a declared adapter interface, not hardcoded.

### Step 2: Define negotiation and capability claims (control plane)

A domain does not just "send data." It declares what it supports:

- Which primitive sub-variants (e.g., P21-A conservative vs. P21-B predictive)
- Uncertainty semantics supported
- Budget parameters (track cap, delta cap, emission frequency)
- Schema versions supported

This becomes a **CapabilityDescriptor + SchemaDescriptor** that can be hashed, stored, and used for runtime routing decisions.

**Sterling anchors:**

| Concept | Existing type | Location | Status |
|---------|--------------|----------|--------|
| Capability flags | `WorldCapabilities` | `core/worlds/base.py` | Implemented; structural eligibility checks use `PrimitiveRegistry` |
| Capability descriptors | `CapabilityDescriptorV1` (per-primitive, content-addressed) | `core/domains/capability_descriptor.py` | Implemented; `(primitive_id, contract_version)` versioned keys |
| Capability claims registry | `CapabilityClaimRegistry` with `(domain_id, primitive_id, contract_version)` keys | `core/domains/capability_claim_registry.py` | Implemented; registry hash from VERIFIED entries only |
| Schema descriptors | `EvidenceSchemaV0` (schema_id, schema_version, payload_hash) | `core/proofs/evidence_schema_registry.py` | Exists; covers evidence |
| Capability claims | `ClaimCapability` enum | `core/induction/scenario_suite.py` | Exists; covers induction capabilities |
| Domain identity | `DomainSpecV1.domain_id` (content-addressed) | `core/domains/domain_spec_v1.py` | Exists; per-domain |
| Domain declarations | `DomainDeclarationV1` (long-lived, content-addressed, primitive claims) | `core/domains/domain_handshake.py` | Implemented; binds primitive claims to domains |

### Step 3: Build fixtures and prove portability

Fixtures are not examples. They are **executable meaning**. When a second domain passes the same conformance suite, you have evidence the meaning is domain-agnostic.

**Minimum:** Two fixture sets from structurally different domains must pass the same conformance suite for a primitive to be considered domain-agnostic.

**Sterling anchors:**

| Concept | Existing type | Location |
|---------|--------------|----------|
| Fixture governance | `GoldenFixtureDescriptor` (content-addressed, schema-versioned) | `core/induction/golden_fixture_descriptor.py` |
| Transfer claims | `StageLTransferClaimV1` (source/target domain descriptors) | `core/proofs/stage_l_transfer_claim.py` |
| H2 capability summary | `h2_capability_summary.v1.json` (regime results, safeguards) | `core/proofs/schemas/` |

### Step 4: Domain implements adapter, passes certification in CI

The adapter is the **only domain-specific glue** Sterling should ever need. The certification run produces an artifact that the domain can cite when claiming the primitive.

**Sterling anchors:**

| Concept | Existing type | Location |
|---------|--------------|----------|
| Adapter protocol | `WorldAdapter` (structural subtyping) | `core/worlds/base.py:452–522` |
| Hypothesis capability | `HypothesisCapability` protocol | `core/worlds/base.py:529–604` |
| Governance hooks | `emit_observations()`, `verify_prediction()` | `core/worlds/base.py` (W-OBS-1, W-TEST-1) |
| Certification pipeline | `E2ECertificationPipeline` | `core/induction/e2e_certification_pipeline.py` |
| Protocol conformance tests | `test_world_adapter_protocol_conformance.py` | `tests/unit/` |

### Step 5: Register the capability claim with evidence

Sterling must be able to answer: "Does this domain implement P01?" without reading code.

A registry entry contains:

| Field | Purpose | Sterling anchor |
|-------|---------|----------------|
| `capability_id` | e.g., `p01.a` | `CapabilityClaimEntry.capability_id` in `core/domains/capability_claim_registry.py` |
| `contract_version` | e.g., `p01@1.0` | `CapabilityClaimEntry.contract_version`; versioned key `(domain_id, primitive_id, contract_version)` |
| `conformance_suite_hash` | Content hash of the testkit suite | `ConformanceSuiteV1.suite_id` in `core/domains/conformance_suite.py`; includes `suite_impl_ref` |
| `fixture_hashes` | Which fixture streams were used | `FixtureRef` in `ConformanceSuiteV1`; `GoldenFixtureDescriptor` |
| `results_hash` | Proof artifact or deterministic summary | `EvidenceSchemaV0.payload_hash` pattern |
| `budget_declaration` | Caps used during certification | `BudgetDeclaration` in `core/domains/capability_descriptor.py` |

**Status:** The `CapabilityClaimRegistry` (`core/domains/capability_claim_registry.py`) now supports primitive-level claims with versioned keys `(domain_id, primitive_id, contract_version)`. Registry hash is computed from VERIFIED entries only. The `data/certified_capabilities/index.json` file provides the persistent index. The certified operators index (`data/certified_operators/index.json`) continues to track operator-level certification separately.

### Step 6: Runtime handshake, enforcement, fail-closed

When the system runs:

1. Domain announces `(capabilities, schema_versions, budgets, epoch)` on connect.
2. Sterling enforces the schema version and invariants that are enforceable online (sequence monotonicity, boundedness, fail-closed on malformed events).
3. Anything not enforceable online remains enforceable via CI certification plus post-hoc audit sampling.

**Sterling anchors:**

| Mechanism | Existing type | Location |
|-----------|--------------|----------|
| Fail-closed validation | `RunIntent.is_strict` | `core/governance/run_intent.py` |
| Domain declaration (long-lived) | `DomainDeclarationV1` (content-addressed, primitive claims) | `core/domains/domain_handshake.py` |
| Domain session (ephemeral) | `DomainSessionV1` (KG ref, operator pack, NOT content-addressed) | `core/domains/domain_handshake.py` |
| Proof-backed routing | `SterlingEngine._check_primitive_eligibility()` | `core/engine/core.py`; checks structural flags + VERIFIED claims by default |
| Structural-only opt-in | `SterlingOptions.structural_only = True` | `core/worlds/base.py`; skips claim verification |
| Schema version gating | `DomainSpecV1.verify_domain_id()` | `core/domains/domain_spec_v1.py` |
| Evidence validation | `EvidenceSchemaRegistry.validate_evidence_items()` | `core/proofs/evidence_schema_registry.py` |
| Missing dependency enforcement | `CertifyingModeError`, `ConfigurationError` | `core/governance/run_intent.py` |

---

## 4. Domain coupling prevention rules

These rules prevent Sterling from becoming "the first domain it learned, with extra steps."

### Rule 1: Sterling contracts never mention domain object models

No "Minecraft entity," no "camera frame," no "Kubernetes pod," no "WordNet synset." Only "evidence item," "track summary," "operator signature," "KG claim."

**Prevents:** "The engine only works if the world looks like the first world it learned."

**Enforcement:** AST-based import boundary checks in `tests/unit/test_domain_coupling_prevention.py` (Rule 1: no domain imports in contract modules; Rule 5: forbidden domain terms). Structural, not string-based — harder to game.

### Rule 2: Domain semantics enter only through declared, injectable components

If "threat classification" differs by domain, it is an injected classifier (or declared extension), not a hardcoded mapping in the primitive.

**Prevents:** "P21 means 'hostile mobs' forever."

**Enforcement:** `WorldAdapter` protocol is the injection boundary. Primitives reference adapters, not domain types.

### Rule 3: Feature vocabularies must be namespaced and treated as opaque by default

If evidence items carry `features`, Sterling treats them as opaque payload unless an explicit extension says otherwise. Any feature used for semantics must be declared by schema + invariant tests.

**Prevents:** "One team starts relying on `fuse_state` and now the primitive is secretly shaped like one domain."

**Enforcement:** `ObservationIR.payload` is `Dict[str, Any]`. Sterling never pattern-matches on payload keys outside of declared schema contracts. AST-based checks in `test_domain_coupling_prevention.py` (Rule 4: no isinstance against concrete adapter subclasses for capability routing).

### Rule 4: Any semantic strengthening must be introduced as an extension capability

Base primitive stays minimal. Richer semantics become optional extensions with their own invariants and sub-claims.

**Prevents:** Breaking contracts when a new domain needs different semantics.

**Enforcement:** `ClaimCapability` enum grows additively. Existing claims are never widened.

### Rule 5: Contract drift must be caught structurally and semantically

Structural drift: schema field changes detected by hash comparison (`DomainSpecV1.verify_domain_id()`).

Semantic drift: fixtures + determinism harnesses + invariant probes detect meaning shifts masked by compatible JSON.

**Prevents:** Silent meaning shifts.

**Enforcement:** CI runs conformance suites against golden fixtures. Hash mismatches block promotion.

### Rule 6: Online enforcement is fail-closed; offline enforcement is cert-based

If a required invariant can be enforced at runtime (sequence order, required fields, boundedness), enforce it and reject violations. If it cannot be fully enforced online (determinism across runs, transfer properties), certify it and store proof.

**Prevents:** "Garbage in" poisoning cognition.

**Enforcement:** `RunIntent.CERTIFYING` and `RunIntent.PROMOTION` are fail-closed. `RunIntent.DEV` is permissive with explicit witnesses.

---

## 5. Dynamic, long-standing contracts

The trick is to treat "dynamic" as "versioned streams of artifacts," not as ad-hoc runtime coupling.

### 5.1 Schema ingestion (long-standing compatibility)

| Requirement | Sterling mechanism | Location |
|------------|-------------------|----------|
| Every message references a schema_id | `EvidenceSchemaV0.schema_id` | `core/proofs/evidence_schema_registry.py` |
| Schema evolution rules are explicit | Additive fields OK; semantic changes require major version bump | `core/contracts/schema_registry.py` policy |
| Domains declare supported versions | `DomainSpecV1.kernel_binding.kernel_version` | `core/domains/domain_spec_v1.py` |
| Sterling chooses one or refuses | `EvidenceSchemaRegistry.validate_evidence_items()` | `core/proofs/evidence_schema_registry.py` |

### 5.2 Operator ingestion (capability expansion without hard-baking)

Operators are declared by signature, not trusted because they exist. They are trusted because they pass an **operator conformance harness**.

| Requirement | Sterling mechanism | Location |
|------------|-------------------|----------|
| Typed operator signatures | `OperatorSignature` (preconditions, effects, costs) | `core/operators/registry_types.py` |
| Conformance harness | Stage K certification pipeline | `core/induction/e2e_certification_pipeline.py` |
| Certified operator index | Content-addressed entries with cert_ref, policy_hash | `data/certified_operators/index.json` |
| Composition by signature | Operator category system (S/M/P/K/C) | `core/operators/registry_types.py` |

Sterling composes operators by signature and declared semantics, not by domain identity.

### 5.3 KG ingestion (knowledge without ontology lock-in)

| Requirement | Sterling mechanism | Location |
|------------|-------------------|----------|
| KG transport format (Sterling-owned) | `KGRef` (logical_id, schema_id, content_hash) | `kg/registry.py` |
| Ontology alignment artifacts | `DomainDescriptor.structural_signature` | `core/proofs/stage_l_transfer_claim.py` |
| Namespace isolation | `KGRegistration` with schema_id versioning | `kg/registry.py` |
| Universal meta-predicates | Operator category mapping (edge type → S/M/P/K/C) | `kg/ontology.py` |

Reasoning that depends on namespaced predicates is always mediated by an explicit alignment artifact or a domain-specific adapter, which itself can be certified.

---

## 6. Reusable contract shape template

Every new primitive contract should follow this shape:

```
Contract types       → stable, minimal, domain-agnostic
Message envelope     → {request_version, capability_id, domain_id, stream_id,
                        epoch, seq, tick_id, payload}
Capability descriptor → {capability_id, contract_version, supported_extensions,
                         budgets, determinism_class}
Schema descriptor    → {schema_id, schema_version, schema_hash}
Conformance suites   → base + extension suites (content-addressed)
Proof artifacts      → result hashes, fixture hashes, suite hash
```

**Sterling types that implement this shape:**

| Template slot | Existing type | Coverage |
|---------------|--------------|----------|
| Message envelope | `SolveRequestV1` / `SolveResponseV1` | Partial (schemas exist; not yet in code as dataclasses) |
| Capability descriptor | `CapabilityDescriptorV1` + `PrimitiveSpecV1` | Full (`core/domains/capability_descriptor.py`, `core/domains/primitive_spec.py`) |
| Capability claims | `CapabilityClaimRegistry` with versioned keys | Full (`core/domains/capability_claim_registry.py`; `(domain_id, primitive_id, contract_version)`) |
| Conformance suite descriptor | `ConformanceSuiteV1` with `suite_impl_ref` | Full (`core/domains/conformance_suite.py`; content-addressed with code identity binding) |
| Domain declaration | `DomainDeclarationV1` (long-lived, content-addressed) | Full (`core/domains/domain_handshake.py`; primitive claims, budgets, extensions) |
| Domain session | `DomainSessionV1` (ephemeral, NOT content-addressed) | Full (`core/domains/domain_handshake.py`; KG ref, operator pack) |
| Schema descriptor | `EvidenceSchemaV0` + `CanonicalSchemaEntry` | Full (152+ schemas registered) |
| Conformance suites | `ScenarioConfigV1` + `SuiteConfigV1` | Full (machine-readable suite configs) |
| Proof artifacts | `EvidenceSchemaV0.payload_hash` + certified operators index | Full |
| Runtime routing | `SterlingEngine._check_primitive_eligibility()` | Full (proof-backed by default; structural-only opt-in) |

---

## 7. The meta-principle

When you feel the urge to "absorb" code from a rig into Sterling, translate that urge into:

> **"What is the invariant we learned, and how do we encode it as a contract + test + proof?"**

Code can remain in the domain as long as the meaning is standardized and certifiable. Sterling accumulates **capability claims backed by proof artifacts**, not domain-specific implementations.

---

## 8. Implementation priority

The absorption pipeline (Steps 0–6) infrastructure is now built. Here is the current status:

| Component | Status | What exists | What's needed |
|-----------|--------|-------------|---------------|
| Primitive specs (doc) | Complete | P01–P21 in `primitives/` | — |
| Primitive specs (code) | Complete | `PrimitiveSpecV1` + `PrimitiveRegistry` in `core/domains/`; P01–P05 factories; `data/primitive_specs/index.json` | P06–P21 factory functions (define as rigs prove them) |
| Capability descriptors | Complete | `CapabilityDescriptorV1` (per-primitive, content-addressed) in `core/domains/capability_descriptor.py` | — |
| Capability claims registry | Complete | `CapabilityClaimRegistry` with `(domain_id, primitive_id, contract_version)` versioned keys; registry hash from VERIFIED only | — |
| Conformance suite descriptors | Complete | `ConformanceSuiteV1` with `suite_impl_ref` code identity binding in `core/domains/conformance_suite.py` | Suite runner (descriptor exists; execution is domain-owned) |
| Domain declarations | Complete | `DomainDeclarationV1` (long-lived, content-addressed) + `DomainSessionV1` (ephemeral) in `core/domains/domain_handshake.py` | Automatic declaration generation from adapter introspection |
| Runtime routing | Complete | `SterlingEngine._check_primitive_eligibility()` — proof-backed by default, structural-only opt-in via `structural_only=True` | — |
| Anti-leak CI enforcement | Complete | `tests/unit/test_domain_coupling_prevention.py` — 5 rules, AST-based import boundary + structural flag alignment + no-isinstance routing | — |
| Schema registry | Complete | 4 new schemas: `sterling.primitive_spec.v1`, `sterling.conformance_suite.v1`, `sterling.domain_declaration.v1`, `sterling.domain_session.v1` | — |
| I/O contract bridge | Missing | `SterlingRequest`/`SterlingResponse` exist; `SolveRequestV1` schema exists | Bridge types connecting them (domain_spec, observations, context, trace_bundle, explanation_bundle) |

### What's deferred

- P06–P21 factory functions in `PrimitiveSpecV1` (only P01–P05 now; add as rigs prove them)
- Conformance suite runner (Layer 2 defines the descriptor, not execution)
- Automatic declaration generation from adapter introspection
- Full extension negotiation beyond simple presence check
- `suite_impl_ref` generation tooling (manual for now, automated later)

### First consumers

The natural first consumer of the full pipeline is **P21 (Entity Belief Maintenance)**, which exercises every step: domain-specific perception → Sterling-owned belief contract → conformance suite → certification → capability claim. If the pipeline works for P21, it works for all primitives.

The conscious-bot project (TypeScript) has capsule types, conformance suites, and reference fixtures for P21. The Sterling-side infrastructure (`DomainDeclarationV1`, `CapabilityClaimRegistry`, runtime routing) is now ready to receive those claims. The remaining integration work is wiring `SterlingDomainDeclaration.implementsPrimitives` to create `DomainDeclarationV1` instances that can be validated against the `CapabilityClaimRegistry`.
