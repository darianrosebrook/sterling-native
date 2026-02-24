> **NOTE: Quarantined v1 canonical contract. Non-authoritative for Sterling Native. See [promotion criteria](README.md).**

# Claim and Schema System Contract

**This is a canonical definition. Do not edit without a version bump or CANONICAL-CHANGE PR label.**

**Version**: 1.1
**Date**: 2026-02-17
**Author**: @darianrosebrook
**Status**: Implemented

### Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.1 | 2026-02-17 | Fixed CanonicalSchemaEntry fields (§13), added index_policy to semantic_core exclusions (§3.2), added TextClaimConfig/ConfidenceContext/EvidenceRef dataclasses (§12), added text_claim_schemas.py and capability_claim_registry.py to source index (§16) |
| 1.0 | 2026-02-17 | Initial version |

---

## 1. Thesis

Sterling's claim system is a versioned, content-addressed semantic memory substrate. Claims are typed semantic assertions with evidence tracking, temporal/modal scoping, conflict detection, and deterministic canonicalization. The system implements a semantic ledger: claims are committed via operators, deduplicated by canonical signature, and assembled into bounded decision packets for reasoning.

This document specifies the claim lifecycle: schema definition, instance creation, validation, signature computation, conflict detection, delta processing, decision packet assembly, and certified failure handling.

---

## 2. SlotDef: Schema Role Definition

Source: `core/memory/schema_base.py:10-29`

```python
@dataclass(frozen=True)
class SlotDef:
    role: str                          # Slot name (e.g., "agent", "patient")
    type: str                          # EntityID, ConceptID, LiteralID, SchemaRef
    cardinality: str = "1"             # 1, 0..1, 1..*, 0..*
    resolver: Optional[str] = None     # Resolution strategy
    canonicalizer: Optional[str] = None # Canonicalization function
    indexable: bool = True             # Include in materialized indexes
    ordered: bool = False              # If True, list order preserved in signature
```

### 2.1 Cardinality

| Cardinality | Min | Max | Meaning |
|-------------|-----|-----|---------|
| `"1"` | 1 | 1 | Exactly one value required |
| `"0..1"` | 0 | 1 | Optional single value |
| `"1..*"` | 1 | unlimited | At least one value required |
| `"0..*"` | 0 | unlimited | Zero or more values |

### 2.2 Semantic Core

For hash computation, the semantic core includes: `role`, `type`, `cardinality`, `ordered`. Non-semantic metadata (`resolver`, `canonicalizer`, `indexable`) is excluded.

---

## 3. SchemaDef: Claim Type Definition

Source: `core/memory/schema_base.py:32-67`

```python
@dataclass(frozen=True)
class SchemaDef:
    schema_id: str                     # Globally unique identifier
    kind: str                          # ENTITY, RELATION, EVENT, STATE, GOAL, CONSTRAINT, META
    slots: List[SlotDef] = []
    constraints: List[str] = []        # Validation constraints
    evidence_policy: Dict[str, Any] = {
        "min_evidence": 1,
        "allowed_modalities": ["observation", "derivation"]
    }
    index_policy: Dict[str, Any] = {"primary_slots": []}
    migration_policy: Optional[Dict[str, Any]] = None
    external_anchors: Optional[List[str]] = None
    description: Optional[str] = None  # Non-normative metadata
```

### 3.1 Schema Kinds

| Kind | Purpose |
|------|---------|
| `ENTITY` | Named entity assertion (person, place, thing) |
| `RELATION` | Relationship between entities |
| `EVENT` | Temporal event assertion |
| `STATE` | State-of-affairs assertion |
| `GOAL` | Goal or intention |
| `CONSTRAINT` | Constraint or rule |
| `META` | Meta-level claim (system definition, model description) |

### 3.2 Schema Hash Computation

Source: `core/memory/schema_base.py:62-66`

```python
def get_hash(self) -> str:
    core = self.semantic_core()
    s = json.dumps(core, sort_keys=True)
    return hashlib.sha256(s.encode("utf-8")).hexdigest()
```

Semantic core for hashing includes: `schema_id`, `kind`, `slots` (sorted by role), `constraints` (sorted), `evidence_policy`, `migration_policy`. Excludes: `index_policy`, `description`, `external_anchors`.

### 3.3 Schema Registration Invariant

Source: `core/memory/registry_store.py:60-70`

Re-registering a schema with the same `schema_id` but different semantic hash raises `SchemaRegistrationError`. Identical re-registration is a no-op.

