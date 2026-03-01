---
authority: reference
status: advisory
---

# Absorption Pipeline

**Advisory -- not normative.** This document describes design rationale for
Sterling's capability absorption pipeline and differentiation thesis. Do not
cite as canonical. See [parity audit](../../architecture/v1_v2_parity_audit.md)
for capability status.

## Core Rule

Sterling owns semantics at the level of contracts and invariants, not at the
level of domain objects or domain algorithms. Domains own sensors, object
models, raw feeds, and implementation details. Sterling owns the definition of
what it means for a capability to exist, how it is tested, how it is claimed,
and how it composes with other capabilities.

## Three Boundary Separations

These separations must remain explicit. Blurring any of them produces domain
coupling that is invisible until a second domain attempts to implement the same
primitive.

### 1. Data Plane vs Control Plane

The **data plane** is the shape of information crossing the boundary:
envelopes, deltas, snapshots, operator signatures. It must remain stable.

The **control plane** is how Sterling and a domain negotiate what they can do:
capability claims, schema versions, feature flags, budgets. It must be explicit
and auditable.

**Failure mode if blurred:** A schema field rename silently changes behavior.
A domain starts sending extra fields and Sterling starts depending on them.

### 2. Structural Contract vs Semantic Contract

**Structural contract:** types, required fields, enums, versioning,
determinism requirements at the serialization layer.

**Semantic contract:** invariants, ordering constraints, monotonicity rules,
boundedness, fail-closed rules, and how uncertainty behaves.

Most systems stop at structural. That is how they become domain-coupled: the
only thing shared is a JSON shape, but not the meaning.

### 3. Domain Ontology vs Sterling Ontology

Domains will always have their own class labels, taxonomy depth, and feature
vocabularies. Sterling must not internalize those as core concepts. Domain
vocabulary must be namespaced, versioned, and either treated as opaque tokens
or explicitly aligned via a governed mapping artifact.

## Six-Step Capability Absorption Pipeline

Absorption is a pipeline of artifacts, not a pipeline of code reuse. When
Sterling encounters a new capability, the goal is to extract the invariant,
encode it as a contract with tests and proof, and register it so any future
domain can claim the same capability by passing the same gates.

1. **External domain presents data and actions.** Identify the primitive
   boundary. Write down what Sterling needs and what it guarantees.

2. **Define capsule contracts, conformance suites, and invariants.** No
   domain imports. No domain constants. If a capsule contains a reference to a
   domain-specific concept, that concept must be injected through a declared
   adapter interface.

3. **Build fixtures and prove portability.** Two fixture sets from
   structurally different domains must pass the same conformance suite for a
   primitive to be considered domain-agnostic.

4. **Domain implements adapter, passes certification.** The adapter is the
   only domain-specific glue Sterling should ever need.

5. **Register the capability claim with evidence.** Sterling must answer
   "Does this domain implement this primitive?" without reading code.

6. **Runtime handshake, enforcement, fail-closed.** Domain announces
   capabilities on connect. Sterling enforces schema versions and invariants
   online. Anything not enforceable online remains enforceable via CI
   certification and post-hoc audit.

## Six Domain Coupling Prevention Rules

1. **Sterling contracts never mention domain object models.** No entity
   names, no domain taxonomy. Only "evidence item," "operator signature,"
   "capability claim."

2. **Domain semantics enter only through declared, injectable components.**

3. **Feature vocabularies must be namespaced and treated as opaque by
   default.**

4. **Any semantic strengthening must be introduced as an extension
   capability.** Base primitive stays minimal.

5. **Contract drift must be caught structurally and semantically.** Hashes
   detect structural drift; conformance suites detect semantic drift.

6. **Online enforcement is fail-closed; offline enforcement is cert-based.**

## Sterling Differentiators vs Graph-RAG

Sterling diverges from graph-RAG and agent-framework architectures along
several design commitments:

- **Edge-relative plasticity.** Sterling tracks fine-grained edge statistics
  (times taken, times available, weight) approximating a policy over paths,
  not just node-level scalar importance.

- **Compression-gated decay.** Nodes are not gradually dropped. The system
  must first summarize an episode into a dedicated node, then mark the
  detailed path as decay-eligible. This creates a two-level semantic
  hierarchy: detailed micro-paths and compressed episode summaries.

- **Episodic summaries as structural objects.** Summaries are nodes in the
  same graph space with attached latent vectors, connected to the micro-nodes
  they compress, and used for path selection. Not text blobs in a database.

- **Transformer-as-codec stance.** The transformer is a replaceable module
  for parsing input and rendering output. It is not the cognitive core. No
  free-form chain-of-thought in the decision loop. All state transitions use
  operators.

## The Wikipedia Game as Mental Model

The clearest mental model for Sterling's path-finding thesis: start on a
random Wikipedia article and reach a target article in N hyperlink hops. Over
time, players discover landmarks (pages from which the target is always
reachable), learn good routes, identify dead zones, and compress experience
into "get to X and you are basically there."

Sterling's path algebra, SWM, and episode summaries formalize this pattern:

- Nodes = operators, frames, facts, schemas, summaries
- Edges = typed relations with plasticity statistics
- Episodes = reasoning sessions through the graph
- Landmarks = episodic summaries and global anchors with high edge weight
- Dead ends = nodes whose outgoing edges rarely appear on successful paths

The system remembers routes, not word sequences. It compresses local detail
into reusable landmarks. It retains the transformer only for translating
between symbols and sentences at I/O boundaries.
