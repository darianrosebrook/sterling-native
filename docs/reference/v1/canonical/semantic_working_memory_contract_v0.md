> **NOTE: Quarantined v1 canonical contract. Non-authoritative for Sterling Native. See [promotion criteria](README.md).**

# Semantic Working Memory Contract v0

**This is a canonical definition. Do not edit without a version bump or CANONICAL-CHANGE PR label.**

**Schema (doc-only)**: `sterling.semantic_working_memory_contract.v0`
**Version**: `0.0.3`
**Owner**: `core/linguistics`, `core/memory`
**Status**: Active
**Date**: 2026-01

---

## 1. Purpose

Sterling operates in semantic space. Language is I/O only. This contract defines how Sterling's internal meaning-state grows and decays without degenerating into a context window abstraction, while preserving auditability and deterministic replay.

This contract specifies:

- A four-status model for graph content (Committed / Shadow / Weak / Frontier).
- Projection boundaries (PlannerView / RetrievalView) as hard API rules.
- Lifecycle operators for eviction, rehydration, verification, and promotion.
- A cryptographic chain requirement (State -> Episode -> State) that makes "where State A came from" non-negotiable.
- A context axis requirement that enables multiple concurrent truth contexts without forcing premature collapse.
- Myelin sheath semantics: certified corridors that define new State A baselines.

## 2. Core Invariants

### INV-SWM-01: Meaning-State Authority

The Committed graph is Sterling's authoritative meaning-state. It is not required to mirror the raw string.

### INV-SWM-02: Provenance Substrate

Raw text and derived observations are retained as immutable provenance artifacts (content-addressed). Every committed node/edge MUST be transitively traceable to evidence atoms via PromotionRecords and witnesses.

### INV-SWM-03: Planning Boundary (MUST, Fail-Closed)

Only Committed nodes and edges participate in legality checks and path algebra. Shadow, Weak, and Frontier content MUST NOT change successor legality. They MAY affect ranking and prioritization only where explicitly permitted by a named policy.

**Enforcement**: Any search or planning algorithm MUST accept only a PlannerView projection. Attempts to feed non-Committed structures into legality computation MUST fail closed. This is not advisory; it is a hard API rule. Violation in certifying mode is a gate failure.

### INV-SWM-04: Determinism Under Identical Inputs

Given identical inputs (same provenance artifacts, same operator implementations, same policy configuration hashes, same context_ref), MeaningStateDigest computation and EpisodeTrace generation MUST produce identical results.

### INV-SWM-05: No Silent Semantic Drift

Changes to the Committed meaning-state MUST occur only through operators that emit:
1. A canonical delta patch (conforming to `sterling.linguistic_delta_patch.v0` or successor).
2. A witness bundle that binds the change to provenance artifacts and precondition checks.

No other mechanism is permitted.

### INV-SWM-06: Bounded Working Set

Materialized in-memory subgraphs MUST be bounded by explicit budgets. Exceeding budgets triggers eviction and/or compaction operations. These operations MUST NOT alter Committed semantics except via explicit, witnessed transformations (i.e., VerifyAndPromote).

### INV-SWM-07: Chain of Custody

Every MeaningStateDigest MUST include a `parent_state_id` linking to the prior state, except for genesis states (which use a sentinel value). The chain of `(state -> episode -> state)` MUST be traversable back to a genesis state or certified checkpoint for any state that participates in certifying or promotion runs.

## 3. Data Model: Four Statuses

### 3.1 Committed (Authoritative)

**Definition**: Nodes and edges that participate in reasoning, legality checks, and path algebra.

**Requirements**:
- MUST have a PromotionRecord (or genesis record) linking to evidence and witnesses.
- MUST be included in MeaningStateDigest computation.
- MUST be replayable deterministically.
- MUST satisfy all structural invariants from the Linguistic IR Contract v0 (I1-I9).

### 3.2 Shadow (Assumed but Unverified)

**Definition**: Structures that have semantic shape (typed nodes/edges matching Committed schemas) but are explicitly non-authoritative until verified.

**Requirements**:
- MUST be excluded from PlannerView.
- MUST carry `VerificationRequirements`: a record of what checks and evidence are needed for promotion.
- MAY be presented in explanations as "assumed" with explicit uncertainty marking.
- MUST NOT be referenced by Committed nodes/edges. A Committed node that references a Shadow node is a validation error (fail-closed).

**Use case**: Rehydrated context that was previously evicted. Comes back as Shadow, not as truth.

### 3.3 Weak (Ephemeral / Convenience)

**Definition**: Soft relationships and caches intended to improve retrieval and heuristics, not semantics.

**Requirements**:
- MUST be excluded from PlannerView.
- MUST be safe to drop without semantic loss. Dropping all Weak content MUST NOT change MeaningStateDigest.
- MUST carry decay metadata: `retention_score`, `last_touched_step`, `policy_tag`.
- MUST NOT be referenced by Committed or Shadow nodes/edges.

**Use case**: Embedding-derived similarity edges, lexical co-occurrence hints, candidate retrieval caches.

### 3.4 Frontier (Hypotheses / Unlit Hallways)

**Definition**: Candidate interpretations, suggested links, and proposed groundings that have not been committed. This status is defined by the Linguistic IR Contract v0 and includes `Hypothesis`, `Candidate`, `PromotionRecord`, and `GeneratorRef`.

**Requirements**:
- MUST NOT be referenced by Committed nodes/edges except via PromotionRecords (which record the promotion, not a live dependency).
- MUST include generator provenance and deterministic candidate ordering (`score DESC, candidate_id ASC`).
- MAY be pruned by policy (top-k per hypothesis kind per attachment point) without changing Committed semantics or MeaningStateDigest.

**Relationship to Weak**: Frontier is structurally richer than Weak (typed hypotheses with candidates and generators). Weak edges have no hypothesis lifecycle. Do not conflate them.

## 4. Projection Boundaries

### 4.1 PlannerView (Authoritative Reasoning View)

PlannerView is a projection of the internal IR containing:

- All Committed nodes and edges required for legality and successor generation.
- Optionally: minimal provenance handles (IDs only) for explanation linking. These MUST NOT be used for reasoning.

PlannerView MUST exclude:

- Shadow nodes and edges.
- Weak nodes and edges.
- Frontier hypotheses and candidates.
- Decay metadata, retention scores, and materialization flags.

**INV-PV-01**: Any search or planning algorithm MUST accept PlannerView only. Passing a non-projected IR to a planning algorithm is a contract violation. In certifying mode, this MUST fail closed.

**INV-PV-02**: PlannerView serialization MUST NOT include any node/edge whose status is not Committed. A PlannerView that contains a non-Committed element is structurally invalid.

### 4.2 RetrievalView (Rehydration and Explanation View)

RetrievalView MAY include:

