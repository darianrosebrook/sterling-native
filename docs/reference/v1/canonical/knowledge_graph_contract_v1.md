> **NOTE: Quarantined v1 canonical contract. Non-authoritative for Sterling Native. See [promotion criteria](README.md).**

# Knowledge Graph and Domain Registration Contract

**This is a canonical definition. Do not edit without a version bump or CANONICAL-CHANGE PR label.**

**Version**: 1.1
**Date**: 2026-02-17
**Author**: @darianrosebrook
**Status**: Implemented

### Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.1 | 2026-02-17 | Added TruthKG created_at field (§6.3), added WordNetPathSupport/WikidataStatementSupport dataclass fields (§7.3), added ClaimPattern/ClaimMatch dataclasses (§8.4-8.5), added missing KGRegistry methods (§5.2), added KGHashMismatchError and exceptions.py to source index |
| 1.0 | 2026-02-17 | Initial version |

---

## 1. Thesis

Sterling's knowledge graph infrastructure provides content-addressed, registry-managed knowledge bases with external sense bridges and typed entity schemas. The KG Registry owns KG handles globally so that state objects remain serializable and copyable with cost proportional to semantic deltas, not world size. The domain specification system (`core/domains/`) provides content-addressed capability declarations, conformance suites, and action surface contracts that govern what operators are allowed to do.

This document specifies the KG identity system (KGRef), the KG Registry lifecycle, entity/relation schemas, external sense bridges (WordNet, Wikidata), and the domain specification contracts (DomainSpecV1, ActionSurfaceV1, CapabilityDescriptorV1, ConformanceSuiteV1, DomainDeclarationV1).

---

## 2. KGRef: Content-Addressed Identity

Source: `core/kg/registry.py:64-128`

```python
@dataclass(frozen=True)
class KGRef:
    logical_name: str      # Human-readable (e.g., "wordnet", "wiki")
    schema_id: str         # KG structure ID (e.g., "sterling.wordnet_kg.v1")
    schema_version: str    # Schema version (e.g., "1")
    content_hash: str      # SHA256 of canonical artifact bytes
```

### 2.1 Identity Semantics

Identity is determined by `(schema_id, schema_version, content_hash)`. The `logical_name` is for human readability only and does NOT participate in equality or hashing.

**Invariants**:
- `content_hash` must be in format `sha256:<hex>` (64 hex chars after prefix).
- `schema_id` follows Sterling naming: `sterling.<kg_type>.v<N>`.
- `__hash__` and `__eq__` use the identity tuple, excluding `logical_name`.

### 2.2 Content Hash Computation

Source: `core/kg/registry.py:159-196`

```python
def compute_content_hash_from_bytes(artifact_bytes: bytes) -> str:
    return f"sha256:{hashlib.sha256(artifact_bytes).hexdigest()}"

def compute_content_hash_from_file(path: Path, chunk_size: int = 65536) -> str:
    # Streaming SHA256 for large files
```

Hash is computed from raw artifact bytes (not JSON-parsed content). Streaming reads prevent memory spikes on large files.

---

## 3. KGMutability

Source: `core/kg/registry.py:44-57`

```python
class KGMutability(str, Enum):
    SHARED_READ_ONLY = "shared_read_only"   # Shared across states, immutable
    OWNED_MUTABLE = "owned_mutable"         # Owned by single context, mutable
```

---

## 4. KGRegistration

Source: `core/kg/registry.py:135-151`

```python
@dataclass
class KGRegistration:
    kg_ref: KGRef
    kg_handle: Any             # The actual KG object (FullKG, TruthKG, etc.)
    registered_at: datetime
    mutability: KGMutability = KGMutability.SHARED_READ_ONLY
    metadata: Dict[str, Any] = field(default_factory=dict)
    node_count: Optional[int] = None
    edge_count: Optional[int] = None
```

---

## 5. KGRegistry: Context-Aware Registry

Source: `core/kg/registry.py:204-517`

### 5.1 Design Properties

