> **NOTE: Quarantined v1 canonical contract. Non-authoritative for Sterling Native. See [promotion criteria](README.md).**

# Linguistic IR Contract v0

**Schema**: `sterling.linguistic_ir.v0`
**Version**: `0.0.2`
**Canonicalization**: `ling/v0`
**Owner**: `core/linguistics`

## 1. Purpose and Trust Boundary

The Linguistic IR v0 is a four-partition typed intermediate representation for linguistic analysis. Each partition carries a different level of authority:

| Partition | Authority | Contains |
|-----------|-----------|----------|
| **Surface** | Observational | Document, Segment, Token, Mention |
| **Structure** | Observational | Syntax (from core.text.ir.SyntaxLayer) |
| **Committed** | Authoritative | EntityRef, Predicate, Proposition, ScopeOperator, DiscourseLink, Attribution, ValueNode |
| **Frontier** | Advisory | Hypothesis, Candidate, PromotionRecord |

- **Surface/Structure** are observational: they record what was parsed but carry no reasoning authority.
- **Committed** is the authoritative reasoning substrate: operators produce committed structure via typed patches with witnesses.
- **Frontier** is advisory: hypotheses are unresolved ambiguities that must be explicitly promoted to become committed.

## 2. Determinism and Canonicalization

### Content-Addressed IDs

All node IDs are content-addressed using `sha256` with a type prefix:

```
{prefix}_{sha256(canonical_parts)[:16]}
```

### Canonical JSON

Serialization uses deterministic JSON:
- `json.dumps(obj, sort_keys=True, separators=(",", ":"))`
- All tuples are sorted by primary ID before serialization (Invariant I9)
- Float values are quantized to 6 decimal places in hash-contributing contexts

### Content Hash

`LinguisticIR.compute_content_hash()` produces:
```
ling_ir:{sha256(canonical_json)[:32]}
```

The canonical dict excludes the Structure partition (observational only) but includes all other partitions.

## 3. Hole Semantics

Explicit holes (not yet implemented in v0) indicate regions where analysis was intentionally not performed. Rules:
- Operators MUST NOT introspect inside holes
- Holes are represented as explicit nodes, not silent omission
- A missing partition field (e.g., `document=None`) is not a hole — it means the partition is empty

## 4. Observation vs Audit Split for Digests

- `base_ir_digest` in `LinguisticDeltaPatchV0` is **audit-plane only** — used for replay verification
- Observation-plane serialization may omit digests per policy
- `patch_id` is computed from canonical ops + base + version, not from the patch dict itself

## 5. Patch Contract

### LinguisticDeltaPatchV0

**Schema**: `sterling.linguistic_delta_patch.v0` (version `0.0.1`)

Operations (`PatchOp`):
| OpKind | Description |
|--------|-------------|
| `add_node` | Add a node to a partition |
| `remove_node` | Remove a node (requires stronger witness justification) |
| `add_edge` | Add a typed edge (e.g., role, scope) |
| `remove_edge` | Remove a typed edge |
| `set_field` | Set a field on an existing node |
| `retire_hypothesis` | Move hypothesis from UNEXPLORED/EXPLORED to PROMOTED/REJECTED |
| `create_hypothesis` | Add a new hypothesis to the frontier |

### Canonicalization Rules

Ops are canonically sorted by:
1. `op_kind_rank` (add_node=0, remove_node=1, ..., create_hypothesis=6)
2. `stable_key(payload)` (deterministic JSON of payload)

This ensures the same set of operations always produces the same `patch_id`, regardless of insertion order.

## 6. Operator Contract

### Protocol

Every operator implements `LinguisticOperator`:
- `operator_id: str` — unique operator name
- `operator_impl_digest: str` — binds to actual code (prevents drift)
- `apply(ir, ctx, inputs) -> OperatorResult`

### Fail-Closed Semantics

If any precondition fails:
- `patch = None` (no mutation)
- `witness` contains the failing `PreconditionCheck` entries
- The caller MUST NOT apply a patch when `patch is None`

### OperatorWitnessV0

**Schema**: `sterling.operator_witness.v0` (version `0.0.1`)

Every operator application emits a witness containing:
- `operator_id` and `operator_impl_digest`
- `inputs` — the parameters provided
- `preconditions` — list of `PreconditionCheck(check_id, result, detail)`
- `anchors` — document spans that anchor surface-derived structure (mandatory for surface-derived nodes)
- `external_provenance` — references to external evidence (syntax layer, embedding model, etc.)
- `emitted_patch_digest` — digest of the patch produced (empty if preconditions failed)