- Committed + Shadow + Weak + Frontier content.
- Provenance artifacts and index handles.
- Embedding and lexical retrieval structures.

**INV-RV-01**: RetrievalView MAY propose candidates and materialize context, but MUST NOT affect Committed semantics except through explicit operators that produce patches and witnesses (i.e., VerifyAndPromote).

## 5. State, Episode, and Cryptographic Chain of Custody

### 5.1 MeaningStateDigest v0

**Schema**: `sterling.meaning_state_digest.v0`

A MeaningStateDigest is a content-addressed summary of the Committed meaning-state plus the metadata needed to detect drift.

**Fields**:

| Field | Type | Description |
|-------|------|-------------|
| `schema_id` | str | `"sterling.meaning_state_digest.v0"` |
| `schema_version` | str | `"0.0.1"` |
| `state_id` | str | Content-addressed hash (see S-1) |
| `committed_graph_digest` | str | SHA256 of canonical Committed subgraph |
| `policy_digest` | str | Hash of retention/projection policy configuration |
| `operator_registry_digest` | str | Hash of `operator_id -> operator_impl_digest` mappings |
| `provenance_root_digest` | str | Hash root of provenance artifacts reachable via PromotionRecords |
| `context_ref` | ContextRef | Domain and worldline context (see 5.4) |
| `parent_state_id` | str | state_id of the prior state. Genesis sentinel: `"genesis:0"` |

**Requirement S-1 (State Identity)**:

```
state_id = sha256(
    committed_graph_digest
    || policy_digest
    || operator_registry_digest
    || provenance_root_digest
    || context_ref.canonical_digest()
    || parent_state_id
)[:32]
```

Format: `"msd_{hex}"` (32 hex chars).

**Requirement S-2 (Canonical Serialization)**: `committed_graph_digest` MUST be computed using `json.dumps(obj, sort_keys=True, separators=(",", ":"))` over the Committed partition only, consistent with the Linguistic IR Contract v0 canonicalization rules.

### 5.2 EpisodeTrace v0

**Schema**: `sterling.episode_trace.v0`

An EpisodeTrace represents one transition step from State A to State B.

**Fields**:

| Field | Type | Description |
|-------|------|-------------|
| `schema_id` | str | `"sterling.episode_trace.v0"` |
| `schema_version` | str | `"0.0.1"` |
| `episode_id` | str | Content-addressed hash (see E-1) |
| `state_in` | str | MeaningStateDigest.state_id of entry state |
| `state_out` | str | MeaningStateDigest.state_id of exit state |
| `patch_digest` | str | SHA256 of canonical delta patch |
| `witness_digest` | str | SHA256 of witness bundle |
| `evidence_root_digest` | str | Hash root of all evidence artifacts referenced |
| `operator_sequence` | tuple | Ordered list of `OperatorStep` (see below) |
| `context_ref` | ContextRef | Domain and worldline context (see 5.4) |
| `myelin_delta` | str or None | sheath_id if a sheath was created/promoted in this episode |

**OperatorStep**:

| Field | Type | Description |
|-------|------|-------------|
| `operator_id` | str | Operator name |
| `operator_impl_digest` | str | Code-binding digest |
| `invocation_digest` | str | Hash of operator inputs for this step |

**Requirement E-1 (Episode Identity)**:

```
episode_id = sha256(
    state_in
    || patch_digest
    || witness_digest
    || evidence_root_digest
    || operator_sequence_digest
    || context_ref.canonical_digest()
)[:32]
```

Format: `"ept_{hex}"` (32 hex chars).

`operator_sequence_digest` is the SHA256 of the canonically-serialized operator_sequence tuple.

**Requirement E-2 (Chain Integrity)**: For normal episodes, the MeaningStateDigest corresponding to `state_out` MUST have `parent_state_id == state_in`. The only exception is a "state reset" episode type, which is gated, witnessed, and MUST record the reason for the chain break.

**Requirement E-3 (Anti-Forgery)**: Any change to any `operator_impl_digest` in the operator_sequence MUST change the `episode_id`. This ensures that replaying an episode with different operator code produces a detectably different trace.

### 5.3 Proof of State A

To treat a state as authoritative in certifying or promotion mode, Sterling MUST be able to provide:

1. The state's `committed_graph_digest`.
2. The chain of `episode_id` values back to a genesis state or a certified checkpoint.
3. For each episode in the chain: `patch_digest` + `witness_digest` + `evidence_root_digest`.

This is the cryptographic chain: State A is not an assertion; it is the head of a verifiable chain.

### 5.4 Context Axis (ContextRef)

A ContextRef identifies the domain, worldline, and definitional context under which a state or episode exists. This enables reasoning about "two truths" (e.g., different domains, different revision histories) without forcing premature collapse.

**Fields**:

| Field | Type | Description |
|-------|------|-------------|
| `domain_id` | str | World/domain identifier. Uses `world_name` vocabulary (lowercase: `"wordnet"`, `"pn"`, `"escape"`, etc.) |
| `worldline_id` | str | Identifies a specific reasoning lineage or branch. Default: `"main"` |
| `revision_id` | str | Monotonic revision counter or hash within a worldline |
| `evidence_root_digest` | str | Hash of the evidence corpus available to this context |
| `definitions_digest` | str | Hash of the definitional substrate (schemas, ontology versions, grounding KB snapshots) |

**Requirement C-1 (Canonical Digest)**:

```
context_ref.canonical_digest() = sha256(
    domain_id || worldline_id || revision_id
    || evidence_root_digest || definitions_digest
)[:16]
```

Format: `"ctx_{hex}"` (16 hex chars).

**Requirement C-2 (Context Sensitivity)**: Two states with different `context_ref.canonical_digest()` values MUST produce different `state_id` values, even if their `committed_graph_digest` is identical. This prevents cross-context conflation.

## 6. Myelin Sheath: Certified Corridors

### 6.1 Definition

A Myelin Sheath is a canonicalized, fully-committed node/edge path and operator corridor through an episode. It is not a cache. It is a certified corridor: applying it produces the same semantic transition under the same preconditions, and its use is auditable and replayable.

### 6.2 Semantics

- A fully committed myelin sheath defines a canonical interpretation of an episode corridor and can be treated as the "State A baseline" for subsequent reasoning in that domain context.
- Sheath content is immutable once promoted. Any change to the corridor creates a new sheath artifact with a new `sheath_id`, and therefore a new meaning-state transition. Sheaths are append-only.

### 6.3 Constraints

**M-1 (Immutability)**: Once promoted, a sheath MUST NOT be edited. Modifying any corridor element yields a different `sheath_id`. "Editing" a sheath means creating a new sheath.

**M-2 (Authority Boundary)**: Every node and edge in the sheath corridor MUST have Committed status. Shadow, Weak, and Frontier elements are forbidden in corridor definitions.