- **Thread-safe**: Uses `RLock` for re-entrant calls.
- **Idempotent registration**: Same `(kg_ref, kg_handle)` pair is a no-op; different handle for same ref raises `KGAlreadyRegisteredError`.
- **Lazy validation**: Validated at execution boundaries, not construction time.
- **Strong references**: Explicit cleanup via `unregister()`.
- **Singleton + injection**: Supports both `KGRegistry.instance()` (ergonomics) and explicit context injection (tests/replay).

### 5.2 Key Methods

| Method | Purpose |
|--------|---------|
| `register(kg_ref, kg_handle, mutability, metadata)` | Register KG with pre-computed hash |
| `get(kg_ref) -> Optional[Any]` | Get handle, return None if not found |
| `get_or_raise(kg_ref) -> Any` | Get handle, raise `KGNotRegisteredError` if not found |
| `unregister(kg_ref) -> bool` | Remove KG from registry |
| `find_by_logical_name(name) -> list[KGRef]` | Convenience lookup by name |
| `validate(kg_ref) -> bool` | Check if registered with matching hash |
| `get_registration(kg_ref) -> Optional[KGRegistration]` | Get full registration entry with metadata |
| `list_registered() -> list[KGRef]` | List all registered KGRefs |
| `clear()` | Clear all registrations |
| `__len__()` | Number of registered KGs |
| `__contains__(kg_ref)` | Check if KGRef is registered |

### 5.3 Test Isolation

Source: `core/kg/registry.py:284-311`

```python
@classmethod
@contextmanager
def isolated_context(cls) -> Iterator["KGRegistry"]:
    # Saves singleton state, yields fresh registry, restores on exit
```

RAII-style cleanup ensures test isolation without leaked state.

---

## 6. Entity and Relation Types

Source: `core/kg/types.py`

### 6.1 Entity

Source: `core/kg/types.py:200-261`

```python
@dataclass
class Entity:
    entity_id: str
    name: str
    entity_type: str          # PERSON, ORGANIZATION, SOFTWARE, CONCEPT, etc.
    properties: Dict[str, Any] = field(default_factory=dict)
    aliases: List[str] = field(default_factory=list)
    senses: List[ExternalSense] = field(default_factory=list)
    schema_version: str = "entity-v2.0"
```

Convenience methods: `get_wikidata_qid()` and `get_wordnet_synset()` return the primary linked ID with confidence >= 0.9.

### 6.2 Relation

Source: `core/kg/types.py:269-302`

```python
@dataclass
class Relation:
    relation_id: str
    subject_id: str
    predicate: str
    object_id: str
    properties: Dict[str, Any] = field(default_factory=dict)
    confidence: float = 1.0
    wikidata_property: Optional[str] = None  # P-ID mapping
```

### 6.3 TruthKG Container

Source: `core/kg/types.py:310-387`

```python
@dataclass
class TruthKG:
    entities: Dict[str, Entity] = field(default_factory=dict)
    relations: List[Relation] = field(default_factory=list)
    version: str = "v2.0"
    created_at: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
```

Key methods: `add_entity()`, `add_relation()`, `get_entity()`, `get_relations_for_entity()`, `get_neighbors()`, `to_dict()`, `from_dict()`, `load_from_json()`, `save_to_json()`.

---

## 7. External Sense Bridges

Source: `core/kg/types.py:26-192`

### 7.1 KnowledgeBase

```python
class KnowledgeBase(str, Enum):
    WIKIDATA = "wikidata"
    WORDNET = "wordnet"
    DBPEDIA = "dbpedia"       # Reserved for future
```

### 7.2 ExternalSense

Source: `core/kg/types.py:34-87`

```python
@dataclass
class ExternalSense:
    kb: KnowledgeBase
    id: str                  # Q-ID for Wikidata, synset ID for WordNet
    confidence: float = 1.0  # [0.0, 1.0]
    source: str = "unknown"  # Provenance: "manual_kg_link_v1", etc.
    label: Optional[str] = None
    description: Optional[str] = None
```

**Design principle**: External senses are OPTIONAL — core training works without them. Bridges are explicit edges, not collapsed identities.

**Invariants**:
- `kb` must be a valid `KnowledgeBase` enum value.
- ID format: Wikidata `Q\d+` (e.g., "Q28865"), WordNet synset format.
- At most one sense per `(kb, entity)` pair should have `confidence == 1.0`.