## 7. Promotion Contract

### Default: 1 EntityRef per Mention

During ingestion, each mention gets its own EntityRef. Coreference is NOT committed at ingestion time. Instead:
1. `ProposeCoref` creates a `COREF` hypothesis in the frontier
2. `PromoteCorefMerge` promotes the hypothesis, merging mentions into a single EntityRef

### PromotionRecord

Every promotion produces:
- `prom_id` — content-addressed from hypothesis and candidate
- `hyp_id` — the hypothesis being promoted
- `chosen_candidate_id` — which candidate was selected
- `applied_operator_id` — which operator performed the promotion
- `delta_digest` — digest of the patch applied
- `witnesses` — additional witness references

### Invariants

- A promoted hypothesis MUST have `status=PROMOTED` and `promotion is not None`
- Hypothesis IDs are content-addressed from `(attached_to, kind, generator_id, canonical_candidates)`
- Candidate ordering is deterministic: `(score DESC, candidate_id ASC)`

## 8. Structural Invariants

The `LinguisticIR.validate()` method checks 9 invariants:

| # | Invariant | Severity |
|---|-----------|----------|
| I1 | Mention spans reference valid segment IDs in document | Error |
| I2 | EntityRef mention_ids all exist in mentions | Error |
| I3 | Predicate anchors reference valid segment IDs | Error |
| I4 | Proposition predicate_id exists in predicates | Error |
| I5 | RoleEdge target_ids exist in entity_refs, propositions, or value_nodes | Error |
| I6 | ScopeOperator target_prop_id exists in propositions | Error |
| I7 | Scope operators form a DAG (no cycles) | Error |
| I8 | No hypothesis ID collides with any committed partition ID | Error |
| I9 | All tuples sorted by their primary ID field (canonical ordering) | Error |
| I10 | No Committed node references a Shadow/Weak node (leak check) | Error |

## 9. v0 Operators

| # | Operator | Effect | Partition |
|---|----------|--------|-----------|
| 1 | MakeMention | add_node(Mention) | Surface |
| 2 | MakeEntityRef | add_node(EntityRef) | Committed |
| 3 | MakePredicate | add_node(Predicate) | Committed |
| 4 | MakeProposition | add_node(Proposition) + add_edge(role) | Committed |
| 5 | AttachNegation | add_node(ScopeOperator/NEGATION) | Committed |
| 6 | AttachModality | add_node(ScopeOperator/MODALITY) | Committed |
| 7 | ProposeCoref | create_hypothesis(COREF) | Frontier |
| 8 | PromoteCorefMerge | add_node(merged EntityRef) + retire_hypothesis | Committed+Frontier |
| 9 | ProposeDiscourseLink | create_hypothesis(DISCOURSE_LINK) | Frontier |
| 10 | PromoteDiscourseLink | add_node(DiscourseLink) + retire_hypothesis | Committed+Frontier |

## 10. Type System

Source: `core/linguistics/ir_v0/types.py`

### 10.1 Status Enums

**NodeStatus** (str, Enum): `COMMITTED`, `SHADOW`, `WEAK`

### 10.2 Linguistic Enums

| Enum | Values |
|------|--------|
| `MentionType` | NAME, PRONOUN, NOMINAL, OTHER |
| `PredKind` | EVENT, STATE, RELATION |
| `PropositionStatus` | ASSERTED, QUESTIONED, COMMANDED, HYPOTHETICAL |
| `ScopeKind` | NEGATION, MODALITY, QUANTIFIER, TEMPORAL, FOCUS, OTHER |
| `DiscourseRelation` | CAUSE, CONTRAST, ELABORATION, TEMPORAL_SEQUENCE, CONDITION, EXPLANATION, CONCESSION, OTHER |
| `AttitudeType` | SAY, BELIEVE, HEAR, INFER, QUOTE, REPORT, OTHER |
| `ValueType` | STRING, NUMBER, TIME, DATE, MEASURE, OTHER |
| `HypothesisKind` | COREF, ENTITY_GROUNDING, SENSE_GROUNDING, DISCOURSE_LINK, IMPLICIT_ARG, PARSE_ALT, OTHER |
| `HypothesisStatus` | UNEXPLORED, EXPLORED, PROMOTED, REJECTED, STALE |

### 10.3 Morphological Feature Enums