---

## 4. Scoping: Temporal and Modal

### 4.1 ModalScope

Source: `core/memory/schema_base.py:69-74`

```python
class ModalScope(str, Enum):
    ACTUAL = "actual"              # Holds in the real world
    HYPOTHETICAL = "hypothetical"  # Hypothetical scenario
    COUNTERFACTUAL = "counterfactual"  # Counter-to-fact scenario
```

**Governance**: Only `ACTUAL` claims participate in conflict detection. `HYPOTHETICAL` and `COUNTERFACTUAL` claims are isolated.

### 4.2 TemporalScope

Source: `core/memory/schema_base.py:77-108`

```python
@dataclass(frozen=True)
class TemporalScope:
    valid_from: Optional[str] = None   # ISO8601 or None (eternal start)
    valid_until: Optional[str] = None  # ISO8601 or None (eternal end)
    granularity: str = "instant"       # instant, day, epoch, eternal
```

**Overlap detection** (`schema_base.py:93-101`): `None` bounds are treated as unbounded. Two scopes overlap when `self_start <= other_end and other_start <= self_end`, using `""` for unbounded start and `"~"` for unbounded end (sorts after all ISO8601 strings).

---

## 5. ClaimInstance: Semantic Assertion

Source: `core/memory/claim.py:12-54`

```python
@dataclass
class ClaimInstance:
    schema_id: str
    slots: Dict[str, Any]
    epistemic_status: Literal["asserted", "hypothesis"] = "asserted"
    qualifiers: Dict[str, Any] = field(default_factory=dict)
    polarity: Literal["pos", "neg", "unk"] = "pos"
    support_set: List[str] = field(default_factory=list)  # Evidence Atom IDs
    derivation_op_id: Optional[str] = None
    canonical_signature: Optional[str] = None
    temporal_scope: Optional[TemporalScope] = None
    modal_scope: ModalScope = ModalScope.ACTUAL
```

### 5.1 Canonical Signature Computation

Source: `core/memory/claim.py:28-54`

The signature is a SHA-256 hash of the canonical JSON representation of the claim's **semantic content**:

1. Canonicalize slots based on schema (apply `canonicalizer` if defined).
2. Sort unordered lists for signature stability.
3. Build signature payload:

```python
{
    "schema_id": str,
    "slots": Dict[str, Any],       # Canonicalized, sorted keys
    "epistemic_status": str,
    "polarity": str,
    "qualifiers": Dict[str, Any],  # Sorted keys
    "modal_scope": str,
    "temporal_scope": Dict | None
}
```

4. Hash: `sha256(json.dumps(payload, sort_keys=True).encode("utf-8")).hexdigest()`

**Excluded from signature**: `support_set`, `derivation_op_id`, `canonical_signature`. These are non-semantic metadata.

**Key property**: Same semantic content always produces the same signature. Claims are content-addressed.

### 5.2 Validation

Source: `core/memory/registry_logic.py:66-117`

Validation checks at commit time:

1. **Schema existence**: `schema_id` must be registered.
2. **Required slots**: Slots with `min_cardinality > 0` must be present.
3. **Unknown slots**: Fail-closed — unknown slot names raise `SchemaValidationError`.
4. **Type checking**: Slot values must match declared types.
5. **Cardinality enforcement**: Values must satisfy min/max constraints.
6. **Evidence policy**: Asserted claims must satisfy `min_evidence`.
7. **Temporal validity**: `temporal_scope.is_valid()` must be True if present.
8. **Signature recomputation**: Canonical signature is ALWAYS recomputed — never trusted from input.

**Signature taint protection** (`registry_logic.py:111-117`):
```python
computed_sig = instance.compute_signature(schema)
if instance.canonical_signature and instance.canonical_signature != computed_sig:
    logger.warning(f"Overwriting tainted signature ...")
instance.canonical_signature = computed_sig
```

---

## 6. ClaimDelta: Semantic Changes

Source: `core/memory/claim.py:57-65`

```python
@dataclass(frozen=True)
class ClaimDelta:
    adds: List[ClaimInstance] = []
    updates: List[ClaimInstance] = []  # Preserve signature, change slots
    deletes: List[str] = []            # Canonical signatures to tombstone
    merges: List[Dict[str, Any]] = []  # Combine multiple claims
    splits: List[Dict[str, Any]] = []  # Split one claim into multiple
```

### 6.1 Processing Order

Source: `core/memory/registry_logic.py:609-645`