### 7.3 External Support

Source: `core/kg/types.py:151-192`

```python
@dataclass
class ExternalSupport:
    wordnet_paths: List[WordNetPathSupport] = field(default_factory=list)
    wikidata_statements: List[WikidataStatementSupport] = field(default_factory=list)
    overall_support: Optional[Literal["positive", "contradict", "mixed", "missing"]] = None
    source: str = "unknown"
```

**WordNetPathSupport**:

```python
@dataclass
class WordNetPathSupport:
    from_id: str                    # Source synset ID
    to_id: str                      # Target synset ID
    relation_chain: List[str]       # e.g., ["hypernym", "instance_of"]
    support: Literal["positive", "contradict", "missing"] = "positive"
    path_length: Optional[int] = None
```

**WikidataStatementSupport**:

```python
@dataclass
class WikidataStatementSupport:
    subject: str                    # Q-ID
    property: str                   # P-ID (e.g., "P31" for instance_of)
    object: str                     # Q-ID or literal value
    support: Literal["positive", "contradict", "missing"] = "positive"
    property_label: Optional[str] = None  # Human-readable property name
```

---

## 8. Entity Schemas

Source: `core/kg/schemas.py`

### 8.1 Schema Types

```python
class PropertyType(Enum):
    STRING, INTEGER, FLOAT, BOOLEAN, DATE, ENTITY_REF, LIST

class ClaimType(Enum):
    PROPERTY_VALUE, PROPERTY_COMPARISON, RELATION_EXISTS,
    RELATION_NEGATION, TYPE_ASSERTION, ROLE_ASSERTION
```

### 8.2 PropertySpec

Source: `core/kg/schemas.py:60-78`

```python
@dataclass
class PropertySpec:
    name: str
    property_type: PropertyType
    aliases: List[str] = []
    description: str = ""
    min_value: Optional[float] = None
    max_value: Optional[float] = None
    target_types: List[str] = []
    inverse_of: Optional[str] = None
```

### 8.3 ClaimPattern

```python
@dataclass
class ClaimPattern:
    claim_type: ClaimType
    property_name: Optional[str] = None
    relation_predicate: Optional[str] = None
    patterns: List[str] = []       # Regex patterns for matching claim text
    keywords: List[str] = []       # Keywords indicating this claim type
    supports_negation: bool = True
```

### 8.4 ClaimMatch

```python
@dataclass
class ClaimMatch:
    matched: bool
    claim_type: Optional[ClaimType] = None
    property_name: Optional[str] = None
    claimed_value: Optional[Any] = None
    target_entity: Optional[str] = None
    is_negated: bool = False
    confidence: float = 0.0
```

### 8.5 EntitySchema (Abstract Base)

Source: `core/kg/schemas.py:116-147`

Subclasses define schemas for entity types:

| Schema | Type | Entity Types |
|--------|------|-------------|
| `PersonSchema` | `schema:Person` | PERSON |
| `OrganizationSchema` | `schema:Organization` | ORGANIZATION |
| `SoftwareSchema` | `schema:SoftwareApplication` | SOFTWARE, TOOL |
| `ProgrammingLanguageSchema` | `schema:ComputerLanguage` | PROGRAMMING_LANGUAGE |
| `PolicyTierSchema` | `sterling:PolicyTier` | POLICY_TIER |
| `ConceptSchema` | `schema:Concept` | CONCEPT, PROCESS |

Each schema provides:
- `properties` → `Dict[str, PropertySpec]`
- `claim_patterns` → `List[ClaimPattern]`
- `match_claim(claim_text, entity)` → `ClaimMatch`
- `evaluate_claim(match, entity, kg)` → `(support_score, contradict_score)`

---

## 9. Domain Specification System

Source: `core/domains/`

### 9.1 DomainSpecV1

Source: `core/domains/domain_spec_v1.py`

The domain spec system provides content-addressed contracts for domain capabilities:

**Content-addressed identity**: All domain artifacts use domain-separated SHA-256 hashing with canonical JSON. Metadata fields (name, description, timestamps) are excluded from hashes.

