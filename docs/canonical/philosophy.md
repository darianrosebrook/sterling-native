---
status: v2 canonical
authority: canonical
date: 2026-02-24
supersedes: "v1 philosophy — see [absorption_pipeline.md](../reference/design_rationale/absorption_pipeline.md) for the v1 absorption pipeline details"
---
# Sterling Native Design Philosophy

---

## Core rule

Sterling owns semantics at the level of **contracts and invariants**, not at the level of domain objects or domain algorithms. Domains own sensors, object models, raw feeds, and implementation tricks. Sterling owns the definition of "what it means" for a capability to exist, how it is tested, how it is claimed, and how it composes with other capabilities.

Every architectural decision follows from that single rule.

---

## 1. The compilation boundary is the architectural spine

All domain payloads enter Sterling through a single function:

```
compile(payload, schema_descriptor, registry_snapshot) → ByteState
```

ByteState is the only runtime truth. Nothing bypasses compilation. This is the hardest constraint in the system and the one that prevents domain coupling.

**Four-layer authority stack vs. crate packaging:** The conceptual authority stack is four layers: **Carrier → State → Operator → Search**. Each layer depends only on the layer below it. The repo currently uses three crates with one-way dependencies: `kernel` (carrier + state + operators + proof/hash) ← `search` (search engine + tape) ← `harness` (orchestration + bundles + verification + worlds). This is a packaging choice, not an authority change — the authority stack remains four layers regardless of how many crates express them.

**Governing decisions:** [ADR 0001](../adr/0001-compilation-boundary.md), [`bytestate_compilation_boundary.md`](bytestate_compilation_boundary.md)

---

## 2. Three boundary separations

These separations must remain explicit. Blurring any of them produces domain coupling that is invisible until a second domain attempts to implement the same primitive.

### 2.1 Data plane vs. control plane

- **Data plane:** The shape of information crossing the boundary — envelopes, deltas, snapshots, KG fragments, operator signatures. Must remain boring and stable.
- **Control plane:** How Sterling and a domain negotiate capabilities — claims, schema versions, feature flags, budgets, epochs. Must be explicit and auditable.

### 2.2 Structural contract vs. semantic contract

- **Structural contract:** Types, required fields, enums, versioning, monotonic sequencing, determinism at the serialization layer.
- **Semantic contract:** Invariants, ordering constraints, monotonicity rules, boundedness, fail-closed rules, uncertainty behavior.

Most teams stop at structural. Sterling builds semantic contracts (conformance suites) on top.

### 2.3 Domain ontology vs. Sterling core ontology

Domains have their own class labels, taxonomy depth, and feature vocabularies. Sterling requires domain-specific vocabulary to be namespaced, versioned, and either:

1. Treated as **opaque tokens**, or
2. Explicitly aligned via a **mapping artifact** with its own governance.

---

## 3. Neural is advisory, never authoritative

Neural components (LLMs, encoders, rankers) participate under strict constraints:

| May do | May NOT do |
|--------|------------|
| Parse input into IR | Create operators |
| Rank candidates | Bypass preconditions |
| Compress representations | Mutate state directly |
| Realize output from IR | Override governance |

This is enforced by API shape, not by policy document. The neural component never sees a mutable state handle.

**Governing decision:** [ADR 0003](../adr/0003-neural-advisory-only.md), [`neural_usage_contract.md`](neural_usage_contract.md)

---

## 4. Evidence is layered, not monolithic

Sterling Native produces distinct evidence artifacts, each certifying a different layer of computation:

| Layer | Evidence artifact | What it certifies |
|-------|------------------|-------------------|
| **Carrier** | `ByteTrace` (`.bst1`) | Deterministic compile→apply execution. Frame-by-frame replay with O(1) divergence localization. |
| **Search** | `SearchTapeV1` (`.stap`) + `SearchGraphV1` (`search_graph.json`) | Deterministic search: expansion ordering, candidate outcomes, termination. Tape has chain-hash integrity; graph is the canonical transcript. |

Both coexist in an `ArtifactBundleV1` and are verified independently. The tape is the minimal hot-loop recorder; the graph is the analysis-friendly view. In Cert mode, tape→graph canonical byte equivalence is required — proving both describe identical search behavior.

**Within each layer, the binary evidence format is authoritative.** ByteTrace is the replay spine for carrier execution (StateGraph is derived). SearchTape is the evidence spine for search (SearchGraph is the derived canonical view, verified equivalent under Cert).

**Governing decision:** [ADR 0002](../adr/0002-byte-trace-is-canonical.md) (ByteTrace authority for carrier layer; the same principle extends to SearchTape for the search layer)

---

## 5. Operators are governed by taxonomy and contract

The S/M/P/K/C taxonomy (Seek, Memorize, Perceive, Knowledge, Control) is the canonical operator classification. Each operator has a declared signature with preconditions and effects. Sterling enforces these contracts at runtime — an operator can only apply if its preconditions are satisfied, and it may only modify state within its declared write-set.

**Governing decision:** [ADR 0004](../adr/0004-operator-taxonomy-names.md)

---

## 6. Two governance modes: DEV and CERTIFIED

- **CERTIFIED:** Fail-closed. All invariants enforced. Produces promotion-eligible artifacts.
- **DEV:** Permissive. Requires explicit witnesses. Cannot produce promotion-eligible artifacts.

There is no middle ground. An artifact is either promotion-eligible or it isn't.

---

## 7. Capability absorption is artifact-based, not code-based

When Sterling encounters a new capability proven by a rig, the goal is not to copy the rig's code. The goal is to extract the **invariant**, encode it as a **contract + test + proof**, and register it so any future domain can claim the same capability by passing the same gates.

The pipeline:

1. Identify the primitive boundary → what Sterling needs, what it guarantees
2. Define the capsule (Sterling-owned) → contract types, conformance suites, invariants
3. Build fixtures and prove portability → two domains, same suite
4. Domain implements adapter, passes certification
5. Register the capability claim with evidence
6. Runtime handshake, enforcement, fail-closed

**Detailed absorption pipeline reference:** [`docs/reference/design_rationale/absorption_pipeline.md`](../reference/design_rationale/absorption_pipeline.md) *(advisory)*

---

## 8. Domain coupling prevention rules

1. Sterling contracts never mention domain object models.
2. Domain semantics enter only through declared, injectable components.
3. Feature vocabularies are namespaced and opaque by default.
4. Semantic strengthening is introduced as extension capabilities.
5. Contract drift is caught structurally (hash comparison) and semantically (conformance suites).
6. Online enforcement is fail-closed; offline enforcement is certification-based.

---

## 9. The meta-principle

When you feel the urge to absorb code from a rig into Sterling, translate that urge into:

> **"What is the invariant we learned, and how do we encode it as a contract + test + proof?"**

Code can remain in the domain. Sterling accumulates **capability claims backed by proof artifacts**, not domain-specific implementations.