| Enum | Values |
|------|--------|
| `Number` | SINGULAR, PLURAL, DUAL, UNKNOWN |
| `Person` | FIRST, SECOND, THIRD, UNKNOWN |
| `Gender` | MASCULINE, FEMININE, NEUTER, COMMON, UNKNOWN |
| `Definiteness` | DEFINITE, INDEFINITE, UNKNOWN |
| `Tense` | PAST, PRESENT, FUTURE, UNKNOWN |
| `Aspect` | PERFECTIVE, IMPERFECTIVE, PROGRESSIVE, PERFECT, UNKNOWN |
| `Mood` | INDICATIVE, SUBJUNCTIVE, IMPERATIVE, UNKNOWN |
| `Voice` | ACTIVE, PASSIVE, MIDDLE, UNKNOWN |

### 10.4 Feature Payloads

**EntityFeatures** (frozen): `number`, `person`, `gender`, `definiteness` (all default UNKNOWN)

**PredFeatures** (frozen): `tense`, `aspect`, `mood`, `voice` (all default UNKNOWN)

### 10.5 Envelope Types

| Type | Fields |
|------|--------|
| `Span` | doc_id, start_char, end_char |
| `TokenSpan` | seg_id, start_tok, end_tok |
| `NodeRef` | id, node_type |
| `NodeEnvelope` | node_type, id, payload |
| `EdgeEnvelope` | edge_type, src (NodeRef), dst (NodeRef), label?, anchor?, features? |

### 10.6 Surface Partition Types

**Token** (frozen): `tok_idx`, `text`, `char_start`, `char_end`, `whitespace_after=""`, `lemma?`, `pos?`

**Segment** (frozen): `seg_id`, `doc_id`, `text`, `char_offset`, `tokens=()`, `kind="sentence"`, `speaker?`

**Document** (frozen): `doc_id`, `raw_text`, `segments=()`, `pipeline_id?`

**Mention** (frozen): `men_id`, `seg_id`, `span` (TokenSpan), `text`, `mention_type`, `head_lemma`, `entity_features?`, `node_status=COMMITTED`

### 10.7 Committed Partition Types

**EntityRef** (frozen): `ent_id`, `mention_ids=()`, `canonical_label=""`, `entity_features?`, `grounding?`, `node_status=COMMITTED`

**Predicate** (frozen): `pred_id`, `anchor` (TokenSpan), `lemma`, `pred_kind`, `pred_features?`, `sense?`, `node_status=COMMITTED`

**RoleEdge** (frozen): `role_name`, `target_id`, `anchor?`

**Proposition** (frozen): `prop_id`, `predicate_id`, `roles=()`, `scope_operator_ids=()`, `attribution_id?`, `status=ASSERTED`, `node_status=COMMITTED`

**ScopeOperator** (frozen): `scope_id`, `kind`, `target_prop_id`, `anchor?`, `payload={}`, `node_status=COMMITTED`

**DiscourseLink** (frozen): `link_id`, `relation`, `source_prop_id`, `target_prop_id`, `anchor?`, `node_status=COMMITTED`

**Attribution** (frozen): `attr_id`, `prop_id`, `source_ent_id`, `attitude`, `anchor?`, `node_status=COMMITTED`

**ValueNode** (frozen): `value_id`, `value_type`, `value`, `anchor?`, `node_status=COMMITTED`

## 11. LinguisticIR Container

Source: `core/linguistics/ir_v0/container.py`

### 11.1 Fields

```
LinguisticIR (frozen)
├── schema_id: str                    # "sterling.linguistic_ir.v0"
├── schema_version: str               # "0.0.1"
├── canonicalization_version: str      # "ling/v0"
│
├── document: Document?               # Surface partition
├── mentions: tuple[Mention, ...]     # Surface partition
├── syntax: Any?                      # Structure (observational, excluded from hash)
│
├── entity_refs: tuple[EntityRef, ...]       # Committed
├── predicates: tuple[Predicate, ...]        # Committed
├── propositions: tuple[Proposition, ...]    # Committed
├── scope_operators: tuple[ScopeOperator, ...] # Committed
├── discourse_links: tuple[DiscourseLink, ...] # Committed
├── attributions: tuple[Attribution, ...]    # Committed
├── value_nodes: tuple[ValueNode, ...]       # Committed
│
├── hypotheses: tuple[Hypothesis, ...]       # Frontier
└── _is_planner_view: bool                   # Operational tag (not hashed)
```

### 11.2 Projection Methods