**Canonicalization prefixes** (domain-separated):

| Artifact | Prefix |
|----------|--------|
| DomainSpec | `domain_spec_canon/v1:` |
| CapabilityDescriptor | `capability_descriptor_canon/v1:` |
| PrimitiveSpec | `primitive_spec_canon/v1:` |
| ConformanceSuite | `conformance_suite_canon/v1:` |
| DomainDeclaration | `domain_declaration_canon/v1:` |

### 9.2 StateSchemaCommitV1

Source: `core/domains/domain_spec_v1.py:222-286`

```python
@dataclass(frozen=True)
class StateSchemaCommitV1:
    schema_id: str
    schema_version: str
    field_declarations: Tuple[FieldDeclaration, ...]
    canonical_projection_fields: FrozenSet[str]  # Fields in semantic hash
    invariant_names: Tuple[str, ...]             # Predicates that must hold
    invariant_dependencies: Tuple[InvariantDependency, ...]  # Explicit field deps
```

Declares "what a state looks like": field types, canonicalization rules, and invariant predicates.

### 9.3 SemanticProbe

Source: `core/domains/domain_spec_v1.py:289-333`

```python
@dataclass(frozen=True)
class SemanticProbe:
    probe_id: str
    probe_type: str        # "goal", "progress", "invariant"
    description: str
    deterministic: bool = True    # MUST be True for certification
    monotonic: Optional[bool] = None
    bounded: Optional[bool] = None
```

Deterministic predicates for testing "meaning changed". All probes MUST be deterministic for certification.

### 9.4 ActionSurfaceV1

Source: `core/domains/action_surface.py:72-200+`

```python
@dataclass(frozen=True)
class ActionSurfaceV1:
    allowed_op_types: FrozenSet[OpType]
    target_namespaces: FrozenSet[str]
    writable_fields: FrozenSet[str]       # Fully qualified: "namespace.field_name"
    readonly_fields: FrozenSet[str]        # Must never be modified
    max_ops_per_operator: Optional[int] = None
    schema_version: str = "action_surface/v1"
```

**OpType enum** (10 operation types):
`SET_SURFACE`, `SET_ENUM`, `INSERT_EDGE`, `REMOVE_EDGE`, `TRAVERSE`, `SET_FIELD`, `APPEND_LIST`, `REMOVE_LIST`, `CHECK_CONDITION`, `EMIT_RESULT`

**Invariants**:
- `readonly_fields ∩ writable_fields = ∅` (must be disjoint).
- Unknown `op_types` are rejected (fail-closed).
- Supports wildcard patterns: `"namespace.*"` matches any field in namespace.
- `is_operation_permitted(op_type, namespace, field)` → `bool`.

### 9.5 CapabilityDescriptorV1

Source: `core/domains/capability_descriptor.py:124-299`

```python
@dataclass(frozen=True)
class CapabilityDescriptorV1:
    claim_id: str                    # sha256(proof_fields)
    primitive_id: str                # e.g., "p01"
    contract_version: str            # e.g., "p01@1.0"
    domain_id: str
    conformance_suite_hash: str
    fixture_hashes: Tuple[str, ...] = ()
    results_hash: str = ""
    budget_declaration: Optional[BudgetDeclaration] = None
    determinism_class: DeterminismClass = DeterminismClass.FULLY_DETERMINISTIC
    supported_extensions: Tuple[str, ...] = ()
    schema_version: str = "capability_descriptor/v1"
```

**DeterminismClass**: `FULLY_DETERMINISTIC`, `SEED_DETERMINISTIC`, `NONDETERMINISTIC`.

**Post-init validation**: `claim_id` must match computed hash. Missing proof refs prevent certification (fail-closed).

### 9.6 ConformanceSuiteV1

Source: `core/domains/conformance_suite.py:83-150+`

```python
@dataclass(frozen=True)
class ConformanceSuiteV1:
    suite_id: str                    # sha256(proof_fields)
    primitive_id: str
    contract_version: str
    suite_impl_ref: str              # Content hash of suite source (Pivot 4)
    fixture_refs: Tuple[FixtureRef, ...]
    determinism_requirement: str
    expected_evidence_schemas: Tuple[EvidenceSchemaRef, ...]
    gate_ids: Tuple[str, ...]
    schema_version: str = "conformance_suite/v1"
```

