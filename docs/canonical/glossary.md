---
status: "v2 canonical — curated vocabulary for Sterling Native."
authority: canonical
scope: "Definitions enforced by contracts, invariants, and ADRs. For the comprehensive v1 glossary (historical), see docs/reference/v1/glossary_full.md."
---
# Sterling Native Glossary

---

## Carrier Layer

**Code32:** A 32-bit identity atom composed of `[domain: u8, kind: u8, local_id: u16le]` (8/8/16 byte-structured layout). Every semantic entity in Sterling has exactly one Code32 identity. The registry is bijective: one entity ↔ one Code32. See [`code32_bytestate.md`](code32_bytestate.md).

**ByteState:** The canonical packed runtime representation of Sterling's semantic state. ByteStateV1 uses a two-plane encoding (identity plane + status plane) at ~640 bytes per state snapshot. ByteState is the only runtime truth — all reasoning operates on compiled ByteState, not on IR objects directly. See [`bytestate_compilation_boundary.md`](bytestate_compilation_boundary.md).

**ByteTrace:** The canonical persisted trace format for replay verification and certification. ByteTrace is an append-only sequence of ByteState snapshots plus operator application metadata. It is the replay spine — StateGraph is a derived view rendered from ByteTrace. See [ADR 0002](../adr/0002-byte-trace-is-canonical.md).

**Compilation Boundary:** The function `compile(payload, schema_descriptor, registry_snapshot) → ByteState`. This is the architectural spine of Sterling Native. Domain payloads enter through compilation; nothing bypasses it. See [ADR 0001](../adr/0001-compilation-boundary.md).

---

## State Layer

**UtteranceState:** The annotated state of a single user utterance or sentence. Contains layered linguistic information (syntax, semantics) for one unit of text. Sterling enforces canonical utterance granularity: one UtteranceState per sentence or snippet.

**WorldState:** External context and environment state that accompanies an UtteranceState. Includes domain-specific observations, dialogue phase, and any external signals relevant to reasoning.

**StateGraph:** *(v1 terminology)* The graph of state nodes connected by operator applications (edges). In Sterling Native, the search-layer equivalent is `SearchGraphV1` (`search_graph.json`) — a deterministic derived view rendered from `SearchTapeV1`. At the carrier layer, `ByteTraceV1` is the canonical persistence format with no separate graph artifact. See [ADR 0002](../adr/0002-byte-trace-is-canonical.md).

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

**SearchNodeV1:** A node in the search graph representing a compiled state plus metadata (depth, cost, parent, state fingerprint). Nodes are created during frontier expansion and recorded in the search transcript.

**BestFirstFrontier:** The frontier data structure for search. Orders nodes by `(f_cost, depth, creation_order)` — lower cost first, shallower first, older first. Uses a visited set keyed by state fingerprint for deduplication.

**SearchGraphV1:** The canonical JSON artifact (`search_graph.json`) recording the full search transcript: expansion events, candidate records, node summaries, metadata bindings, and termination reason. Deterministic across runs given identical inputs. This is the search-layer analogue of ByteTrace — the auditable record of what happened.

**SearchPolicyV1:** Controls search behavior: dedup strategy (`DedupKeyV1`), step budget, candidate limits per node, pruning policy. Captured as part of the bundle's metadata bindings and bound into the verification report.

**SearchTapeV1 (.stap):** Binary hot-loop event log recording the search process with minimal overhead. Contains a canonical JSON header (binding fields: world_id, registry_digest, policy digests, scorer_digest, root state fingerprint, schema version), record frames for each search event, and a footer. Chain-hash integrity across records ensures tamper detection during parsing.

**Tape chain hash:** A running SHA-256 hash accumulated across tape records. Each record's hash input includes the previous record's hash, forming a hash chain. Verified by the tape reader during parsing — any tampered record breaks the chain.

**Tape→graph equivalence:** A Cert-mode verification check: render a parsed tape into `SearchGraphV1`, serialize to canonical JSON bytes, and compare byte-for-byte to the bundle's `search_graph.json`. Proves the tape and graph describe identical search behavior.

**ValueScorer:** Trait for scoring candidate actions during search. Implementations: `UniformScorer` (all candidates score equally — baseline) and `TableScorer` (per-candidate bonuses from an injected lookup table with digest binding).

**Episode:** One reasoning session from initial state to goal or failure. All intermediate states, applied operators, and evidence are recorded for replay and audit.

**CommitIndex:** A monotonic counter indexing each committed operator application in an episode. Ensures no gaps or out-of-order events in the recorded sequence.

**Landmark:** A discovered state that reliably leads to goal satisfaction. Sterling compresses experience into landmarks: "if we can reach this state, the goal is easy." Landmarks are durable memory artifacts. (Not yet implemented in v2.)

---

## Governance

**DEV / CERTIFIED:** Sterling Native's two governance modes:
- **CERTIFIED mode:** Fail-closed. All invariants enforced. Produces promotion-eligible artifacts.
- **DEV mode:** Permissive. Requires explicit witnesses. Cannot produce promotion-eligible artifacts.

> **Legacy note:** v1 used multiple RunIntent modes (CERTIFYING, PROMOTION, REPLAY, EXPLORATORY, DEV). These are collapsed into DEV/CERTIFIED for Sterling Native.

**Neural Usage Contract:** Neural components (LLMs, encoders) may parse, rank, compress, and realize. They may NOT create operators, bypass preconditions, or mutate state. Enforced by API shape, not by policy. See [`neural_usage_contract.md`](neural_usage_contract.md) and [ADR 0003](../adr/0003-neural-advisory-only.md).