**Order**: Updates → Deletes → Merges → Splits → Adds

This ensures dependencies are satisfied: updates require existing claims; adds may reference merge/split results.

### 6.2 Operation Semantics

| Operation | Input | Effect |
|-----------|-------|--------|
| **Add** | `ClaimInstance` | Insert or deduplicate (union support sets) |
| **Update** | `ClaimInstance` with existing `canonical_signature` | Replace slots, re-index |
| **Delete** | `canonical_signature` | Tombstone (claim stays in ledger, marked deleted) |
| **Merge** | `{source_signatures, merged_claim}` | Delete sources, create merged claim, redirect map |
| **Split** | `{source_signature, split_claims}` | Delete source, create split claims, split map |

### 6.3 Deduplication

Source: `core/memory/registry_logic.py:634-645`

When adding a claim whose `canonical_signature` already exists, the existing claim's `support_set` is unioned with the new claim's support set. No duplicate claims are created.

---

## 7. SemanticOp: Ledger Entry

Source: `core/memory/claim.py:68-78`

```python
@dataclass(frozen=True)
class SemanticOp:
    op_id: str                     # Unique operation ID
    operator_id: str               # Operator that produced this change
    args: Dict[str, Any]           # Operator arguments
    delta: ClaimDelta              # Semantic changes
    timestamp: str                 # ISO8601 timestamp
    content_hash: str              # Deterministic hash of {operator_id, args, delta}
    support: List[str] = []        # Evidence Atom IDs
```

### 7.1 Content Hash Computation

Source: `core/memory/registry_logic.py:677-690`

```python
delta_dict = {
    "adds": sorted([inst.canonical_signature for inst in delta.adds]),
    "updates": sorted([inst.canonical_signature for inst in delta.updates]),
    "deletes": sorted(delta.deletes),
    "merges": sorted(merge_signatures),
    "splits": sorted(split_signatures),
}
content_payload = {"operator_id": operator_id, "args": args, "delta": delta_dict}
content_json = json.dumps(content_payload, sort_keys=True)
content_hash = hashlib.sha256(content_json.encode("utf-8")).hexdigest()
```

Operations are append-only. The ledger is immutable once written.

---

## 8. Conflict Detection

### 8.1 ConflictSet

Source: `core/memory/conflict.py:10-113`

```python
@dataclass(frozen=True)
class ConflictSet:
    conflict_id: str                   # Content-addressed ID
    conflict_content_hash: str         # sha256(canonical_json(content))
    schema_id: str                     # Claim schema family
    policy_id: str                     # Conflict policy applied
    identity_key_roles: Tuple[str, ...]  # Roles defining "same fact"
    identity_key_values: Tuple[str, ...]  # Canonical values for those roles
    claim_signatures: Tuple[str, ...]  # Signatures involved (sorted)
    conflict_reason: str               # polarity_mismatch, temporal_overlap
    scope: Dict[str, Any]              # Modal/temporal overlap region
    created_at: str                    # ISO8601
    created_by_op_id: str              # Semantic op that introduced the conflict
```

### 8.2 Detection Algorithm

Source: `core/memory/registry_logic.py:142-291`

**Trigger**: When a new claim is committed to the registry.

**Preconditions**:
- Claim must be `epistemic_status="asserted"` and `modal_scope=ACTUAL`.
- Schema must define at least one `indexable` slot.

**Steps**:
1. Build identity key from indexable slots (sorted).
2. Extract identity values from claim slots.
3. Find candidates: same `schema_id`, same identity values, asserted + actual.
4. Check conflict conditions:
   - **Polarity mismatch**: `new.polarity != existing.polarity`
   - **Temporal overlap**: `new.temporal_scope.overlaps(existing.temporal_scope)`
5. Create `ConflictSet` if conflict detected.
6. Update conflict indexes for O(1) lookup.

### 8.3 Conflict Indexes

Source: `core/memory/registry_store.py:41-42`

| Index | Type | Purpose |
|-------|------|---------|
| `_index_conflicts_by_sig` | `Dict[str, Set[str]]` | Claim signature → Conflict IDs |
| `_index_conflicts_by_identity` | `Dict[Tuple, Set[str]]` | Identity key → Conflict IDs |

---

## 9. Decision Packets

### 9.1 PacketBudget

Source: `core/memory/packet.py:32-38`

```python
@dataclass
class PacketBudget:
    max_claims: int = 100
    max_ops_fetched: int = 1000
    max_assembly_time_ms: int = 5000
```

