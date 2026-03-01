---
authority: reference
status: advisory
---

# Capability Absorption Pipeline

**Advisory -- not normative.** This document describes a design blueprint for
absorbing new domains and capabilities into Sterling without creating parallel
semantics or "Sterling-light" drift. Do not cite as canonical. See the canonical
docs and ADRs for what is actually enforced today.

## Proven by Existing Code

These are the current, verifiable boundary decisions and enforcement surfaces:

- **Compilation boundary and carrier artifacts** exist and are enforced by the
  Rust substrate. See
  [bytestate compilation boundary](../../canonical/bytestate_compilation_boundary.md)
  and `kernel/src/carrier/`.
- **Canonicalization and domain-separated hashing** are centralized and locked
  via `HashDomain` and canonical encoding rules. See
  [core constraints](../../canonical/core_constraints.md),
  [search evidence contract](../../canonical/search_evidence_contract.md),
  and `kernel/src/proof/`.
- **Neural components are advisory-only** (never authoritative for decisions);
  this is pinned by [ADR 0003](../../adr/0003-neural-advisory-only.md).
- **Certification authority boundary** (control plane vs evidence
  generator/verifier) is pinned by
  [ADR 0006](../../adr/0006-certification-authority-boundary.md).
- **Evidence packaging and nesting** decisions are pinned by
  [ADR 0007](../../adr/0007-evidence-packaging-nesting.md).
- **Schema extension policy** is additive-only (no silent semantic
  reinterpretation) per
  [ADR 0008](../../adr/0008-schema-extension-additive-fields.md).
- **World harness and search contracts** exist as the structural integration
  surface for domains. See the
  [parity audit](../../architecture/v1_v2_parity_audit.md) capability D1 and
  the corresponding crate surfaces in `harness/` and `search/`.

Everything below is a design blueprint unless explicitly promoted into canonical
docs or ADRs.

## Design Blueprint (Proposed)

### Core Rule

Sterling owns semantics at the level of contracts and invariants, not at the
level of domain objects or domain algorithms. Domains own sensors, object
models, raw feeds, and implementation details. Sterling owns the definition of
what it means for a capability to exist, how it is tested, how it is claimed,
and how it composes with other capabilities.

### Authority Boundaries

Goal: preserve a single semantic authority and avoid duplicating interpretation
logic across layers.

**1. Data plane vs control plane**

- Data plane: evidence artifacts, carrier compilation, deterministic replay and
  verification.
- Control plane: orchestration, prompts/UX, policy selection, scheduling.
- Failure mode if blurred: a schema field rename silently changes behavior. A
  domain starts sending extra fields and Sterling starts depending on them.

**2. Structural contract vs semantic contract**

- Structural contract: types, required fields, enums, versioning, determinism
  requirements at the serialization layer.
- Semantic contract: invariants, ordering constraints, monotonicity rules,
  boundedness, fail-closed rules, and how uncertainty behaves.
- Most systems stop at structural. That is how they become domain-coupled: the
  only thing shared is a JSON shape, but not the meaning.

**3. Domain ontology vs Sterling ontology**

- Domain ontology describes domain-specific entities and relations.
- Sterling ontology describes invariant reasoning constructs (state, operators,
  evidence, policy).
- Domain vocabulary must be namespaced, versioned, and either treated as opaque
  tokens or explicitly aligned via a governed mapping artifact.

### Six-Step Absorption Pipeline

Absorption is a pipeline of artifacts, not a pipeline of code reuse. When
Sterling encounters a new capability, the goal is to extract the invariant,
encode it as a contract with tests and proof, and register it so any future
domain can claim the same capability by passing the same gates.

1. **Define the claim surface.** What does this domain/capability claim to do?
   What falsifiers must exist? What evidence artifacts must exist for
   certification?

2. **Define the world contract surface.** Specify the state representation,
   operator set, goal predicate, and observation model. Ensure the world is
   representable as a governed search problem (no hidden semantics in glue
   code).

3. **Compile into the carrier boundary.** Define a compilation mapping into the
   carrier representation (ByteState/Code32). Ensure compilation is
   deterministic and replayable.

4. **Record evidence and enforce replay.** Produce a complete evidence bundle
   that can be verified and replayed. Fail closed on missing artifacts or
   mismatched digests.

5. **Add governance hooks.** Bind runs to policy snapshots and verification
   profiles. Ensure any higher-layer claim is reducible to Rust-verifiable
   artifacts (per ADR 0006).

6. **Promote and admit.** Admit only after transfer evidence exists (multiple
   fixtures, stress axes) and after regression harnesses exist.

### Domain Coupling Prevention Rules

1. **Sterling contracts never mention domain object models.** No entity names,
   no domain taxonomy. Only "evidence item," "operator signature," "capability
   claim."
2. **Domain semantics enter only through declared, injectable components.**
3. **Feature vocabularies must be namespaced and treated as opaque by default.**
4. **Any semantic strengthening must be introduced as an extension capability.**
   Base primitive stays minimal.
5. **Contract drift must be caught structurally and semantically.** Hashes
   detect structural drift; conformance suites detect semantic drift.
6. **Online enforcement is fail-closed; offline enforcement is cert-based.**

### Verification Checklist (for Future Promotions)

A capability is "absorbable" when:

- Its world contract is deterministic under certification.
- Its evidence bundle is complete and replayable.
- Its claims are falsifiable with explicit stress axes.
- Its integration does not introduce new authority surfaces.
- Its schema changes are additive and audited.

## Sterling Differentiators vs Graph-RAG

Sterling diverges from graph-RAG and agent-framework architectures along
several design commitments. These are aspirational design goals, not claims
about shipped features:

- **Edge-relative plasticity.** Sterling would track fine-grained edge
  statistics (times taken, times available, weight) approximating a policy over
  paths, not just node-level scalar importance.

- **Compression-gated decay.** Nodes would not be gradually dropped. The system
  would first summarize an episode into a dedicated node, then mark the detailed
  path as decay-eligible.

- **Episodic summaries as structural objects.** Summaries would be nodes in the
  same graph space with attached latent vectors, connected to the micro-nodes
  they compress.

- **Transformer-as-codec stance.** The transformer is a replaceable module for
  parsing input and rendering output. It is not the cognitive core. No free-form
  chain-of-thought in the decision loop. All state transitions use operators.

## The Wikipedia Game as Mental Model

The clearest mental model for Sterling's path-finding thesis: start on a random
Wikipedia article and reach a target article in N hyperlink hops. Over time,
players discover landmarks (pages from which the target is always reachable),
learn good routes, identify dead zones, and compress experience into "get to X
and you are basically there."

The proposed path algebra, SWM, and episode summaries would formalize this
pattern:

- Nodes = operators, frames, facts, schemas, summaries
- Edges = typed relations with plasticity statistics
- Episodes = reasoning sessions through the graph
- Landmarks = episodic summaries and global anchors with high edge weight
- Dead ends = nodes whose outgoing edges rarely appear on successful paths

The system would remember routes, not word sequences. It would compress local
detail into reusable landmarks. It would retain the transformer only for
translating between symbols and sentences at I/O boundaries.
