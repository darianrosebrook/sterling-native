---
authority: reference
status: advisory
date: 2026-03-01
capability: knowledge_graph
parity_capabilities: [D1, D2]
---

# Knowledge Graph

**Advisory — not normative.** This document describes proof obligations for future v2 work. Do not cite as canonical. See [parity audit](../../architecture/v1_v2_parity_audit.md) for capability status.

## Overview

The knowledge graph provides content-addressed entity identity, typed relations, and domain specification for Sterling's reasoning substrate. The sterling Python repo defined a knowledge graph contract with content-addressed references (KGRef), a thread-safe singleton registry, entity and relation types, external sense bridges (WordNet, Wikidata), and a domain specification system for capability absorption. This reference captures the proof obligations for integrating knowledge graph capabilities into the native substrate.

The native substrate already has a foundational piece: RegistryV1 provides bijective Code32-to-ConceptID mapping in the kernel. The knowledge graph extends this from concept identity to entity identity, relations, and structured domain knowledge.

## Key Concepts

### Content-Addressed Entity Identity (KGRef)

Every entity in the knowledge graph has a content-addressed identity — a digest computed from the entity's canonical representation. This is KGRef: a stable, collision-resistant identifier that does not depend on insertion order, database row IDs, or mutable labels.

In the native substrate, this maps to the existing ContentHash pattern. An entity's KGRef would be computed via `canonical_hash(HashDomain, &canonical_entity_bytes)` using a dedicated knowledge-graph domain separator from the HashDomain enum. The entity's canonical form would follow the same rules as other canonical JSON artifacts: sorted keys, compact encoding, ASCII-only keys.

### Entity and Relation Types

The knowledge graph stores typed entities (concepts, objects, agents, locations) and typed relations between them (is-a, has-part, causes, enables). Both entities and relations carry content-addressed identity. A relation is a triple `(subject_ref, relation_type, object_ref)` where each component is content-addressed.

The typing system must be extensible: new entity and relation types can be added without modifying core infrastructure. This parallels the OperatorRegistryV1 pattern — new operators are registered, not hard-coded.

### Domain Specifications

A domain specification (DomainSpec) describes a capability domain: what entities exist, what relations hold, what actions are possible, and what conformance looks like. Domain specs are the mechanism for capability absorption — when Sterling encounters a new domain (e.g., a new API, a new tool surface), the domain spec defines what the system can know and do within that domain.

Each domain spec binds:
- An entity schema (what kinds of entities exist in this domain)
- An action surface (what operators are legal in this domain, mapping to OperatorRegistryV1 entries)
- A conformance suite (tests that verify the system's understanding of the domain)

### External Sense Bridges

The sterling Python repo defined bridges to external knowledge sources (WordNet for lexical semantics, Wikidata for world knowledge). These bridges are advisory: they provide candidate entity mappings and relation suggestions, but the knowledge graph's authoritative content is determined by what has been committed through governed operations.

This follows the trust boundary principle: external sources are advisory input, not authority. A bridge produces candidate KGRefs; committing those refs into the graph is a governed operator.

### Compilation Boundary Integration

The open design question is how the knowledge graph integrates with the compilation boundary. Currently, `compile()` takes a payload and a RegistryV1 and produces ByteState. If the knowledge graph informs compilation (e.g., entity-aware state encoding), the compilation boundary must accept KG context without breaking the existing determinism guarantees.

Options include: KG as a pre-compilation lookup (resolve entities before compile), KG as a registry extension (entities are concept IDs in an extended registry), or KG as a separate artifact bound into the evidence chain but not part of compilation.

## Design Decisions (Open)

| Decision | Options | Constraint |
|----------|---------|------------|
| KGRef computation | `canonical_hash(HashDomain::KnowledgeGraph, bytes)` or reuse ContentHash directly | Dedicated domain separator preferred for collision isolation |
| Entity storage | In-memory registry, persistent artifact, or hybrid | Must be content-addressed and bundle-compatible |
| Relation representation | Triple store, adjacency lists, or ByteState-encoded | Must support content-addressed identity for individual relations |
| Domain spec format | Canonical JSON artifact or Rust type | Canonical JSON allows cross-language consumption; Rust type is faster |
| Compilation integration | Pre-compile lookup, registry extension, or separate artifact | Must not break existing compile() determinism |
| Bridge governance | Bridges as operators or as pre-processing | Operators integrate with existing governance; pre-processing is simpler |
| Registry relationship | Extend RegistryV1 or define KGRegistryV1 | RegistryV1 maps Code32 to ConceptID; KG may need richer mapping |

## Proof Obligations for v2

1. **Content-addressed entity identity.** Every entity has a KGRef computed via `canonical_hash` with a dedicated domain separator. Entity identity does not depend on insertion order, storage location, or mutable labels.

2. **Content-addressed relations.** Every relation triple has a content hash computed from the canonical representation of `(subject_ref, relation_type, object_ref)`. Relations are individually addressable and verifiable.

3. **KG artifacts are bundle-compatible.** Knowledge graph state (entities, relations, domain specs) can be serialized as canonical JSON artifacts, included in an ArtifactBundleV1, and verified by the existing `verify_bundle()` pipeline.

4. **Governed mutations.** Every KG mutation (entity creation, relation assertion, entity merge, domain spec registration) is a governed operation — either a registered operator or an explicit harness-level action with auditable evidence. No silent KG state changes.

5. **External bridges are advisory.** Data imported from external sources (WordNet, Wikidata, or any future bridge) enters the KG as advisory input. Committing external data into the authoritative KG is a separate governed operation. No external source is automatically authoritative.

6. **Domain specs are content-addressed artifacts.** Each DomainSpec has a ContentHash and can be included in an evidence bundle. The spec's entity schema, action surface, and conformance suite are individually verifiable components.

7. **Replay-linkable KG state.** Given an initial KG state and a sequence of governed mutations, any intermediate KG state is reconstructable. The mutation history forms a hash chain analogous to the memory MeaningStateDigest chain.

8. **Conformance suites are executable.** A domain spec's conformance suite is a set of verifiable assertions, not documentation. Each assertion maps to a test that can be run against the current KG state and world implementation.

9. **No compilation boundary violation.** If KG context informs compilation, the integration must preserve `compile()`'s determinism guarantee: same payload + same registry + same KG context = same ByteState, verifiable by replay.

## Parity Audit Reference

This document covers knowledge graph aspects relevant to capabilities **D1** (World harness contract — specifically domain specification as a world-level concern) and **D2** (Transfer packs / certification packets — specifically how KG artifacts participate in transfer evidence) from the [parity audit](../../architecture/v1_v2_parity_audit.md).

### What exists today (verifiable)

- RegistryV1 bijective Code32-to-ConceptID mapping — `kernel/src/carrier/registry.rs` (A1, A3: **Implemented**)
- Content-addressed artifacts with canonical JSON — `kernel/src/proof/canon.rs` (A2: **Implemented**)
- World harness contract with typed trait bounds — `harness/src/contract.rs` (D1: **Implemented, structural**)

### What is proposed (not implemented)

- Entity identity beyond concept IDs (a KGRef type computed via `canonical_hash` with dedicated domain separator)
- Typed relation storage (a triple store with content-addressed relation identity)
- Domain specification system for capability absorption (a DomainSpec type with entity schema + action surface)
- External sense bridges with advisory governance (bridges as advisory input, commitment as governed operator)
- A TransferPack schema tying claims to artifacts to verification profiles (D2)

See Import Group A (Truth-regime world diversity) in the parity audit for how domain specs connect to the broader world diversity goal — each new truth regime would be accompanied by a domain spec defining its entity schema and action surface.