### 9.2 PacketSlice

Source: `core/memory/packet.py:65-84`

```python
@dataclass
class PacketSlice:
    claim: ClaimInstance
    inclusion_rationale: str
    relevance_score: float = 1.0
    slice_kind: Literal["atomic", "abstract", "drilldown"] = "atomic"
    drilldown_parent_signature: Optional[str] = None
    drilldown_rank: Optional[int] = None
```

**Post-init invariants** (`packet.py:77-83`):
- `slice_kind="drilldown"` requires non-empty `drilldown_parent_signature`.
- `slice_kind="drilldown"` requires non-None `drilldown_rank`.

### 9.3 PacketMetrics

Source: `core/memory/packet.py:42-62`

```python
@dataclass
class PacketMetrics:
    claims_included: int = 0
    claims_considered: int = 0
    ops_fetched: int = 0
    assembly_time_ms: float = 0.0
    budget_exhausted: bool = False
    exhaustion_reason: Optional[str] = None   # "max_claims" | "max_assembly_time_ms"
    indexed_retrieval: bool = False
```

### 9.4 DecisionPacket

Source: `core/memory/packet.py:86-244`

```python
@dataclass
class DecisionPacket:
    packet_id: str
    task_spec: Dict[str, Any]
    slices: List[PacketSlice] = []
    conflicts: List[ConflictSet] = []
    metrics: PacketMetrics = field(default_factory=PacketMetrics)
    version: str = "1"
    diagnostics: Optional[PacketDiagnostics] = None
```

**Semantic paths** (included in packet hash): `schema_id`, `version`, `task_spec`, `slices`, `conflicts`, metrics fields (`claims_included`, `claims_considered`, `budget_exhausted`, `exhaustion_reason`, `indexed_retrieval`).

**Non-semantic paths** (excluded): `packet_id`, `metrics.ops_fetched`, `metrics.assembly_time_ms`, `diagnostics`.

### 9.5 META Claim Poisoning Prevention

Source: `core/memory/packet.py:556-564`

META claims are excluded from packets by default. Override requires either `task_spec.allow_meta=True` or `task_spec.schema_ids` explicitly including the META schema.

---

## 10. Projection and Salience

### 10.1 ProjectionPacketV1

Source: `core/memory/projection.py:84-200`

```python
@dataclass(frozen=True)
class ProjectionPacketV1:
    projection_id: str
    projection_content_hash: str
    schema_id: str = "sterling.projection_packet.v1"
    task_fingerprint: str
    policy_id: str
    conflict_policy_ids: Tuple[str, ...]
    budget: ProjectionBudgetV1
    metrics: ProjectionMetricsV1
    claim_slices: Tuple[ClaimSliceV1, ...]
    conflict_slices: Tuple[ConflictSliceV1, ...]
    schema_version: str = "1"
```

### 10.2 Salience Computation

Source: `core/memory/projection.py:259-302`

Salience factors:

| Factor | Range | Computation |
|--------|-------|-------------|
| `task_match` | 0.0–1.0 | 1.0 if `schema_id` in task_spec, else 0.0 |
| `trust_tier` | 0.8–1.0 | From handover metadata |
| `abstraction_status` | 1.0+ | `1.0 + (source_claim_count / 100.0)` for abstractions |
| `temporal_relevance` | 0.1–1.0 | Based on temporal overlap with task context |
| `support_mass` | 0.0–1.0 | `min(len(support_set) / 3.0, 1.0)` |
| `conflict_attention` | 0.0 or 1.0 | 1.0 if claim touches any conflict |

**Formula** (`projection.py:286-293`):
```python
salience_score = (
    task_match * trust_tier * abstraction_status *
    (0.5 + 0.5 * temporal_relevance) *
    (0.5 + 0.5 * support_mass) +
    conflict_attention
)
```

---

## 11. Certified Failure

Source: `core/memory/failure.py:38-125`

```python
@dataclass(frozen=True)
class CertifiedFailure:
    failure_id: str
    failure_content_hash: str
    task_spec: Dict[str, Any]
    failure_reason: str
    failure_severity: str
    explanation: str
    evidence_refs: Tuple[str, ...]
    blocking_claims: Tuple[str, ...]
    blocking_conflicts: Tuple[str, ...]
    budget_at_failure: Dict[str, Any]
    memory_state_ref: str
    created_at: str
    created_by_op_id: str
    recovery_options: Tuple[str, ...]
    can_retry: bool
```