**Invariants (INV-CORE-01 through INV-CORE-11):** The architectural invariants that define Sterling's non-negotiable properties. See [`core_constraints.md`](core_constraints.md).

---

## Evidence Layer

**ArtifactBundleV1:** A content-addressed evidence container produced by the harness. Contains named artifacts (each with content hash and normative flag), a manifest, a digest basis (the normative projection), and a bundle digest. Verified fail-closed by `verify_bundle()`.

**Normative artifact:** An artifact that participates in the bundle's digest basis. Normative artifacts are canonicalized and cryptographically bound into the bundle identity. Examples: `search_graph.json`, `policy_snapshot.json`, `search_tape.stap`. Observational artifacts (like `.bst1` traces) are bound indirectly via hash commitments in the normative verification report.

**Digest basis:** The canonical JSON projection of a bundle's normative artifacts, sorted by name. The bundle digest is computed over the digest basis bytes under a domain-separated hash (`DOMAIN_BUNDLE_DIGEST`).

**VerificationProfile:** Controls verification strictness for bundles.
- **Base:** Verifies integrity (content hashes, manifest, metadata bindings) when evidence is present. Tape is optional — bundles from earlier milestones without tape pass Base verification.
- **Cert:** Requires tape presence. Adds tape→graph canonical byte equivalence. Cert is the promotion-eligible profile.

`verify_bundle()` defaults to Base. `verify_bundle_with_profile(bundle, Cert)` enables the stricter profile.

> **Two independent axes:** DEV/CERTIFIED (above) controls *run eligibility* — whether a run may produce promotion-grade artifacts. Base/Cert controls *verification strictness* — what post-hoc checks are applied to a bundle. These are orthogonal: a CERTIFIED run's bundle can be verified at Base or Cert profile; a DEV run cannot produce promotion-eligible bundles regardless of verification profile.

**PolicySnapshotV1:** A canonical JSON artifact capturing the policy configuration used for a run. Included as a normative bundle artifact and bound into the verification report via `policy_digest`.

**Verification report:** A normative JSON artifact (`verification_report.json`) containing binding digests that cross-reference other artifacts in the bundle: `search_graph_digest`, `policy_digest`, `tape_digest`, `codebook_hash`, and optional `scorer_digest`. The report is the "glue" that binds independently-hashed artifacts into a coherent evidence package.

**Two evidence layers:** Sterling Native produces distinct evidence artifacts certifying different layers:
1. **Carrier replay** — `ByteTrace` (`.bst1`): proves deterministic compile→apply execution.
2. **Search replay** — `SearchTapeV1` (`.stap`) + `SearchGraphV1`: proves deterministic search execution with chain-hash integrity and (Cert) tape→graph equivalence.

Both can coexist in a bundle and are verified independently.

### Corridor Binding Artifacts

**OperatorRegistryV1:** A normative bundle artifact (`operator_registry.json`) containing the operator catalog: op IDs, signatures, effect kinds, precondition/effect masks, and contract metadata. Its content hash is bound as `operator_set_digest` in the verification report, search graph metadata, and tape header. `apply()` requires a registry snapshot — there is no callable path that bypasses it. See `kernel/src/operators/operator_registry.rs`.

**ConceptRegistryV1:** A normative bundle artifact (`concept_registry.json`) containing the `RegistryV1` canonical bytes — the Code32↔ConceptID bijective mapping used during compilation. Its semantic digest (computed via `HashDomain::RegistrySnapshot`) is bound into the compilation manifest (`registry_hash`) and graph metadata (`registry_digest`). In Cert mode, presence is mandatory and compilation replay uses it to prove the compilation boundary is reproducible. See `kernel/src/carrier/registry.rs`.

**CompilationManifestV1:** A normative bundle artifact (`compilation_manifest.json`) recording compilation boundary provenance: schema descriptor, registry digest, payload hash, root identity digest, and root evidence digest. Field-level coherence is verified against graph metadata and the verification report by `verify_bundle()`. This artifact enables offline audit of what inputs produced the initial ByteState.

**Registry Coherence:** The invariant that `registry_digest` / `registry_hash` values in `compilation_manifest.json`, `search_graph.json` metadata, `verification_report.json`, and the `concept_registry.json` artifact all agree. Enforced by `verify_bundle()` Steps 12b–12c (RCOH-001, CRART-001).

**Root State Fingerprint:** The SHA-256 digest of the initial ByteState before any operator application. Bound in the `SearchTapeV1` header and `search_graph.json` metadata. Proves that search frontier initialization is deterministic and traceable to a specific compilation output (BIND-001, IDCOH-001).

**Compilation Replay:** The process of reconstructing identical ByteState from bundle-shipped inputs (`concept_registry.json` bytes, schema descriptor from `compilation_manifest.json`, fixture payload) via `compile()`. Cert-mode verification performs this replay and asserts byte-identical output, proving the compilation boundary is reproducible without access to the original domain state (CREPLAY-001).

## Learning

**Dual-Stream Evidence:** Sterling's induction substrate uses two evidence streams:
- **Stream A (SemanticDeltaIR):** What changed in the utterance's semantic content when an operator was applied.
- **Stream B (ObservationIR):** What happened in the world/dialogue as a result of the transition.

Together they link "what changed" with "why it mattered" for credit assignment. (Not yet implemented in v2.)

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
