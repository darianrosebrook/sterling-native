> **NOTE: Quarantined v1 canonical contract. Non-authoritative for Sterling Native. See [promotion criteria](README.md).**

# The Sterling Reasoning Framework

**Author**: @darianrosebrook
**Version**: 1.3
**Last Updated**: February 17, 2026
**Status**: Canonical Theory Document

> **Note**: This document now references canonical definitions in `docs/canonical/`. Core Constraints v1, the Neural Usage Contract, and Evaluation Gates are maintained as single-source artifacts to prevent documentation drift.

---

## Sterling's Goal

> **Sterling's Goal**: Build a domain-agnostic reasoning engine that:
>
> 1. Represents linguistic/semantic content as structured **UtteranceStates** with layered annotations (syntax, semantics, semiotics, pragmatics, latent)
> 2. Traverses a **state graph** of possible interpretations using typed operators (S/M/P/K/C)
> 3. Compresses semantic content into **latent representations** for efficient similarity search and value estimation
> 4. Maintains all reasoning as **auditable, symbolic inference** (no hallucination at the IR level)
> 5. Adapts to **multiple domains** through pluggable world interfaces (KG, constraints, rewards)

This goal statement is shared across all Sterling theory documents. See `GLOSSARY.md` for canonical terminology.

---

## Overview

Sterling is a structured, symbolic reasoning engine for language that represents each _UtteranceState_ with layered linguistic annotations and traverses a search graph of possible interpretations.

---

## Domain Agnosticism

Sterling's core reasoning engine is **domain-agnostic**. The operator taxonomy (S/M/P/K/C), state-graph search, and latent compression apply regardless of domain. Specific domains are implemented as **worlds** that provide:

- **KG vocabulary**: Entity kinds and relation types
- **Constraint handlers**: Domain-specific validation logic
- **Reward signals**: Success metrics for search
- **Lexicon**: Terminology definitions

CAWS/linguistics is **World #1** because it provides a precise, rule-heavy sandbox for proving the core thesis. Future worlds (code refactoring, spec editing, planning) will exercise different aspects of the same engine.

For the world interface specification, see [World Adapter Protocol](world_adapter_protocol_v1.md).

---

## Relationship to StateGraph and Worlds

Sterling operates at multiple levels of abstraction:

- **UtteranceState**: What we reason _about_ (linguistic content with layered annotations)
- **StateGraph**: How we reason _over time_ (nodes are states, edges are operator applications)
- **KnowledgeContext**: The activated slice of the KG for a given reasoning step
- **World**: Where domain knowledge and constraints live (pluggable adapter)

A **StateGraph node** is a bundle: `{UtteranceState(s) + KnowledgeContext + search-local metadata}`. This prevents inventing a third, slightly different "State" concept.

---

## Non-Goals

Sterling is explicitly **not** trying to:

- **Replace LLMs at language generation** (surface form production) - transformers handle text-to-IR and IR-to-text translation
- Be a general-purpose chatbot or open-domain conversational agent
- Use the latent space for inventing new facts (only compression and value estimation)
- Achieve human-level general intelligence in v1
- Support arbitrary natural language generation without symbolic grounding

Sterling **is** trying to:

- **Replace LLMs as the reasoning substrate** (the cognitive core / long-horizon state machine)
- Prove that structured semantic search outperforms token-context scaling for reasoning tasks
- Demonstrate that neural components can be confined to I/O and compression without becoming the cognitive core

This is the key distinction: transformers handle the translation between symbols and sentences, but the semantic navigation - the actual reasoning - happens in the graph. See [docs/canonical/north_star.md](../canonical/north_star.md) for the full operational definition.

These boundaries help prevent scope creep and keep the research focused.

---

## Governance Invariants (Core Constraints v1)

Sterling enforces 11 architectural invariants that block merge/deploy if violated. These are the "constitution" that prevents Sterling from drifting into "just another LLM agent with a fancy index."

<!-- BEGIN CANONICAL: core_constraints_v1 -->
| ID | Constraint | Description |
|----|------------|-------------|
| INV-CORE-01 | No Free-Form CoT | No generative LLM chain-of-thought in the decision loop |
| INV-CORE-02 | Explicit State | All task state in UtteranceState + KG, not transformer KV cache |
| INV-CORE-03 | Structural Memory | Episode summaries + path algebra for long-horizon, not transcript prompts |
| INV-CORE-04 | No Phrase Routing | No phrase dictionary or regex-based routing; all routing via scored search |
| INV-CORE-05 | Computed Bridges | Cross-domain bridges computed at runtime, not static lookup tables |
| INV-CORE-06 | Contract Signatures | Landmark/operator signatures are typed contracts, not learned embeddings |
| INV-CORE-07 | Explicit Bridge Costs | Domain transitions carry explicit costs with hysteresis |
| INV-CORE-08 | No Hidden Routers | All routing decisions auditable via StateGraph; no secret bypasses |
| INV-CORE-09 | Oracle Separation | No future/oracle knowledge in inference inputs; only in training signals |
| INV-CORE-10 | Value Target Contract | Canonical value targets versioned and hash-verified |
| INV-CORE-11 | Sealed External Interface | External tools cannot mutate internal state except via governed operators with declared write-sets |

See [docs/canonical/core_constraints_v1.md](../canonical/core_constraints_v1.md) for full details.
<!-- END CANONICAL: core_constraints_v1 -->

### Neural Usage Contract

**Core Principle: Neural is advisory; Symbolic is authoritative.**

Neural components may rank or prioritize **already-legal** symbolic moves (hybrid value function), but they cannot create new operators, bypass operator preconditions, mutate KG/UtteranceState directly, introduce new facts, or override symbolic logic.

See [docs/canonical/neural_usage_contract.md](../canonical/neural_usage_contract.md) for the full contract.

---

## Proof Systems (Auditability Made Executable)

**Status**: Implemented (TD-12, MS), Partial (TD-12 + MS integration)

Sterling's goal of "auditable, symbolic inference" is made executable by two **proof systems**:

| System | Purpose | Relationship |
|--------|---------|--------------|
| **TD-12** | Run/step certification with hash-locked artifacts | Certifies reasoning run outputs |
| **MS (Memory Substrate)** | Long-horizon semantic ledger with replay verification | Extends TD-12 guarantees across time |

**Key Distinction**: The reasoning engine is **not** a verification system. It produces states, IR, and traces. The proof systems (TD-12, MS) verify that those outputs are correct and replayable.

**Proof Composition** (not a chain):
- TD-12 certificates can reference MS certificates via `ms_certificate_ref`
- TD-12 verifier re-runs MS verification (authoritative, not trusting summaries)
- Each system has its own policies, levels, and semantics

See `README.md#proof-composition` for overview. See `docs/versions/MS/` for Memory Substrate theory and implementation.

---

## Scope and Primary Object

Sterling's primary object is an **utterance** (sentence + context) analyzed through layered linguistic annotations. General reasoning over knowledge graphs is enabled through operators that query and update WorldState, but the core unit of analysis remains the UtteranceState. This document focuses on single-utterance analysis and its interaction with background knowledge. Multi-utterance discourse is handled, in theory, by the same machinery.

At its core, an **UtteranceState** encapsulates the current analysis of an input (or hypothesized) sentence across multiple levels:

- **Syntax (Structure):** a parsed representation (e.g. parse tree, part-of-speech tags, morphological features).
- **Semantics (Meaning):** the conceptual or logical content derived from the syntax. While syntax shows how words form sentences, semantics captures _what is meant_ – the propositions or semantic relations expressed[medium.com](https://medium.com/@adecressac/blog-6-what-is-semantics-understanding-meaning-in-language-220a85b84e40#:~:text=Semantics%20is%20a%20cornerstone%20of,In%20this).
- **Semiotics (Signification):** the association between signs (words or symbols) and their referents or abstract meanings. In practice, this includes lexical senses and sense relations (e.g. WordNet synsets or other lexicon entries) that tie surface forms to concepts.
- **Pragmatics (Contextual Use):** discourse context, speaker intent, implicatures, and real-world knowledge that disambiguate or enrich meaning. Pragmatic analysis interprets _how_ the utterance is used (and what it implies) based on situational context and conversational norms.
- **Latent Compression (Embedding Vector):** a fixed-dimensional vector (e.g. from an autoencoder or transformer model) that compactly represents the utterance's content. This "latent" layer allows semantic similarity and retrieval (e.g. nearest-neighbor from a training corpus) but is not itself the source of truth. Rather, it serves as a backup memory or sketch of meaning that can guide but not override the symbolic analysis.

## Neural Compression Layer

The latent layer in Sterling is not merely a backup memory, but a **compressed index over meaning-space** that enables operating in a high-dimensional, continuous space while keeping ground-truth reasoning symbolic. This compression architecture is central to Sterling's research direction of achieving high-dimensional, highly compressed representations more capable than traditional LLMs.

### Compression Architecture

**Input**: The latent encoder takes as input:

- Semantic IR graph (events, entities, relations)
- Local KG neighborhood (entities and frames activated for this utterance)
- Optionally: syntax structure and pragmatic context

**Output**: High-dimensional latent representation. The format here is flexible (for example dense vectors or fixed-layout carrier-aligned tensors), but it must be:

- Fixed-dimensional (same size regardless of input complexity)
- Dense (compact representation of semantic content)
- Structured (preserves relationships in compressed form)

**Training Objective**: The encoder is trained to:

- Reconstruct IR/KG neighborhood from latent (reconstruction loss)
- Predict value function scores (value prediction loss)
- Or both (multi-task learning)

### Latent Roles

The latent representation serves three primary functions:

1. **Fast Similarity Search**: Nearest-neighbor retrieval of related IRs from a training corpus or memory bank. This enables rapid lookup of similar semantic structures without full symbolic reasoning.

2. **State-Value Estimation**: The latent encoding feeds into the neural value function `V_neural(s_latent)` that predicts branch promise for search algorithms. This provides soft heuristics that complement symbolic constraints.

3. **Long-Horizon Memory**: Stable anchors for previously encountered meanings. The latent serves as a compressed "fingerprint" that can be stored and retrieved across reasoning sessions, enabling cross-episode learning and memory.

### Critical Guarantee

**The latent never directly generates new facts.** All factual updates must be justified at the symbolic IR layer. The latent is used for:

- Retrieval (finding similar structures)
- Heuristics (estimating branch promise)
- Memory (storing compressed representations)

But it cannot create new semantic content. All new propositions must come from operator applications over symbolic IR, ensuring that reasoning remains auditable and grounded.

This aligns with the **Neural Usage Contract**: neural is advisory, symbolic is authoritative. Neural encoders are permitted for compression/indexing/scoring, and may rank or prioritize already-legal symbolic moves. But they are non-authoritative: they cannot emit new operators, mutate KG/UtteranceState, introduce new facts, or override symbolic constraints. See [docs/canonical/neural_usage_contract.md](../canonical/neural_usage_contract.md).

### Connection to Compression Thesis

This compression architecture enables Sterling to operate in a high-dimensional latent space (potentially more expressive than traditional token embeddings) while maintaining symbolic grounding. The latent acts as a "semantic sketch" that guides search and enables efficient similarity operations, but the source of truth remains the explicit IR graph. This design allows Sterling to leverage the representational power of high-dimensional spaces without sacrificing the interpretability and safety of symbolic reasoning.

In Sterling, each layer is populated and updated as reasoning proceeds. For example, an initial sentence may first be parsed syntactically, then semantic frames or logical predicates are attached, then pragmatic context is applied. This multi-layer structure ensures that _syntax_ and _semantics_ are kept separate (syntax provides shape, semantics provides content[medium.com](https://medium.com/@adecressac/blog-6-what-is-semantics-understanding-meaning-in-language-220a85b84e40#:~:text=Semantics%20is%20a%20cornerstone%20of,In%20this)), and that context (pragmatics) can modify or select among literal interpretations. For instance, the word "bank" may have two semantic senses (river bank vs. financial bank); the pragmatic layer uses context to pick the intended sense. Frame semantics further grounds word meanings: each content word evokes a _frame_ of background knowledge (e.g. "sell" evokes a commerce frame with seller, buyer, goods, money)[en.wikipedia.org](https://en.wikipedia.org/wiki/Frame%5Fsemantics%5F%28linguistics%29#:~:text=Frame%20semantics%20is%20a%20theory,the%20money%2C%20the%20relation%20between). Thus, understanding an utterance requires accessing the relevant frames or encyclopedic knowledge associated with its words.

# State Model Specification

Sterling operates on a three-tier state hierarchy that separates utterance-level analysis from discourse-level context and world knowledge. This separation ensures that reasoning steps are auditable and that domain knowledge remains modular and pluggable.

## State Hierarchy

### UtteranceState

An **UtteranceState** represents the analysis of a single sentence or utterance in its local context. It contains:

- **syntax**: Parse tree, part-of-speech tags, dependency relations, morphological features
- **semantics**: Semantic IR (see below) representing logical/graph structures encoding meaning
- **semiotics**: Lexical sense mappings (WordNet synset IDs, custom lexicon entries) linking surface forms to concepts
- **pragmatics**: Local discourse links, speaker intent, implicatures relevant to this utterance
- **latent**: Fixed-dimensional vector encoding (see Neural Compression Layer below) for similarity search and heuristics

Each UtteranceState is self-contained for a single utterance but may reference entities and frames from WorldState.

**Note on latent scope**: In the initial design, latents are defined at the UtteranceState level: each utterance has one `latent_vector` summarizing its semantic IR and local knowledge context. Future extensions may define latents at DiscourseState or WorldState level (e.g., conversation embeddings, knowledge-slice embeddings), but those are out of scope for this document.

### DiscourseState

A **DiscourseState** (out of scope for this document's initial framing) is a collection of UtteranceStates with a discourse graph that tracks:

- **utterances**: Ordered set of UtteranceStates
- **coreference links**: Pronoun and definite description resolution across utterances
- **rhetorical relations**: Discourse structure (elaboration, contrast, cause-effect, etc.)
- **timeline**: Temporal ordering and event sequencing
- **speaker roles**: Participant tracking in multi-party dialogue

Multi-utterance reasoning uses the same operators and machinery as single-utterance analysis, but operates over DiscourseState rather than individual UtteranceStates.

### WorldState (KnowledgeContext)

A **WorldState** (also called **KnowledgeContext**) represents the slice of the knowledge graph currently activated for reasoning. It contains:

- **activated entities**: Entities, concepts, and frames currently in play
- **active rules**: Logical constraints and inference rules relevant to the current query
- **assumptions**: Propositions assumed for hypothetical reasoning
- **counterfactuals**: Alternative world scenarios being explored
- **provenance**: Source tracking for each fact (KB lookup, derivation, assumption)

WorldState is queried and updated by K-operators (Knowledge Graph operators) but remains separate from UtteranceState to maintain modularity. Multiple UtteranceStates may share the same WorldState.

## Layer Invariants

To ensure clean separation of concerns and prevent architectural drift, Sterling enforces the following invariants:

**I1 – Structural Separation**: Syntax is the only layer that encodes phrase structure and word order. Semantics may refer to token spans but does not store phrase structure as such. This ensures syntax provides shape while semantics provides content.

**I2 – Truth-Conditional Semantics**: The semantics layer consists of logical/graph structures (Sterling IR) whose nodes and edges are typed (event, entity, role, relation, modality, polarity, etc.). These structures encode truth-conditional content that can be evaluated against world knowledge. Sterling IR is designed to support at least first-order logic (FOL) level expressivity with: quantified statements ("all", "some"), predicates over entities, and basic modal tags (world_id, modality). This expressivity grounds the entailment examples and proof path concepts described throughout this document.

**I3 – Sense Anchoring**: Every semantic predicate is anchored to one or more lexical senses (semiotics layer) via stable IDs (e.g., WordNet synset ID, custom KG node ID). This ensures that meaning is grounded in lexical knowledge and can be traced back to surface forms.

**I4 – Latents as Summaries**: Latent vectors are summaries of (syntax + semantics + context) for retrieval and scoring purposes. They are never the ground-truth representation of meaning. All factual updates must be justified at the symbolic IR layer.

## Sterling IR Sketch

The semantics layer is represented as a typed graph IR (Sterling IR) that is the canonical, machine-manipulable representation of meaning. This IR serves as the interface between syntax (surface form) and world knowledge (KG), enabling operators to reason over structured semantic content.

### IR Structure

Sterling IR is a directed, typed graph with the following node and edge types:

**Node Types**:

- **events**: Predicates representing actions, states, or processes (e.g., "sell", "be", "arrive")
- **entities**: Referents for objects, people, places, concepts (e.g., "John", "cat", "bank")
- **attributes**: Properties or qualities (e.g., "red", "tall", "mammal")
- **propositions**: Complete truth-conditional statements that can be evaluated

**Edge Types**:

- **role fillings**: Connect events to entities filling semantic roles (agent, patient, instrument, location, etc.)
- **modifiers**: Connect attributes to entities or events they modify
- **coreference**: Link entities that refer to the same referent
- **temporal links**: Order events temporally (before, after, during, etc.)
- **logical relations**: Entailment, contradiction, independence between propositions

### IR Examples

For the sentence "John sells books to Mary":

```
Nodes:
  - event_1: [type: sell, lemma: sell, sense_id: wn:synset_12345]
  - entity_1: [type: person, text: John, sense_id: wn:synset_67890]
  - entity_2: [type: person, text: Mary, sense_id: wn:synset_67890]
  - entity_3: [type: object, text: books, sense_id: wn:synset_11111]

Edges:
  - (event_1, agent, entity_1)
  - (event_1, patient, entity_3)
  - (event_1, recipient, entity_2)
```

For the sentence "Cats are mammals":

```
Nodes:
  - event_1: [type: copula, lemma: be]
  - entity_1: [type: concept, text: cats, sense_id: wn:synset_cat]
  - entity_2: [type: concept, text: mammals, sense_id: wn:synset_mammal]

Edges:
  - (event_1, subject, entity_1)
  - (event_1, predicate_nominal, entity_2)
  - (entity_1, hyponym_of, entity_2)  # semantic relation from KG
```

### Connection Points

Sterling IR connects to other system components as follows:

- **Operators**: M-operators (Meaning operators) read and write IR nodes/edges. S-operators (Structural) populate IR from syntax. K-operators (Knowledge Graph) align IR entities with KG nodes.

- **KG Lookups**: IR entity nodes reference KG concept IDs via sense_id fields. Frame operators retrieve frame schemas from KG and instantiate them as IR structures.

- **Autoencoders**: IR is encoded into latent vectors for similarity search and value estimation. The encoder learns to compress IR + local KG neighborhood into fixed-dimensional representations while preserving semantic content.

**Implementation Note**: The current Sterling Light implementation uses IR_V1, which is a minimal, syntax-focused representation (syntax, dependency, spans) used in the current prototype. The semantic IR described here is IR_V2, the target representation that will be populated by semantic operators and connected to the knowledge graph.

## Data Model Sketch

### UtteranceState Fields

- `utterance_id`: Unique identifier for this utterance
- `surface_text`: Original input text
- `syntax`: Parse tree structure, POS tags, dependency relations
- `semantic_ir`: Sterling IR graph (see Sterling IR Sketch below)
- `semiotic_mappings`: Map from token indices to sense IDs (WordNet synsets, etc.)
- `pragmatic_context`: Local discourse links, speaker intent, implicatures
- `latent_vector`: Fixed-dimensional encoding for similarity search
- `score`: Current value estimate for search prioritization
- `visit_count`: Number of times this state has been expanded
- `age`: Timestamp or step count for decay calculations
- `novelty_flag`: Boolean indicating if this state introduces previously unseen structures

### WorldState Fields

- `active_entities`: Set of entity/concept IDs currently in play
- `active_frames`: Set of semantic frames activated for reasoning
- `active_rules`: Set of inference rules currently applicable
- `assumptions`: Map from assumption ID to proposition (for hypothetical reasoning)
- `world_id`: Identifier for the world context (actual, hypothetical, scenario_1, etc.)
- `provenance_map`: Map from proposition to its source (KB, derivation, assumption)

# Reasoning Operators

Sterling defines a set of discrete _operators_ that transition between UtteranceStates in a state graph. These operators are modular functions that perform local updates to one layer at a time, much like inference rules in a logic.

## Operator Taxonomy

Operators are organized into five categories based on their primary function and scope:

### S-operators (Structural)

Act primarily on syntax and semiotics layers:

- **ApplySyntaxRule**: Incrementally builds or modifies the parse tree (e.g., attach a noun phrase or verb phrase)
- **AttachModifier**: Links modifiers (adjectives, adverbs) to their heads in the parse structure
- **DisambiguatePOS**: Resolves part-of-speech ambiguity using context

**Scope**: Local to UtteranceState. Read: syntax, semiotics. Write: syntax.

### M-operators (Meaning)

Act on semantic IR:

- **InferFrame**: Given a predicate or content word, retrieve its semantic frame (from a FrameNet- or ontology-based knowledge base) and attach frame roles (e.g., identify "seller" and "buyer" roles in a transaction frame)[en.wikipedia.org](https://en.wikipedia.org/wiki/Frame%5Fsemantics%5F%28linguistics%29#:~:text=Frame%20semantics%20is%20a%20theory,the%20money%2C%20the%20relation%20between)
- **ComposeLogicalForm**: Combine semantic roles and predicates into a logical form (e.g., predicate-argument representation or lambda-calculus formula) that encodes truth-conditional content
- **ApplyEntailmentRule**: Detect that one logical form entails another (e.g., "All cats are animals" entails "Some animals exist" given existence assumptions)

**Scope**: Local to UtteranceState. Read: semantic IR, semiotics. Write: semantic IR.

### P-operators (Pragmatic/Discourse)

Require discourse or world state context:

- **ResolveReference**: Identify the referents of pronouns or definite descriptions using discourse context (pragmatics)
- **DeriveImplicature**: Infer pragmatic implications using Gricean reasoning (e.g., "Some" implicates "not all"), adding pragmatic constraints to the state
- **UpdateCommonGround**: Update shared knowledge between speaker and listener based on utterance acceptance

**Scope**: Can inspect DiscourseState or WorldState. Read: semantic IR, pragmatics, DiscourseState. Write: pragmatics, semantic IR.

### K-operators (Knowledge Graph)

Query or update KG-derived views:

- **AddWorldKnowledge**: Incorporate background knowledge from domain KGs (e.g., if "Socrates is mortal" and "All men are mortal," derive that "Socrates is a man" leads to entailment)
- **CheckConstraint**: Verify that semantic IR satisfies type constraints or selectional restrictions from KG
- **UnifyEntities**: Link IR entities to KG concept nodes, merging referents that refer to the same real-world entity

**Scope**: Can read and update WorldState. Read: semantic IR, WorldState. Write: semantic IR, WorldState.

**Note on KG mutability**: By default, K-operators never mutate the underlying knowledge graph itself; they only update WorldState (the active in-memory view), e.g., adding inferred facts, entity alignments, or assumption tags. Persistent KG evolution is handled by separate offline processes. This ensures that reasoning is non-self-modifying with respect to the knowledge base, improving safety and testability.

### C-operators (Control/Meta)

Affect the search process itself:

- **PruneUnsatisfied**: Eliminate or penalize interpretations whose semantic propositions conflict with known facts or with earlier assumptions
- **ReweightState**: Adjust the score of a state based on new evidence or heuristics
- **PromoteNovelBranch**: Boost the priority of states that introduce previously unseen structures or concepts

**Scope**: Can inspect multiple states in search graph. Read: UtteranceState, search graph metadata. Write: state scores, search metadata.

## Operator Contracts

Every operator declares:

1. **Preconditions**: Which layers and states it reads. For example, `ApplySyntaxRule` requires a certain substructure to extend, whereas `DeriveImplicature` requires a completed logical form and context.

2. **Effects**: Which layers it writes. S-operators write to syntax; M-operators write to semantic IR; P-operators write to pragmatics and IR; K-operators write to IR and WorldState; C-operators write to search metadata.

3. **Scope**: Whether it is local to a single UtteranceState or can inspect/update DiscourseState/WorldState. This prevents "action at a distance" and keeps reasoning steps auditable.

This contract system ensures that operators are composable and that each reasoning step can be traced to specific layer updates.

## Operator Purity

Operators are logically pure with respect to UtteranceState / WorldState / DiscourseState: all changes happen via their declared write sets, and repeated calls with the same inputs yield the same outputs (modulo controlled randomness). Operators do not have side effects beyond their declared write layers (e.g., no global logging, usage counters, or external state mutations).

This purity property enables:

- **Deterministic replay**: Ability to reproduce exact inference chains for debugging and verification
- **Differential testing**: Compare operator outputs before/after model updates
- **Composability**: Operators can be safely combined and tested in isolation

## Determinism and Branching

Conceptually, operators are logical moves that transform states. Implementation-wise, an operator may:

- Return one successor deterministically (e.g., `ApplyEntailmentRule` when entailment is provable)
- Return a set of candidate successors with scores (e.g., `DisambiguatePOS` may return multiple POS candidates with confidence scores)
- Return an empty set if preconditions are not met

The search algorithm uses operator outputs to build the search graph. Operators that return multiple candidates create branching points in the search space, enabling exploration of alternative interpretations.

# Search-Based Reasoning

Sterling's reasoning is a **graph search** over UtteranceStates. Starting from an initial state (typically containing only the raw input words and maybe basic parse partials), the engine applies operators to generate successor states, building a tree/graph of possibilities.

## Goal States and Termination

Search continues until one or more **goal states** are reached. Goal states are defined per task type:

### Parsing/Understanding Task

A goal state satisfies:

- Semantic IR fully populated: all content words have semantic representations
- No unresolved roles: all obligatory frame roles are filled or explicitly marked as unknown
- No contradictions: semantic propositions are consistent with each other and with WorldState
- All obligatory frames filled: required semantic frames are instantiated

### Question Answering Task

A goal state satisfies:

- Semantic IR contains a proposition that answers the question
- Answer has provenance: the proposition is either directly asserted, derived from KB facts, or inferred via logical rules
- Answer is complete: all parts of the question are addressed

### Entailment/Contradiction Task

A goal state satisfies:

- Relation between two propositions is determined: entails, contradicts, or independent
- Valid proof path exists: the relation is justified by a chain of logical inferences or KG lookups
- Proof is minimal: the path uses the most direct available reasoning steps

### General Termination Conditions

In general, a goal predicate `Goal(s)` is defined per task. Search terminates when:

- A state satisfies `Goal(s)` with value above threshold, OR
- Compute budget is exhausted without finding such a state

The compute budget may be defined as maximum search depth, maximum number of states expanded, or maximum time elapsed.

## Hybrid Value Function

Sterling's search is guided by a hybrid value function that combines symbolic correctness checks with neural heuristics:

### V_symbolic(s)

The symbolic component has two roles:

- **Validator**: `Valid(s) ∈ {true, false}` that can immediately veto impossible states (contradictions, hard constraint violations)
- **Soft score**: `V_sym_soft(s) ∈ ℝ` that measures how complete and goal-aligned a valid state is (e.g., frame completeness, progress toward answering the question)

The validator checks:

- **Consistency**: No contradictions between propositions in semantic IR
- **Hard constraints**: Semantic content satisfies selectional restrictions and type constraints from KG

The soft score measures:

- **Frame completeness**: How many obligatory frame roles are filled
- **Goal alignment**: How much progress the state makes toward satisfying the goal predicate

### V_neural(s)

The neural component provides soft heuristics:

- Learned on top of latent encoding of state
- Predicts "will this branch lead to a good goal state?"
- Consumed by A\*/MCTS as heuristic function
- Trained on successful vs unsuccessful search paths

V_neural(s) returns a continuous score indicating estimated promise of the branch.

### Score Formula

Each state `s` maintains a score used for search prioritization. The scoring function is applied as follows:

If `Valid(s) == false`:

- Prune `s` immediately (do not enqueue in frontier)

Else:

```
Score(s) = V_sym_soft(s) + V_neural(s_latent) + NoveltyBonus(s) - DecayPenalty(s)
```

Where:

- `Valid(s)`: Hard validator that vetoes impossible states
- `V_sym_soft(s)`: Soft symbolic score measuring completeness and goal alignment
- `V_neural(s_latent)`: Learned heuristic over latent encoding
- `NoveltyBonus(s)`: Small positive bonus (e.g., +0.1) for introducing previously unseen structures
- `DecayPenalty(s)`: Penalty based on age and lack of reinforcement, computed as `decay_rate * age * (1 - visit_count / max_visits)`

### Search Algorithm Integration

The hybrid value function enables search algorithms like A\* or MCTS to:

1. **Symbolic veto**: `Valid(s)` immediately prunes impossible states (contradictions, constraint violations)
2. **Neural prioritization**: Among feasible states, `V_neural` prioritizes branches most likely to succeed
3. **Exploration balance**: `NoveltyBonus` encourages exploration; `DecayPenalty` prevents search explosion

This approach ensures that search focuses computational resources on promising, valid interpretations while maintaining the ability to explore novel alternatives.

## Search Strategy

This deliberative, search-driven approach resembles recent "Tree-of-Thoughts" methods in AI, and contrasts with single-pass LLM decoding. By systematically exploring multiple branches, Sterling can resolve ambiguity and avoid greedy mistakes. Prior work has shown that such search-augmented reasoning "yields substantial gains on complex, multi-step reasoning tasks" compared to linear generation[arxiv.org](https://arxiv.org/html/2510.09988v1#:~:text=self,89). In Sterling, each branch of the search graph corresponds to a coherent interpretation path (e.g. choice of lexical sense, grammar attachment, implicature) and is guided by heuristic scoring. Common search strategies (breadth-first, depth-first, A\*, Monte-Carlo Tree Search, etc.) can be used; indeed, framing reasoning as search into knowledge-space is a general paradigm that unifies many deliberative AI methods[arxiv.org](https://arxiv.org/html/2510.09988v1#:~:text=self,89).

Importantly, Sterling explicitly tracks both intermediate and final states. Unlike a black-box neural model, one can trace exactly which rules and knowledge produced each inference. This allows, for example, ranking multiple final parses by plausibility: one can score partial UtteranceStates with value estimates or rewards (see below) and choose the path with highest estimated value at goal. This is akin to "System 2" slow reasoning in human cognition[arxiv.org](https://arxiv.org/html/2510.09988v1#:~:text=computation%20at%20inference%20time%20to,the%20effectiveness%20and%20autonomy%20of), where an agent deliberates carefully rather than relying solely on a fast heuristic. In fact, current research notes that allocating extra computation at inference time (analogous to Sterling's search) parallels human slow thinking and can dramatically improve accuracy on hard problems[arxiv.org](https://arxiv.org/html/2510.09988v1#:~:text=computation%20at%20inference%20time%20to,the%20effectiveness%20and%20autonomy%20of)[arxiv.org](https://arxiv.org/html/2510.09988v1#:~:text=self,89).

# Learning Analogues: Weighting, Decay, and Novelty

Sterling incorporates mechanisms inspired by cognitive and reinforcement learning to manage the search. Each transition (operator application) is scored and weighted. A **value estimate** or reward model assesses how promising a given UtteranceState is (e.g. how well it aligns with world knowledge or goal requirements). This is analogous to the value function in reinforcement learning or the priority score in heuristic search. For instance, if an interpretation yields highly plausible semantic content, its state receives a higher score and is more likely to be expanded.

## Scoring Formula and State Metadata

Each state `s` maintains metadata used for search prioritization and learning:

- **score**: Current value estimate `Score(s)` used for search (priority in the frontier)
- **visit_count**: Number of times this state has been expanded
- **age**: Timestamp or step count for decay calculations
- **novelty_flag**: Boolean indicating if this state introduces previously unseen structures or concepts

### Scoring Function Reference

We reuse the hybrid scoring function defined in §Hybrid Value Function. The scoring combines:

- **Validator**: `Valid(s)` checks hard constraints and can immediately prune impossible states
- **Soft scores**: `V_sym_soft(s)` measures completeness and goal alignment; `V_neural(s_latent)` provides learned heuristics
- **Modifiers**: `NoveltyBonus(s)` and `DecayPenalty(s)` bias search toward exploration and efficiency

Novelty and decay are small modifiers that bias the search, not core correctness signals. The symbolic validator can veto impossible states; the neural component prioritizes among feasible ones.

## Timescale Clarification

Sterling's learning mechanisms operate at different timescales:

### Per-Query Learning

Within a single search episode:

- State scores are updated as search progresses
- Visit counts increment as states are expanded
- Age increases with search depth
- Novelty flags are set when new structures are encountered

This enables the search to adapt its exploration strategy within a single reasoning task.

### Cross-Episode Learning

Across multiple queries (future extension):

- Neural value function `V_neural` is trained offline on successful vs unsuccessful search paths
- Latent encoder is trained to compress IR + KG neighborhoods effectively
- Operator preferences may be adjusted based on long-term success rates

Cross-episode learning is a later extension and is not part of the initial problem framing.

## Learning Levels

Sterling's learning happens at two levels:

### Offline Learning

Training the neural components:

- **Latent encoder**: Learns to compress IR + KG neighborhood into fixed-dimensional latents
- **Neural value function**: Learns to predict branch promise from latent encodings
- Training data: Successful vs unsuccessful search paths from previous episodes

### Online Learning

Adjusting search behavior within a single episode:

- **State scores**: Updated based on new evidence or goal progress
- **Operator preferences**: May be adjusted based on which operators lead to promising states
- **Decay application**: Unused paths gradually lose priority

Online learning enables adaptive search without requiring retraining of neural components.

## Decay and Pruning

To prevent the search from exploding, Sterling uses **decay/pruning**: as in memory, unused or low-value paths gradually lose weight and are discarded. Psychologists describe how memory traces _fade_ over time if not rehearsed[simplypsychology.org](https://www.simplypsychology.org/forgetting.html#:~:text=Trace%20decay%20theory%20states%20that,term%20memory); Sterling similarly reduces the score of a path if it hasn't been reinforced by recent evidence or has low utility. This ensures that computation focuses on fruitful branches rather than accumulating redundant possibilities.

Decay is applied based on:

- **Age**: Older states receive larger decay penalties
- **Lack of reinforcement**: States that haven't been expanded recently decay faster
- **Low utility**: States with consistently low scores decay more quickly

## Novelty and Exploration

Conversely, Sterling rewards **novel** inferences to encourage exploration. In reinforcement learning and neurobiology, novel stimuli evoke dopamine signals that accelerate learning[hfsp.org](https://www.hfsp.org/hfsp-news/novelty-speeds-learning-dopamine#:~:text=The%20findings%20demonstrate%20that%20dopamine,algorithms%20and%20improve%20their%20efficiency). Sterling mimics this by giving a novelty bonus to newly generated states (e.g. interpretations that introduce a previously unseen frame or a rare construction). This bonus helps the search discover informative alternatives that might otherwise be overlooked, just as biological agents seek novel experiences. Empirical studies report that adding a novelty bonus "can speed up machine learning algorithms and improve their efficiency"[hfsp.org](https://www.hfsp.org/hfsp-news/novelty-speeds-learning-dopamine#:~:text=The%20findings%20demonstrate%20that%20dopamine,algorithms%20and%20improve%20their%20efficiency).

Novelty is detected when a state introduces:

- Previously unseen semantic frames
- Rare grammatical constructions
- Uncommon lexical sense combinations
- Novel entity-relation patterns

In summary, Sterling's weighting scheme – combining reward-based prioritization, decay of unused paths, and novelty incentives – mirrors cognitive learning: it balances exploitation (refining known-good interpretations) with exploration (seeking new insights) in a neuro-inspired way[simplypsychology.org](https://www.simplypsychology.org/forgetting.html#:~:text=Trace%20decay%20theory%20states%20that,term%20memory)[hfsp.org](https://www.hfsp.org/hfsp-news/novelty-speeds-learning-dopamine#:~:text=The%20findings%20demonstrate%20that%20dopamine,algorithms%20and%20improve%20their%20efficiency).

# Modularity and Domain Knowledge

A core design principle of Sterling is modularity. The reasoning engine itself is agnostic to any particular domain: all domain knowledge (lexicons, ontologies, frame inventories, etc.) is pluggable. For example, a general-purpose Sterling instance might use WordNet-style synonyms and FrameNet frames to understand everyday language. In a specialized medical Sterling, one would swap in a medical ontology (or UMLS knowledge graph) and a symptom lexicon. Internally, each knowledge source acts like a graph or database: FrameNet provides frames for words[en.wikipedia.org](https://en.wikipedia.org/wiki/Frame%5Fsemantics%5F%28linguistics%29#:~:text=Frame%20semantics%20is%20a%20theory,the%20money%2C%20the%20relation%20between), WordNet provides semantic relations between synsets (as noted in early knowledge-graph efforts[en.wikipedia.org](https://en.wikipedia.org/wiki/Knowledge%5Fgraph#:~:text=Some%20early%20knowledge%20graphs%20were,In%202005%2C%20Marc%20Wirk%20founded)), and custom KGs supply domain facts.

## Plugin Interface Architecture

Sterling core never hard-codes WordNet vs UMLS vs Wikidata. It operates against a small set of abstract interfaces, which concrete modules can implement for different domains. This plug-in architecture enables Sterling to act as a _generalist_ (using broad commonsense/KG resources) or an _expert_ (loaded with niche domain schemas), simply by changing its knowledge modules. The core search machinery and operators remain the same.

### Lexicon Module Interface

Given a lemma, part-of-speech, and language, the Lexicon Module returns:

- **sense_ids**: List of lexical sense identifiers (e.g., WordNet synset IDs, custom lexicon entries)
- **basic_features**: Subcategorization frames, selectional preferences, morphological features
- **semantic_relations**: Hypernyms, hyponyms, synonyms, antonyms for each sense

**Example implementations**:

- WordNet Lexicon: Maps to WordNet synsets and relations
- Medical Lexicon: Maps to UMLS concept unique identifiers (CUIs) and medical terminology
- Custom Domain Lexicon: Maps to domain-specific sense inventories

### Frame/Schema Module Interface

Given a sense ID or predicate, the Frame/Schema Module returns:

- **frame_id**: Identifier for the semantic frame (e.g., FrameNet frame ID, custom schema ID)
- **roles**: List of frame roles (agent, patient, instrument, location, etc.) with:
  - Role name and type
  - Optionality (obligatory vs optional)
  - Type restrictions (what kinds of entities can fill this role)
- **constraints**: Selectional restrictions, co-occurrence constraints, frame-to-frame relations

**Example implementations**:

- FrameNet Module: Retrieves FrameNet frames and role definitions
- Medical Schema Module: Retrieves UMLS semantic types and relations
- Custom Domain Schema: Retrieves domain-specific event schemas

### Knowledge Graph Module Interface

Given an entity/concept ID and relation type, the Knowledge Graph Module returns:

- **neighbors**: Related entities/concepts via the specified relation type
- **type_hierarchies**: Is-a, instance-of, and other taxonomic relations
- **constraints**: Type constraints, value restrictions, cardinality constraints

**Example implementations**:

- Wikidata Module: Queries Wikidata for entity relations and type hierarchies
- UMLS KG Module: Queries UMLS knowledge graph for medical concept relations
- Custom Domain KG: Queries domain-specific knowledge graphs

### Interface Guarantees

These interfaces ensure that:

1. **Domain agnosticism**: Sterling core never assumes WordNet vs UMLS vs custom resources
2. **Swappable modules**: Different domains can be supported by implementing the interfaces
3. **Consistent operator behavior**: Operators work the same way regardless of underlying knowledge sources
4. **Extensibility**: New knowledge sources can be integrated by implementing the interfaces

This architecture bridges the gap between "modular reasoning engine" and "we can ship a medical Sterling vs a legal Sterling vs a linguistics Sterling" – the same core engine with different knowledge modules.

Sterling treats a knowledge graph as a graph-structured database of concepts and relations[en.wikipedia.org](https://en.wikipedia.org/wiki/Knowledge%5Fgraph#:~:text=In%20knowledge%20representation%20and%20reasoning,2). As Wikipedia notes, a _knowledge graph_ represents "interlinked descriptions of entities" (objects, events, or abstract concepts) together with the _semantics_ of their relationships[en.wikipedia.org](https://en.wikipedia.org/wiki/Knowledge%5Fgraph#:~:text=In%20knowledge%20representation%20and%20reasoning,2). In Sterling, querying or inferring over such a graph is done via operators (e.g. **AddWorldKnowledge**, **ApplyEntailmentRule**).

# Example Use Cases

- **Ambiguity Resolution:** Sterling can disambiguate lexical or structural ambiguities by searching alternative parses and meanings. For instance, the word _“bat”_ is lexically ambiguous (animal vs. sports equipment)[medium.com](https://medium.com/@adecressac/blog-6-what-is-semantics-understanding-meaning-in-language-220a85b84e40#:~:text=Ambiguity%20arises%20when%20a%20word,they%20saw%20had%20a%20telescope). Sterling’s search would create two branches: one attaching the animal sense, one attaching the equipment sense. Pragmatic context (e.g. surrounding words about darkness or baseball) would then raise the score of the appropriate branch, causing the other to decay. Similarly, structural ambiguity (“I saw the man with the telescope” could mean either the observer used a telescope, or the man had a telescope[medium.com](https://medium.com/@adecressac/blog-6-what-is-semantics-understanding-meaning-in-language-220a85b84e40#:~:text=Ambiguity%20arises%20when%20a%20word,they%20saw%20had%20a%20telescope)) is handled by having two parse states and using semantics/pragmatics to choose between them.
- **Synonymy and Entailment Explanation:** By encoding word relations and logic, Sterling can recognize when two sentences express the same meaning or when one entails another. For example, it would identify "begin" and "start" as synonyms (from a lexicon) and recognize that "big" and "large" are near-synonyms (not hypernyms; a hypernym example would be "animal" as a hypernym of "cat"). In logical form, sentences sharing identical truth conditions are _paraphrases_[medium.com](https://medium.com/@adecressac/blog-6-what-is-semantics-understanding-meaning-in-language-220a85b84e40#:~:text=Sentences%20that%20share%20the%20same,form%20of%20a%20sentence%20changes): e.g. "The police arrested the burglar" and "The burglar was arrested by the police" have the same proposition[medium.com](https://medium.com/@adecressac/blog-6-what-is-semantics-understanding-meaning-in-language-220a85b84e40#:~:text=Sentences%20that%20share%20the%20same,form%20of%20a%20sentence%20changes). Sterling can generate one from the other by applying transformations that preserve truth (e.g. passive-active voice rules) and confirm that their truth conditions align. For entailment, if Sterling knows "There is at least one cat" and "All cats are mammals," it can derive that "Some mammals are cats" logically follows (the existence assumption is required for this entailment). Alternatively, Sterling can use a different entailment example that doesn't require existential import, such as deriving "All cats are animals" from "All cats are mammals" and "All mammals are animals" via transitivity.
- **Contradiction Detection:** Because Sterling maintains explicit truth-conditional content, it can spot incompatible states. For instance, the statements "All swans are white" and "There is a black swan" cannot both be true. Contradictions are localized to branches in the search graph: a contradiction in one branch does not immediately poison other branches. This paraconsistent/non-monotonic framing allows Sterling to explore multiple interpretations simultaneously, even if some contain contradictions. If one branch yields a proposition and a later branch yields its negation, Sterling's **DetectContradiction** operator will flag that state path as invalid (or assign it a very low score). The search will thus prefer consistent interpretations, pruning contradictory inferences while preserving other valid branches.
- **Paraphrase Generation:** Given an input, Sterling can search for alternative expressions with the same meaning. By systematically replacing words with synonyms (from its lexicon) and reordering or rephrasing via syntax rules, Sterling can generate paraphrases. Since it checks truth conditions, it can ensure the paraphrase is equivalent: e.g. swapping active/passive voice (as above) or alternative predicate synonyms. (Recall that paraphrases “express the same proposition, despite having different structures”[medium.com](https://medium.com/@adecressac/blog-6-what-is-semantics-understanding-meaning-in-language-220a85b84e40#:~:text=Sentences%20that%20share%20the%20same,form%20of%20a%20sentence%20changes).)
- **Context-Aware Disambiguation:** Sterling excels at cases where world knowledge or pragmatics decides meaning. For example, resolving pronouns or implied meanings. In dialogue, “It’s chilly” might implicate “Close the window” in context. Sterling’s **DeriveImplicature** operator would infer the likely intent. Or if someone says “I saw a bat,” and earlier context was about night baseball, Sterling would use that context to choose between the mammal or baseball sense of _bat_. In each case, multiple candidate states are explored, and the one consistent with broader context wins.

Each of these cases illustrates how Sterling's layered, state-based reasoning can handle linguistic subtleties that stump purely statistical models.

## Minimal v1 Task Set

For v1, we target three core tasks:

1. **Utterance-level semantic parsing to Sterling IR**: Given a sentence, populate semantic IR with events, entities, roles, and relations
2. **Single-sentence entailment/contradiction against a small KG**: Determine if one proposition entails, contradicts, or is independent of another, with proof trace
3. **Ambiguity resolution (word sense + structural) with explanation**: Disambiguate lexical and structural ambiguities, providing reasoning trace

These tasks provide a focused scope for initial implementation and evaluation. Future versions may expand to question answering, multi-utterance discourse, and planning tasks.

# Towards Safe, Compositional, Grounded Reasoning

By construction, Sterling's outputs are _meaning-grounded_ and _compositional_. Every inference is the result of an explicit rule application or knowledge lookup, so the system's reasoning can be audited. This stands in stark contrast to large end-to-end transformer models, which generate text by statistical pattern matching. Those models are prone to producing fluent but **false** statements – a phenomenon often called AI "hallucination"[en.wikipedia.org](https://en.wikipedia.org/wiki/Hallucination%5F%28artificial%5Fintelligence%29#:~:text=In%20the%20field%20of%20artificial,This%20term%20draws%20a). In fact, modern LLMs frequently "embed plausible-sounding random falsehoods" into their outputs[en.wikipedia.org](https://en.wikipedia.org/wiki/Hallucination%5F%28artificial%5Fintelligence%29#:~:text=For%20example%2C%20a%20chatbot%20,reliability%20of%20LLMs%20in%20high). Such hallucinations are a major challenge for deploying AI in high-stakes areas, because the model can confidently assert things with no factual basis.

## Non-Hallucinatory Internal Reasoning

Sterling's **internal reasoning** is non-hallucinatory with respect to its knowledge base: every proposition in the semantic IR is either input (directly asserted), explicitly assumed (for hypothetical reasoning), or logically derived (via operators from other propositions). If Sterling has no evidence for a proposition, that branch simply fails rather than inventing content.

This guarantee applies to the semantic IR layer – the internal representation of meaning. Sterling cannot create new facts at the IR level without justification. As Wikipedia notes, LLM hallucinations are "responses generated by AI that contain false or misleading information presented as fact"[en.wikipedia.org](https://en.wikipedia.org/wiki/Hallucination%5F%28artificial%5Fintelligence%29#:~:text=In%20the%20field%20of%20artificial,This%20term%20draws%20a), whereas Sterling's IR would never contain a factually unsupported proposition.

## Modality and Worlds

To support rich reasoning including counterfactuals and hypotheticals, Sterling distinguishes between different **worlds** or **contexts**:

### World Types

Each proposition in the semantic IR is tagged with:

- **world_id**: Identifier for the world context (e.g., `actual`, `hypothetical`, `scenario_1`, `counterfactual_1`)
- **modality**: How the proposition is asserted (`asserted`, `assumed`, `desired`, `feared`, `hypothetical`)

### Fact vs Assumption

Sterling only treats a proposition as a **fact in the actual world** if it is:

- Directly asserted and accepted (from input or KB lookup), OR
- Derived from other actual-world facts via logical rules

Hypothetical and counterfactual propositions live in separate worlds and are prevented from contaminating actual-world knowledge. For example:

- "Suppose all cats could fly" creates a hypothetical world where this assumption holds
- Reasoning within that world can derive consequences ("Cats would need wings")
- But these consequences are tagged with the hypothetical world_id and do not affect actual-world knowledge

### Counterfactual Support

Sterling explicitly supports counterfactual and hypothetical reasoning:

- **"Suppose X"**: Creates a hypothetical world with assumption X
- **"Imagine Y"**: Similar to suppose, may include multiple assumptions
- **"If we assumed Z"**: Conditional reasoning with explicit assumption tracking

These capabilities are essential for:

- Planning (exploring consequences of actions)
- Explanation (showing what would happen under different assumptions)
- Creative reasoning (exploring alternative scenarios)

The key is that all hypothetical content is clearly tagged and separated from actual-world facts, preserving the non-hallucination guarantee for actual-world reasoning.

## Surface Text Generation

Even if the IR is sound, a natural language generation (NLG) module that verbalizes IR could introduce errors:

- Omitting qualifiers (e.g., failing to mention a proposition is hypothetical)
- Picking wrong lexical items (e.g., choosing ambiguous words)
- Introducing ambiguity (e.g., generating sentences with multiple interpretations)

To address this, **surface text generation is constrained to be a faithful verbalization of the IR**. Errors in NLG are detectable as mismatches between IR and text, not mysterious hallucinations from a black-box decoder. The IR serves as the ground truth; NLG is a deterministic or near-deterministic mapping from IR to surface form.

This separation ensures that:

- Internal reasoning (IR level) is non-hallucinatory
- Surface generation errors are identifiable and correctable
- The system can verify that output text matches intended IR content

## Safety Guarantees Summary

Sterling's design – layered semantics, explicit logic, grounded search, and world/modality tracking – opens a path to _safe_ reasoning:

1. **Internal reasoning is auditable**: Every inference step is traceable to operator applications and knowledge lookups
2. **No unsupported facts**: IR cannot contain propositions without justification
3. **Hypothetical reasoning is explicit**: Counterfactuals and assumptions are clearly tagged and separated
4. **Surface generation is constrained**: NLG must faithfully represent IR content
5. **Conclusions are traceable**: Each final answer can be traced back to premises and reasoning steps

Taken together, these constraints mean that:

- The semantic IR is never populated with unsupported actual-world facts
- Hypothetical content is explicitly segregated by world_id and modality
- Surface text is a constrained rendering of IR, not a free-running generator

In other words, Sterling's "hallucination surface" is sharply bounded: errors are either missing inferences or mismatches between IR and wording, not spontaneous invented facts.

These guarantees make Sterling suitable for high-stakes applications where correctness and auditability are essential.

## External I/O Sketch (v1)

From the outside, using Sterling involves:

**Input**:

- `task_type`: One of `semantic_parse`, `entailment`, `ambiguity_resolution`
- `surface_text`: The input sentence(s) to analyze
- `optional_world_assumptions`: Propositions to assume for hypothetical reasoning (default: empty)
- `optional_KG_handles`: References to specific knowledge graph modules or slices to use (default: system default KG)

**Output**:

- `UtteranceState`: With populated `semantic_ir` and `latent_vector`
- `task_specific_result`:
  - For `semantic_parse`: The complete semantic IR graph
  - For `entailment`: Relation (`entails`, `contradicts`, `independent`) plus proof trace
  - For `ambiguity_resolution`: Disambiguated interpretation plus explanation trace
- `optional_proof_trace`: Sequence of operator applications and cited facts that led to the result

This I/O contract enables downstream products to integrate Sterling while maintaining clear boundaries between input assumptions, reasoning process, and output results.

In summary, the Sterling framework embodies a modular, symbolic approach to language understanding. It leverages structured knowledge (lexicons, frames, graphs) and a cognitive-style search process (with weighting, decay, and exploration) to build interpretations that are transparent and verifiable. This “slow,” System-2–like reasoning engine complements statistical NLP methods; in fact, as recent AI research shows, augmenting models with search-based reasoning often yields dramatically better results on complex tasks[arxiv.org](https://arxiv.org/html/2510.09988v1#:~:text=self,89). By integrating symbolic composition with flexible domain knowledge, Sterling offers a blueprint for future AI systems that truly understand meaning, rather than just predict word sequences.

**Sources:** Foundational concepts drawn from linguistic semantics (e.g. frame semantics[en.wikipedia.org](https://en.wikipedia.org/wiki/Frame%5Fsemantics%5F%28linguistics%29#:~:text=Frame%20semantics%20is%20a%20theory,the%20money%2C%20the%20relation%20between), paraphrastic truth conditions[medium.com](https://medium.com/@adecressac/blog-6-what-is-semantics-understanding-meaning-in-language-220a85b84e40#:~:text=Sentences%20that%20share%20the%20same,form%20of%20a%20sentence%20changes)), cognitive psychology (memory decay[simplypsychology.org](https://www.simplypsychology.org/forgetting.html#:~:text=Trace%20decay%20theory%20states%20that,term%20memory), novelty in learning[hfsp.org](https://www.hfsp.org/hfsp-news/novelty-speeds-learning-dopamine#:~:text=The%20findings%20demonstrate%20that%20dopamine,algorithms%20and%20improve%20their%20efficiency)), and AI research on search-based reasoning and LLM limitations[arxiv.org](https://arxiv.org/html/2510.09988v1#:~:text=Within%20this%20landscape%2C%20deliberative%20search,89)[en.wikipedia.org](https://en.wikipedia.org/wiki/Hallucination%5F%28artificial%5Fintelligence%29#:~:text=In%20the%20field%20of%20artificial,This%20term%20draws%20a)[en.wikipedia.org](https://en.wikipedia.org/wiki/Hallucination%5F%28artificial%5Fintelligence%29#:~:text=For%20example%2C%20a%20chatbot%20,reliability%20of%20LLMs%20in%20high). These underline Sterling’s design principles of compositional, meaning-grounded reasoning.

## Implementation Mapping

This section maps the theory concepts above to the actual implementation. For detailed field-level specifications, see the dedicated canonical specs.

### Core Classes

| Theory Concept | Implementation Class | Source File |
|---------------|---------------------|-------------|
| State Graph Search | `ImmutableSearchTree` | `core/reasoning/search.py` |
| Search Node (search tree) | `SearchNode` | `core/reasoning/search.py` |
| Search Configuration | `SearchConfig` | `core/reasoning/search.py` |
| State Node (semantic state) | `StateNode` | `core/state_model.py` |
| UtteranceState | `UtteranceState` | `core/state_model.py` |
| WorldState / KnowledgeContext | `WorldState` | `core/state_model.py` |
| Episode Graph | `StateGraph` | `core/reasoning/state_graph.py` |
| Episode Graph Node | `SearchNode` (state_graph) | `core/reasoning/state_graph.py` |
| Episode Graph Edge | `OperatorEdge` | `core/reasoning/state_graph.py` |
| Search Health | `SearchHealthAccumulator` | `core/search_health.py` |
| Transition Features | `FeatureSpec` (38-dim) | `core/reasoning/value_features.py` |
| Transition Scorer | `TransitionScorer` (MLP) | `core/reasoning/value_features.py` |
| Reasoning Loop | `SterlingReasoningLoop` | `core/reasoning/loop/main.py` |
| Hybrid Value Function | `HybridValueFunction` | `core/value/hybrid.py` |

### Search Strategies

| Strategy | Implementation |
|----------|---------------|
| Best-First (A*-style) | `SearchStrategy.BEST_FIRST` (default) |
| Beam Search | `SearchStrategy.BEAM` |
| Breadth-First | `SearchStrategy.BREADTH_FIRST` |
| Depth-First | `SearchStrategy.DEPTH_FIRST` |

### Scoring Formula (Implementation)

The theory's `Score(s) = V_sym_soft(s) + V_neural(s_latent) + NoveltyBonus(s) - DecayPenalty(s)` is implemented as:

```
score = value_weight * value_score - g_cost - heuristic_weight * h_cost
      + novelty_bonus + op_bonus - invariant_penalty
```

Where `value_score` comes from `HybridValueFunction` (8 component heads), `h_cost` from heuristic function, and `g_cost` is cumulative path cost. See [Value Function Components](value_function_components_v1.md) for details.

### Related Canonical Specs

| Spec | Covers |
|------|--------|
| [State Model Contract](state_model_contract_v1.md) | StateNode, UtteranceState, WorldState fields |
| [Value Function Components](value_function_components_v1.md) | HybridValueFunction, component heads, scoring |
| [Operator Registry Contract](operator_registry_contract_v1.md) | Operator taxonomy (S/M/P/K/C), registry |
| [World Adapter Protocol](world_adapter_protocol_v1.md) | Domain-agnostic world interface |
| [Core Constraints](core_constraints_v1.md) | INV-CORE-01 through INV-CORE-11 |
| [Neural Usage Contract](neural_usage_contract.md) | Neural advisory / symbolic authoritative boundary |

### Source File Index (Reasoning)

| File | Purpose |
|------|---------|
| `core/reasoning/search.py` | ImmutableSearchTree, SearchNode, SearchConfig, SearchResult |
| `core/reasoning/state_graph.py` | StateGraph, OperatorEdge, SearchNodeType, EdgeKind |
| `core/reasoning/value_features.py` | FeatureSpec (38-dim), TransitionScorer, make_transition_features |
| `core/reasoning/planner.py` | Explanation planning from rule results |
| `core/reasoning/loop/main.py` | SterlingReasoningLoop orchestrator |
| `core/reasoning/loop/search_strategies.py` | BFS, best-first, greedy, hybrid strategies |
| `core/reasoning/loop/types.py` | ReasoningTask, ReasoningResult, ReasoningPhase |
| `core/state_model.py` | StateNode, UtteranceState, WorldState, SyntaxLayer |
| `core/search_health.py` | SearchHealthAccumulator, TerminationReason |

---

Citations

[![](https://www.google.com/s2/favicons?domain=https://medium.com&sz=32)Blog 6: What is Semantics? Understanding Meaning in Language | by Antoine Decressac (#LinguisticallyYours) | Mediumhttps://medium.com/@adecressac/blog-6-what-is-semantics-understanding-meaning-in-language-220a85b84e40](https://medium.com/@adecressac/blog-6-what-is-semantics-understanding-meaning-in-language-220a85b84e40#:~:text=Semantics%20is%20a%20cornerstone%20of,In%20this)[![](https://www.google.com/s2/favicons?domain=https://en.wikipedia.org&sz=32)Frame semantics (linguistics) - Wikipediahttps://en.wikipedia.org/wiki/Frame_semantics\_(linguistics)](https://en.wikipedia.org/wiki/Frame%5Fsemantics%5F%28linguistics%29#:~:text=Frame%20semantics%20is%20a%20theory,the%20money%2C%20the%20relation%20between)[![](https://www.google.com/s2/favicons?domain=https://arxiv.org&sz=32)Unifying Tree Search Algorithm and Reward Design for LLM Reasoning: A Surveyhttps://arxiv.org/html/2510.09988v1](https://arxiv.org/html/2510.09988v1#:~:text=self,89)[![](https://www.google.com/s2/favicons?domain=https://arxiv.org&sz=32)Unifying Tree Search Algorithm and Reward Design for LLM Reasoning: A Surveyhttps://arxiv.org/html/2510.09988v1](https://arxiv.org/html/2510.09988v1#:~:text=computation%20at%20inference%20time%20to,the%20effectiveness%20and%20autonomy%20of)[![](https://www.google.com/s2/favicons?domain=https://www.simplypsychology.org&sz=32)Theories of Forgetting in Psychologyhttps://www.simplypsychology.org/forgetting.html](https://www.simplypsychology.org/forgetting.html#:~:text=Trace%20decay%20theory%20states%20that,term%20memory)[![](https://www.google.com/s2/favicons?domain=https://www.hfsp.org&sz=32)Novelty speeds up learning with dopamine | Human Frontier Science Programhttps://www.hfsp.org/hfsp-news/novelty-speeds-learning-dopamine](https://www.hfsp.org/hfsp-news/novelty-speeds-learning-dopamine#:~:text=The%20findings%20demonstrate%20that%20dopamine,algorithms%20and%20improve%20their%20efficiency)[![](https://www.google.com/s2/favicons?domain=https://en.wikipedia.org&sz=32)Knowledge graph - Wikipediahttps://en.wikipedia.org/wiki/Knowledge_graph](https://en.wikipedia.org/wiki/Knowledge%5Fgraph#:~:text=Some%20early%20knowledge%20graphs%20were,In%202005%2C%20Marc%20Wirk%20founded)[![](https://www.google.com/s2/favicons?domain=https://en.wikipedia.org&sz=32)Knowledge graph - Wikipediahttps://en.wikipedia.org/wiki/Knowledge_graph](https://en.wikipedia.org/wiki/Knowledge%5Fgraph#:~:text=In%20knowledge%20representation%20and%20reasoning,2)[![](https://www.google.com/s2/favicons?domain=https://medium.com&sz=32)Blog 6: What is Semantics? Understanding Meaning in Language | by Antoine Decressac (#LinguisticallyYours) | Mediumhttps://medium.com/@adecressac/blog-6-what-is-semantics-understanding-meaning-in-language-220a85b84e40](https://medium.com/@adecressac/blog-6-what-is-semantics-understanding-meaning-in-language-220a85b84e40#:~:text=Ambiguity%20arises%20when%20a%20word,they%20saw%20had%20a%20telescope)[![](https://www.google.com/s2/favicons?domain=https://medium.com&sz=32)Blog 6: What is Semantics? Understanding Meaning in Language | by Antoine Decressac (#LinguisticallyYours) | Mediumhttps://medium.com/@adecressac/blog-6-what-is-semantics-understanding-meaning-in-language-220a85b84e40](https://medium.com/@adecressac/blog-6-what-is-semantics-understanding-meaning-in-language-220a85b84e40#:~:text=Sentences%20that%20share%20the%20same,form%20of%20a%20sentence%20changes)[![](https://www.google.com/s2/favicons?domain=https://en.wikipedia.org&sz=32)Hallucination (artificial intelligence) - Wikipediahttps://en.wikipedia.org/wiki/Hallucination\_(artificial_intelligence)](https://en.wikipedia.org/wiki/Hallucination%5F%28artificial%5Fintelligence%29#:~:text=In%20the%20field%20of%20artificial,This%20term%20draws%20a)[![](https://www.google.com/s2/favicons?domain=https://en.wikipedia.org&sz=32)Hallucination (artificial intelligence) - Wikipediahttps://en.wikipedia.org/wiki/Hallucination\_(artificial_intelligence)](https://en.wikipedia.org/wiki/Hallucination%5F%28artificial%5Fintelligence%29#:~:text=For%20example%2C%20a%20chatbot%20,reliability%20of%20LLMs%20in%20high)[![](https://www.google.com/s2/favicons?domain=https://arxiv.org&sz=32)Unifying Tree Search Algorithm and Reward Design for LLM Reasoning: A Surveyhttps://arxiv.org/html/2510.09988v1](https://arxiv.org/html/2510.09988v1#:~:text=Within%20this%20landscape%2C%20deliberative%20search,89)

All Sources

[![](https://www.google.com/s2/favicons?domain=https://medium.com&sz=32)medium](https://medium.com/@adecressac/blog-6-what-is-semantics-understanding-meaning-in-language-220a85b84e40#:~:text=Semantics%20is%20a%20cornerstone%20of,In%20this)[![](https://www.google.com/s2/favicons?domain=https://en.wikipedia.org&sz=32)en.wikipedia](https://en.wikipedia.org/wiki/Frame%5Fsemantics%5F%28linguistics%29#:~:text=Frame%20semantics%20is%20a%20theory,the%20money%2C%20the%20relation%20between)[![](https://www.google.com/s2/favicons?domain=https://arxiv.org&sz=32)arxiv](https://arxiv.org/html/2510.09988v1#:~:text=self,89)[![](https://www.google.com/s2/favicons?domain=https://www.simplypsychology.org&sz=32)simplypsychology](https://www.simplypsychology.org/forgetting.html#:~:text=Trace%20decay%20theory%20states%20that,term%20memory)[![](https://www.google.com/s2/favicons?domain=https://www.hfsp.org&sz=32)hfsp](https://www.hfsp.org/hfsp-news/novelty-speeds-learning-dopamine#:~:text=The%20findings%20demonstrate%20that%20dopamine,algorithms%20and%20improve%20their%20efficiency)