### 11.1 Failure Reasons

Source: `core/memory/failure.py:18-27`

| Reason | Meaning |
|--------|---------|
| `MISSING_EVIDENCE` | Claims lack required evidence |
| `UNRESOLVED_CONFLICT` | Blocking conflicts exist |
| `BUDGET_EXHAUSTED` | Resource limits exceeded |
| `PARTIAL_OBSERVABILITY` | Incomplete information |
| `POLICY_BLOCKED` | Policy constraint violated |
| `TOOL_FAILURE` | Tool execution failed |
| `ABSTRACTION_EXPANSION_EXHAUSTED` | Abstraction expansion failed |

### 11.2 Failure Severities

Source: `core/memory/failure.py:30-35`

| Severity | Meaning |
|----------|---------|
| `BLOCKING` | Cannot proceed with any action |
| `DEGRADED` | Can proceed with reduced confidence |
| `RECOVERABLE` | Can retry or request more info |

### 11.3 Recovery Options

- `"add_evidence"`: Add evidence to support missing claims
- `"downgrade_to_hypothesis"`: Downgrade uncertain claims
- `"increase_budget"`: Increase budget limits
- `"narrow_scope"`: Narrow task scope

---

## 12. Text Claim Projection

Source: `core/memory/text_claims.py:146-200`

Claims can be deterministically projected from `UtteranceState` layers:

### 12.1 ClaimTier

```python
class ClaimTier(str, Enum):
    SEMANTIC = "semantic"  # Default: entities, events, relations
    PHRASE = "phrase"       # Opt-in: NPs, VPs, dependency facts
    TOKEN = "token"        # Debug only: POS, lemma, head
```

TOKEN tier requires explicit opt-in (`allow_token_tier=True`) to prevent budget explosion.

### 12.2 TextClaimConfig

```python
@dataclass(frozen=True)
class TextClaimConfig:
    tier: ClaimTier = ClaimTier.SEMANTIC
    max_claims: int = 100
    include_polarity: bool = True
    include_pn_type: bool = True
    include_coreference: bool = False
    allow_token_tier: bool = False              # Explicit gate for token tier
    confidence_context: Optional[ConfidenceContext] = None
```

### 12.3 ConfidenceContext

```python
@dataclass(frozen=True)
class ConfidenceContext:
    parser_trust_class: str = "UNKNOWN"
    base_confidence: float = 0.7
    coverage_pct: float = 100.0
    hole_pct: float = 0.0
```

Factory: `make_confidence_context(parser_trust_class, coverage_pct, hole_pct)` — looks up base confidence from `PARSER_TRUST_CONFIDENCE` mapping.

### 12.4 EvidenceRef

```python
@dataclass(frozen=True)
class EvidenceRef:
    atom_id: str                # Domain-separated: "TEXT_IR_V1|<digest>"
    note: Optional[str] = None
```

Pointer handle for claim support. Uses `TEXT_IR_V1|` prefix to prevent confusion with future EvidenceAtom IDs.

### 12.5 ClaimSourceKind

```python
class ClaimSourceKind(str, Enum):
    OBSERVED = "observed"  # Parser-produced, directly from text
    DERIVED = "derived"    # Operator-produced, transformation result
```

This prevents "provenance laundering" where derived semantics appear supported by original text.

### 12.6 Confidence Calibration

Source: `core/memory/text_claims.py:64-99`

Confidence is derived from parser trust class, not hardcoded:

```python
confidence = base_confidence * coverage_factor * (1.0 - hole_penalty)
```

| Parser Trust Class | Base Confidence |
|-------------------|-----------------|
| `NORMALIZER_ONLY` | 0.6 |
| `STRUCTURE_ONLY` | 0.8 |
| `SEMANTIC_PROVIDER` | 0.9 |
| `UNKNOWN` | 0.7 |

---

## 13. Canonical Schema Registry

Source: `core/contracts/schema_registry.py:13-192`

Sterling defines canonical schemas via `CanonicalSchemaEntry`:

```python
@dataclass(frozen=True)
class CanonicalSchemaEntry:
    schema_id: str              # Schema identifier (e.g., "sterling.text_intake_ir.v1")
    schema_version: str         # Semantic version (e.g., "1.0.0")
    owner: str                  # Owning module path (e.g., "core/text")
    canonical_doc: str          # Path to canonical documentation
    hash_critical: bool         # Whether included in closure hash
```