**Pivot 4**: `suite_impl_ref` (content hash of suite source code) is included in `suite_id` hash computation, preventing "same metadata, different implementation" attacks.

### 9.7 DomainDeclarationV1

Source: `core/domains/domain_handshake.py:60-150+`

```python
@dataclass(frozen=True)
class DomainDeclarationV1:
    declaration_id: str              # sha256(proof_fields)
    domain_id: str
    primitive_claims: Tuple[PrimitiveClaimRef, ...]
    supported_extensions: Tuple[str, ...]
    budget_profile: Optional[BudgetDeclaration] = None
    schema_version: str = "domain_declaration/v1"
```

Long-lived capability commitment. Changes only on re-certification. Content-addressed via `declaration_id` derived from proof fields.

---

## 10. Invariants Summary

1. **Content-addressed KG identity**: KGRef uses `(schema_id, schema_version, content_hash)` — `logical_name` is cosmetic.
2. **Hash from bytes**: KG content hash is computed from raw artifact bytes, not JSON-parsed content.
3. **Idempotent registration**: Same `(ref, handle)` is a no-op; same ref with different handle is an error.
4. **Thread-safe registry**: All registry operations are protected by `RLock`.
5. **Fail-closed at boundaries**: `get_or_raise()` enforces KG availability at execution boundaries.
6. **Optional senses**: External sense bridges are optional — core training works without them.
7. **Disjoint read/write**: `ActionSurfaceV1.readonly_fields ∩ writable_fields = ∅`.
8. **Deterministic probes**: All `SemanticProbe` instances must have `deterministic=True` for certification.
9. **Content-addressed domains**: All domain artifacts (specs, descriptors, suites, declarations) derive identity from proof fields only; metadata excluded.
10. **Domain-separated hashing**: Each artifact type uses a distinct canonicalization prefix.
11. **Suite implementation binding**: ConformanceSuiteV1 binds `suite_impl_ref` to prevent implementation substitution.

---

## 11. Source File Index

| File | Purpose |
|------|---------|
| `core/kg/__init__.py` | Package exports |
| `core/kg/types.py` | Entity, Relation, TruthKG, ExternalSense, KnowledgeBase |
| `core/kg/registry.py` | KGRef, KGRegistration, KGRegistry, KGMutability |
| `core/kg/schemas.py` | EntitySchema, PropertySpec, ClaimPattern, concrete schemas |
| `core/kg/exceptions.py` | KGRegistryError, KGAlreadyRegisteredError, KGNotRegisteredError, KGHashMismatchError |
| `core/domains/domain_spec_v1.py` | DomainSpecV1, StateSchemaCommitV1, SemanticProbe, ProbeCommitV1 |
| `core/domains/action_surface.py` | ActionSurfaceV1, OpType |
| `core/domains/capability_descriptor.py` | CapabilityDescriptorV1, BudgetDeclaration, DeterminismClass |
| `core/domains/primitive_spec.py` | PrimitiveSpecV1, CertificationGateRef |
| `core/domains/conformance_suite.py` | ConformanceSuiteV1, FixtureRef, EvidenceSchemaRef |
| `core/domains/domain_handshake.py` | DomainDeclarationV1, PrimitiveClaimRef |

---

## 12. Relationship to Other Canonical Documents

| Document | Relationship |
|----------|-------------|
| [Hashing Contracts](hashing_contracts_v1.md) | KG content hashes use raw SHA-256; domain artifacts use domain-separated canonical JSON hashing |
| [Governance Certification](governance_certification_contract_v1.md) | GovernanceContext governs strictness for domain certification gates |
| [Proof Evidence System](proof_evidence_system_v1.md) | Conformance suites produce evidence bundles consumed by the proof pipeline |
| [State Model](state_model_contract_v1.md) | StateSchemaCommitV1 declares state shape; SemanticProbes test state transitions |
| [Claim Schema System](claim_schema_system_v1.md) | Entity schemas define claim patterns that map to claim verification |