- `to_planner_view()` — Returns Committed-only projection with I10 leak check. Sets `_is_planner_view=True`.
- `to_retrieval_view()` — Returns full IR (all partitions).
- `require_planner_view()` — Fail-closed guard: checks both tag AND structural content. Raises `ValueError` if non-Committed content present (prevents spoofing via manual tag construction).

### 11.3 Lookup Methods

`get_mention()`, `get_entity_ref()`, `get_predicate()`, `get_proposition()`, `get_hypothesis()`, `get_scope_operator()`, `get_discourse_link()`, `get_attribution()`, `get_value_node()` — all return Optional.

## 12. Working Set Budget and Retention

### 12.1 WSBudget

Source: `core/linguistics/ir_v0/budget.py`

| Profile | max_nodes | max_edges | max_frontier |
|---------|-----------|-----------|--------------|
| MINIMAL | 50 | 100 | 3 |
| STANDARD | 500 | 2000 | 10 |
| LARGE | 5000 | 20000 | 25 |

### 12.2 RetentionPolicy

Source: `core/linguistics/ir_v0/retention.py`

Default weights: `w_recency=1.0`, `w_frequency=0.5`, `w_certification=0.1`

Formula: `retention_score = w_recency * recency + w_frequency * frequency + w_certification * certification_level`

**RetentionState** (mutable): `recency`, `frequency`, `certification_level`

**RetentionTable**: Maps `node_id → RetentionState`. Method `scored_nodes(policy)` returns (node_id, score) sorted by score ascending (eviction candidates first).

### 12.3 PinSet

Source: `core/linguistics/ir_v0/pin_set.py`

Nodes in a PinSet (goal_nodes, inflight_nodes, corridor_nodes) cannot be evicted by EvictWS. Operational only — not hashed.

## 13. Acceptance Tests

The test suite (`tests/unit/linguistics/`) validates:

- **Structural**: All 10 invariants detected and reported (`test_ir_v0_container.py`)
- **Determinism**: Content hash stability, serialization roundtrips, float quantization (`test_ir_v0_determinism.py`)
- **Promotion**: Hypothesis lifecycle, PromotionRecord requirements (`test_ir_v0_frontier.py`)
- **Fail-closed**: Every operator returns `patch=None` on precondition failure (`test_ir_v0_operators.py`)
- **Backward compat**: TextIR adapter, UtteranceState property, schema registry (`test_ir_v0_backward_compat.py`)

## 14. Source File Index

| File | Defines |
|------|---------|
| `core/linguistics/ir_v0/types.py` | All enums, NodeType, feature payloads, envelope types, surface/committed/frontier dataclasses |
| `core/linguistics/ir_v0/container.py` | LinguisticIR container, validate(), projections, content hashing |
| `core/linguistics/ir_v0/frontier.py` | Hypothesis, Candidate, GeneratorRef, PromotionRecord, HypothesisKind/Status |
| `core/linguistics/ir_v0/patch_v0.py` | LinguisticDeltaPatchV0, PatchOp, OpKind, canonicalization ranking |
| `core/linguistics/ir_v0/witness_v0.py` | OperatorWitnessV0, PreconditionCheck, AnchorRef, ExternalProvenance |
| `core/linguistics/ir_v0/meaning_state.py` | MeaningStateDigest, compute_state_id, GENESIS_SENTINEL |
| `core/linguistics/ir_v0/episode_trace.py` | EpisodeTrace, OperatorStep, compute_episode_id |
| `core/linguistics/ir_v0/context_ref.py` | ContextRef, canonical_digest |
| `core/linguistics/ir_v0/myelin_sheath.py` | MyelinSheath, CanonicalCorridor, CorridorStep, compute_sheath_id |
| `core/linguistics/ir_v0/budget.py` | WSBudget, MINIMAL/STANDARD/LARGE profiles |
| `core/linguistics/ir_v0/retention.py` | RetentionPolicy, RetentionState, RetentionTable |
| `core/linguistics/ir_v0/pin_set.py` | PinSet, EMPTY_PIN_SET |

---

## Changelog

### v0.0.2 (2026-02-17)
- **§8**: Added I10 invariant (Committed→Shadow/Weak leak check)
- **§10**: Added complete type system — 18 enums, 2 feature payloads, 5 envelope types, 4 surface types, 8 committed types
- **§11**: Added LinguisticIR container fields, projection methods (to_planner_view, to_retrieval_view, require_planner_view), lookup methods
- **§12**: Added WSBudget (3 profiles), RetentionPolicy/State/Table, PinSet
- **§13**: Updated acceptance test invariant count (9→10)
- **§14**: Added source file index (12 files)