**Note**: Backward compatibility alias `SchemaEntry = CanonicalSchemaEntry` exists (renamed to avoid collision with `core.proofs.artifact_hashing.SchemaEntry`).

The `CANONICAL_SCHEMAS` list contains built-in schema definitions that ship with Sterling. Lookup functions:

- `list_schemas()`: Return all entries.
- `schema_index()`: Return `Dict[schema_id, CanonicalSchemaEntry]`.
- `get_schema(schema_id)`: Return entry or `None`.

---

## 14. Meta-Schemas

Source: `core/memory/meta_schemas.py`

Sterling defines foundational meta-schemas for self-description:

| Schema ID | Kind | Description |
|-----------|------|-------------|
| `sterling.meta.system_definition.v1` | META | Defines a system, component, or boundary |
| `sterling.meta.representation_model.v1` | META | Defines how meaning is represented |
| `sterling.meta.reasoning_model.v1` | META | Defines logic/regime for reasoning |
| `sterling.meta.auditability_model.v1` | META | Defines provenance and audit requirements |
| `sterling.meta.reversibility_model.v1` | META | Defines rollback mechanisms |
| `sterling.meta.bounding_model.v1` | META | Defines resource and scope boundaries |

---

## 15. Invariants Summary

1. **Content-addressed**: Claims are identified by their canonical signature (SHA-256 of semantic content).
2. **Signature recomputation**: Signatures are ALWAYS recomputed at validation — never trusted from input.
3. **Deduplication by signature**: Adding a claim with an existing signature unions support sets, never duplicates.
4. **Tombstone deletes**: Deleted claims remain in the ledger, marked as deleted.
5. **Schema immutability**: Re-registering a schema with different semantic hash is an error.
6. **Asserted evidence**: Asserted claims must satisfy `min_evidence` from schema's evidence policy.
7. **Modal isolation**: Only `ACTUAL` claims participate in conflict detection.
8. **META exclusion**: META claims are excluded from decision packets by default.
9. **Delta ordering**: Updates → Deletes → Merges → Splits → Adds.
10. **Deterministic ops**: SemanticOp `content_hash` is deterministic (same semantic changes = same hash).
11. **Drilldown completeness**: Drilldown slices must have parent signature and rank.
12. **Token tier gate**: TOKEN-tier claim projection requires explicit opt-in.

---

## 16. Source File Index

| File | Purpose |
|------|---------|
| `core/memory/schema_base.py` | SlotDef, SchemaDef, ModalScope, TemporalScope |
| `core/memory/claim.py` | ClaimInstance, ClaimDelta, SemanticOp |
| `core/memory/conflict.py` | ConflictSet, conflict content hashing |
| `core/memory/registry_store.py` | ClaimSchemaRegistry, materialized indexes |
| `core/memory/registry_logic.py` | Validation, conflict detection, delta processing |
| `core/memory/packet.py` | DecisionPacket, PacketSlice, PacketBudget, PacketMetrics |
| `core/memory/projection.py` | ProjectionPacketV1, salience computation |
| `core/memory/failure.py` | CertifiedFailure, failure reasons/severities |
| `core/memory/text_claims.py` | Text claim projection, ClaimTier, ConfidenceContext, TextClaimConfig |
| `core/memory/text_claim_schemas.py` | Text-specific SchemaDef registrations (entity, event, relation, polarity, token) |
| `core/memory/meta_schemas.py` | Meta-schema definitions |
| `core/memory/policy.py` | FailurePolicy, VerificationPolicy, AbstractionPolicyV1 |
| `core/memory/abstractions.py` | MemoryAbstractionV1, AbstractionIndexV1 |
| `core/contracts/schema_registry.py` | CanonicalSchemaEntry, CANONICAL_SCHEMAS |
| `core/domains/capability_claim_registry.py` | CapabilityClaimEntry, CapabilityClaimRegistry |

---

## 17. Relationship to Other Canonical Documents

| Document | Relationship |
|----------|-------------|
| [Governance Certification](governance_certification_contract_v1.md) | GovernanceContext provides strictness for claim validation |
| [Hashing Contracts](hashing_contracts_v1.md) | Claim signatures use SHA-256; canonical JSON serialization rules apply |
| [Proof Evidence System](proof_evidence_system_v1.md) | Evidence bundles reference claim support sets |
| [State Model](state_model_contract_v1.md) | UtteranceState layers are the source for text claim projection |
| [Semantic Working Memory](semantic_working_memory_contract_v0.md) | SWM manages the claim registry lifecycle |
