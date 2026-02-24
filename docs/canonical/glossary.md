# Sterling Native Glossary

**Status:** v2 canonical — curated vocabulary for Sterling Native.
**Scope:** Definitions enforced by contracts, invariants, and ADRs. For the comprehensive v1 glossary (historical), see [`docs/reference/v1/glossary_full.md`](../reference/v1/glossary_full.md).

---

## Carrier Layer

**Code32:** A 32-bit identity atom composed of `(domain: 5 bits, kind: 5 bits, local_id: 22 bits)`. Every semantic entity in Sterling has exactly one Code32 identity. The registry is bijective: one entity ↔ one Code32. See [`code32_bytestate.md`](code32_bytestate.md).

**ByteState:** The canonical packed runtime representation of Sterling's semantic state. ByteStateV1 uses a two-plane encoding (identity plane + status plane) at ~640 bytes per state snapshot. ByteState is the only runtime truth — all reasoning operates on compiled ByteState, not on IR objects directly. See [`bytestate_compilation_boundary.md`](bytestate_compilation_boundary.md).

**ByteTrace:** The canonical persisted trace format for replay verification and certification. ByteTrace is an append-only sequence of ByteState snapshots plus operator application metadata. It is the replay spine — StateGraph is a derived view rendered from ByteTrace. See [ADR 0002](../adr/0002-byte-trace-is-canonical.md).

**Compilation Boundary:** The function `compile(payload, schema_descriptor, registry_snapshot) → ByteState`. This is the architectural spine of Sterling Native. Domain payloads enter through compilation; nothing bypasses it. See [ADR 0001](../adr/0001-compilation-boundary.md).

---

## State Layer

**UtteranceState:** The annotated state of a single user utterance or sentence. Contains layered linguistic information (syntax, semantics) for one unit of text. Sterling enforces canonical utterance granularity: one UtteranceState per sentence or snippet.

**WorldState:** External context and environment state that accompanies an UtteranceState. Includes domain-specific observations, dialogue phase, and any external signals relevant to reasoning.

**StateGraph:** The graph of state nodes connected by operator applications (edges). Append-only and deterministic — once a state and transition are added, they are never altered. StateGraph is a derived view from ByteTrace, not the canonical persistence format.

---

## Operator Layer

**Operator Taxonomy (S/M/P/K/C):** Sterling's five operator categories, locked by [ADR 0004](../adr/0004-operator-taxonomy-names.md):

| Code | Name | Intent |
|------|------|--------|
| **S** | Seek | Explore or navigate state space (expansion, neighbor enumeration) |
| **M** | Memorize | Commit, consolidate, or update durable state (landmarking, retention) |
| **P** | Perceive | Interpret observations, update beliefs, incorporate evidence |
| **K** | Knowledge | Query or extend world knowledge within the governed substrate |
| **C** | Control | Manage search flow (budgeting, selection, stopping, orchestration) |

> **Legacy note:** v1 documentation used "Structural, Meaning, Pragmatic" for S/M/P. Those names are retired.

**Operator:** An abstract, reusable definition of a state transformation — includes category (S/M/P/K/C), precondition logic, and effect function. Operators are registered in an OperatorRegistry with typed signatures.

**Operator Application:** A specific event where an operator is applied to a particular state, producing successor states. Each application gets an entry in ByteTrace with operator ID, input state, resulting state, and commit index.

**Certified Operator:** An operator that has been learned through induction and verified through the certification pipeline before promotion to the production operator set. Certification produces a proof artifact (certificate) including evidence of performance, validity of preconditions/effects, and verification of system invariants.

---

## Search Layer

**Episode:** One reasoning session from initial state to goal or failure. All intermediate states, applied operators, and evidence are recorded in ByteTrace for replay and audit.

**CommitIndex:** A monotonic counter indexing each committed operator application in an episode. Ensures no gaps or out-of-order events in the recorded sequence.

**Landmark:** A discovered state that reliably leads to goal satisfaction. Sterling compresses experience into landmarks: "if we can reach this state, the goal is easy." Landmarks are durable memory artifacts.

---

## Governance

**DEV / CERTIFIED:** Sterling Native's two governance modes:
- **CERTIFIED mode:** Fail-closed. All invariants enforced. Produces promotion-eligible artifacts.
- **DEV mode:** Permissive. Requires explicit witnesses. Cannot produce promotion-eligible artifacts.

> **Legacy note:** v1 used multiple RunIntent modes (CERTIFYING, PROMOTION, REPLAY, EXPLORATORY, DEV). These are collapsed into DEV/CERTIFIED for Sterling Native.

**Neural Usage Contract:** Neural components (LLMs, encoders) may parse, rank, compress, and realize. They may NOT create operators, bypass preconditions, or mutate state. Enforced by API shape, not by policy. See [`neural_usage_contract.md`](neural_usage_contract.md) and [ADR 0003](../adr/0003-neural-advisory-only.md).

**Invariants (INV-CORE-01 through INV-CORE-11):** The architectural invariants that define Sterling's non-negotiable properties. See [`core_constraints.md`](core_constraints.md).

---

## Evidence and Learning

**Dual-Stream Evidence:** Sterling's induction substrate uses two evidence streams:
- **Stream A (SemanticDeltaIR):** What changed in the utterance's semantic content when an operator was applied.
- **Stream B (ObservationIR):** What happened in the world/dialogue as a result of the transition.

Together they link "what changed" with "why it mattered" for credit assignment.

**Claim / Falsifier:** The core unit of Sterling's proof system. A **claim** is a testable assertion about a capability. A **falsifier** is a concrete condition that would disprove the claim. Claims without falsifiers are not admissible.

---

## Domain Integration

**Capsule:** A Sterling-owned contract package for a capability primitive. Contains contract types, conformance suites, invariants, and proof artifact definitions. No domain imports, no domain constants.

**Rig:** A domain-owned certification surface that produces evidence streams, provides adapters connecting a domain to a capsule, and generates proof artifacts. The rig does not define the primitive — the capsule does.

**Transfer Pack:** A portable evidence bundle demonstrating that a capability transfers across domains. Requires conformance suite results from at least two structurally different domains.

**Capability Primitive (P01–P21):** A formally specified capability Sterling should be able to claim. Each primitive defines a formal signature, proof requirements, minimal substrate requirements, and certification gates. See [`docs/specs/primitives/`](../specs/primitives/00_INDEX.md).

---

## Compilation and Encoding

**Schema Descriptor:** A versioned, content-addressed description of a data format. Used in the compilation boundary to ensure payloads are compiled against known schemas.

**Registry Snapshot:** A point-in-time capture of the Code32 registry state. Part of the compilation boundary triple `(payload, schema_descriptor, registry_snapshot)`.

**Epoch:** A bounded interval of stable schema/registry state. Schema changes or registry expansions trigger epoch transitions with explicit handshake protocols.