**M-3 (Provenance)**: Every sheath MUST include an `evidence_root_digest` and a witness chain demonstrating why each corridor step is legal.

**M-4 (Registry Binding)**: A sheath records the `operator_registry_digest` at promotion time. Application of the sheath MUST verify this digest matches the current registry; otherwise fail closed.

### 6.4 MyelinSheath Schema v0

**Schema**: `sterling.myelin_sheath.v0`

**Fields**:

| Field | Type | Description |
|-------|------|-------------|
| `schema_id` | str | `"sterling.myelin_sheath.v0"` |
| `schema_version` | str | `"0.0.1"` |
| `sheath_id` | str | Content-addressed hash (see MS-1) |
| `derived_from_episode_id` | str | EpisodeTrace that produced this sheath |
| `entry_state_id` | str | MeaningStateDigest.state_id at corridor entry |
| `exit_state_id` | str | MeaningStateDigest.state_id at corridor exit |
| `canonical_corridor` | CanonicalCorridor | Ordered node/edge/operator steps (see below) |
| `preconditions_digest` | str | Hash of the sheath's precondition set |
| `evidence_root_digest` | str | Hash of referenced evidence |
| `operator_registry_digest` | str | Hash of operator registry at promotion time |
| `context_ref` | ContextRef? | Domain and worldline context (Optional, default None) |
| `scope` | str | Applicability scope (uses `scope_key` format: `"world={world}/task={task}/"`) |

**CanonicalCorridor**:

| Field | Type | Description |
|-------|------|-------------|
| `node_ids` | tuple[str, ...] | Ordered committed node IDs |
| `edge_ids` | tuple[str, ...] | Ordered committed edge IDs or stable edge descriptors |
| `operator_steps` | tuple[CorridorStep, ...] | Ordered operator applications with digests |

**CorridorStep** (distinct from EpisodeTrace's OperatorStep — includes affected elements):

| Field | Type | Description |
|-------|------|-------------|
| `operator_id` | str | Operator name |
| `operator_impl_digest` | str | Code-binding digest |
| `affected_node_ids` | tuple[str, ...] | Node IDs affected by this step (default ()) |
| `affected_edge_ids` | tuple[str, ...] | Edge IDs affected by this step (default ()) |

**Requirement MS-1 (Sheath Identity)**:

```
sheath_id = sha256(
    derived_from_episode_id
    || entry_state_id
    || canonical_corridor_digest
    || preconditions_digest
    || evidence_root_digest
    || operator_registry_digest
)[:32]
```

Format: `"mys_{hex}"` (32 hex chars).

### 6.5 Operational Use

The planner MAY treat sheath corridors as "certified macros" for search acceleration, but ONLY if all of the following hold:

1. The current state satisfies the sheath's preconditions.
2. The `operator_registry_digest` matches the current registry.
3. The `evidence_root_digest` and witnesses are resolvable.
4. The `context_ref` is compatible with the current reasoning context.

If any condition is not met, the sheath MUST NOT be applied. This is fail-closed, not best-effort.

### 6.6 Interpreting "New State A"

When a myelin sheath is promoted, the resulting state A' is the state whose digest is the head after applying the episode that promoted the sheath, plus a certified reference to the sheath artifact. The sheath does not replace the state; it is a certified corridor that becomes part of the state's provenance root and can be reused as an invariant macro going forward. The chain requirement (INV-SWM-07) ensures you can always show how State A' was reached.

## 7. Lifecycle Operators

All lifecycle changes MUST be expressed as operators emitting patches and witnesses, and MUST be recorded in EpisodeTraces.

### 7.1 EvictWS (Working Set Eviction)

**Purpose**: Reduce materialized in-memory working set size without changing Committed semantics.

**Inputs**: Budget (node/edge caps), retention policy digest.

**Outputs**: Patch that modifies materialization flags and WS residency. May prune Weak and Frontier caches. Witness records: budget state, eviction decisions, retention scores used.

**INV-EWS-01**: EvictWS MUST NOT change MeaningStateDigest. It affects only which content is in-memory, not what is Committed.

**INV-EWS-02**: EvictWS MUST NOT delete Committed nodes from the Persistent Substrate. It only dematerializes them from the Working Set.

### 7.2 RehydrateShadow

**Purpose**: Reintroduce previously evicted or non-materialized context as Shadow structures for potential verification and promotion.

**Inputs**: Seeds (node IDs, mention spans, query handles), budget (k-hop cap, max nodes), retrieval policy digest.

**Outputs**: Patch that materializes selected structures as Shadow (semantic shape) and Weak (convenience edges), plus Frontier hypotheses as needed. Witness records: retrieval seeds, budgets, index digests, returned set digests.

**INV-RS-01**: RehydrateShadow MUST NOT add Committed nodes or edges. All rehydrated content enters as Shadow or Weak.

**INV-RS-02**: Rehydration MUST be bounded by declared budgets. Unbounded "load everything" is a contract violation.

### 7.3 VerifyAndPromote

**Purpose**: Promote Shadow or Frontier items into Committed, producing a new meaning-state.

**Inputs**: Selected hypothesis/candidate IDs, required evidence packets, verification checks.

**Outputs**: Patch that adds/updates Committed nodes/edges, retires Shadow/Frontier items, emits PromotionRecords. Optionally emits a MyelinSheath if the corridor qualifies. Witness includes: precondition checks, evidence roots, operator_impl_digests.

**INV-VP-01**: VerifyAndPromote is the ONLY allowed path from Shadow or Frontier to Committed. No other operator may change content from non-Committed to Committed.

**INV-VP-02**: VerifyAndPromote MUST change MeaningStateDigest (it is adding to Committed). The new digest MUST include the promoted content.

**INV-VP-03**: Every promotion MUST produce a PromotionRecord linking to evidence, witnesses, and the hypothesis/shadow nodes that were promoted.

### 7.4 CompactToSummary (Reserved for v0; Recommended for v1)

**Purpose**: Reduce graph size by collapsing an old subgraph into a summary node while retaining replay links.

**Constraint**: MUST preserve a reversible pointer to the original artifacts via evidence roots and episode chain references. The compaction artifact MUST be replay-verifiable: given the summary node and the original artifacts, you can reconstruct the original subgraph.

This operator is defined but not required for v0 implementation.

### 7.5 ApplySheath

**Purpose**: Apply a certified MyelinSheath corridor to accelerate reasoning (replay without re-deriving).

**Inputs**: MyelinSheath artifact, current state, operator registry.

**Outputs**: Patch applying the corridor's transition. Witness records: sheath_id, precondition verification, registry digest match.

**INV-AS-01**: ApplySheath MUST verify `operator_registry_digest` matches current registry. Mismatch fails closed.

**INV-AS-02**: ApplySheath MUST verify all preconditions are satisfied before application.

Source: `core/linguistics/operators/v0/apply_sheath.py`

### 7.6 PromoteToSheath

**Purpose**: Promote a verified corridor to a MyelinSheath (certified acceleration artifact).

**Inputs**: Corridor specification (node/edge/operator sequence), evidence, preconditions.

**Outputs**: New MyelinSheath artifact with computed `sheath_id`. Witness records: corridor derivation, evidence roots.

Source: `core/linguistics/operators/v0/promote_to_sheath.py`

### 7.7 IR Construction Operators

The following operators construct Committed IR elements from text analysis:

| Operator | Class | Purpose | Source |
|----------|-------|---------|--------|
| `MakeMention` | `MakeMention` | Create mention node from text span | `operators/v0/make_mention.py` |
| `MakeEntityRef` | `MakeEntityRef` | Create entity reference from mention IDs | `operators/v0/make_entity_ref.py` |
| `MakePredicate` | `MakePredicate` | Create predicate node from lemmatized text | `operators/v0/make_predicate.py` |
| `MakeProposition` | `MakeProposition` | Create proposition (predicate + roles) | `operators/v0/make_proposition.py` |
| `AttachModality` | `AttachModality` | Attach modality annotations to propositions | `operators/v0/attach_modality.py` |
| `AttachNegation` | `AttachNegation` | Attach negation markers to predicates/propositions | `operators/v0/attach_negation.py` |

### 7.8 Frontier Lifecycle Operators

These operators manage the hypothesis lifecycle (propose → promote):

| Operator | Class | Purpose | Source |
|----------|-------|---------|--------|
| `ProposeCoref` | `ProposeCoref` | Propose coreference hypothesis (Frontier) | `operators/v0/propose_coref.py` |
| `PromoteCorefMerge` | `PromoteCorefMerge` | Promote coreference merge to Committed | `operators/v0/promote_coref_merge.py` |
| `ProposeDiscourseLink` | `ProposeDiscourseLink` | Propose discourse link hypothesis (Frontier) | `operators/v0/propose_discourse_link.py` |
| `PromoteDiscourseLink` | `PromoteDiscourseLink` | Promote discourse link to Committed | `operators/v0/promote_discourse_link.py` |

All operators share the signature: `apply(ir: LinguisticIR, ctx: OperatorContext, inputs: OperatorInputs) -> OperatorResult`.

## 8. "Use It or Lose It" Reinforcement (Permitted Scope)

Reinforcement scoring MAY influence:

- WS residency (what stays loaded in the Working Set).
- Retrieval ordering and candidate ranking in Frontier.
- Weak edge retention and pruning thresholds.

Reinforcement scoring MUST NOT:

- Create Committed nodes or edges.
- Change legality or successor generation.
- Alter MeaningStateDigest except via VerifyAndPromote.

A practical retention score uses:

```
retention_score(x) =
    a * recency(x)
  + b * frequency(x)
  + c * goal_distance(x)
  + d * certification_level(x)
  - e * neighborhood_cost(x)
```

Thresholds:
- Below theta_1: evict from WS (keep in Persistent Substrate).
- Below theta_2: compress to summary witness node (CompactToSummary, when available).
- Below theta_3: prune (only if Weak or Frontier; Shadow becomes inert and excluded from planning but is not deleted).

Coefficients and thresholds are policy configuration, included in `policy_digest`.

## 9. Acceptance Tests

### A. Projection Safety

- **T-SWM-01**: PlannerView contains only Committed nodes and edges. No Shadow, Weak, or Frontier content present.
- **T-SWM-02**: Attempt to include Shadow/Weak/Frontier in legality check fails closed (raises, does not silently permit).
- **T-SWM-03**: A Committed node referencing a Shadow node fails validation (leak check).
- **T-SWM-04**: RetrievalView includes all four statuses without error.

### B. Chain Integrity

- **T-SWM-05**: For any EpisodeTrace, `state_out.parent_state_id == state_in.state_id`.
- **T-SWM-06**: Changing any `operator_impl_digest` in an EpisodeTrace changes the `episode_id`.
- **T-SWM-07**: Genesis state uses sentinel `parent_state_id = "genesis:0"`.

### C. Deterministic Replay

- **T-SWM-08**: Replaying an EpisodeTrace from `state_in` yields identical `state_out` digest.
- **T-SWM-09**: Replaying a chain of episodes yields identical head state digest.
- **T-SWM-10**: Same context_ref with same inputs yields same state_id.

### D. Myelin Sheath Immutability and Authority

- **T-SWM-11**: A promoted sheath's `sheath_id` is stable across runs with identical inputs.
- **T-SWM-12**: Applying a sheath under satisfied preconditions yields the same patch and state_out.
- **T-SWM-13**: Modifying any corridor element yields a different `sheath_id`.
- **T-SWM-14**: Wrong `operator_registry_digest` makes ApplySheath fail closed.
- **T-SWM-15**: Missing evidence makes ApplySheath fail closed.

### E. Bounded WS Without Reread

- **T-SWM-16**: With strict WS budgets, tasks requiring older context succeed via RehydrateShadow + VerifyAndPromote without loading entire history.
- **T-SWM-17**: Rehydration is bounded by declared budgets (k-hop, node cap).

### F. Status Semantics

- **T-SWM-18**: EvictWS does not change MeaningStateDigest.
- **T-SWM-19**: RehydrateShadow does not produce Committed deltas.
- **T-SWM-20**: VerifyAndPromote changes MeaningStateDigest; other lifecycle ops do not.
- **T-SWM-21**: Dropping all Weak content does not change MeaningStateDigest.
- **T-SWM-22**: Frontier pruning (top-k per attachment) does not change MeaningStateDigest.

### G. Context Axis

- **T-SWM-23**: Two states with different `context_ref` but identical committed graphs produce different `state_id` values.
- **T-SWM-24**: ContextRef canonical digest is deterministic across runs.

## 10. Implementation Notes (v0 Decisions)

- Default Committed core remains small. Shadow and Frontier can be large.
- Provenance artifacts are content-addressed and immutable. "Relocation" creates a new artifact referencing the old one plus a mapping witness.
- Weak edges are explicitly non-authoritative and excluded from PlannerView.
- EpisodeTrace is append-only and forms the audit spine.
- Myelin sheaths are append-only certified corridors.
- v0 does not implement CompactToSummary; subgraphs are only evicted from WS, not compacted.
- v0 retention scoring uses recency + frequency + certification_level only. Goal-distance and neighborhood-cost scoring are deferred.
- Existing Linguistic IR v0 types (Committed partition, Frontier partition) map directly into the Committed and Frontier statuses defined here. Shadow and Weak are new statuses to be added.
- `context_ref.worldline_id` defaults to `"main"` for single-lineage reasoning. Multi-worldline support is defined but not required for v0.

## 11. Relationship to Other Contracts

- **Linguistic IR Contract v0**: Defines the type system for Committed and Frontier partitions. This contract adds Shadow and Weak statuses plus lifecycle and projection rules on top of that type system.
- **Text I/O Contract v1**: Text is surface; IR is authority (INV-TIO-01 through INV-TIO-05). This contract governs the IR that Text I/O produces.
- **Core Constraints v1**: Meaning in IR+KG, not text (constraint 2). This contract formalizes how that meaning grows and decays.
- **Neural Usage Contract**: Embeddings and neural scores are advisory. This contract's Weak status and reinforcement scope rules enforce that boundary.

## 12. Projection System

Source: `core/memory/projection.py`

The projection system assembles task-specific views of memory for reasoning, selecting relevant claims and conflicts within a budget.

### 12.1 ProjectionPacketV1

**Schema**: `sterling.projection_packet.v1`

A frozen, content-addressed packet containing the claims and conflicts selected for a specific task.

| Field | Type | Description |
|-------|------|-------------|
| `projection_id` | str | Unique identifier |
| `projection_content_hash` | str | Content hash ("sha256:...") |
| `schema_id` | str | Schema identifier |
| `task_fingerprint` | str | Hash of the task specification |
| `policy_id` | str | Policy used for projection |
| `conflict_policy_ids` | tuple[str, ...] | Conflict policies applied |
| `budget` | ProjectionBudgetV1 | Budget constraints |
| `metrics` | ProjectionMetricsV1 | Assembly metrics |
| `claim_slices` | tuple[ClaimSliceV1, ...] | Selected claim summaries |
| `conflict_slices` | tuple[ConflictSliceV1, ...] | Selected conflict summaries |

### 12.2 ProjectionBudgetV1

| Field | Type | Description |
|-------|------|-------------|
| `max_claim_slices` | int | Maximum claims to include |
| `max_conflict_slices` | int | Maximum conflicts to include |
| `max_assembly_time_ms` | int | Time budget for assembly |

Factory: `from_task_spec(task_spec, fallback)` extracts budget from task specification.

### 12.3 ProjectionMetricsV1

| Field | Type | Description |
|-------|------|-------------|
| `candidates_considered` | int | Total candidates evaluated |
| `claims_included` | int | Claims that made the cut |
| `conflicts_available` | int | Total conflicts found |
| `conflicts_included` | int | Conflicts included in packet |
| `indexed_retrieval_used` | bool | Whether index was used |
| `budget_exhausted` | bool | Whether budget was exhausted |
| `exhaustion_reason` | str? | Why budget was exhausted |
| `assembly_time_ms` | float | Actual assembly time |

### 12.4 ClaimSliceV1 and ConflictSliceV1

**ClaimSliceV1**: Summarizes a claim for inclusion in a projection packet. Fields: `claim_signature`, `schema_id`, `salience_score`, `salience_factors`, `inclusion_rationale`, `touched_conflict_ids`, `claim_ref`.

**ConflictSliceV1**: Summarizes a conflict for inclusion. Fields: `conflict_id`, `conflict_content_hash`, `schema_id`, `policy_id`, `salience_score`, `salience_factors`, `inclusion_rationale`, `claim_signatures`, `identity_key_roles`, `identity_key_values`, `conflict_reason`, `scope`.

---

## 13. Memory Certification (M4/M5)

Source: `core/memory/certification.py`

The certification system produces verifiable artifacts that prove a reasoning run was conducted correctly — or document why it failed.

### 13.1 ArtifactRefV1

Source: `core/memory/certification.py:30`

> **Note**: A separate `ArtifactRefV1` class exists in `core/induction/artifact_closure.py:90` for the induction pipeline. The two classes share the same name and purpose but are defined independently. The memory/certification `ArtifactRefV1` documented here is the one used throughout the SWM certification system. See also [Operator Induction Contract §8.2](operator_induction_contract_v1.md).

A frozen reference to any content-addressed artifact:

| Field | Type | Description |
|-------|------|-------------|
| `schema_id` | str | Schema identifier |
| `content_hash` | str | `"sha256:..."` (semantic hash) |
| `path` | str | Relative path or logical key |
| `byte_size` | int? | Optional size hint |
| `bytes_hash` | str? | Optional transport hash (pre-parse verification) |

### 13.2 MSRunManifestV1

**Schema**: `sterling.ms_run_manifest.v1`

Captures the full environment and inputs for a reasoning run.

| Field | Type | Description |
|-------|------|-------------|
| `run_id` | str | Human/logging identity (non-semantic) |
| `created_at` | str | ISO 8601 (non-semantic) |
| `repo` | dict | VCS info: commit, dirty flag |
| `environment` | dict | Python version, platform, deps_lock_hash |
| `memory_head` | dict | op_head_id, op_head_content_hash, ops_ledger_ref |
| `schema_closure_ref` | ArtifactRefV1 | All schemas used |
| `policy_closure_ref` | ArtifactRefV1 | All policies used |
| `index_snapshot_refs` | dict[str, ArtifactRefV1] | Index snapshots |
| `inputs` | dict | task_spec_hash, budget_hash |
| `anchor_snapshot_refs` | dict[str, ArtifactRefV1] | Anchor snapshots |

Non-semantic fields (excluded from content hash): `run_id`, `created_at`, `content_hash`.

### 13.3 MSReportV1

**Schema**: `sterling.ms_report.v1`

The result of a reasoning run, including integrity checks and outcome.

| Field | Type | Description |
|-------|------|-------------|
| `run_id` | str | Links to manifest |
| `op_head_id` | str | Registry ledger head |
| `rebuild` | dict | claims_hash, conflicts_hash, indexes_hash, projection_hash |
| `integrity_checks` | dict | schema_validation, index_integrity, conflict_integrity |
| `conflict_stats` | dict | total, by_reason, by_schema |
| `budget_stats` | dict | projection, packet (exhausted flags) |
| `outcome` | OutcomeV1 | Decision or certified failure |

### 13.4 OutcomeV1

Sum type for reasoning outcomes:

| Field | Type | Description |
|-------|------|-------------|
| `kind` | `"DECISION"` \| `"CERTIFIED_FAILURE"` | Outcome type |
| `decision_packet_ref` | ArtifactRefV1? | Present when kind="DECISION" |
| `certified_failure_ref` | ArtifactRefV1? | Present when kind="CERTIFIED_FAILURE" |
| `failure_codes` | tuple[str, ...] | Failure codes (e.g., `"MS5.BUDGET.EXHAUSTED"`) |

### 13.5 MSCertificateV1

**Schema**: `sterling.ms_certificate.v1`

The final certificate binding manifest + report + outcome.

| Field | Type | Description |
|-------|------|-------------|
| `certificate_id` | str | Content-addressed (`"msc_..."`) |
| `created_at` | str | ISO 8601 (non-semantic) |
| `run_manifest_ref` | ArtifactRefV1 | Links to manifest |
| `report_ref` | ArtifactRefV1 | Links to report |
| `outcome` | OutcomeV1 | Decision or failure |
| `projection_packet_ref` | ArtifactRefV1? | Optional projection reference |
| `spec_hash` | str? | Optional spec hash |
| `signature` | dict? | Optional cryptographic signature |

### 13.6 CertifiedFailureV1

**Schema**: `sterling.certified_failure.v1`

When reasoning fails, the failure itself is certified — not hidden.

| Field | Type | Description |
|-------|------|-------------|
| `failure_id` | str | Content-addressed (`"fail_..."`) |
| `created_at` | str | ISO 8601 (non-semantic) |
| `op_head_id` | str | Registry head at failure time |
| `policy_id` | str | Policy in effect |
| `failure_codes` | tuple[str, ...] | Sorted failure codes |
| `causal_facts` | dict | What caused the failure |
| `recommended_next_actions` | tuple[dict, ...] | Sorted recovery suggestions |

---

## 14. Memory Failure System

Source: `core/memory/failure.py`

Structured failure handling with verifiable outcomes. Failures are not exceptions — they are first-class artifacts.

### 14.1 Failure Constants

**FailureReason** (string constants, not enum):
- `MISSING_EVIDENCE`, `UNRESOLVED_CONFLICT`, `BUDGET_EXHAUSTED`
- `PARTIAL_OBSERVABILITY`, `POLICY_BLOCKED`, `TOOL_FAILURE`
- `ABSTRACTION_EXPANSION_EXHAUSTED`

**FailureSeverity** (string constants, not enum):
- `BLOCKING` — Cannot proceed with any action
- `DEGRADED` — Can proceed with reduced confidence
- `RECOVERABLE` — Can retry or request more info

### 14.2 CertifiedFailure (failure.py)

A frozen, content-addressed failure record:

| Field | Type | Description |
|-------|------|-------------|
| `failure_id` | str | Content-addressed ID |
| `failure_content_hash` | str | Deterministic hash |
| `task_spec` | dict | Task that failed |
| `failure_reason` | str | One of FailureReason constants |
| `failure_severity` | str | One of FailureSeverity constants |
| `explanation` | str | Human-readable explanation |
| `evidence_refs` | tuple[str, ...] | Evidence available at failure |
| `blocking_claims` | tuple[str, ...] | Claims that blocked progress |
| `blocking_conflicts` | tuple[str, ...] | Conflicts that blocked progress |
| `budget_at_failure` | dict | Budget state at failure |
| `memory_state_ref` | str | Memory state reference |
| `created_at` | str | ISO 8601 |
| `created_by_op_id` | str | Operator that detected failure |
| `recovery_options` | tuple[str, ...] | Suggested recovery actions |
| `can_retry` | bool | Whether retry is possible |

### 14.3 Key Functions

- `create_certified_failure(...)` — Factory with computed hash and ID
- `detect_failure(task_spec, registry, packet, policy)` — Detect failure conditions
- `check_budget_exhaustion(task_spec, registry, packet, policy)` — Check for budget exhaustion
- `get_recovery_suggestions(failure)` — Generate recovery suggestions

---

## 15. Memory Abstractions (L3 Compression)

Source: `core/memory/abstractions.py`, `core/memory/abstraction_expansion.py`

Memory abstractions compress sets of atomic claims into verifiable summaries. This is not lossy compression in the information-theoretic sense — every abstraction records exactly what was lost and provides a path back to the atomic sources.

### 15.1 MemoryAbstractionV1

**Schema**: `sterling.memory_abstraction.v1`

A frozen, content-addressed envelope that records the transformation from many atomic claims to one abstract claim.

| Field | Type | Description |
|-------|------|-------------|
| `kind` | `AbstractionKind` | `"cluster"`, `"summarize"`, or `"generalize"` |
| `source_claim_signatures` | `Tuple[str, ...]` | Sorted, unique signatures of source claims |
| `abstract_claim_signature` | `str` | Signature of the produced abstract ClaimInstance |
| `abstract_schema_id` | `str` | Schema of the abstract claim |
| `source_anchor_ids` | `Tuple[str, ...]` | Union of all anchors from source claims |
| `preserved_anchor_ids` | `Tuple[str, ...]` | Anchors retained in the abstract claim |
| `dropped_anchor_ids` | `Tuple[str, ...]` | Anchors lost — explicit loss disclosure |
| `source_conflict_ids` | `Tuple[str, ...]` | Active conflict IDs touching source claims |
| `conflict_posture` | `ConflictPosture` | `"none"`, `"contested"`, `"split"`, or `"deferred"` |
| `loss_metrics` | `Dict[str, float]` | Quantified loss: `anchor_retention`, `support_reduction`, `conflict_coverage` |
| `policy_id` | `str` | Abstraction policy identifier |
| `policy_hash` | `str` | Content hash of policy configuration |
| `source_ops_head` | `Optional[str]` | Registry ledger head at abstraction time (default None) |
| `schema_id` | `str` | `"sterling.memory_abstraction.v1"` |
| `version` | `str` | `"v1"` |
| `hash_ruleset_id` | `Optional[str]` | Hash ruleset used for content hashing (default None) |

**Non-semantic fields** (excluded from content hash): `abstraction_id`, `content_hash`, `created_at`, `created_by`.

**Semantic/non-semantic split**: The `SEMANTIC_PATHS`, `OPTIONAL_SEMANTIC_PATHS`, and `NON_SEMANTIC_PATHS` frozensets define which fields participate in the content hash. Development-mode guardrails (`assert_paths_present`, `assert_paths_absent`) verify this split at runtime.

**Content hash**: Computed via `to_semantic_dict()` → hash ruleset registry → `sha256_canonical()` fallback.

### 15.2 Loss Metrics

Source: `core/memory/abstractions.py:201-246`

Every abstraction quantifies its loss along three axes:

| Metric | Range | Meaning |
|--------|-------|---------|
| `anchor_retention` | 0.0–1.0 | Fraction of source anchors preserved in abstract claim |
| `support_reduction` | >= 1.0 | Number of source claims compressed into one |
| `conflict_coverage` | 0.0–1.0 | Whether conflicts touching sources are acknowledged |

**AbstractionPolicyV1** (source: `core/memory/policy.py:294-331`) sets verifiable limits:
- `min_anchor_retention`: 0.8 (default) — abstractions losing >20% of anchors are rejected.
- `require_conflict_disclosure`: `True` — abstractions touching conflicts must declare a posture.
- `auto_compress_threshold`: 0.2 — claims below this salience are candidates for abstraction.

### 15.3 Abstraction Expansion

Source: `core/memory/abstraction_expansion.py`

When abstraction density threatens task correctness, the reasoner can expand abstractions back to their atomic sources.

**Trigger conditions** (any triggers expansion):
1. `abstraction_ratio > expand_threshold` (diagnostic-based)
2. `requires_atomic_evidence == True` (task requirement)
3. Abstract claim participates in a conflict (conflict trigger)
4. User explicitly sets `view.abstractions.expand` (deterministic)

**Budget model**: `ExpansionBudget` tracks cost units:
- `read_abstraction_envelope`: 1 unit
- `load_source_claim`: 1 unit per claim
- `load_conflict`: 1 unit per conflict
- `recheck_budget`: separate budget for tool staleness rechecks (prevents verification storms)

**Ordering**: Conflict witnesses are loaded first (deterministic: sorted by signature then conflict IDs), then remaining sources. This ensures conflict-relevant evidence is never budget-starved.

**Cache**: `CachedExpansionResult` stores deterministic selections (signatures + metadata, not materialized claims). Cache key: `(abstraction_id, op_head_id, policy_hash, max_sources)`. Eviction is deterministic (lexicographically highest abstraction_id first). Maximum 256 entries.

### 15.4 AbstractionIndexV1

**Schema**: `sterling.abstraction_index.v1`

O(1) lookup index for abstraction artifacts. Tied to a specific `op_head_id` for deterministic invalidation.

| Field | Type | Description |
|-------|------|-------------|
| `op_head_id` | `str` | Registry ledger head at index creation time |
| `by_id` | `Dict[str, Dict[str, str]]` | `abstraction_id → {content_hash, path}` |
| `by_content_hash` | `Dict[str, str]` | Reverse lookup: `content_hash → abstraction_id` |
| `schema_id` | `str` | `"sterling.abstraction_index.v1"` |
| `schema_version` | `str` | `"1"` |
| `content_hash` | `str` | Content hash of the index itself |

**Invariant**: Index is never authoritative over hashes — loaded artifacts are always verified. Index invalidation is explicit: `op_head_id` mismatch with current registry head invalidates the entire index.

---

## 16. Agent Handover Protocol

Source: `core/memory/handover.py`

The handover protocol enables multi-agent state transfer with cryptographic integrity. Agents exchange "sealed envelopes" containing pinned ledger slices, claims, and evidence.

### 16.1 AgentIdentityV1

**Schema**: `sterling.agent_identity.v1`

| Field | Type | Description |
|-------|------|-------------|
| `agent_id` | `str` | Canonical immutable agent identifier |
| `key_id` | `str` | Versioned key identifier |
| `public_key` | `str` | Ed25519 public key (hex) |
| `valid_from` | `str` | ISO 8601 validity start |
| `valid_to` | `Optional[str]` | ISO 8601 validity end (None = no expiry) |
| `signer_agent_id` | `Optional[str]` | For delegated trust |
| `signer_signature` | `Optional[str]` | Attestation signature |

### 16.2 AgentHandoverV1

**Schema**: `sterling.agent_handover.v1`

The sealed envelope for multi-agent state transfer.

| Field | Type | Description |
|-------|------|-------------|
| `handover_id` | `str` | Unique UUID for idempotency |
| `source_agent_id` | `str` | Sending agent |
| `target_agent_id` | `str` | Receiving agent |
| `ops_ledger_ref` | `ArtifactRefV1` | Pinned contiguous ledger segment |
| `claims_ref` | `ArtifactRefV1` | Exported ClaimInstance set |
| `evidence_refs` | `Tuple[ArtifactRefV1, ...]` | Pinned evidence (ToolObservations) |
| `source_ms_report_ref` | `ArtifactRefV1` | Hash-lock of source's latest MSReport |
| `signature_key_id` | `str` | Binding to specific agent key |
| `source_signature` | `str` | Ed25519 signature of `handover_content_hash` |
| `scope_constraints` | `ScopeConstraintsV1` | What the recipient may do |
| `budget_allocation` | `float` | Budget units transferred |

**Non-semantic fields**: `source_signature`, `handover_content_hash`, `created_at`.

**Content hash**: Computed from semantic projection via hash ruleset registry, same pattern as MemoryAbstractionV1.

### 16.3 ScopeConstraintsV1

Defines what the recipient agent is allowed to do with imported state:

| Field | Type | Description |
|-------|------|-------------|
| `allowed_schema_ids` | `Tuple[str, ...]` | Which claim schemas may be accessed |
| `allowed_operations` | `Tuple[str, ...]` | `"read"`, `"challenge"`, `"refine"` |
| `reexport_allowed` | `bool` | Whether recipient may re-export to a third agent |
| `max_delegation_depth` | `int` | Maximum chain of delegations |

### 16.4 Provenance Laundering Detection

Source: `core/memory/handover.py:247-318`

`detect_provenance_laundering()` checks for four violation types:

| Code | Severity | Condition |
|------|----------|-----------|
| `MSX.SLICE.MISSING_OP` | CRITICAL | Claim references an op not in the ledger slice |
| `MSX.EVIDENCE.MISSING_ATOM` | CRITICAL | Tool-observation claim's evidence atom missing from `evidence_refs` |
| `MSX.BUDGET.MISMATCH` | CRITICAL | Consumed budget exceeds allocation |
| `MSX.CONSTRAINT.VIOLATION` | CRITICAL | Claim schema not in `allowed_schema_ids` |

All violations are recorded in `LaunderingReportV1` — an append-only audit artifact.

### 16.5 Handover Invariants

1. **Sealed envelope**: Once signed, a handover is immutable. The signature covers the `handover_content_hash`, which covers all semantic fields.
2. **Scope enforcement**: Recipients may only access schemas listed in `allowed_schema_ids` and perform operations listed in `allowed_operations`.
3. **Budget continuity**: Budget consumed in the ops ledger must not exceed `budget_allocation`.
4. **Provenance preservation**: Every claim must trace back to an op in the pinned ledger; every tool-observation claim must have its evidence in `evidence_refs`.
5. **No laundering**: Claims cannot be imported without their full provenance chain. `detect_provenance_laundering()` is the enforcement gate.

---

## 17. Relationship to Other Contracts

- **Linguistic IR Contract v0**: Defines the type system for Committed and Frontier partitions. This contract adds Shadow and Weak statuses plus lifecycle and projection rules on top of that type system.
- **Text I/O Contract v1**: Text is surface; IR is authority (INV-TIO-01 through INV-TIO-05). This contract governs the IR that Text I/O produces.
- **Core Constraints v1**: Meaning in IR+KG, not text (constraint 2). This contract formalizes how that meaning grows and decays.
- **Neural Usage Contract**: Embeddings and neural scores are advisory. This contract's Weak status and reinforcement scope rules enforce that boundary.
- **[Claim Schema System](claim_schema_system_v1.md)**: ClaimInstance is the atomic unit stored in SWM. MemoryAbstractionV1 compresses claim sets.
- **[Proof Evidence System](proof_evidence_system_v1.md)**: Agent handover signatures use Ed25519 from the proof system. ArtifactRefV1 is shared infrastructure.

## 18. Source File Index

| File | Purpose |
|------|---------|
| `core/linguistics/ir_v0/meaning_state.py` | MeaningStateDigest, state_id computation |
| `core/linguistics/ir_v0/episode_trace.py` | EpisodeTrace, OperatorStep, episode_id computation |
| `core/linguistics/ir_v0/context_ref.py` | ContextRef, canonical_digest computation |
| `core/linguistics/ir_v0/myelin_sheath.py` | MyelinSheath, CanonicalCorridor, CorridorStep |
| `core/linguistics/operators/v0/evict_ws.py` | EvictWS lifecycle operator |
| `core/linguistics/operators/v0/rehydrate_shadow.py` | RehydrateShadow lifecycle operator |
| `core/linguistics/operators/v0/verify_and_promote.py` | VerifyAndPromote lifecycle operator |
| `core/linguistics/operators/v0/compact_to_summary.py` | CompactToSummary lifecycle operator |
| `core/linguistics/operators/v0/apply_sheath.py` | ApplySheath lifecycle operator |
| `core/linguistics/operators/v0/promote_to_sheath.py` | PromoteToSheath lifecycle operator |
| `core/linguistics/operators/v0/make_mention.py` | MakeMention IR construction operator |
| `core/linguistics/operators/v0/make_entity_ref.py` | MakeEntityRef IR construction operator |
| `core/linguistics/operators/v0/make_predicate.py` | MakePredicate IR construction operator |
| `core/linguistics/operators/v0/make_proposition.py` | MakeProposition IR construction operator |
| `core/linguistics/operators/v0/attach_modality.py` | AttachModality IR construction operator |
| `core/linguistics/operators/v0/attach_negation.py` | AttachNegation IR construction operator |
| `core/linguistics/operators/v0/propose_coref.py` | ProposeCoref frontier operator |
| `core/linguistics/operators/v0/promote_coref_merge.py` | PromoteCorefMerge frontier operator |
| `core/linguistics/operators/v0/propose_discourse_link.py` | ProposeDiscourseLink frontier operator |
| `core/linguistics/operators/v0/promote_discourse_link.py` | PromoteDiscourseLink frontier operator |
| `core/memory/projection.py` | ProjectionPacketV1, ClaimSliceV1, ConflictSliceV1, budgets, metrics |
| `core/memory/certification.py` | MSRunManifestV1, MSReportV1, MSCertificateV1, CertifiedFailureV1, ArtifactRefV1, OutcomeV1 |
| `core/memory/failure.py` | CertifiedFailure, FailureReason/Severity constants, detect_failure |
| `core/memory/abstractions.py` | MemoryAbstractionV1, AbstractionIndexV1, loss metrics |
| `core/memory/abstraction_expansion.py` | Expansion triggers, budget model, cached expansion |
| `core/memory/handover.py` | AgentHandoverV1, AgentIdentityV1, ScopeConstraintsV1, laundering detection |
| `core/memory/policy.py` | VerificationPolicy, FailurePolicy, AbstractionPolicyV1 |
| `core/memory/canonical.py` | Hash ruleset registry, sha256_canonical |
| `core/memory/conflict.py` | Conflict detection and resolution |
| `core/memory/packet.py` | Packet assembly utilities |
| `core/memory/registry_store.py` | Registry persistence |
| `core/memory/registry_types.py` | Registry type definitions |
| `core/memory/registry_logic.py` | Registry business logic |
| `core/memory/registry.py` | Registry public API (re-exports) |
| `core/memory/schema_base.py` | Schema base classes |
| `core/memory/schema.py` | Schema definitions |
| `core/memory/meta_schemas.py` | Meta-schema definitions |
| `core/memory/claim.py` | ClaimInstance storage and retrieval |
| `core/memory/anchors.py` | Memory anchor points |
| `core/memory/certificate_builder.py` | Certificate construction helpers |
| `core/memory/concept_resolver.py` | Concept resolution |
| `core/memory/concept_store.py` | Concept storage |
| `core/memory/concepts.py` | Concept type definitions |
| `core/memory/expansion_policy.py` | Abstraction expansion policy |
| `core/memory/index_canonical.py` | Canonical index computation |
| `core/memory/landmark_extractor.py` | Landmark extraction from episodes |
| `core/memory/landmark_gate.py` | Landmark gating logic |
| `core/memory/landmark_node.py` | Landmark node type |
| `core/memory/landmark_registry.py` | Landmark registration and lookup |
| `core/memory/manifest_builder.py` | Run manifest construction helpers |
| `core/memory/packet_query.py` | Packet query interface |
| `core/memory/reasoner_integration.py` | Reasoner integration bridge |
| `core/memory/report_builder.py` | Report construction helpers |
| `core/memory/semantic_coverage.py` | Semantic coverage metrics |
| `core/memory/text_claim_schemas.py` | Text-domain claim schemas |
| `core/memory/text_claims.py` | Text-domain claim extraction |
| `core/memory/text_negative_controls.py` | Text-domain negative control generation |
| `core/memory/text_packet_consumer.py` | Text-domain packet consumer |
| `core/memory/text_projection_report.py` | Text-domain projection reporting |
| `core/memory/tool_loops.py` | Tool-loop memory integration |
| `core/memory/verifier.py` | Memory verification orchestrator |
| `core/memory/verifier_engine.py` | Verification engine |
| `core/memory/verifier_io.py` | Verifier I/O types |
| `core/memory/verifier_types.py` | Verifier type definitions |
| `core/linguistics/operators/v0/scope_attach_common.py` | Shared scope attachment infrastructure for lifecycle operators |

## 19. Version History

| Version | Date | Changes |
|---------|------|---------|
| v0.0.1 | 2026-01 | Initial specification |
| v0.0.2 | 2026-02 | Added §12 Memory Abstractions, §13 Agent Handover Protocol |
| v0.0.3 | 2026-02 | §6.4: Fixed MyelinSheath context_ref as Optional, added CorridorStep (distinct from OperatorStep). §7: Added 12 missing lifecycle operators (ApplySheath, PromoteToSheath, IR construction, frontier lifecycle). §12: Added Projection System (ProjectionPacketV1, budgets, metrics, claim/conflict slices). §13: Added Memory Certification (MSRunManifestV1, MSReportV1, MSCertificateV1, CertifiedFailureV1, ArtifactRefV1, OutcomeV1). §14: Added Memory Failure System (FailureReason, FailureSeverity, CertifiedFailure). §15.1: Added missing MemoryAbstractionV1 fields (source_ops_head, schema_id, version, hash_ruleset_id). §15.4: Added missing AbstractionIndexV1 fields (schema_id, schema_version, content_hash). §18: Expanded source file index from 5 to 33 files. |
